//! Usage: Schema migrations and input sanitization for settings upgrades.

use super::defaults::*;
use super::types::{AppSettings, CodexHomeMode};
use crate::shared::error::AppResult;

pub(super) fn normalize_cli_priority_order(input: &[String]) -> Vec<String> {
    let mut order = Vec::with_capacity(crate::shared::cli_key::SUPPORTED_CLI_KEYS.len());

    for cli_key in input {
        if !crate::shared::cli_key::is_supported_cli_key(cli_key) {
            continue;
        }
        if order.iter().any(|item| item == cli_key) {
            continue;
        }
        order.push(cli_key.clone());
    }

    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        if order.iter().any(|item| item == cli_key) {
            continue;
        }
        order.push(cli_key.to_string());
    }

    order
}

pub(super) fn normalize_codex_home_override(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.eq_ignore_ascii_case("config.toml") {
        return String::new();
    }

    for suffix in ["/config.toml", "\\config.toml"] {
        if trimmed.len() > suffix.len()
            && trimmed[trimmed.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
        {
            return trimmed[..trimmed.len() - suffix.len()]
                .trim_end_matches(['/', '\\'])
                .to_string();
        }
    }

    trimmed.to_string()
}

pub(super) fn sanitize_codex_home_override(settings: &mut AppSettings) -> bool {
    let normalized = normalize_codex_home_override(&settings.codex_home_override);
    let mut changed = settings.codex_home_override != normalized;
    settings.codex_home_override = normalized;

    if settings.codex_home_mode != CodexHomeMode::Custom && !settings.codex_home_override.is_empty()
    {
        settings.codex_home_override.clear();
        changed = true;
    }

    if settings.codex_home_mode == CodexHomeMode::Custom && settings.codex_home_override.is_empty()
    {
        settings.codex_home_mode = CodexHomeMode::UserHomeDefault;
        changed = true;
    }

    changed
}

pub(super) fn sanitize_cli_priority_order(settings: &mut AppSettings) -> bool {
    let normalized = normalize_cli_priority_order(&settings.cli_priority_order);
    let changed = settings.cli_priority_order != normalized;
    settings.cli_priority_order = normalized;
    changed
}

pub(super) fn sanitize_failover_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.failover_max_attempts_per_provider == 0 {
        settings.failover_max_attempts_per_provider = DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER;
        changed = true;
    }
    if settings.failover_max_providers_to_try == 0 {
        settings.failover_max_providers_to_try = DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY;
        changed = true;
    }

    if settings.failover_max_attempts_per_provider > MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER {
        settings.failover_max_attempts_per_provider = MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER;
        changed = true;
    }

    if settings.failover_max_providers_to_try > MAX_FAILOVER_MAX_PROVIDERS_TO_TRY {
        settings.failover_max_providers_to_try = MAX_FAILOVER_MAX_PROVIDERS_TO_TRY;
        changed = true;
    }

    let providers = settings.failover_max_providers_to_try.max(1);
    let max_attempts_for_providers = (MAX_FAILOVER_TOTAL_ATTEMPTS / providers).max(1);
    if settings.failover_max_attempts_per_provider > max_attempts_for_providers {
        settings.failover_max_attempts_per_provider = max_attempts_for_providers;
        changed = true;
    }

    changed
}

pub(super) fn sanitize_circuit_breaker_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.circuit_breaker_failure_threshold == 0 {
        settings.circuit_breaker_failure_threshold = DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD;
        changed = true;
    }
    if settings.circuit_breaker_open_duration_minutes == 0 {
        settings.circuit_breaker_open_duration_minutes =
            DEFAULT_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES;
        changed = true;
    }

    if settings.circuit_breaker_failure_threshold > MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD {
        settings.circuit_breaker_failure_threshold = MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD;
        changed = true;
    }
    if settings.circuit_breaker_open_duration_minutes > MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES {
        settings.circuit_breaker_open_duration_minutes = MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES;
        changed = true;
    }

    changed
}

pub(super) fn sanitize_provider_cooldown_seconds(settings: &mut AppSettings) -> bool {
    if settings.provider_cooldown_seconds > MAX_PROVIDER_COOLDOWN_SECONDS {
        settings.provider_cooldown_seconds = MAX_PROVIDER_COOLDOWN_SECONDS;
        return true;
    }
    false
}

