//! Usage: Claude (Anthropic) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use crate::gateway::upstream_identity;
use crate::shared::http_body::read_text_with_limit;
use axum::http::{HeaderMap, HeaderValue};
use chrono::DateTime;
use std::future::Future;
use std::pin::Pin;

pub(crate) struct ClaudeOAuthProvider {
    endpoints: OAuthEndpoints,
}

const CLAUDE_LIMITS_RESPONSE_BODY_LIMIT: usize = 1024 * 1024;

impl ClaudeOAuthProvider {
    pub(crate) fn new() -> Self {
        // Support custom OAuth client via env vars
        let client_id = std::env::var("AIO_CLAUDE_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string());
        let client_secret = std::env::var("AIO_CLAUDE_OAUTH_CLIENT_SECRET").ok();

        Self {
            endpoints: OAuthEndpoints {
                // Subscription (Pro/Max) login must go through the claude.ai OAuth app
                // (same as official Claude Code CLI and sub2api); the
                // platform.claude.com/oauth/authorize Console entry stopped redirecting
                // back to the localhost callback (~2026-07), hanging the login flow.
                auth_url: "https://claude.ai/oauth/authorize",
                token_url: "https://platform.claude.com/v1/oauth/token",
                client_id,
                client_secret,
                scopes: vec![
                    "org:create_api_key",
                    "user:profile",
                    "user:inference",
                    "user:sessions:claude_code",
                    "user:mcp_servers",
                    "user:file_upload",
                ],
                redirect_host: "localhost",
                callback_path: "/callback",
                default_callback_port: 54545,
            },
        }
    }
}

impl OAuthProvider for ClaudeOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "claude"
    }

    fn provider_type(&self) -> &'static str {
        "claude_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        "https://api.anthropic.com/v1"
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![("code", "true")]
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        // OAuth uses Bearer auth only — do NOT set x-api-key (that's for API key auth)
        insert_bearer_auth(headers, access_token, "claude oauth")?;

        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        // Ensure oauth-2025-04-20 is always present in anthropic-beta.
        // Merge with any existing beta flags from the client request.
        let existing_beta = headers
            .get("anthropic-beta")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let required_flags = [
            "claude-code-20250219",
            "oauth-2025-04-20",
            "interleaved-thinking-2025-05-14",
            "context-management-2025-06-27",
            "prompt-caching-scope-2026-01-05",
        ];

        let mut flags: Vec<&str> = if existing_beta.is_empty() {
            Vec::new()
        } else {
            existing_beta.split(',').map(|s| s.trim()).collect()
        };

        for flag in &required_flags {
            if !flags.iter().any(|f| f == flag) {
                flags.push(flag);
            }
        }

        let merged = flags.join(",");
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_str(&merged)
                .unwrap_or_else(|_| HeaderValue::from_static("oauth-2025-04-20")),
        );

        // Mimic Claude Code CLI User-Agent and stainless headers.
        headers.insert(
            "user-agent",
            HeaderValue::from_static(upstream_identity::CLAUDE_CODE_USER_AGENT),
        );

        headers.insert(
            "x-stainless-os",
            HeaderValue::from_static(upstream_identity::claude_stainless_os()),
        );
        headers.insert(
            "x-stainless-runtime",
            HeaderValue::from_static(upstream_identity::CLAUDE_STAINLESS_RUNTIME),
        );
        headers.insert(
            "x-stainless-arch",
            HeaderValue::from_static(upstream_identity::claude_stainless_arch()),
        );
        headers.insert(
            "x-stainless-lang",
            HeaderValue::from_static(upstream_identity::CLAUDE_STAINLESS_LANG),
        );
        headers.insert(
            "x-stainless-package-version",
            HeaderValue::from_static(upstream_identity::CLAUDE_STAINLESS_PACKAGE_VERSION),
        );
        Ok(())
    }

    fn fetch_limits(
        &self,
        client: &reqwest::Client,
        access_token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthLimitsResult, String>> + Send + '_>> {
        let token = access_token.to_string();
        let client = client.clone();
        Box::pin(async move {
            let resp = client
                .get("https://api.anthropic.com/api/oauth/usage")
                .header("Authorization", format!("Bearer {}", token))
                .header("anthropic-beta", "oauth-2025-04-20")
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| format!("claude limits fetch failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("claude limits fetch status: {}", resp.status()));
            }

            let body =
                read_text_with_limit(resp, CLAUDE_LIMITS_RESPONSE_BODY_LIMIT, "claude limits")
                    .await
                    .map_err(|e| format!("claude limits body read failed: {e}"))?;
            let mut json: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("claude limits parse failed: {e}"))?;
            // Convert ISO 8601 resets_at to Unix seconds for frontend countdown
            fn inject_reset_at_unix(json: &mut serde_json::Value, key: &str) {
                if let Some(window) = json.get_mut(key) {
                    if let Some(resets_at) = window.get("resets_at").and_then(|v| v.as_str()) {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(resets_at) {
                            if let Some(obj) = window.as_object_mut() {
                                obj.insert(
                                    "reset_at".to_string(),
                                    serde_json::json!(dt.timestamp()),
                                );
                            }
                        }
                    }
                }
            }
            inject_reset_at_unix(&mut json, "five_hour");
            inject_reset_at_unix(&mut json, "seven_day");

            Ok(OAuthLimitsResult {
                raw_json: Some(json),
                ..Default::default()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn endpoints_use_claude_ai_authorize_and_platform_token() {
        let provider = ClaudeOAuthProvider::new();
        let endpoints = provider.endpoints();

        // Authorize must stay on claude.ai: the platform.claude.com Console entry
        // stopped redirecting back to the localhost callback (login hangs).
        assert_eq!(endpoints.auth_url, "https://claude.ai/oauth/authorize");
        assert_eq!(
            endpoints.token_url,
            "https://platform.claude.com/v1/oauth/token"
        );
    }

    #[test]
    fn inject_upstream_headers_uses_centralized_identity_markers() {
        let provider = ClaudeOAuthProvider::new();
        let mut headers = HeaderMap::new();

        provider
            .inject_upstream_headers(&mut headers, "access-token")
            .expect("inject headers");

        assert_eq!(
            headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Bearer access-token")
        );
        assert_eq!(
            headers.get("user-agent").and_then(|v| v.to_str().ok()),
            Some(upstream_identity::CLAUDE_CODE_USER_AGENT)
        );
        assert_eq!(
            headers.get("x-stainless-os").and_then(|v| v.to_str().ok()),
            Some(upstream_identity::claude_stainless_os())
        );
        assert_eq!(
            headers
                .get("x-stainless-arch")
                .and_then(|v| v.to_str().ok()),
            Some(upstream_identity::claude_stainless_arch())
        );
        assert_eq!(
            headers
                .get("x-stainless-package-version")
                .and_then(|v| v.to_str().ok()),
            Some(upstream_identity::CLAUDE_STAINLESS_PACKAGE_VERSION)
        );
    }
}
