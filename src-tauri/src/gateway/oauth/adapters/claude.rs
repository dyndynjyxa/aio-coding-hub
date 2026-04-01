//! Usage: Claude (Anthropic) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use axum::http::{HeaderMap, HeaderValue};
use std::future::Future;
use std::pin::Pin;

pub(crate) struct ClaudeOAuthProvider {
    endpoints: OAuthEndpoints,
}

impl ClaudeOAuthProvider {
    pub(crate) fn new() -> Self {
        // Support custom OAuth client via env vars
        let client_id = std::env::var("AIO_CLAUDE_OAUTH_CLIENT_ID")
            .unwrap_or_else(|_| "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string());
        let client_secret = std::env::var("AIO_CLAUDE_OAUTH_CLIENT_SECRET").ok();

        Self {
            endpoints: OAuthEndpoints {
                auth_url: "https://platform.claude.com/oauth/authorize",
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

        // Mimic Claude Code CLI User-Agent and stainless headers
        headers.insert("user-agent", HeaderValue::from_static("claude-code/2.1.45"));

        // Use actual OS instead of hardcoded Windows
        let os = if cfg!(target_os = "windows") {
            "Windows"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        };
        headers.insert("x-stainless-os", HeaderValue::from_static(os));
        headers.insert("x-stainless-runtime", HeaderValue::from_static("node"));
        headers.insert("x-stainless-arch", HeaderValue::from_static("x64"));
        headers.insert("x-stainless-lang", HeaderValue::from_static("js"));
        headers.insert(
            "x-stainless-package-version",
            HeaderValue::from_static("0.52.0"),
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

            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("claude limits parse failed: {e}"))?;

            Ok(OAuthLimitsResult {
                raw_json: Some(json),
                ..Default::default()
            })
        })
    }
}
