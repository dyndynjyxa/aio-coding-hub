//! Usage: Codex (OpenAI / ChatGPT) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use crate::shared::http_body::read_text_with_limit;
use axum::http::{HeaderMap, HeaderValue};
use std::future::Future;
use std::pin::Pin;

pub(crate) struct CodexOAuthProvider {
    endpoints: OAuthEndpoints,
}

const CODEX_LIMITS_RESPONSE_BODY_LIMIT: usize = 1024 * 1024;

impl CodexOAuthProvider {
    pub(crate) fn new() -> Self {
        Self {
            endpoints: OAuthEndpoints {
                auth_url: "https://auth.openai.com/oauth/authorize",
                token_url: "https://auth.openai.com/oauth/token",
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann".to_string(),
                client_secret: None,
                scopes: vec![
                    "openid",
                    "profile",
                    "email",
                    "offline_access",
                    "api.connectors.read",
                    "api.connectors.invoke",
                ],
                redirect_host: "localhost",
                callback_path: "/auth/callback",
                default_callback_port: 1455,
            },
        }
    }
}

impl OAuthProvider for CodexOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "codex"
    }

    fn provider_type(&self) -> &'static str {
        "codex_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        "https://chatgpt.com/backend-api/codex"
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("id_token_add_organizations", "true"),
            ("codex_cli_simplified_flow", "true"),
            ("originator", "codex_cli_rs"),
        ]
    }

    fn resolve_effective_token(
        &self,
        token_set: &OAuthTokenSet,
        stored_id_token: Option<&str>,
    ) -> (String, Option<String>) {
        // Store the raw access_token as the effective token (used for Bearer auth and limits queries).
        // The id_token is stored separately for extracting chatgpt-account-id header.
        let id_token = token_set
            .id_token
            .as_deref()
            .or(stored_id_token)
            .filter(|v| !v.trim().is_empty())
            .map(str::to_string);
        (token_set.access_token.clone(), id_token)
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        insert_bearer_auth(headers, access_token, "codex oauth")?;
        headers.insert("originator", HeaderValue::from_static("codex_cli_rs"));
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
                .get("https://chatgpt.com/backend-api/wham/usage")
                .header("Authorization", format!("Bearer {}", token))
                .header(
                    "User-Agent",
                    format!(
                        "{} (Debian 13.0.0; x86_64) WindowsTerminal",
                        crate::gateway::oauth::DEFAULT_OAUTH_USER_AGENT
                    ),
                )
                .header("Content-Type", "application/json")
                .send()
                .await
                .map_err(|e| format!("codex limits fetch failed: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("codex limits fetch status: {}", resp.status()));
            }

            let body = read_text_with_limit(resp, CODEX_LIMITS_RESPONSE_BODY_LIMIT, "codex limits")
                .await
                .map_err(|e| format!("codex limits body read failed: {e}"))?;
            let json: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| format!("codex limits parse failed: {e}"))?;

            Ok(OAuthLimitsResult {
                raw_json: Some(json),
                ..Default::default()
            })
        })
    }
}
