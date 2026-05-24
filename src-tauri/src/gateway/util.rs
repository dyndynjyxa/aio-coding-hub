use super::proxy::GatewayErrorCode;
use axum::http::{header, HeaderMap};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default hard cap on request body size read into memory. The gateway keeps a
/// full body buffer for failover and request transforms, so the default should
/// be high enough for normal AI CLI traffic while still preventing accidental
/// desktop-scale memory spikes. Override with `AIO_GATEWAY_MAX_REQUEST_BODY_MB`.
const DEFAULT_MAX_REQUEST_BODY_MB: usize = 128;
const MIN_REQUEST_BODY_MB: usize = 1;
const MAX_REQUEST_BODY_MB: usize = 500;

fn parse_request_body_limit_mb(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_REQUEST_BODY_MB)
        .clamp(MIN_REQUEST_BODY_MB, MAX_REQUEST_BODY_MB)
}

pub(super) fn max_request_body_bytes() -> usize {
    parse_request_body_limit_mb(
        std::env::var("AIO_GATEWAY_MAX_REQUEST_BODY_MB")
            .ok()
            .as_deref(),
    )
    .saturating_mul(1024 * 1024)
}

/// Diagnostic threshold (not a rejection limit). When a request body is larger
/// than this AND the `model` field cannot be inferred from body/query/path,
/// the gateway returns a 400 with a helpful message, because a missing model
/// on an oversized body is almost always an upstream-client bug (truncation,
/// streaming body, chunking misbehavior). Requests >= this size with a
/// resolvable model are forwarded normally.
///
/// Aligned with claude-code-hub's `LARGE_REQUEST_BODY_BYTES` heuristic.
pub(super) const LARGE_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;

pub(super) const MAX_INTROSPECTION_BODY_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_DEBUG_BODY_PREVIEW_BYTES: usize = 4 * 1024;
pub(super) const MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES: usize = 256;
const FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES: usize = 256;

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(super) fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn hash_u64_of_bytes(input: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn header_value_trimmed<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

pub(super) fn extract_idempotency_key_hash(headers: &HeaderMap) -> Option<u64> {
    for key in [
        "idempotency-key",
        "x-idempotency-key",
        "x-stainless-idempotency-key",
    ] {
        if let Some(value) = header_value_trimmed(headers, key) {
            return Some(hash_u64_of_bytes(value.as_bytes()));
        }
    }
    None
}

fn normalize_query_for_fingerprint(query: Option<&str>) -> Option<String> {
    let raw = query.map(str::trim).filter(|v| !v.is_empty())?;
    let mut pairs: Vec<&str> = raw.split('&').filter(|part| !part.is_empty()).collect();
    if pairs.is_empty() {
        return None;
    }

    let mut seen_keys: HashSet<&str> = HashSet::with_capacity(pairs.len());
    let has_duplicate_keys = pairs.iter().any(|part| {
        let key = part.split_once('=').map(|(k, _)| k).unwrap_or(part);
        !seen_keys.insert(key)
    });

    if !has_duplicate_keys {
        pairs.sort_unstable();
    }

    Some(pairs.join("&"))
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }

    let mut end = 0usize;
    for (idx, ch) in value.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    &value[..end]
}

