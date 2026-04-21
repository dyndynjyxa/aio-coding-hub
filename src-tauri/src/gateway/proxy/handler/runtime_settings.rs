//! Runtime settings resolution for the gateway proxy handler.

use crate::gateway::response_fixer;
use crate::settings;

pub(super) const DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 5;
pub(super) const DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 5;

#[derive(Debug, Clone)]
pub(super) struct HandlerRuntimeSettings {
    pub(super) verbose_provider_error: bool,
    pub(super) intercept_warmup: bool,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) enable_billing_header_rectifier: bool,
    pub(super) cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) provider_base_url_ping_cache_ttl_seconds: u32,
    pub(super) enable_codex_session_id_completion: bool,
    pub(super) enable_claude_metadata_user_id_injection: bool,
    pub(super) max_attempts_per_provider: u32,
    pub(super) max_providers_to_try: u32,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_stream_idle_timeout_secs: u32,
    pub(super) upstream_request_timeout_non_streaming_secs: u32,
}

pub(super) fn handler_runtime_settings(
    settings_cfg: Option<&settings::AppSettings>,
    is_claude_count_tokens: bool,
) -> HandlerRuntimeSettings {
    let verbose_provider_error = settings_cfg
        .map(|cfg| cfg.verbose_provider_error)
        .unwrap_or(true);

    let enable_thinking_signature_rectifier = settings_cfg
        .map(|cfg| cfg.enable_thinking_signature_rectifier)
        .unwrap_or(true)
        && !is_claude_count_tokens;

    let enable_thinking_budget_rectifier = settings_cfg
        .map(|cfg| cfg.enable_thinking_budget_rectifier)
        .unwrap_or(true)
        && !is_claude_count_tokens;
    let enable_billing_header_rectifier = settings_cfg
        .map(|cfg| cfg.enable_billing_header_rectifier)
        .unwrap_or(true);
    let cx2cc_settings = settings_cfg
        .map(crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::from_app_settings)
        .unwrap_or_default();

    let enable_response_fixer = settings_cfg
        .map(|cfg| cfg.enable_response_fixer)
        .unwrap_or(true);
    let response_fixer_fix_encoding = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_encoding)
        .unwrap_or(true);
    let response_fixer_fix_sse_format = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_sse_format)
        .unwrap_or(true);
    let response_fixer_fix_truncated_json = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_truncated_json)
        .unwrap_or(true);
    let response_fixer_max_json_depth = settings_cfg
        .map(|cfg| cfg.response_fixer_max_json_depth)
        .unwrap_or(response_fixer::DEFAULT_MAX_JSON_DEPTH as u32);
    let response_fixer_max_fix_size = settings_cfg
        .map(|cfg| cfg.response_fixer_max_fix_size)
        .unwrap_or(response_fixer::DEFAULT_MAX_FIX_SIZE as u32);

    let mut max_attempts_per_provider = settings_cfg
        .map(|cfg| cfg.failover_max_attempts_per_provider.max(1))
        .unwrap_or(DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER);
    let mut max_providers_to_try = settings_cfg
        .map(|cfg| cfg.failover_max_providers_to_try.max(1))
        .unwrap_or(DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY);

    if is_claude_count_tokens {
        max_attempts_per_provider = 1;
        max_providers_to_try = 1;
    }

    HandlerRuntimeSettings {
        verbose_provider_error,
        intercept_warmup: settings_cfg
            .map(|cfg| cfg.intercept_anthropic_warmup_requests)
            .unwrap_or(false),
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        enable_billing_header_rectifier,
        cx2cc_settings,
        enable_response_fixer,
        response_fixer_stream_config: response_fixer::ResponseFixerConfig {
            fix_encoding: response_fixer_fix_encoding,
            fix_sse_format: response_fixer_fix_sse_format,
            fix_truncated_json: response_fixer_fix_truncated_json,
            max_json_depth: response_fixer_max_json_depth as usize,
            max_fix_size: response_fixer_max_fix_size as usize,
        },
        response_fixer_non_stream_config: response_fixer::ResponseFixerConfig {
            fix_encoding: response_fixer_fix_encoding,
            fix_sse_format: false,
            fix_truncated_json: response_fixer_fix_truncated_json,
            max_json_depth: response_fixer_max_json_depth as usize,
            max_fix_size: response_fixer_max_fix_size as usize,
        },
        provider_base_url_ping_cache_ttl_seconds: settings_cfg
            .map(|cfg| cfg.provider_base_url_ping_cache_ttl_seconds)
            .unwrap_or(settings::DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS),
        enable_codex_session_id_completion: settings_cfg
            .map(|cfg| cfg.enable_codex_session_id_completion)
            .unwrap_or(true),
        enable_claude_metadata_user_id_injection: settings_cfg
            .map(|cfg| cfg.enable_claude_metadata_user_id_injection)
            .unwrap_or(true)
            && !is_claude_count_tokens,
        max_attempts_per_provider,
        max_providers_to_try,
        provider_cooldown_secs: settings_cfg
            .map(|cfg| cfg.provider_cooldown_seconds as i64)
            .unwrap_or(settings::DEFAULT_PROVIDER_COOLDOWN_SECONDS as i64),
        upstream_first_byte_timeout_secs: settings_cfg
            .map(|cfg| cfg.upstream_first_byte_timeout_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS),
        upstream_stream_idle_timeout_secs: settings_cfg
            .map(|cfg| cfg.upstream_stream_idle_timeout_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS),
        upstream_request_timeout_non_streaming_secs: settings_cfg
            .map(|cfg| cfg.upstream_request_timeout_non_streaming_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS),
    }
}
