//! Codex CLI authentication strategy.

use super::strategy::CliAuthStrategy;
use axum::http::{header, HeaderMap, HeaderValue};

pub(super) struct CodexAuthStrategy;

impl CliAuthStrategy for CodexAuthStrategy {
    fn cli_key_str(&self) -> &'static str {
        "codex"
    }

    fn inject_api_key_auth(&self, headers: &mut HeaderMap, api_key: &str) {
        let value = format!("Bearer {api_key}");
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            headers.insert(header::AUTHORIZATION, header_value);
        }
    }

    fn ensure_required_headers(&self, _headers: &mut HeaderMap) {
        // Codex has no additional required headers.
    }
}
