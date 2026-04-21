//! Claude CLI authentication strategy.

use super::strategy::CliAuthStrategy;
use axum::http::{HeaderMap, HeaderValue};

pub(super) struct ClaudeAuthStrategy;

impl CliAuthStrategy for ClaudeAuthStrategy {
    fn cli_key_str(&self) -> &'static str {
        "claude"
    }

    fn inject_api_key_auth(&self, headers: &mut HeaderMap, api_key: &str) {
        if let Ok(header_value) = HeaderValue::from_str(api_key) {
            headers.insert("x-api-key", header_value);
        }
        if !headers.contains_key("anthropic-version") {
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
    }

    fn ensure_required_headers(&self, headers: &mut HeaderMap) {
        if !headers.contains_key("anthropic-version") {
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
    }
}
