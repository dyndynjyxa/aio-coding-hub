//! Types for provider configuration and gateway selection.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub(super) const DEFAULT_PRIORITY: i64 = 100;
pub(super) const MAX_MODEL_NAME_LEN: usize = 200;
pub(crate) const CX2CC_BRIDGE_TYPE: &str = "cx2cc";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DailyResetMode {
    Fixed,
    Rolling,
}

impl DailyResetMode {
    pub(super) fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "fixed" => Some(Self::Fixed),
            "rolling" => Some(Self::Rolling),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Rolling => "rolling",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAuthMode {
    ApiKey,
    Oauth,
}

impl ProviderAuthMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::Oauth => "oauth",
        }
    }
}

pub(crate) fn is_cx2cc_bridge(source_provider_id: Option<i64>, bridge_type: Option<&str>) -> bool {
    source_provider_id.is_some() || bridge_type == Some(CX2CC_BRIDGE_TYPE)
}

fn take_first_chars(value: &str, max_chars: usize) -> String {
    if value.chars().nth(max_chars).is_none() {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

/// A single custom HTTP header injected into upstream requests for a provider.
/// Used for gateways that require non-standard identity/auth headers beyond the
/// CLI's built-in auth (e.g. `X-User-Id`, `X-Domain`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ProviderCustomHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ProviderUpsertParams {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub auth_mode: Option<ProviderAuthMode>,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cost_multiplier: f64,
    pub priority: Option<i64>,
    pub claude_models: Option<ClaudeModels>,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<DailyResetMode>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub stream_idle_timeout_seconds: Option<u32>,
    pub extension_values: Option<Vec<ProviderExtensionValuesInput>>,
    pub custom_headers: Option<Vec<ProviderCustomHeader>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct ClaudeModels {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,
}

pub(super) fn normalize_model_slot(raw: Option<String>) -> Option<String> {
    let value = raw.map(|v| v.trim().to_string());
    let value = value.as_deref().unwrap_or("");
    if value.is_empty() {
        return None;
    }
    if value.chars().nth(MAX_MODEL_NAME_LEN).is_some() {
        return Some(take_first_chars(value, MAX_MODEL_NAME_LEN));
    }
    Some(value.to_string())
}

impl ClaudeModels {
    pub(super) fn normalized(self) -> Self {
        Self {
            main_model: normalize_model_slot(self.main_model),
            reasoning_model: normalize_model_slot(self.reasoning_model),
            haiku_model: normalize_model_slot(self.haiku_model),
            sonnet_model: normalize_model_slot(self.sonnet_model),
            opus_model: normalize_model_slot(self.opus_model),
        }
    }

    pub(crate) fn has_any(&self) -> bool {
        self.main_model.is_some()
            || self.reasoning_model.is_some()
            || self.haiku_model.is_some()
            || self.sonnet_model.is_some()
            || self.opus_model.is_some()
    }

    pub(crate) fn map_model(&self, original_model: &str, has_thinking: bool) -> String {
        let model_lower = original_model.to_ascii_lowercase();

        // 1) 按模型类型匹配（子串）
        if model_lower.contains("haiku") {
            if let Some(model) = self.haiku_model.as_deref() {
                return model.to_string();
            }
        }
        if model_lower.contains("opus") {
            if let Some(model) = self.opus_model.as_deref() {
                return model.to_string();
            }
        }
        if model_lower.contains("sonnet") {
            if let Some(model) = self.sonnet_model.as_deref() {
                return model.to_string();
            }
        }

        // 2) thinking 模式在未命中具体模型槽位时使用推理模型
        if has_thinking {
            if let Some(model) = self.reasoning_model.as_deref() {
                return model.to_string();
            }
        }

        // 3) 主模型兜底
        if let Some(model) = self.main_model.as_deref() {
            return model.to_string();
        }

        // 4) 无映射：保持原样
        original_model.to_string()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderBaseUrlMode {
    Order,
    Ping,
}

impl ProviderBaseUrlMode {
    pub(super) fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "order" => Some(Self::Order),
            "ping" => Some(Self::Ping),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Order => "order",
            Self::Ping => "ping",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderExtensionValues {
    pub plugin_id: String,
    pub namespace: String,
    pub values: serde_json::Value,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderExtensionValuesInput {
    pub plugin_id: String,
    pub namespace: String,
    pub values: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderSummary {
    pub id: i64,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub claude_models: ClaudeModels,
    pub enabled: bool,
    pub priority: i64,
    pub cost_multiplier: f64,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: DailyResetMode,
    pub daily_reset_time: String,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub tags: Vec<String>,
    pub note: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub auth_mode: String,
    pub oauth_provider_type: Option<String>,
    pub oauth_email: Option<String>,
    pub oauth_expires_at: Option<i64>,
    pub oauth_last_error: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub stream_idle_timeout_seconds: Option<u32>,
    pub extension_values: Vec<ProviderExtensionValues>,
    pub custom_headers: Vec<ProviderCustomHeader>,
    pub api_key_configured: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderRouteRow {
    pub provider_id: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderForGateway {
    pub id: i64,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub api_key_plaintext: String,
    pub claude_models: ClaudeModels,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: DailyResetMode,
    pub daily_reset_time: String,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub auth_mode: String,
    pub oauth_provider_type: Option<String>,
    pub source_provider_id: Option<i64>,
    #[allow(dead_code)] // Will be read when failover_loop uses bridge_type for dispatch.
    pub bridge_type: Option<String>,
    pub stream_idle_timeout_seconds: Option<u32>,
    pub extension_values: Vec<ProviderExtensionValues>,
    pub custom_headers: Vec<ProviderCustomHeader>,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayProvidersSelection {
    pub sort_mode_id: Option<i64>,
    pub providers: Vec<ProviderForGateway>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClaudeTerminalLaunchContext {
    /// The credential to pass as ANTHROPIC_API_KEY to `claude` CLI.
    /// For `api_key` mode this is the stored api_key; for `oauth` mode it is the OAuth access token.
    pub api_key_plaintext: String,
}

impl ProviderForGateway {
    pub(crate) fn is_cx2cc_bridge(&self) -> bool {
        is_cx2cc_bridge(self.source_provider_id, self.bridge_type.as_deref())
    }

    pub(crate) fn get_effective_claude_model(
        &self,
        requested_model: &str,
        has_thinking: bool,
    ) -> String {
        self.claude_models.map_model(requested_model, has_thinking)
    }
}

#[derive(Debug, Clone)]
pub(super) struct DecodedProviderRow {
    pub id: i64,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub claude_models: ClaudeModels,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: DailyResetMode,
    pub daily_reset_time: String,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub auth_mode: String,
    pub oauth_provider_type: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub custom_headers: Vec<ProviderCustomHeader>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderOAuthDetails {
    pub id: i64,
    pub cli_key: String,
    pub oauth_provider_type: String,
    pub oauth_access_token: String,
    pub oauth_refresh_token: Option<String>,
    pub oauth_id_token: Option<String>,
    pub oauth_token_uri: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub oauth_expires_at: Option<i64>,
    pub oauth_email: Option<String>,
    pub oauth_refresh_lead_s: i64,
    pub oauth_last_refreshed_at: Option<i64>,
}

pub(super) fn claude_models_from_json(raw: &str) -> ClaudeModels {
    serde_json::from_str::<ClaudeModels>(raw)
        .ok()
        .unwrap_or_default()
        .normalized()
}

pub(super) fn tags_from_json(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

pub(super) fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    tags.into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .filter(|v| seen.insert(v.clone()))
        .collect()
}

/// Parse the stored `custom_headers_json` column into a list of headers.
/// Tolerates malformed JSON by returning an empty list.
pub(super) fn custom_headers_from_json(raw: &str) -> Vec<ProviderCustomHeader> {
    serde_json::from_str::<Vec<ProviderCustomHeader>>(raw)
        .ok()
        .map(normalize_custom_headers)
        .unwrap_or_default()
}

/// Clean custom headers before persistence: trim, drop entries with an empty
/// name, and de-duplicate by case-insensitive header name (last write wins).
pub(super) fn normalize_custom_headers(
    headers: Vec<ProviderCustomHeader>,
) -> Vec<ProviderCustomHeader> {
    let mut by_name: Vec<ProviderCustomHeader> = Vec::new();
    for header in headers {
        let name = header.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let value = header.value.trim().to_string();
        let lower = name.to_ascii_lowercase();
        if let Some(existing) = by_name
            .iter_mut()
            .find(|h| h.name.to_ascii_lowercase() == lower)
        {
            existing.value = value;
        } else {
            by_name.push(ProviderCustomHeader { name, value });
        }
    }
    by_name
}
