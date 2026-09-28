//! Sanitized fingerprint for the final per-attempt upstream request.

use axum::body::Bytes;
use axum::http::{header, HeaderMap, Method};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

const COMPONENT_MAX_BYTES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UpstreamRequestFingerprint {
    pub(super) key: u64,
    pub(super) debug: String,
}

pub(super) fn compute_upstream_request_fingerprint(
    method: &Method,
    url: &reqwest::Url,
    headers: &HeaderMap,
    body: &Bytes,
) -> UpstreamRequestFingerprint {
    let body_len = body.len();
    let body_hash = hash_u64(body.as_ref());
    let query = normalize_query(url.query()).unwrap_or_else(|| "-".to_string());
    let headers_debug = sanitized_headers_component(headers);
    let debug = format!(
        "v1|method={}|path={}|query={}|headers={headers_debug}|len={body_len}|body_hash={body_hash:016x}",
        bound_component(method.as_str()),
        bound_component(url.path()),
        bound_component(&query),
    );
    let key = hash_u64(debug.as_bytes());
    UpstreamRequestFingerprint { key, debug }
}

fn hash_u64(input: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn normalize_query(query: Option<&str>) -> Option<String> {
    let raw = query.map(str::trim).filter(|v| !v.is_empty())?;
    let raw_pairs: Vec<&str> = raw.split('&').filter(|part| !part.is_empty()).collect();
    if raw_pairs.is_empty() {
        return None;
    }

    let mut seen_keys: HashSet<&str> = HashSet::with_capacity(raw_pairs.len());
    let has_duplicate_keys = raw_pairs.iter().any(|part| {
        let key = part.split_once('=').map(|(k, _)| k).unwrap_or(part);
        !seen_keys.insert(key)
    });

    let mut pairs: Vec<String> = raw_pairs.into_iter().map(sanitized_query_part).collect();
    if !has_duplicate_keys {
        pairs.sort_unstable();
    }
    Some(pairs.join("&"))
}

fn sanitized_query_part(part: &str) -> String {
    if let Some((key, value)) = part.split_once('=') {
        return format!("{}=[value_len:{}]", bound_component(key), value.len());
    }
    format!("[bare_query_len:{}]", part.len())
}

fn bound_component(value: &str) -> String {
    if value.len() <= COMPONENT_MAX_BYTES {
        return value.to_string();
    }
    let mut end = 0usize;
    for (idx, ch) in value.char_indices() {
        let next = idx + ch.len_utf8();
        if next > COMPONENT_MAX_BYTES {
            break;
        }
        end = next;
    }
    format!(
        "{}...[len={},hash={:016x}]",
        &value[..end],
        value.len(),
        hash_u64(value.as_bytes())
    )
}

fn sanitized_headers_component(headers: &HeaderMap) -> String {
    let mut parts: Vec<String> = headers
        .iter()
        .map(|(name, value)| {
            let name = name.as_str();
            if value.is_sensitive() || is_sensitive_header(name) {
                return format!("{name}=[redacted]");
            }
            if is_identity_header(name) {
                let value = value.to_str().unwrap_or("<non-utf8>");
                return format!("{name}={}", bound_component(value));
            }
            name.to_string()
        })
        .collect();
    parts.sort();
    parts.join(",")
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "x-api-key"
            | "x-goog-api-key"
            | "cookie"
            | "set-cookie"
            | "chatgpt-account-id"
    )
}