pub(super) fn sanitize_provider_base_url_ping_cache_ttl_seconds(
    settings: &mut AppSettings,
) -> bool {
    let mut changed = false;

    if settings.provider_base_url_ping_cache_ttl_seconds == 0 {
        settings.provider_base_url_ping_cache_ttl_seconds =
            DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS;
        changed = true;
    }

    if settings.provider_base_url_ping_cache_ttl_seconds
        > MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
    {
        settings.provider_base_url_ping_cache_ttl_seconds =
            MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS;
        changed = true;
    }

    changed
}

pub(super) fn sanitize_upstream_timeouts(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.upstream_first_byte_timeout_seconds > MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS {
        settings.upstream_first_byte_timeout_seconds = MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS;
        changed = true;
    }
    if settings.upstream_stream_idle_timeout_seconds > MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS {
        settings.upstream_stream_idle_timeout_seconds = MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS;
        changed = true;
    }
    if settings.upstream_stream_idle_timeout_seconds > 0
        && settings.upstream_stream_idle_timeout_seconds < MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS
    {
        settings.upstream_stream_idle_timeout_seconds = MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS;
        changed = true;
    }
    if settings.upstream_request_timeout_non_streaming_seconds
        > MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS
    {
        settings.upstream_request_timeout_non_streaming_seconds =
            MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS;
        changed = true;
    }

    changed
}

pub(super) fn sanitize_response_fixer_limits(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.response_fixer_max_json_depth == 0 {
        settings.response_fixer_max_json_depth = DEFAULT_RESPONSE_FIXER_MAX_JSON_DEPTH;
        changed = true;
    }
    if settings.response_fixer_max_json_depth > MAX_RESPONSE_FIXER_MAX_JSON_DEPTH {
        settings.response_fixer_max_json_depth = MAX_RESPONSE_FIXER_MAX_JSON_DEPTH;
        changed = true;
    }

    if settings.response_fixer_max_fix_size == 0 {
        settings.response_fixer_max_fix_size = DEFAULT_RESPONSE_FIXER_MAX_FIX_SIZE;
        changed = true;
    }
    if settings.response_fixer_max_fix_size > MAX_RESPONSE_FIXER_MAX_FIX_SIZE {
        settings.response_fixer_max_fix_size = MAX_RESPONSE_FIXER_MAX_FIX_SIZE;
        changed = true;
    }

    changed
}

// -- Schema migrations --

fn migrate_disable_upstream_timeouts(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v7: Align defaults with "0 = disabled" semantics and migrate existing configs to disabled.
    if schema_version_present && settings.schema_version >= SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS
    {
        return false;
    }

    let mut changed = false;

    // If the schema version is missing, force a write to persist the current schema_version so we
    // don't re-run migrations on every startup.
    if !schema_version_present {
        changed = true;
    }

    if settings.schema_version != SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS {
        settings.schema_version = SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS;
        changed = true;
    }

    if settings.upstream_first_byte_timeout_seconds != 0 {
        settings.upstream_first_byte_timeout_seconds = 0;
        changed = true;
    }
    if settings.upstream_stream_idle_timeout_seconds != 0 {
        settings.upstream_stream_idle_timeout_seconds = 0;
        changed = true;
    }
    if settings.upstream_request_timeout_non_streaming_seconds != 0 {
        settings.upstream_request_timeout_non_streaming_seconds = 0;
        changed = true;
    }

    changed
}

/// Generic schema migration helper for versions that only bump `schema_version`.
///
/// Returns `true` if the settings were modified (i.e. migration was applied).
/// Migrations that need additional field changes (e.g. `migrate_disable_upstream_timeouts`)
/// should NOT use this helper.
fn migrate_bump_schema_version(
    settings: &mut AppSettings,
    schema_version_present: bool,
    target_version: u32,
) -> bool {
    if schema_version_present && settings.schema_version >= target_version {
        return false;
    }

    let mut changed = false;

    // If schema_version is missing, force a write to persist schema_version so we don't keep
    // "migrating" on every startup.
    if !schema_version_present {
        changed = true;
    }

    if settings.schema_version != target_version {
        settings.schema_version = target_version;
        changed = true;
    }

    changed
}

fn migrate_add_gateway_rectifiers(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v8: Add CCH v0.4.1-aligned gateway rectifier toggles (default disabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_GATEWAY_RECTIFIERS,
    )
}

