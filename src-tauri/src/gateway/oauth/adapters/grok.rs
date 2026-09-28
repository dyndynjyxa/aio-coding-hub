//! Usage: Grok (xAI) OAuth adapter — browser PKCE + RFC 8628 device code.
//!
//! Contract mirrors official `xai-org/grok-build` OAuth2 client:
//! issuer `https://auth.x.ai`, public client id, frozen scopes, and
//! `referrer=grok-build` / `x-grok-client-*` identity headers.

use crate::gateway::oauth::provider_trait::*;
use axum::http::{HeaderMap, HeaderValue};

/// Public xAI Grok CLI OAuth client ID (shared by official Grok Build CLI).
pub(crate) const GROK_OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";

/// OIDC issuer and fixed endpoints (also discoverable via
/// `https://auth.x.ai/.well-known/openid-configuration`).
pub(crate) const GROK_AUTH_URL: &str = "https://auth.x.ai/oauth2/authorize";
pub(crate) const GROK_TOKEN_URL: &str = "https://auth.x.ai/oauth2/token";
pub(crate) const GROK_DEVICE_AUTHORIZATION_URL: &str = "https://auth.x.ai/oauth2/device/code";

/// Default upstream for SuperGrok / subscription OAuth tokens (CLI chat proxy).
pub(crate) const GROK_OAUTH_DEFAULT_BASE_URL: &str = "https://cli-chat-proxy.grok.com/v1";

/// Official grok-build usage-attribution referrer (authorize + device-code form).
pub(crate) const GROK_OAUTH_REFERRER: &str = "grok-build";

/// Client-surface values for `x-grok-client-surface` (device-flow metrics).
pub(crate) const GROK_CLIENT_SURFACE_UI: &str = "ui";

/// Recommended Grok Build CLI version string for `x-grok-client-version`.
/// Overridable via `AIO_GROK_CLIENT_VERSION` when xAI bumps the identity contract.
const GROK_DEFAULT_CLIENT_VERSION: &str = "0.2.93";

/// Preferred loopback callback port (falls back to an ephemeral port if busy).
/// Matches grok-build local-dev fixed port; production grok-build uses OS ephemeral.
const GROK_DEFAULT_CALLBACK_PORT: u16 = 56121;

/// Frozen personal OAuth2 scopes from grok-build (`default_oauth2_scopes`).
const GROK_OAUTH_SCOPES: &[&str] = &[
    "openid",
    "profile",
    "email",
    "offline_access",
    "grok-cli:access",
    "api:access",
    "conversations:read",
    "conversations:write",
];

pub(crate) struct GrokOAuthProvider {
    endpoints: OAuthEndpoints,
}

impl GrokOAuthProvider {
    pub(crate) fn new() -> Self {
        let client_id = std::env::var("AIO_GROK_OAUTH_CLIENT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| GROK_OAUTH_CLIENT_ID.to_string());

        Self {
            endpoints: OAuthEndpoints {
                auth_url: GROK_AUTH_URL,
                token_url: GROK_TOKEN_URL,
                client_id,
                client_secret: None,
                scopes: GROK_OAUTH_SCOPES.to_vec(),
                // Official Grok Build uses 127.0.0.1 loopback + /callback (RFC 8252).
                redirect_host: "127.0.0.1",
                callback_path: "/callback",
                default_callback_port: GROK_DEFAULT_CALLBACK_PORT,
            },
        }
    }
}

/// Resolve the client version identity header sent to auth.x.ai / cli-chat-proxy.
pub(crate) fn grok_client_version() -> String {
    std::env::var("AIO_GROK_CLIENT_VERSION")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| GROK_DEFAULT_CLIENT_VERSION.to_string())
}

impl OAuthProvider for GrokOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "grok"
    }

    fn provider_type(&self) -> &'static str {
        "grok_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        GROK_OAUTH_DEFAULT_BASE_URL
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        // Matches grok-build `build_authorize_url` referrer default.
        vec![("referrer", GROK_OAUTH_REFERRER)]
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        insert_bearer_auth(headers, access_token, "grok oauth")?;
        // Mirror grok-build client identity on subscription proxy calls.
        let version = grok_client_version();
        if let Ok(value) = HeaderValue::from_str(&version) {
            headers.insert("x-grok-client-version", value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn endpoints_match_official_xai_oidc() {
        let provider = GrokOAuthProvider::new();
        let endpoints = provider.endpoints();

        assert_eq!(endpoints.auth_url, GROK_AUTH_URL);
        assert_eq!(endpoints.token_url, GROK_TOKEN_URL);
        assert_eq!(endpoints.client_id, GROK_OAUTH_CLIENT_ID);
        assert_eq!(endpoints.redirect_host, "127.0.0.1");
        assert_eq!(endpoints.callback_path, "/callback");
        assert_eq!(endpoints.scopes, GROK_OAUTH_SCOPES);
        assert!(endpoints.scopes.contains(&"conversations:read"));
        assert!(endpoints.scopes.contains(&"conversations:write"));
        assert!(endpoints.scopes.contains(&"grok-cli:access"));
        assert!(endpoints.scopes.contains(&"offline_access"));
    }

    #[test]
    fn authorize_params_include_official_referrer() {
        let provider = GrokOAuthProvider::new();
        assert!(provider
            .extra_authorize_params()
            .contains(&("referrer", GROK_OAUTH_REFERRER)));
    }

    #[test]
    fn inject_upstream_headers_sets_bearer_and_client_version() {
        let provider = GrokOAuthProvider::new();
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
            headers
                .get("x-grok-client-version")
                .and_then(|v| v.to_str().ok()),
            Some(GROK_DEFAULT_CLIENT_VERSION)
        );
    }

    #[test]
    fn provider_type_and_cli_key() {
        let provider = GrokOAuthProvider::new();
        assert_eq!(provider.cli_key(), "grok");
        assert_eq!(provider.provider_type(), "grok_oauth");
        assert_eq!(provider.default_base_url(), GROK_OAUTH_DEFAULT_BASE_URL);
    }
}