fn is_identity_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "user-agent"
            | "originator"
            | "anthropic-version"
            | "anthropic-beta"
            | "x-stainless-os"
            | "x-stainless-runtime"
            | "x-stainless-arch"
            | "x-stainless-lang"
            | "x-stainless-package-version"
            | "x-goog-api-client"
            | "x-aio-gateway-forwarded"
    ) || lower == header::ACCEPT_ENCODING.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::upstream_identity;
    use axum::http::HeaderValue;

    #[test]
    fn fingerprint_redacts_credentials_and_keeps_identity_markers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        );
        headers.insert("x-api-key", HeaderValue::from_static("sk-secret"));
        headers.insert("cookie", HeaderValue::from_static("sid=cookie-secret"));
        headers.insert(
            "chatgpt-account-id",
            HeaderValue::from_static("acct-secret"),
        );
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static(upstream_identity::CODEX_CLI_USER_AGENT),
        );
        headers.insert(
            "originator",
            HeaderValue::from_static(upstream_identity::CODEX_CLI_ORIGINATOR),
        );
        headers.insert(
            "session_id",
            HeaderValue::from_static("01234567-89ab-cdef-0123-456789abcdef"),
        );
        headers.insert(
            "x-session-id",
            HeaderValue::from_static("abcdef01-2345-6789-abcd-ef0123456789"),
        );
        headers.insert(
            "x-aio-gateway-forwarded",
            HeaderValue::from_static("aio-coding-hub"),
        );
        headers.insert(
            "x-aio-upstream-meta-url",
            HeaderValue::from_static("https://internal.example/meta-secret"),
        );
        headers.insert("x-custom", HeaderValue::from_static("present"));
        let url = reqwest::Url::parse("https://example.test/v1/responses?b=2&a=1").unwrap();

        let fingerprint = compute_upstream_request_fingerprint(
            &Method::POST,
            &url,
            &headers,
            &Bytes::from_static(br#"{"input":"secret prompt"}"#),
        );

        assert!(fingerprint
            .debug
            .contains("query=a=[value_len:1]&b=[value_len:1]"));
        assert!(fingerprint.debug.contains("authorization=[redacted]"));
        assert!(fingerprint.debug.contains("x-api-key=[redacted]"));
        assert!(fingerprint.debug.contains("cookie=[redacted]"));
        assert!(fingerprint.debug.contains("chatgpt-account-id=[redacted]"));
        assert!(fingerprint.debug.contains(&format!(
            "user-agent={}",
            upstream_identity::CODEX_CLI_USER_AGENT
        )));
        assert!(fingerprint.debug.contains(&format!(
            "originator={}",
            upstream_identity::CODEX_CLI_ORIGINATOR
        )));
        assert!(fingerprint.debug.contains("session_id"));
        assert!(fingerprint.debug.contains("x-session-id"));
        assert!(fingerprint
            .debug
            .contains("x-aio-gateway-forwarded=aio-coding-hub"));
        assert!(fingerprint.debug.contains("x-aio-upstream-meta-url"));
        assert!(!fingerprint.debug.contains("secret prompt"));
        assert!(!fingerprint.debug.contains("sk-secret"));
        assert!(!fingerprint.debug.contains("cookie-secret"));
        assert!(!fingerprint.debug.contains("acct-secret"));
        assert!(!fingerprint.debug.contains("meta-secret"));
        assert!(!fingerprint
            .debug
            .contains("01234567-89ab-cdef-0123-456789abcdef"));
        assert!(!fingerprint
            .debug
            .contains("abcdef01-2345-6789-abcd-ef0123456789"));
    }

    #[test]
    fn fingerprint_does_not_log_query_values() {
        let headers = HeaderMap::new();
        let url = reqwest::Url::parse(
            "https://example.test/v1/models?key=secret-value&a=public&access_token=token-value&sig=signature-secret",
        )
        .unwrap();

        let fingerprint =
            compute_upstream_request_fingerprint(&Method::GET, &url, &headers, &Bytes::new());

        assert!(fingerprint
            .debug
            .contains("query=a=[value_len:6]&access_token=[value_len:11]&key=[value_len:12]&sig=[value_len:16]"));
        assert!(!fingerprint.debug.contains("secret-value"));
        assert!(!fingerprint.debug.contains("public"));
        assert!(!fingerprint.debug.contains("signature-secret"));
        assert!(!fingerprint.debug.contains("token-value"));
    }

    #[test]
    fn fingerprint_preserves_duplicate_query_order() {
        let headers = HeaderMap::new();
        let left = reqwest::Url::parse("https://example.test/v1?a=1&a=22").unwrap();
        let right = reqwest::Url::parse("https://example.test/v1?a=22&a=1").unwrap();

        let left =
            compute_upstream_request_fingerprint(&Method::GET, &left, &headers, &Bytes::new());
        let right =
            compute_upstream_request_fingerprint(&Method::GET, &right, &headers, &Bytes::new());

        assert_ne!(left.key, right.key);
    }

    #[test]
    fn fingerprint_does_not_log_bare_query_parts() {
        let headers = HeaderMap::new();
        let url = reqwest::Url::parse("https://example.test/v1?bare-secret-token").unwrap();

        let fingerprint =
            compute_upstream_request_fingerprint(&Method::GET, &url, &headers, &Bytes::new());

        assert!(fingerprint.debug.contains("query=[bare_query_len:17]"));
        assert!(!fingerprint.debug.contains("bare-secret-token"));
    }
}
