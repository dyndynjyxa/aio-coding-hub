use axum::http::{header, HeaderMap, HeaderValue};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
pub(super) const MAX_INTROSPECTION_BODY_BYTES: usize = 2 * 1024 * 1024;

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
    let idem_hash = idempotency_key_hash
        .map(|v| format!("{v:016x}"))
        .unwrap_or_else(|| "-".to_string());

    let debug = format!(
        "v2|cli={cli_key}|method={method}|path={path}|query={}|session={}|model={}|idem_hash={idem_hash}|len={body_len}|body_hash={body_hash:016x}",
        query.unwrap_or("-"),
        session_id.unwrap_or("-"),
        requested_model.unwrap_or("-"),
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
    let debug = format!("v1|gw_unavail|cli={cli_key}|mode={mode}|method={method}|path={path}");

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

fn url_decode_component(input: &str) -> String {
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

pub(super) fn encode_url_component(input: &str) -> String {
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
    let mut url = reqwest::Url::parse(base_url).map_err(|e| format!("GW_INVALID_BASE_URL: {e}"))?;

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

pub(super) fn inject_provider_auth(cli_key: &str, api_key: &str, headers: &mut HeaderMap) {
    headers.remove(header::AUTHORIZATION);
    headers.remove("x-api-key");
    headers.remove("x-goog-api-key");
    headers.remove("x-goog-api-client");

    match cli_key {
        "codex" => {
            let value = format!("Bearer {api_key}");
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.insert(header::AUTHORIZATION, header_value);
            }
        }
        "claude" => {
            let value = format!("Bearer {api_key}");
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.insert(header::AUTHORIZATION, header_value);
            }
            if let Ok(header_value) = HeaderValue::from_str(api_key) {
                headers.insert("x-api-key", header_value);
            }
            if !headers.contains_key("anthropic-version") {
                headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            }
        }
        "gemini" => {
            let trimmed = api_key.trim();
            let oauth_access_token = if trimmed.starts_with("ya29.") {
                Some(trimmed.to_string())
            } else if trimmed.starts_with('{') {
                serde_json::from_str::<serde_json::Value>(trimmed)
                    .ok()
                    .and_then(|v| v.get("access_token").and_then(|v| v.as_str()).map(str::to_string))
            } else {
                None
            };

            if let Some(token) = oauth_access_token {
                let value = format!("Bearer {token}");
                if let Ok(header_value) = HeaderValue::from_str(&value) {
                    headers.insert(header::AUTHORIZATION, header_value);
                }
                if !headers.contains_key("x-goog-api-client") {
                    headers.insert("x-goog-api-client", HeaderValue::from_static("GeminiCLI/1.0"));
                }
            } else if let Ok(header_value) = HeaderValue::from_str(trimmed) {
                headers.insert("x-goog-api-key", header_value);
            }
        }
        _ => {}
    }
}

pub(super) fn ensure_cli_required_headers(cli_key: &str, headers: &mut HeaderMap) {
    if cli_key == "claude" && !headers.contains_key("anthropic-version") {
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    }
}