fn fingerprint_debug_component(value: &str) -> Cow<'_, str> {
    if value.len() <= FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES {
        return Cow::Borrowed(value);
    }

    let hash = hash_u64_of_bytes(value.as_bytes());
    Cow::Owned(format!(
        "{}...[len={},hash={hash:016x}]",
        utf8_prefix(value, FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES),
        value.len()
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compute_request_fingerprint(
    cli_key: &str,
    method: &str,
    path: &str,
    query: Option<&str>,
    session_id: Option<&str>,
    requested_model: Option<&str>,
    idempotency_key_hash: Option<u64>,
    body_bytes: &[u8],
) -> (u64, String) {
    let body_len = body_bytes.len();
    let body_hash = hash_u64_of_bytes(body_bytes);
    let normalized_query = normalize_query_for_fingerprint(query);
    let session_for_fingerprint = if idempotency_key_hash.is_some() {
        None
    } else {
        session_id
    };
    let idem_hash = idempotency_key_hash
        .map(|v| format!("{v:016x}"))
        .unwrap_or_else(|| "-".to_string());
    let cli_debug = fingerprint_debug_component(cli_key);
    let method_debug = fingerprint_debug_component(method);
    let path_debug = fingerprint_debug_component(path);
    let query_debug = normalized_query
        .as_deref()
        .map(fingerprint_debug_component)
        .unwrap_or_else(|| Cow::Borrowed("-"));
    let session_debug = session_for_fingerprint
        .map(fingerprint_debug_component)
        .unwrap_or_else(|| Cow::Borrowed("-"));
    let model_debug = requested_model
        .map(fingerprint_debug_component)
        .unwrap_or_else(|| Cow::Borrowed("-"));

    let debug = format!(
        "v3|cli={}|method={}|path={}|query={}|session={}|model={}|idem_hash={idem_hash}|len={body_len}|body_hash={body_hash:016x}",
        cli_debug.as_ref(),
        method_debug.as_ref(),
        path_debug.as_ref(),
        query_debug.as_ref(),
        session_debug.as_ref(),
        model_debug.as_ref(),
    );

    let mut hasher = DefaultHasher::new();
    debug.hash(&mut hasher);
    (hasher.finish(), debug)
}

pub(super) fn compute_all_providers_unavailable_fingerprint(
    cli_key: &str,
    sort_mode_id: Option<i64>,
    method: &str,
    path: &str,
) -> (u64, String) {
    let mode = sort_mode_id
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let cli_debug = fingerprint_debug_component(cli_key);
    let mode_debug = fingerprint_debug_component(&mode);
    let method_debug = fingerprint_debug_component(method);
    let path_debug = fingerprint_debug_component(path);
    let debug = format!(
        "v2|gw_unavail|cli={}|mode={}|method={}|path={}",
        cli_debug.as_ref(),
        mode_debug.as_ref(),
        method_debug.as_ref(),
        path_debug.as_ref()
    );

    let mut hasher = DefaultHasher::new();
    debug.hash(&mut hasher);
    (hasher.finish(), debug)
}

fn is_gzip_encoded(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .any(|enc| enc.eq_ignore_ascii_case("gzip"))
        })
        .unwrap_or(false)
}

fn gunzip_with_limit(input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = decoder
            .read(&mut buf)
            .map_err(|e| format!("failed to gunzip request body: {e}"))?;
        if n == 0 {
            break;
        }
        if out.len().saturating_add(n) > max_output_bytes {
            return Err(format!(
                "request body gunzip exceeded limit: limit={max_output_bytes} bytes"
            ));
        }
        out.extend_from_slice(&buf[..n]);
    }

    Ok(out)
}

pub(super) fn body_for_introspection<'a>(
    headers: &HeaderMap,
    body_bytes: &'a [u8],
) -> Cow<'a, [u8]> {
    if !is_gzip_encoded(headers) {
        return Cow::Borrowed(body_bytes);
    }

    match gunzip_with_limit(body_bytes, MAX_INTROSPECTION_BODY_BYTES) {
        Ok(decoded) => Cow::Owned(decoded),
        Err(_) => Cow::Borrowed(body_bytes),
    }
}

pub(super) fn lossy_utf8_preview(bytes: &[u8], max_bytes: usize) -> String {
    let preview_len = bytes.len().min(max_bytes);
    let mut preview = String::from_utf8_lossy(&bytes[..preview_len]).to_string();
    if preview_len < bytes.len() {
        preview.push_str(&format!(
            "\n... [truncated at {preview_len}/{} bytes]",
            bytes.len()
        ));
    }
    preview
}

fn is_sensitive_header_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "set-cookie"
            | "x-api-key"
            | "api-key"
            | "x-goog-api-key"
            | "x-auth-token"
            | "x-csrf-token"
            | "x-xsrf-token"
            | "access-token"
            | "refresh-token"
    ) || name.contains("api-key")
        || name.contains("apikey")
        || name.contains("token")
        || name.contains("secret")
        || name.contains("password")
        || name.contains("auth-token")
        || name.contains("session-token")
}

pub(super) fn redacted_headers_for_debug(headers: &HeaderMap) -> String {
    let mut parts = Vec::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        let name = name.as_str();
        let value = if is_sensitive_header_name(name) {
            "[redacted]".to_string()
        } else {
            lossy_utf8_preview(value.as_bytes(), MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES)
        };
        parts.push(format!("{name}: {value}"));
    }
    format!("[{}]", parts.join(", "))
}

pub(crate) fn url_decode_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = bytes[i + 1];
                let lo = bytes[i + 2];
                let hex = |b: u8| -> Option<u8> {
                    match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    }
                };

                if let (Some(hi), Some(lo)) = (hex(hi), hex(lo)) {
                    out.push(hi * 16 + lo);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}

