//! Usage: Core OAuth provider trait – the adapter pattern interface.

use axum::http::HeaderMap;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub(crate) struct OAuthEndpoints {
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Vec<&'static str>,
    pub redirect_host: &'static str,
    pub callback_path: &'static str,
    pub default_callback_port: u16,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub id_token: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct OAuthLimitsResult {
    pub limit_short_label: Option<String>,
    pub limit_5h_text: Option<String>,
    pub limit_weekly_text: Option<String>,
    pub raw_json: Option<serde_json::Value>,
}

pub(crate) trait OAuthProvider: Send + Sync {
    fn cli_key(&self) -> &'static str;
    fn provider_type(&self) -> &'static str;
    fn endpoints(&self) -> &OAuthEndpoints;
    /// Default upstream base URL for this OAuth provider.
    /// Used when a provider has `base_urls = []` (the typical case for OAuth).
    fn default_base_url(&self) -> &'static str;
    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![]
    }
    fn resolve_effective_token(
        &self,
        token_set: &OAuthTokenSet,
        _stored_id_token: Option<&str>,
    ) -> (String, Option<String>) {
        (token_set.access_token.clone(), token_set.id_token.clone())
    }
    fn inject_upstream_headers(
        &self,
        _headers: &mut HeaderMap,
        _access_token: &str,
    ) -> Result<(), String> {
        Ok(())
    }
    /// Discovery-only version identity; adapters that do not use it keep their
    /// ordinary upstream headers. This must not alter inference identity.
    fn inject_model_discovery_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
        _id_token: Option<&str>,
        _client_version: Option<&str>,
    ) -> Result<(), String> {
        self.inject_upstream_headers(headers, access_token)
    }
    fn fetch_limits(
        &self,
        _client: &reqwest::Client,
        _access_token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthLimitsResult, String>> + Send + '_>> {
        Box::pin(async { Ok(OAuthLimitsResult::default()) })
    }
}

pub(crate) fn make_redirect_uri(endpoints: &OAuthEndpoints, port: u16) -> String {
    format!(
        "http://{}:{}{}",
        endpoints.redirect_host, port, endpoints.callback_path
    )
}

/// Insert a `Bearer` Authorization header into the given header map.
///
/// Shared by all OAuth adapters to avoid duplicating the HeaderValue
/// construction and error formatting.
pub(crate) fn insert_bearer_auth(
    headers: &mut HeaderMap,
    access_token: &str,
    provider_label: &str,
) -> Result<(), String> {
    let bearer = format!("Bearer {access_token}");
    let bearer_val = axum::http::HeaderValue::from_str(&bearer).map_err(|e| {
        format!("{provider_label}: invalid access_token for Authorization header: {e}")
    })?;
    headers.insert(axum::http::header::AUTHORIZATION, bearer_val);
    Ok(())
}