fn migrate_add_circuit_breaker_notice(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v9: Add circuit breaker notice toggle (default disabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CIRCUIT_BREAKER_NOTICE,
    )
}

fn migrate_add_provider_base_url_ping_cache_ttl(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v10: Add provider ping cache ttl (seconds), default 60.
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_PROVIDER_BASE_URL_PING_CACHE_TTL,
    )
}

fn migrate_add_codex_session_id_completion(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v11: Add Codex Session ID completion toggle (default disabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CODEX_SESSION_ID_COMPLETION,
    )
}

fn migrate_add_gateway_network_settings(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v12: Add gateway listen mode + WSL network settings (default disabled / all CLI enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_GATEWAY_NETWORK_SETTINGS,
    )
}

fn migrate_add_response_fixer_limits(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v13: Add response fixer config limits (max_json_depth / max_fix_size).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_RESPONSE_FIXER_LIMITS,
    )
}

fn migrate_add_cli_proxy_startup_recovery(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v14: Add CLI proxy startup recovery toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CLI_PROXY_STARTUP_RECOVERY,
    )
}

fn migrate_add_cache_anomaly_monitor(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v15: Add cache anomaly monitor toggle (default disabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CACHE_ANOMALY_MONITOR,
    )
}

fn migrate_add_wsl_host_address_mode(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_WSL_HOST_ADDRESS_MODE,
    )
}

fn migrate_add_task_complete_notify(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v17: Add task complete notification toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_TASK_COMPLETE_NOTIFY,
    )
}

fn migrate_add_cch_base_config(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v18: Add verbose provider error + thinking budget rectifier + claude metadata.user_id injection.
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CCH_BASE_CONFIG,
    )
}

fn migrate_add_start_minimized(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v19: Add start_minimized toggle (default disabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_START_MINIMIZED,
    )
}

fn migrate_add_show_home_heatmap(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v20: Add homepage heatmap visibility toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_SHOW_HOME_HEATMAP,
    )
}

fn migrate_add_home_usage_period(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v21: Add homepage usage window selector (default last15).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_HOME_USAGE_PERIOD,
    )
}

fn migrate_add_show_home_usage(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v22: Add homepage usage visibility toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_SHOW_HOME_USAGE,
    )
}

fn migrate_add_codex_home_override(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v23: Add persisted Codex config directory override (default empty = use default resolution).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CODEX_HOME_OVERRIDE,
    )
}

fn migrate_add_codex_home_mode(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    // v24: Split Codex home resolution into explicit user-home default / follow CODEX_HOME / custom.
    let needs_mode_default =
        !schema_version_present || settings.schema_version < SCHEMA_VERSION_ADD_CODEX_HOME_MODE;
    let changed = migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CODEX_HOME_MODE,
    );

    if needs_mode_default {
        settings.codex_home_mode = if settings.codex_home_override.trim().is_empty() {
            CodexHomeMode::UserHomeDefault
        } else {
            CodexHomeMode::Custom
        };
    }

    changed
}

fn migrate_add_notification_sound(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v25: Add notification sound toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_NOTIFICATION_SOUND,
    )
}

fn migrate_add_cx2cc_settings(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    if !migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CX2CC_SETTINGS,
    ) {
        return false;
    }
    if settings.cx2cc_fallback_model_opus.is_empty() {
        settings.cx2cc_fallback_model_opus = DEFAULT_CX2CC_FALLBACK_MODEL.to_string();
    }
    if settings.cx2cc_fallback_model_sonnet.is_empty() {
        settings.cx2cc_fallback_model_sonnet = DEFAULT_CX2CC_FALLBACK_MODEL.to_string();
    }
    if settings.cx2cc_fallback_model_haiku.is_empty() {
        settings.cx2cc_fallback_model_haiku = DEFAULT_CX2CC_FALLBACK_MODEL.to_string();
    }
    if settings.cx2cc_fallback_model_main.is_empty() {
        settings.cx2cc_fallback_model_main = DEFAULT_CX2CC_FALLBACK_MODEL.to_string();
    }
    settings.cx2cc_disable_response_storage = true;
    settings.cx2cc_enable_reasoning_to_thinking = true;
    settings.cx2cc_drop_stop_sequences = true;
    settings.cx2cc_clean_schema = true;
    settings.cx2cc_filter_batch_tool = true;
    true
}