fn url_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len().saturating_mul(3));
    for b in input.as_bytes() {
        let c = *b as char;
        let is_unreserved = matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~');
        if is_unreserved {
            out.push(c);
            continue;
        }
        out.push('%');
        out.push_str(&format!("{:02X}", b));
    }
    out
}

fn sanitize_model(model: &str) -> Option<String> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    let model = if model.len() > 200 {
        model[..200].to_string()
    } else {
        model.to_string()
    };
    Some(model)
}

fn extract_model_from_query(query: &str) -> Option<String> {
    for part in query.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        if key != "model" {
            continue;
        }
        let decoded = url_decode_component(value);
        return sanitize_model(&decoded);
    }
    None
}

fn extract_model_from_path(path: &str) -> Option<String> {
    let needle = "/models/";
    let idx = path.find(needle)?;
    let rest = &path[idx + needle.len()..];
    if rest.is_empty() {
        return None;
    }

    let end = rest.find(['/', ':', '?']).unwrap_or(rest.len());
    sanitize_model(&rest[..end])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestedModelLocation {
    BodyJson,
    Query,
    Path,
}

#[derive(Debug, Clone)]
pub(super) struct RequestedModelInfo {
    pub(super) model: Option<String>,
    pub(super) location: Option<RequestedModelLocation>,
}

pub(super) fn infer_requested_model_info(
    forwarded_path: &str,
    query: Option<&str>,
    body_json: Option<&serde_json::Value>,
) -> RequestedModelInfo {
    if let Some(value) = body_json {
        if let Some(model) = value.get("model") {
            if let Some(s) = model.as_str() {
                let model = sanitize_model(s);
                return RequestedModelInfo {
                    location: model.as_ref().map(|_| RequestedModelLocation::BodyJson),
                    model,
                };
            }
            if let Some(obj) = model.as_object() {
                if let Some(s) = obj.get("name").and_then(|v| v.as_str()) {
                    let model = sanitize_model(s);
                    return RequestedModelInfo {
                        location: model.as_ref().map(|_| RequestedModelLocation::BodyJson),
                        model,
                    };
                }
                if let Some(s) = obj.get("id").and_then(|v| v.as_str()) {
                    let model = sanitize_model(s);
                    return RequestedModelInfo {
                        location: model.as_ref().map(|_| RequestedModelLocation::BodyJson),
                        model,
                    };
                }
            }
        }
    }

    if let Some(q) = query {
        if let Some(model) = extract_model_from_query(q) {
            return RequestedModelInfo {
                model: Some(model),
                location: Some(RequestedModelLocation::Query),
            };
        }
    }

    let model = extract_model_from_path(forwarded_path);
    RequestedModelInfo {
        location: model.as_ref().map(|_| RequestedModelLocation::Path),
        model,
    }
}

pub(crate) fn encode_url_component(input: &str) -> String {
    url_encode_component(input)
}

pub(super) fn new_trace_id() -> String {
    let ts = now_unix_seconds();
    let seq = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{ts}-{seq}")
}

pub(super) fn strip_hop_headers(headers: &mut HeaderMap) {
    headers.remove(header::CONNECTION);
    headers.remove("keep-alive");
    headers.remove("proxy-connection");
    headers.remove(header::PROXY_AUTHENTICATE);
    headers.remove(header::PROXY_AUTHORIZATION);
    headers.remove(header::TE);
    headers.remove(header::TRAILER);
    headers.remove(header::TRANSFER_ENCODING);
    headers.remove(header::UPGRADE);
}

pub(super) fn build_target_url(
    base_url: &str,
    forwarded_path: &str,
    query: Option<&str>,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|e| format!("{}: {e}", GatewayErrorCode::InvalidBaseUrl.as_str()))?;

    let base_path = url.path().trim_end_matches('/');
    let forwarded_path = if base_path.ends_with("/v1")
        && (forwarded_path == "/v1" || forwarded_path.starts_with("/v1/"))
    {
        forwarded_path.strip_prefix("/v1").unwrap_or(forwarded_path)
    } else if base_path.ends_with("/v1beta")
        && (forwarded_path == "/v1beta" || forwarded_path.starts_with("/v1beta/"))
    {
        forwarded_path
            .strip_prefix("/v1beta")
            .unwrap_or(forwarded_path)
    } else {
        forwarded_path
    };
    let mut combined_path = String::new();
    combined_path.push_str(base_path);
    combined_path.push_str(forwarded_path);

    if combined_path.is_empty() {
        combined_path.push('/');
    }
    if !combined_path.starts_with('/') {
        combined_path.insert(0, '/');
    }

    url.set_path(&combined_path);
    url.set_query(query);
    Ok(url)
}

