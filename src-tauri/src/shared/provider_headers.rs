//! Validation for provider-owned upstream headers. Never include values in errors.
use crate::shared::error::{AppError, AppResult};
use axum::http::{HeaderName, HeaderValue};

pub(crate) fn is_protected(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    name.starts_with("x-aio-")
        || name.starts_with("sec-websocket-")
        || matches!(
            name.as_str(),
            "authorization"
                | "x-api-key"
                | "x-goog-api-key"
                | "x-goog-api-client"
                | "chatgpt-account-id"
                | "proxy-authorization"
                | "proxy-authenticate"
                | "host"
                | "content-length"
                | "content-encoding"
                | "transfer-encoding"
                | "connection"
                | "keep-alive"
                | "te"
                | "trailer"
                | "upgrade"
                | "x-trace-id"
                | "session-id"
                | "session_id"
                | "x-session-id"
                | "x-codex-turn-state"
                | "x-codex-turn-metadata"
        )
}

pub(crate) fn parse(name: &str, value: &str) -> AppResult<(HeaderName, HeaderValue)> {
    let invalid = |message| AppError::new("SEC_INVALID_INPUT", message);
    if name.bytes().any(|b| b.is_ascii_control() && b != b'\t') {
        return Err(invalid("custom header name contains control characters"));
    }
    let name = name.trim_matches([' ', '\t']);
    let name = HeaderName::from_bytes(name.as_bytes())
        .map_err(|_| invalid("custom header name is invalid"))?;
    if is_protected(name.as_str()) {
        return Err(invalid("custom header is managed by the gateway"));
    }
    // Validate before trimming: CR/LF at either end must never disappear silently.
    HeaderValue::from_str(value).map_err(|_| invalid("custom header value is invalid"))?;
    let value = value.trim_matches([' ', '\t']);
    if value.is_empty() {
        return Err(invalid("custom header value is required"));
    }
    let mut value =
        HeaderValue::from_str(value).map_err(|_| invalid("custom header value is invalid"))?;
    value.set_sensitive(true);
    Ok((name, value))
}