fn migrate_enable_default_upstream_timeouts(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // Fresh installs already pick up the new defaults from `AppSettings::default`.
    // Existing installs must preserve explicit `0 = disabled` choices.
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ENABLE_DEFAULT_UPSTREAM_TIMEOUTS,
    )
}

fn migrate_add_billing_header_rectifier(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v28: Add billing header rectifier toggle (default enabled).
    migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_BILLING_HEADER_RECTIFIER,
    )
}

fn migrate_add_cli_priority_order(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    // v29: Add global CLI priority order for tab rendering and default selection.
    if !migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_ADD_CLI_PRIORITY_ORDER,
    ) {
        return false;
    }

    settings.cli_priority_order = normalize_cli_priority_order(&settings.cli_priority_order);
    true
}

fn migrate_raise_stream_idle_timeout_default(
    settings: &mut AppSettings,
    schema_version_present: bool,
) -> bool {
    if !migrate_bump_schema_version(
        settings,
        schema_version_present,
        SCHEMA_VERSION_RAISE_STREAM_IDLE_TIMEOUT_DEFAULT,
    ) {
        return false;
    }

    // Users who got the old 120s default should be bumped to 300s.
    // Users who explicitly set other values (including 0 = disabled) keep their choice.
    if settings.upstream_stream_idle_timeout_seconds == 120 {
        settings.upstream_stream_idle_timeout_seconds =
            DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS;
    }
    true
}

type SettingsMigration = fn(&mut AppSettings, bool) -> bool;

const SETTINGS_MIGRATIONS: [SettingsMigration; 24] = [
    migrate_disable_upstream_timeouts,
    migrate_add_gateway_rectifiers,
    migrate_add_circuit_breaker_notice,
    migrate_add_provider_base_url_ping_cache_ttl,
    migrate_add_codex_session_id_completion,
    migrate_add_gateway_network_settings,
    migrate_add_response_fixer_limits,
    migrate_add_cli_proxy_startup_recovery,
    migrate_add_cache_anomaly_monitor,
    migrate_add_wsl_host_address_mode,
    migrate_add_task_complete_notify,
    migrate_add_cch_base_config,
    migrate_add_start_minimized,
    migrate_add_show_home_heatmap,
    migrate_add_home_usage_period,
    migrate_add_show_home_usage,
    migrate_add_codex_home_override,
    migrate_add_codex_home_mode,
    migrate_add_notification_sound,
    migrate_add_cx2cc_settings,
    migrate_enable_default_upstream_timeouts,
    migrate_add_billing_header_rectifier,
    migrate_add_cli_priority_order,
    migrate_raise_stream_idle_timeout_default,
];

fn apply_settings_migrations(settings: &mut AppSettings, schema_version_present: bool) -> bool {
    let mut changed = false;
    for migration in SETTINGS_MIGRATIONS {
        changed |= migration(settings, schema_version_present);
    }
    changed
}