/// Clear all authentication-related headers (fail-closed pattern).
/// Single source of truth for which headers carry credentials.
pub(super) fn clear_all_auth_headers(headers: &mut HeaderMap) {
    headers.remove(header::AUTHORIZATION);
    headers.remove("x-api-key");
    headers.remove("x-goog-api-key");
    headers.remove("x-goog-api-client");
}

pub(super) fn inject_provider_auth(cli_key: &str, api_key: &str, headers: &mut HeaderMap) {
    clear_all_auth_headers(headers);

    if let Some(strategy) = crate::gateway::cli_auth::global_cli_auth_registry().get(cli_key) {
        strategy.inject_api_key_auth(headers, api_key);
    }
}

pub(super) fn ensure_cli_required_headers(cli_key: &str, headers: &mut HeaderMap) {
    if let Some(strategy) = crate::gateway::cli_auth::global_cli_auth_registry().get(cli_key) {
        strategy.ensure_required_headers(headers);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compute_all_providers_unavailable_fingerprint, compute_request_fingerprint,
        inject_provider_auth, lossy_utf8_preview, normalize_query_for_fingerprint,
        parse_request_body_limit_mb, redacted_headers_for_debug, DEFAULT_MAX_REQUEST_BODY_MB,
        FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES, MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES,
        MAX_REQUEST_BODY_MB, MIN_REQUEST_BODY_MB,
    };
    use axum::http::{header, HeaderMap, HeaderValue};

    #[test]
    fn request_body_limit_uses_default_and_clamps_env_values() {
        assert_eq!(
            parse_request_body_limit_mb(None),
            DEFAULT_MAX_REQUEST_BODY_MB
        );
        assert_eq!(parse_request_body_limit_mb(Some("0")), MIN_REQUEST_BODY_MB);
        assert_eq!(
            parse_request_body_limit_mb(Some("9999")),
            MAX_REQUEST_BODY_MB
        );
        assert_eq!(parse_request_body_limit_mb(Some("64")), 64);
        assert_eq!(
            parse_request_body_limit_mb(Some("not-a-number")),
            DEFAULT_MAX_REQUEST_BODY_MB
        );
    }

    #[test]
    fn normalize_query_sorts_unique_key_pairs() {
        let normalized = normalize_query_for_fingerprint(Some("b=2&a=1&c=3"));
        assert_eq!(normalized.as_deref(), Some("a=1&b=2&c=3"));
    }

    #[test]
    fn normalize_query_keeps_order_when_duplicate_keys_exist() {
        let normalized = normalize_query_for_fingerprint(Some("a=2&a=1&b=3"));
        assert_eq!(normalized.as_deref(), Some("a=2&a=1&b=3"));
    }

    #[test]
    fn lossy_utf8_preview_truncates_without_stringifying_full_body() {
        assert_eq!(lossy_utf8_preview(b"hello", 16), "hello");

        let preview = lossy_utf8_preview(b"abcdef", 3);
        assert!(preview.starts_with("abc"));
        assert!(preview.contains("truncated at 3/6 bytes"));
        assert!(!preview.contains("def"));
    }

    #[test]
    fn redacted_headers_for_debug_masks_sensitive_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        );
        headers.insert("x-api-key", HeaderValue::from_static("sk-secret"));
        headers.insert("x-custom-token", HeaderValue::from_static("custom-secret"));
        headers.insert(header::COOKIE, HeaderValue::from_static("sid=secret"));
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let debug = redacted_headers_for_debug(&headers);

        assert!(debug.contains("authorization: [redacted]"));
        assert!(debug.contains("x-api-key: [redacted]"));
        assert!(debug.contains("x-custom-token: [redacted]"));
        assert!(debug.contains("cookie: [redacted]"));
        assert!(debug.contains("content-type: application/json"));
        assert!(!debug.contains("Bearer secret"));
        assert!(!debug.contains("sk-secret"));
        assert!(!debug.contains("custom-secret"));
        assert!(!debug.contains("sid=secret"));
    }

    #[test]
    fn redacted_headers_for_debug_truncates_large_non_sensitive_values() {
        let mut headers = HeaderMap::new();
        let long_value = "x".repeat(MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES + 16);
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_str(&long_value).expect("valid header value"),
        );

        let debug = redacted_headers_for_debug(&headers);

        assert!(debug.contains("user-agent: "));
        assert!(debug.contains(&format!(
            "truncated at {}/{} bytes",
            MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES,
            MAX_DEBUG_HEADER_VALUE_PREVIEW_BYTES + 16
        )));
    }

    #[test]
    fn fingerprint_ignores_query_pair_order_for_unique_keys() {
        let (left, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("model=a&stream=true"),
            Some("session-1"),
            Some("m1"),
            None,
            b"{}",
        );
        let (right, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("stream=true&model=a"),
            Some("session-1"),
            Some("m1"),
            None,
            b"{}",
        );

        assert_eq!(left, right);
    }

    #[test]
    fn fingerprint_preserves_duplicate_key_order() {
        let (left, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("tag=x&tag=y"),
            Some("session-1"),
            Some("m1"),
            None,
            b"{}",
        );
        let (right, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("tag=y&tag=x"),
            Some("session-1"),
            Some("m1"),
            None,
            b"{}",
        );

        assert_ne!(left, right);
    }

    #[test]
    fn fingerprint_ignores_session_id_when_idempotency_present() {
        let (left, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("model=a"),
            Some("session-a"),
            Some("m1"),
            Some(0x1111),
            b"{}",
        );
        let (right, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("model=a"),
            Some("session-b"),
            Some("m1"),
            Some(0x1111),
            b"{}",
        );

        assert_eq!(left, right);
    }

    #[test]
    fn fingerprint_keeps_session_id_when_idempotency_absent() {
        let (left, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("model=a"),
            Some("session-a"),
            Some("m1"),
            None,
            b"{}",
        );
        let (right, _) = compute_request_fingerprint(
            "claude",
            "POST",
            "/v1/messages",
            Some("model=a"),
            Some("session-b"),
            Some("m1"),
            None,
            b"{}",
        );

        assert_ne!(left, right);
    }

    #[test]
    fn fingerprint_debug_bounds_long_components_without_collapsing_identity() {
        let long_path = format!("/v1/messages/{}", "p".repeat(2048));
        let long_query_a = format!("q={}", "a".repeat(2048));
        let long_query_b = format!("q={}", "b".repeat(2048));
        let long_session = format!("session-{}", "s".repeat(2048));
        let long_model = format!("model-{}", "m".repeat(2048));

        let (left, left_debug) = compute_request_fingerprint(
            "claude",
            "POST",
            &long_path,
            Some(&long_query_a),
            Some(&long_session),
            Some(&long_model),
            None,
            b"{}",
        );
        let (right, right_debug) = compute_request_fingerprint(
            "claude",
            "POST",
            &long_path,
            Some(&long_query_b),
            Some(&long_session),
            Some(&long_model),
            None,
            b"{}",
        );

        assert_ne!(left, right);
        assert_ne!(left_debug, right_debug);
        assert!(left_debug.len() < FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES * 8);
        assert!(left_debug.contains("len="));
        assert!(left_debug.contains("hash="));
        assert!(!left_debug.contains(&"a".repeat(FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES + 1)));
    }

    #[test]
    fn unavailable_fingerprint_debug_bounds_long_path() {
        let long_path = format!("/v1/messages/{}", "p".repeat(4096));
        let (fingerprint, debug) =
            compute_all_providers_unavailable_fingerprint("claude", Some(42), "POST", &long_path);

        assert_ne!(fingerprint, 0);
        assert!(debug.len() < FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES * 3);
        assert!(debug.contains("len="));
        assert!(debug.contains("hash="));
        assert!(!debug.contains(&"p".repeat(FINGERPRINT_DEBUG_COMPONENT_MAX_BYTES + 1)));
    }

    #[test]
    fn inject_provider_auth_claude_uses_x_api_key_only() {
        let mut headers = HeaderMap::new();
        inject_provider_auth("claude", "sk-ant-test", &mut headers);

        assert!(headers.contains_key("x-api-key"));
        assert!(headers.contains_key("anthropic-version"));
        assert!(!headers.contains_key(header::AUTHORIZATION));
    }

    #[test]
    fn inject_provider_auth_codex_uses_authorization_bearer() {
        let mut headers = HeaderMap::new();
        inject_provider_auth("codex", "sk-openai-test", &mut headers);

        assert!(headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .starts_with("Bearer "));
        assert!(!headers.contains_key("x-api-key"));
    }
}
