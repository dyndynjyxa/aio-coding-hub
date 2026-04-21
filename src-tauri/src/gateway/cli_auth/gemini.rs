//! Gemini CLI authentication strategy.

use super::strategy::CliAuthStrategy;
use axum::http::{header, HeaderMap, HeaderValue};

pub(super) struct GeminiAuthStrategy;

impl CliAuthStrategy for GeminiAuthStrategy {
    fn cli_key_str(&self) -> &'static str {
        "gemini"
    }

    fn inject_api_key_auth(&self, headers: &mut HeaderMap, api_key: &str) {
        let trimmed = api_key.trim();

        // Detect OAuth access tokens: either bare `ya29.*` or JSON with `access_token`.
        let oauth_access_token = if trimmed.starts_with("ya29.") {
            Some(trimmed.to_string())
        } else if trimmed.starts_with('{') {
            serde_json::from_str::<serde_json::Value>(trimmed)
                .ok()
                .and_then(|v| {
                    v.get("access_token")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
        } else {
            None
        };

        if let Some(token) = oauth_access_token {
            let value = format!("Bearer {token}");
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.insert(header::AUTHORIZATION, header_value);
            }
            if !headers.contains_key("x-goog-api-client") {
                headers.insert(
                    "x-goog-api-client",
                    HeaderValue::from_static("GeminiCLI/1.0"),
                );
            }
        } else if let Ok(header_value) = HeaderValue::from_str(trimmed) {
            headers.insert("x-goog-api-key", header_value);
        }
    }

    fn ensure_required_headers(&self, _headers: &mut HeaderMap) {
        // Gemini has no additional required headers.
    }
}