pub(super) fn repair_settings(
    settings: &mut AppSettings,
    schema_version_present: bool,
    raw_settings_json: &serde_json::Value,
) -> AppResult<bool> {
    let mut repaired = apply_settings_migrations(settings, schema_version_present);
    repaired |= sanitize_failover_settings(settings);
    repaired |= sanitize_circuit_breaker_settings(settings);
    repaired |= sanitize_provider_cooldown_seconds(settings);
    repaired |= sanitize_provider_base_url_ping_cache_ttl_seconds(settings);
    repaired |= sanitize_upstream_timeouts(settings);
    repaired |= sanitize_response_fixer_limits(settings);
    repaired |= sanitize_codex_home_override(settings);
    repaired |= sanitize_cli_priority_order(settings);
    let canonical = super::persistence::canonical_settings_json(settings)?;
    repaired |= raw_settings_json != &canonical;
    Ok(repaired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::settings::types::default_cli_priority_order;

    // -- sanitize_failover_settings --

    #[test]
    fn sanitize_failover_resets_zero_attempts_to_default() {
        let mut s = AppSettings {
            failover_max_attempts_per_provider: 0,
            failover_max_providers_to_try: 3,
            ..Default::default()
        };
        assert!(sanitize_failover_settings(&mut s));
        assert_eq!(
            s.failover_max_attempts_per_provider,
            DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER
        );
    }

    #[test]
    fn sanitize_failover_resets_zero_providers_to_default() {
        let mut s = AppSettings {
            failover_max_attempts_per_provider: 3,
            failover_max_providers_to_try: 0,
            ..Default::default()
        };
        assert!(sanitize_failover_settings(&mut s));
        assert_eq!(
            s.failover_max_providers_to_try,
            DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY
        );
    }

    #[test]
    fn sanitize_failover_clamps_excessive_attempts() {
        let mut s = AppSettings {
            failover_max_attempts_per_provider: 999,
            failover_max_providers_to_try: 1,
            ..Default::default()
        };
        assert!(sanitize_failover_settings(&mut s));
        assert_eq!(
            s.failover_max_attempts_per_provider,
            MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER
        );
    }

    #[test]
    fn sanitize_failover_clamps_total_product() {
        // 20 * 20 = 400 > MAX_FAILOVER_TOTAL_ATTEMPTS (100)
        let mut s = AppSettings {
            failover_max_attempts_per_provider: 20,
            failover_max_providers_to_try: 20,
            ..Default::default()
        };
        assert!(sanitize_failover_settings(&mut s));
        // attempts_per_provider should be clamped to 100/20 = 5
        assert_eq!(s.failover_max_attempts_per_provider, 5);
    }

    #[test]
    fn sanitize_failover_no_change_for_valid_values() {
        let mut s = AppSettings::default();
        assert!(!sanitize_failover_settings(&mut s));
    }

    // -- sanitize_circuit_breaker_settings --

    #[test]
    fn sanitize_circuit_breaker_resets_zero_threshold() {
        let mut s = AppSettings {
            circuit_breaker_failure_threshold: 0,
            ..Default::default()
        };
        assert!(sanitize_circuit_breaker_settings(&mut s));
        assert_eq!(
            s.circuit_breaker_failure_threshold,
            DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD
        );
    }

    #[test]
    fn sanitize_circuit_breaker_clamps_excessive_duration() {
        let mut s = AppSettings {
            circuit_breaker_open_duration_minutes: 99999,
            ..Default::default()
        };
        assert!(sanitize_circuit_breaker_settings(&mut s));
        assert_eq!(
            s.circuit_breaker_open_duration_minutes,
            MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES
        );
    }

    #[test]
    fn sanitize_circuit_breaker_no_change_for_valid_values() {
        let mut s = AppSettings::default();
        assert!(!sanitize_circuit_breaker_settings(&mut s));
    }

    // -- sanitize_provider_cooldown_seconds --

    #[test]
    fn sanitize_cooldown_clamps_excessive_value() {
        let mut s = AppSettings {
            provider_cooldown_seconds: MAX_PROVIDER_COOLDOWN_SECONDS + 1,
            ..Default::default()
        };
        assert!(sanitize_provider_cooldown_seconds(&mut s));
        assert_eq!(s.provider_cooldown_seconds, MAX_PROVIDER_COOLDOWN_SECONDS);
    }

    #[test]
    fn sanitize_cooldown_allows_zero() {
        let mut s = AppSettings {
            provider_cooldown_seconds: 0,
            ..Default::default()
        };
        assert!(!sanitize_provider_cooldown_seconds(&mut s));
        assert_eq!(s.provider_cooldown_seconds, 0);
    }

    // -- sanitize_provider_base_url_ping_cache_ttl_seconds --

    #[test]
    fn sanitize_ping_cache_ttl_resets_zero_to_default() {
        let mut s = AppSettings {
            provider_base_url_ping_cache_ttl_seconds: 0,
            ..Default::default()
        };
        assert!(sanitize_provider_base_url_ping_cache_ttl_seconds(&mut s));
        assert_eq!(
            s.provider_base_url_ping_cache_ttl_seconds,
            DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
        );
    }

    #[test]
    fn sanitize_ping_cache_ttl_clamps_excessive_value() {
        let mut s = AppSettings {
            provider_base_url_ping_cache_ttl_seconds: MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
                + 1,
            ..Default::default()
        };
        assert!(sanitize_provider_base_url_ping_cache_ttl_seconds(&mut s));
        assert_eq!(
            s.provider_base_url_ping_cache_ttl_seconds,
            MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
        );
    }

    // -- sanitize_upstream_timeouts --

    #[test]
    fn sanitize_upstream_timeouts_clamps_excessive_values() {
        let mut s = AppSettings {
            upstream_first_byte_timeout_seconds: MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS + 1,
            upstream_stream_idle_timeout_seconds: MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS + 1,
            upstream_request_timeout_non_streaming_seconds:
                MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS + 1,
            ..Default::default()
        };
        assert!(sanitize_upstream_timeouts(&mut s));
        assert_eq!(
            s.upstream_first_byte_timeout_seconds,
            MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS
        );
        assert_eq!(
            s.upstream_stream_idle_timeout_seconds,
            MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS
        );
        assert_eq!(
            s.upstream_request_timeout_non_streaming_seconds,
            MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS
        );
    }

    #[test]
    fn sanitize_upstream_timeouts_allows_zero_disabled() {
        let mut s = AppSettings {
            upstream_first_byte_timeout_seconds: 0,
            upstream_stream_idle_timeout_seconds: 0,
            upstream_request_timeout_non_streaming_seconds: 0,
            ..Default::default()
        };
        assert!(!sanitize_upstream_timeouts(&mut s));
    }

    // -- sanitize_response_fixer_limits --

    #[test]
    fn sanitize_response_fixer_resets_zero_depth_to_default() {
        let mut s = AppSettings {
            response_fixer_max_json_depth: 0,
            ..Default::default()
        };
        assert!(sanitize_response_fixer_limits(&mut s));
        assert_eq!(
            s.response_fixer_max_json_depth,
            DEFAULT_RESPONSE_FIXER_MAX_JSON_DEPTH
        );
    }

    #[test]
    fn sanitize_response_fixer_clamps_excessive_depth() {
        let mut s = AppSettings {
            response_fixer_max_json_depth: MAX_RESPONSE_FIXER_MAX_JSON_DEPTH + 1,
            ..Default::default()
        };
        assert!(sanitize_response_fixer_limits(&mut s));
        assert_eq!(
            s.response_fixer_max_json_depth,
            MAX_RESPONSE_FIXER_MAX_JSON_DEPTH
        );
    }

    #[test]
    fn sanitize_response_fixer_resets_zero_size_to_default() {
        let mut s = AppSettings {
            response_fixer_max_fix_size: 0,
            ..Default::default()
        };
        assert!(sanitize_response_fixer_limits(&mut s));
        assert_eq!(
            s.response_fixer_max_fix_size,
            DEFAULT_RESPONSE_FIXER_MAX_FIX_SIZE
        );
    }

    // -- migrate_bump_schema_version --

    #[test]
    fn migrate_bump_skips_when_already_at_target() {
        let mut s = AppSettings {
            schema_version: 10,
            ..Default::default()
        };
        assert!(!migrate_bump_schema_version(&mut s, true, 10));
        assert_eq!(s.schema_version, 10);
    }

    #[test]
    fn migrate_bump_skips_when_above_target() {
        let mut s = AppSettings {
            schema_version: 12,
            ..Default::default()
        };
        assert!(!migrate_bump_schema_version(&mut s, true, 10));
        assert_eq!(s.schema_version, 12);
    }

    #[test]
    fn migrate_bump_applies_when_below_target() {
        let mut s = AppSettings {
            schema_version: 8,
            ..Default::default()
        };
        assert!(migrate_bump_schema_version(&mut s, true, 10));
        assert_eq!(s.schema_version, 10);
    }

    #[test]
    fn migrate_bump_forces_write_when_schema_version_absent() {
        let mut s = AppSettings {
            schema_version: 10,
            ..Default::default()
        };
        // schema_version_present = false forces a write even if version matches
        assert!(migrate_bump_schema_version(&mut s, false, 10));
    }

    // -- migrate_disable_upstream_timeouts --

    #[test]
    fn migrate_disable_upstream_timeouts_resets_nonzero_values() {
        let mut s = AppSettings {
            schema_version: 5,
            upstream_first_byte_timeout_seconds: 30,
            upstream_stream_idle_timeout_seconds: 60,
            upstream_request_timeout_non_streaming_seconds: 120,
            ..Default::default()
        };
        assert!(migrate_disable_upstream_timeouts(&mut s, true));
        assert_eq!(s.upstream_first_byte_timeout_seconds, 0);
        assert_eq!(s.upstream_stream_idle_timeout_seconds, 0);
        assert_eq!(s.upstream_request_timeout_non_streaming_seconds, 0);
        assert_eq!(s.schema_version, SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS);
    }

    #[test]
    fn migrate_disable_upstream_timeouts_skips_when_already_migrated() {
        let mut s = AppSettings {
            schema_version: SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS,
            upstream_first_byte_timeout_seconds: 30,
            ..Default::default()
        };
        assert!(!migrate_disable_upstream_timeouts(&mut s, true));
        // Value should NOT be reset since migration is already applied
        assert_eq!(s.upstream_first_byte_timeout_seconds, 30);
    }

    #[test]
    fn migrate_enable_default_upstream_timeouts_preserves_disabled_values() {
        let mut s = AppSettings {
            schema_version: 26,
            upstream_first_byte_timeout_seconds: 0,
            upstream_stream_idle_timeout_seconds: 0,
            ..Default::default()
        };

        assert!(migrate_enable_default_upstream_timeouts(&mut s, true));
        assert_eq!(
            s.schema_version,
            SCHEMA_VERSION_ENABLE_DEFAULT_UPSTREAM_TIMEOUTS
        );
        assert_eq!(s.upstream_first_byte_timeout_seconds, 0);
        assert_eq!(s.upstream_stream_idle_timeout_seconds, 0);
    }

    #[test]
    fn migrate_enable_default_upstream_timeouts_keeps_existing_nonzero_values() {
        let mut s = AppSettings {
            schema_version: 26,
            upstream_first_byte_timeout_seconds: 15,
            upstream_stream_idle_timeout_seconds: 45,
            ..Default::default()
        };

        assert!(migrate_enable_default_upstream_timeouts(&mut s, true));
        assert_eq!(
            s.schema_version,
            SCHEMA_VERSION_ENABLE_DEFAULT_UPSTREAM_TIMEOUTS
        );
        assert_eq!(s.upstream_first_byte_timeout_seconds, 15);
        assert_eq!(s.upstream_stream_idle_timeout_seconds, 45);
    }

    // -- GatewayListenMode --

    #[test]
    fn gateway_listen_mode_default_is_localhost() {
        assert_eq!(
            super::super::types::GatewayListenMode::default(),
            super::super::types::GatewayListenMode::Localhost,
        );
    }

    // -- AppSettings default --

    #[test]
    fn app_settings_default_has_current_schema_version() {
        let s = AppSettings::default();
        assert_eq!(s.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn app_settings_default_has_expected_port() {
        let s = AppSettings::default();
        assert_eq!(s.preferred_port, DEFAULT_GATEWAY_PORT);
    }

    #[test]
    fn app_settings_default_shows_home_heatmap() {
        let s = AppSettings::default();
        assert!(s.show_home_heatmap);
    }

    #[test]
    fn app_settings_default_shows_home_usage() {
        let s = AppSettings::default();
        assert!(s.show_home_usage);
    }

    #[test]
    fn app_settings_default_has_empty_codex_home_override() {
        let s = AppSettings::default();
        assert!(s.codex_home_override.is_empty());
    }

    #[test]
    fn app_settings_default_uses_user_home_default_codex_mode() {
        let s = AppSettings::default();
        assert_eq!(s.codex_home_mode, CodexHomeMode::UserHomeDefault);
    }

    #[test]
    fn app_settings_default_uses_last15_home_usage_period() {
        use super::super::types::HomeUsagePeriod;
        let s = AppSettings::default();
        assert_eq!(s.home_usage_period, HomeUsagePeriod::Last15);
    }

    #[test]
    fn app_settings_default_sets_cli_priority_order() {
        let s = AppSettings::default();
        assert_eq!(s.cli_priority_order, default_cli_priority_order());
    }

    #[test]
    fn app_settings_default_cache_anomaly_monitor_disabled() {
        let s = AppSettings::default();
        assert!(!s.enable_cache_anomaly_monitor);
    }

    #[test]
    fn migrate_add_cache_anomaly_monitor_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 14,
            ..Default::default()
        };
        assert!(migrate_add_cache_anomaly_monitor(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_CACHE_ANOMALY_MONITOR);
    }

    #[test]
    fn migrate_add_wsl_host_address_mode_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 15,
            ..Default::default()
        };
        assert!(migrate_add_wsl_host_address_mode(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_WSL_HOST_ADDRESS_MODE);
    }

    #[test]
    fn migrate_add_show_home_heatmap_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 19,
            ..Default::default()
        };
        assert!(migrate_add_show_home_heatmap(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_SHOW_HOME_HEATMAP);
    }

    #[test]
    fn migrate_add_home_usage_period_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 20,
            ..Default::default()
        };
        assert!(migrate_add_home_usage_period(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_HOME_USAGE_PERIOD);
    }

    #[test]
    fn migrate_add_show_home_usage_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 21,
            ..Default::default()
        };
        assert!(migrate_add_show_home_usage(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_SHOW_HOME_USAGE);
    }

    #[test]
    fn migrate_add_codex_home_override_bumps_schema_version() {
        let mut s = AppSettings {
            schema_version: 22,
            ..Default::default()
        };
        assert!(migrate_add_codex_home_override(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_CODEX_HOME_OVERRIDE);
    }

    #[test]
    fn migrate_add_codex_home_mode_bumps_schema_version_and_defaults_to_user_home() {
        let mut s = AppSettings {
            schema_version: 23,
            ..Default::default()
        };
        assert!(migrate_add_codex_home_mode(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_CODEX_HOME_MODE);
        assert_eq!(s.codex_home_mode, CodexHomeMode::UserHomeDefault);
    }

    #[test]
    fn migrate_add_codex_home_mode_preserves_legacy_custom_override_as_custom_mode() {
        let mut s = AppSettings {
            schema_version: 23,
            codex_home_override: r"D:\Work\.codex".to_string(),
            ..Default::default()
        };
        assert!(migrate_add_codex_home_mode(&mut s, true));
        assert_eq!(s.codex_home_mode, CodexHomeMode::Custom);
    }

    #[test]
    fn sanitize_cli_priority_order_normalizes_invalid_duplicates_and_missing() {
        let mut s = AppSettings {
            cli_priority_order: vec![
                "codex".to_string(),
                "unknown".to_string(),
                "codex".to_string(),
                "claude".to_string(),
            ],
            ..Default::default()
        };
        assert!(sanitize_cli_priority_order(&mut s));
        assert_eq!(
            s.cli_priority_order,
            vec![
                "codex".to_string(),
                "claude".to_string(),
                "gemini".to_string()
            ]
        );
    }

    #[test]
    fn migrate_add_cli_priority_order_bumps_schema_and_fills_default_order() {
        let mut s = AppSettings {
            schema_version: 28,
            cli_priority_order: Vec::new(),
            ..Default::default()
        };
        assert!(migrate_add_cli_priority_order(&mut s, true));
        assert_eq!(s.schema_version, SCHEMA_VERSION_ADD_CLI_PRIORITY_ORDER);
        assert_eq!(s.cli_priority_order, default_cli_priority_order());
    }

    #[test]
    fn normalize_codex_home_override_keeps_directory_input() {
        assert_eq!(
            normalize_codex_home_override(r"  C:\Users\me\.codex  "),
            r"C:\Users\me\.codex"
        );
    }

    #[test]
    fn normalize_codex_home_override_converts_config_toml_to_parent_dir() {
        assert_eq!(
            normalize_codex_home_override(r"C:\Users\me\.codex\config.toml"),
            r"C:\Users\me\.codex"
        );
    }

    #[test]
    fn sanitize_codex_home_override_trims_and_normalizes() {
        let mut s = AppSettings {
            codex_home_mode: CodexHomeMode::Custom,
            codex_home_override: " ~/.codex/config.toml ".to_string(),
            ..Default::default()
        };
        assert!(sanitize_codex_home_override(&mut s));
        assert_eq!(s.codex_home_override, "~/.codex");
    }

    #[test]
    fn sanitize_codex_home_override_demotes_empty_custom_mode_to_user_home_default() {
        let mut s = AppSettings {
            codex_home_mode: CodexHomeMode::Custom,
            codex_home_override: "   ".to_string(),
            ..Default::default()
        };
        assert!(sanitize_codex_home_override(&mut s));
        assert_eq!(s.codex_home_mode, CodexHomeMode::UserHomeDefault);
        assert!(s.codex_home_override.is_empty());
    }

    #[test]
    fn sanitize_codex_home_override_clears_override_when_mode_is_not_custom() {
        let mut s = AppSettings {
            codex_home_mode: CodexHomeMode::FollowCodexHome,
            codex_home_override: r"D:\Work\.codex".to_string(),
            ..Default::default()
        };
        assert!(sanitize_codex_home_override(&mut s));
        assert_eq!(s.codex_home_mode, CodexHomeMode::FollowCodexHome);
        assert!(s.codex_home_override.is_empty());
    }
}
