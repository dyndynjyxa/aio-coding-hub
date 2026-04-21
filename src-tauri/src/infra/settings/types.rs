//! Usage: Settings struct, field types, enums, and Default implementations.

use super::defaults::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GatewayListenMode {
    Localhost,
    WslAuto,
    Lan,
    Custom,
}

impl Default for GatewayListenMode {
    fn default() -> Self {
        Self::Localhost
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WslHostAddressMode {
    Auto,
    Custom,
}

impl Default for WslHostAddressMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum HomeUsagePeriod {
    #[serde(rename = "last7")]
    #[specta(rename = "last7")]
    Last7,
    #[serde(rename = "last15")]
    #[specta(rename = "last15")]
    Last15,
    #[serde(rename = "last30")]
    #[specta(rename = "last30")]
    Last30,
    #[serde(rename = "month")]
    #[specta(rename = "month")]
    Month,
}

impl Default for HomeUsagePeriod {
    fn default() -> Self {
        Self::Last15
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CodexHomeMode {
    UserHomeDefault,
    FollowCodexHome,
    Custom,
}

impl Default for CodexHomeMode {
    fn default() -> Self {
        Self::UserHomeDefault
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(default)]
pub struct WslTargetCli {
    pub claude: bool,
    pub codex: bool,
    pub gemini: bool,
}

impl Default for WslTargetCli {
    fn default() -> Self {
        Self {
            claude: true,
            codex: true,
            gemini: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct AppSettings {
    pub schema_version: u32,
    pub preferred_port: u16,
    #[serde(default = "default_show_home_heatmap")]
    pub show_home_heatmap: bool,
    #[serde(default = "default_show_home_usage")]
    pub show_home_usage: bool,
    #[serde(default)]
    pub home_usage_period: HomeUsagePeriod,
    // Gateway listen mode (aligned with code-switch-r): localhost / wsl_auto / lan / custom.
    pub gateway_listen_mode: GatewayListenMode,
    // Custom listen address input (host or host:port).
    pub gateway_custom_listen_address: String,
    // WSL auto-config enable switch and target CLI selection.
    pub wsl_auto_config: bool,
    pub wsl_target_cli: WslTargetCli,
    #[serde(default = "default_cli_priority_order")]
    pub cli_priority_order: Vec<String>,
    // WSL host address mode (auto-detect or custom) and custom address.
    pub wsl_host_address_mode: WslHostAddressMode,
    pub wsl_custom_host_address: String,
    // Windows-side Codex config location mode.
    pub codex_home_mode: CodexHomeMode,
    // Optional Codex config directory override. Empty = default resolution.
    pub codex_home_override: String,
    pub auto_start: bool,
    // Start with window hidden when auto-starting (silent startup).
    pub start_minimized: bool,
    pub tray_enabled: bool,
    // Startup crash recovery for CLI proxy takeover (default enabled).
    pub enable_cli_proxy_startup_recovery: bool,
    pub log_retention_days: u32,
    pub provider_cooldown_seconds: u32,
    pub provider_base_url_ping_cache_ttl_seconds: u32,
    pub upstream_first_byte_timeout_seconds: u32,
    pub upstream_stream_idle_timeout_seconds: u32,
    pub upstream_request_timeout_non_streaming_seconds: u32,
    pub update_releases_url: String,
    pub failover_max_attempts_per_provider: u32,
    pub failover_max_providers_to_try: u32,
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_duration_minutes: u32,
    // Circuit breaker notice toggle (default disabled).
    pub enable_circuit_breaker_notice: bool,
    // CCH-aligned gateway feature toggles.
    pub verbose_provider_error: bool,
    pub intercept_anthropic_warmup_requests: bool,
    pub enable_thinking_signature_rectifier: bool,
    pub enable_thinking_budget_rectifier: bool,
    // Billing header rectifier: strip x-anthropic-billing-header from system prompt (default enabled).
    pub enable_billing_header_rectifier: bool,
    // Codex Session ID completion (default enabled).
    pub enable_codex_session_id_completion: bool,
    // Claude metadata.user_id injection (default enabled).
    pub enable_claude_metadata_user_id_injection: bool,
    // Cache anomaly monitor (default disabled).
    pub enable_cache_anomaly_monitor: bool,
    // Task complete notification (default enabled).
    pub enable_task_complete_notify: bool,
    // Notification sound toggle - play custom sound when notifications fire (default enabled).
    pub enable_notification_sound: bool,
    // Response fixer (default enabled).
    pub enable_response_fixer: bool,
    pub response_fixer_fix_encoding: bool,
    pub response_fixer_fix_sse_format: bool,
    pub response_fixer_fix_truncated_json: bool,
    pub response_fixer_max_json_depth: u32,
    pub response_fixer_max_fix_size: u32,
    // CX2CC bridge settings.
    pub cx2cc_fallback_model_opus: String,
    pub cx2cc_fallback_model_sonnet: String,
    pub cx2cc_fallback_model_haiku: String,
    pub cx2cc_fallback_model_main: String,
    pub cx2cc_model_reasoning_effort: String,
    pub cx2cc_service_tier: String,
    pub cx2cc_disable_response_storage: bool,
    pub cx2cc_enable_reasoning_to_thinking: bool,
    pub cx2cc_drop_stop_sequences: bool,
    pub cx2cc_clean_schema: bool,
    pub cx2cc_filter_batch_tool: bool,
    // Upstream proxy settings for gateway outbound requests.
    pub upstream_proxy_enabled: bool,
    pub upstream_proxy_url: String,
    pub upstream_proxy_username: String,
    pub upstream_proxy_password: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            preferred_port: DEFAULT_GATEWAY_PORT,
            show_home_heatmap: DEFAULT_SHOW_HOME_HEATMAP,
            show_home_usage: DEFAULT_SHOW_HOME_USAGE,
            home_usage_period: HomeUsagePeriod::default(),
            gateway_listen_mode: GatewayListenMode::Localhost,
            gateway_custom_listen_address: String::new(),
            wsl_auto_config: false,
            wsl_target_cli: WslTargetCli::default(),
            cli_priority_order: default_cli_priority_order(),
            wsl_host_address_mode: WslHostAddressMode::Auto,
            wsl_custom_host_address: "127.0.0.1".to_string(),
            codex_home_mode: CodexHomeMode::default(),
            codex_home_override: String::new(),
            auto_start: false,
            start_minimized: false,
            tray_enabled: true,
            enable_cli_proxy_startup_recovery: DEFAULT_ENABLE_CLI_PROXY_STARTUP_RECOVERY,
            log_retention_days: DEFAULT_LOG_RETENTION_DAYS,
            provider_cooldown_seconds: DEFAULT_PROVIDER_COOLDOWN_SECONDS,
            provider_base_url_ping_cache_ttl_seconds:
                DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS,
            upstream_first_byte_timeout_seconds: DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS,
            upstream_stream_idle_timeout_seconds: DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS,
            upstream_request_timeout_non_streaming_seconds:
                DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS,
            update_releases_url: DEFAULT_UPDATE_RELEASES_URL.to_string(),
            failover_max_attempts_per_provider: DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER,
            failover_max_providers_to_try: DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY,
            circuit_breaker_failure_threshold: DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD,
            circuit_breaker_open_duration_minutes: DEFAULT_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES,
            enable_circuit_breaker_notice: DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE,
            verbose_provider_error: DEFAULT_VERBOSE_PROVIDER_ERROR,
            intercept_anthropic_warmup_requests: DEFAULT_INTERCEPT_ANTHROPIC_WARMUP_REQUESTS,
            enable_thinking_signature_rectifier: DEFAULT_ENABLE_THINKING_SIGNATURE_RECTIFIER,
            enable_thinking_budget_rectifier: DEFAULT_ENABLE_THINKING_BUDGET_RECTIFIER,
            enable_billing_header_rectifier: DEFAULT_ENABLE_BILLING_HEADER_RECTIFIER,
            enable_codex_session_id_completion: DEFAULT_ENABLE_CODEX_SESSION_ID_COMPLETION,
            enable_claude_metadata_user_id_injection:
                DEFAULT_ENABLE_CLAUDE_METADATA_USER_ID_INJECTION,
            enable_cache_anomaly_monitor: DEFAULT_ENABLE_CACHE_ANOMALY_MONITOR,
            enable_task_complete_notify: DEFAULT_ENABLE_TASK_COMPLETE_NOTIFY,
            enable_notification_sound: DEFAULT_ENABLE_NOTIFICATION_SOUND,
            enable_response_fixer: DEFAULT_ENABLE_RESPONSE_FIXER,
            response_fixer_fix_encoding: DEFAULT_RESPONSE_FIXER_FIX_ENCODING,
            response_fixer_fix_sse_format: DEFAULT_RESPONSE_FIXER_FIX_SSE_FORMAT,
            response_fixer_fix_truncated_json: DEFAULT_RESPONSE_FIXER_FIX_TRUNCATED_JSON,
            response_fixer_max_json_depth: DEFAULT_RESPONSE_FIXER_MAX_JSON_DEPTH,
            response_fixer_max_fix_size: DEFAULT_RESPONSE_FIXER_MAX_FIX_SIZE,
            cx2cc_fallback_model_opus: DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            cx2cc_fallback_model_sonnet: DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            cx2cc_fallback_model_haiku: DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            cx2cc_fallback_model_main: DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            cx2cc_model_reasoning_effort: String::new(),
            cx2cc_service_tier: String::new(),
            cx2cc_disable_response_storage: true,
            cx2cc_enable_reasoning_to_thinking: true,
            cx2cc_drop_stop_sequences: true,
            cx2cc_clean_schema: true,
            cx2cc_filter_batch_tool: true,
            upstream_proxy_enabled: false,
            upstream_proxy_url: String::new(),
            upstream_proxy_username: String::new(),
            upstream_proxy_password: String::new(),
        }
    }
}

fn default_show_home_heatmap() -> bool {
    DEFAULT_SHOW_HOME_HEATMAP
}

fn default_show_home_usage() -> bool {
    DEFAULT_SHOW_HOME_USAGE
}

pub(super) fn default_cli_priority_order() -> Vec<String> {
    crate::shared::cli_key::SUPPORTED_CLI_KEYS
        .iter()
        .map(|cli_key| (*cli_key).to_string())
        .collect()
}
