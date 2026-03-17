//! Usage: Persisted application settings (schema + read/write helpers).

use crate::app_paths;
use crate::shared::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};
use tauri::Manager;

pub const SCHEMA_VERSION: u32 = 20;
const SCHEMA_VERSION_DISABLE_UPSTREAM_TIMEOUTS: u32 = 7;
const SCHEMA_VERSION_ADD_GATEWAY_RECTIFIERS: u32 = 8;
const SCHEMA_VERSION_ADD_CIRCUIT_BREAKER_NOTICE: u32 = 9;
const SCHEMA_VERSION_ADD_PROVIDER_BASE_URL_PING_CACHE_TTL: u32 = 10;
const SCHEMA_VERSION_ADD_CODEX_SESSION_ID_COMPLETION: u32 = 11;
const SCHEMA_VERSION_ADD_GATEWAY_NETWORK_SETTINGS: u32 = 12;
const SCHEMA_VERSION_ADD_RESPONSE_FIXER_LIMITS: u32 = 13;
const SCHEMA_VERSION_ADD_CLI_PROXY_STARTUP_RECOVERY: u32 = 14;
const SCHEMA_VERSION_ADD_CACHE_ANOMALY_MONITOR: u32 = 15;
const SCHEMA_VERSION_ADD_WSL_HOST_ADDRESS_MODE: u32 = 16;
const SCHEMA_VERSION_ADD_TASK_COMPLETE_NOTIFY: u32 = 17;
const SCHEMA_VERSION_ADD_CCH_BASE_CONFIG: u32 = 18;
const SCHEMA_VERSION_ADD_START_MINIMIZED: u32 = 19;
const SCHEMA_VERSION_ADD_SHOW_HOME_HEATMAP: u32 = 20;
pub const DEFAULT_GATEWAY_PORT: u16 = 37123;
pub const MAX_GATEWAY_PORT: u16 = 37199;
const DEFAULT_LOG_RETENTION_DAYS: u32 = 7;
pub const DEFAULT_PROVIDER_COOLDOWN_SECONDS: u32 = 30;
pub const DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS: u32 = 60;
pub const DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS: u32 = 0;
pub const DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS: u32 = 0;
pub const DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS: u32 = 0;
const DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 5;
const DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 5;
const DEFAULT_CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;
const DEFAULT_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES: u32 = 30;
const DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE: bool = false;
const DEFAULT_VERBOSE_PROVIDER_ERROR: bool = true;
const DEFAULT_INTERCEPT_ANTHROPIC_WARMUP_REQUESTS: bool = true;
const DEFAULT_ENABLE_THINKING_SIGNATURE_RECTIFIER: bool = true;
const DEFAULT_ENABLE_THINKING_BUDGET_RECTIFIER: bool = true;
const DEFAULT_ENABLE_CODEX_SESSION_ID_COMPLETION: bool = true;
const DEFAULT_ENABLE_CLAUDE_METADATA_USER_ID_INJECTION: bool = true;
const DEFAULT_ENABLE_CACHE_ANOMALY_MONITOR: bool = false;
const DEFAULT_ENABLE_TASK_COMPLETE_NOTIFY: bool = true;
const DEFAULT_ENABLE_RESPONSE_FIXER: bool = true;
const DEFAULT_ENABLE_CLI_PROXY_STARTUP_RECOVERY: bool = true;
const DEFAULT_SHOW_HOME_HEATMAP: bool = true;
const DEFAULT_RESPONSE_FIXER_FIX_ENCODING: bool = true;
const DEFAULT_RESPONSE_FIXER_FIX_SSE_FORMAT: bool = true;
const DEFAULT_RESPONSE_FIXER_FIX_TRUNCATED_JSON: bool = true;
const DEFAULT_RESPONSE_FIXER_MAX_JSON_DEPTH: u32 = 200;
const DEFAULT_RESPONSE_FIXER_MAX_FIX_SIZE: u32 = 1024 * 1024;
const MAX_PROVIDER_COOLDOWN_SECONDS: u32 = 60 * 60;
const MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS: u32 = 60 * 60;
const MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS: u32 = 60 * 60;
const MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS: u32 = 60 * 60;
const MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS: u32 = 24 * 60 * 60;
const MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 20;
const MAX_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 20;
const MAX_FAILOVER_TOTAL_ATTEMPTS: u32 = 100;
const MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 50;
const MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES: u32 = 24 * 60;
const MAX_RESPONSE_FIXER_MAX_JSON_DEPTH: u32 = 2000;
const MAX_RESPONSE_FIXER_MAX_FIX_SIZE: u32 = 16 * 1024 * 1024;
const LEGACY_IDENTIFIER: &str = "io.aio.gateway";
const DEFAULT_UPDATE_RELEASES_URL: &str = "https://github.com/dyndynjyxa/aio-coding-hub/releases";
const CACHE_TTL: Duration = Duration::from_secs(5);

static LOG_RETENTION_DAYS_FAIL_OPEN_WARNED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct CachedSettings {
    data: AppSettings,
    last_updated: Instant,
}

static SETTINGS_CACHE: OnceLock<RwLock<Option<CachedSettings>>> = OnceLock::new();

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
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
    // Gateway listen mode (aligned with code-switch-r): localhost / wsl_auto / lan / custom.
    pub gateway_listen_mode: GatewayListenMode,
    // Custom listen address input (host or host:port).
    pub gateway_custom_listen_address: String,
    // WSL auto-config enable switch and target CLI selection.
    pub wsl_auto_config: bool,
    pub wsl_target_cli: WslTargetCli,
    // WSL host address mode (auto-detect or custom) and custom address.
    pub wsl_host_address_mode: WslHostAddressMode,
    pub wsl_custom_host_address: String,
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
    // Codex Session ID completion (default enabled).
    pub enable_codex_session_id_completion: bool,
    // Claude metadata.user_id injection (default enabled).
    pub enable_claude_metadata_user_id_injection: bool,
    // Cache anomaly monitor (default disabled).
    pub enable_cache_anomaly_monitor: bool,
    // Task complete notification (default enabled).
    pub enable_task_complete_notify: bool,
    // Response fixer (default enabled).
    pub enable_response_fixer: bool,
    pub response_fixer_fix_encoding: bool,
    pub response_fixer_fix_sse_format: bool,
    pub response_fixer_fix_truncated_json: bool,
    pub response_fixer_max_json_depth: u32,
    pub response_fixer_max_fix_size: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            preferred_port: DEFAULT_GATEWAY_PORT,
            show_home_heatmap: DEFAULT_SHOW_HOME_HEATMAP,
            gateway_listen_mode: GatewayListenMode::Localhost,
            gateway_custom_listen_address: String::new(),
            wsl_auto_config: false,
            wsl_target_cli: WslTargetCli::default(),
            wsl_host_address_mode: WslHostAddressMode::Auto,
            wsl_custom_host_address: "127.0.0.1".to_string(),
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
            enable_codex_session_id_completion: DEFAULT_ENABLE_CODEX_SESSION_ID_COMPLETION,
            enable_claude_metadata_user_id_injection:
                DEFAULT_ENABLE_CLAUDE_METADATA_USER_ID_INJECTION,
            enable_cache_anomaly_monitor: DEFAULT_ENABLE_CACHE_ANOMALY_MONITOR,
            enable_task_complete_notify: DEFAULT_ENABLE_TASK_COMPLETE_NOTIFY,
            enable_response_fixer: DEFAULT_ENABLE_RESPONSE_FIXER,
            response_fixer_fix_encoding: DEFAULT_RESPONSE_FIXER_FIX_ENCODING,
            response_fixer_fix_sse_format: DEFAULT_RESPONSE_FIXER_FIX_SSE_FORMAT,
            response_fixer_fix_truncated_json: DEFAULT_RESPONSE_FIXER_FIX_TRUNCATED_JSON,
            response_fixer_max_json_depth: DEFAULT_RESPONSE_FIXER_MAX_JSON_DEPTH,
            response_fixer_max_fix_size: DEFAULT_RESPONSE_FIXER_MAX_FIX_SIZE,
        }
    }
}

fn default_show_home_heatmap() -> bool {
    DEFAULT_SHOW_HOME_HEATMAP
}

fn sanitize_failover_settings(settings: &mut AppSettings) -> bool {
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

fn sanitize_circuit_breaker_settings(settings: &mut AppSettings) -> bool {
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

fn sanitize_provider_cooldown_seconds(settings: &mut AppSettings) -> bool {
    if settings.provider_cooldown_seconds > MAX_PROVIDER_COOLDOWN_SECONDS {
        settings.provider_cooldown_seconds = MAX_PROVIDER_COOLDOWN_SECONDS;
        return true;
    }
    false
}

fn sanitize_provider_base_url_ping_cache_ttl_seconds(settings: &mut AppSettings) -> bool {
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

fn sanitize_upstream_timeouts(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.upstream_first_byte_timeout_seconds > MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS {
        settings.upstream_first_byte_timeout_seconds = MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS;
        changed = true;
    }
    if settings.upstream_stream_idle_timeout_seconds > MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS {
        settings.upstream_stream_idle_timeout_seconds = MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS;
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

fn sanitize_response_fixer_limits(settings: &mut AppSettings) -> bool {
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

fn settings_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join("settings.json"))
}

fn legacy_settings_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let config_dir = app
        .path()
        .config_dir()
        .map_err(|e| format!("failed to resolve legacy config dir: {e}"))?;

    Ok(config_dir.join(LEGACY_IDENTIFIER).join("settings.json"))
}

fn parse_settings_json(content: &str) -> AppResult<(AppSettings, bool, serde_json::Value)> {
    let raw: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("failed to parse settings.json: {e}"))?;
    let schema_version_present = raw.get("schema_version").is_some();
    let settings: AppSettings = serde_json::from_value(raw.clone())
        .map_err(|e| format!("failed to parse settings.json: {e}"))?;
    Ok((settings, schema_version_present, raw))
}

fn canonical_settings_json(settings: &AppSettings) -> AppResult<serde_json::Value> {
    let serialized =
        serde_json::to_value(settings).map_err(|e| format!("failed to serialize settings: {e}"))?;
    let serialized_defaults = serde_json::to_value(AppSettings::default())
        .map_err(|e| format!("failed to serialize default settings: {e}"))?;

    let serialized_obj = serialized.as_object().ok_or_else(|| {
        "failed to serialize settings: expected settings to serialize as an object".to_string()
    })?;
    let defaults_obj = serialized_defaults.as_object().ok_or_else(|| {
        "failed to serialize default settings: expected defaults to serialize as an object"
            .to_string()
    })?;

    let mut compact = serde_json::Map::new();
    // Always include schema_version to prevent migration heuristics from treating this as a legacy file.
    if let Some(schema_version) = serialized_obj.get("schema_version") {
        compact.insert("schema_version".to_string(), schema_version.clone());
    } else {
        compact.insert(
            "schema_version".to_string(),
            serde_json::json!(SCHEMA_VERSION),
        );
    }

    for (key, value) in serialized_obj {
        if key == "schema_version" {
            continue;
        }

        if let Some(default_value) = defaults_obj.get(key) {
            if value == default_value {
                continue;
            }
        }

        compact.insert(key.clone(), value.clone());
    }

    Ok(serde_json::Value::Object(compact))
}

pub fn read(app: &tauri::AppHandle) -> AppResult<AppSettings> {
    let cache = SETTINGS_CACHE.get_or_init(|| RwLock::new(None));

    if let Ok(guard) = cache.read() {
        if let Some(cached) = guard.as_ref() {
            if cached.last_updated.elapsed() < CACHE_TTL {
                return Ok(cached.data.clone());
            }
        }
    }

    let path = settings_path(app)?;

    if !path.exists() {
        let legacy_path = legacy_settings_path(app)?;
        if legacy_path.exists() {
            let content = std::fs::read_to_string(&legacy_path)
                .map_err(|e| format!("failed to read settings: {e}"))?;
            let (settings, schema_version_present, raw_settings_json) =
                parse_settings_json(&content)?;

            if settings.preferred_port < 1024 {
                return Err(
                    "SEC_INVALID_INPUT: invalid settings.json: preferred_port must be between 1024 and 65535"
                        .to_string()
                        .into(),
                );
            }
            if settings.log_retention_days == 0 {
                return Err(
                    "SEC_INVALID_INPUT: invalid settings.json: log_retention_days must be >= 1"
                        .to_string()
                        .into(),
                );
            }

            // Best-effort migration: copy legacy settings into the new dotdir (do not delete legacy file).
            let mut settings = settings;
            let mut repaired = false;
            repaired |= migrate_disable_upstream_timeouts(&mut settings, schema_version_present);
            repaired |= migrate_add_gateway_rectifiers(&mut settings, schema_version_present);
            repaired |= migrate_add_circuit_breaker_notice(&mut settings, schema_version_present);
            repaired |=
                migrate_add_provider_base_url_ping_cache_ttl(&mut settings, schema_version_present);
            repaired |=
                migrate_add_codex_session_id_completion(&mut settings, schema_version_present);
            repaired |= migrate_add_gateway_network_settings(&mut settings, schema_version_present);
            repaired |= migrate_add_response_fixer_limits(&mut settings, schema_version_present);
            repaired |=
                migrate_add_cli_proxy_startup_recovery(&mut settings, schema_version_present);
            repaired |= migrate_add_cache_anomaly_monitor(&mut settings, schema_version_present);
            repaired |= migrate_add_wsl_host_address_mode(&mut settings, schema_version_present);
            repaired |= migrate_add_task_complete_notify(&mut settings, schema_version_present);
            repaired |= migrate_add_cch_base_config(&mut settings, schema_version_present);
            repaired |= migrate_add_start_minimized(&mut settings, schema_version_present);
            repaired |= migrate_add_show_home_heatmap(&mut settings, schema_version_present);
            repaired |= sanitize_failover_settings(&mut settings);
            repaired |= sanitize_circuit_breaker_settings(&mut settings);
            repaired |= sanitize_provider_cooldown_seconds(&mut settings);
            repaired |= sanitize_provider_base_url_ping_cache_ttl_seconds(&mut settings);
            repaired |= sanitize_upstream_timeouts(&mut settings);
            repaired |= sanitize_response_fixer_limits(&mut settings);
            let canonical = canonical_settings_json(&settings)?;
            repaired |= raw_settings_json != canonical;
            if repaired {
                // best-effort: persist sanitized defaults
            }
            let _ = write(app, &settings);

            if let Ok(mut guard) = cache.write() {
                *guard = Some(CachedSettings {
                    data: settings.clone(),
                    last_updated: Instant::now(),
                });
            }
            return Ok(settings);
        }

        let settings = AppSettings::default();
        // Best-effort: create default settings.json on first read to make the config discoverable/editable.
        let _ = write(app, &settings);

        if let Ok(mut guard) = cache.write() {
            *guard = Some(CachedSettings {
                data: settings.clone(),
                last_updated: Instant::now(),
            });
        }
        return Ok(settings);
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("failed to read settings: {e}"))?;
    let (mut settings, schema_version_present, raw_settings_json) = parse_settings_json(&content)?;

    if settings.preferred_port < 1024 {
        return Err(
            "SEC_INVALID_INPUT: invalid settings.json: preferred_port must be between 1024 and 65535"
                .to_string()
                .into(),
        );
    }
    if settings.log_retention_days == 0 {
        return Err(
            "SEC_INVALID_INPUT: invalid settings.json: log_retention_days must be >= 1"
                .to_string()
                .into(),
        );
    }

    let mut repaired = false;
    repaired |= migrate_disable_upstream_timeouts(&mut settings, schema_version_present);
    repaired |= migrate_add_gateway_rectifiers(&mut settings, schema_version_present);
    repaired |= migrate_add_circuit_breaker_notice(&mut settings, schema_version_present);
    repaired |= migrate_add_provider_base_url_ping_cache_ttl(&mut settings, schema_version_present);
    repaired |= migrate_add_codex_session_id_completion(&mut settings, schema_version_present);
    repaired |= migrate_add_gateway_network_settings(&mut settings, schema_version_present);
    repaired |= migrate_add_response_fixer_limits(&mut settings, schema_version_present);
    repaired |= migrate_add_cli_proxy_startup_recovery(&mut settings, schema_version_present);
    repaired |= migrate_add_cache_anomaly_monitor(&mut settings, schema_version_present);
    repaired |= migrate_add_wsl_host_address_mode(&mut settings, schema_version_present);
    repaired |= migrate_add_task_complete_notify(&mut settings, schema_version_present);
    repaired |= migrate_add_cch_base_config(&mut settings, schema_version_present);
    repaired |= migrate_add_start_minimized(&mut settings, schema_version_present);
    repaired |= migrate_add_show_home_heatmap(&mut settings, schema_version_present);
    repaired |= sanitize_failover_settings(&mut settings);
    repaired |= sanitize_circuit_breaker_settings(&mut settings);
    repaired |= sanitize_provider_cooldown_seconds(&mut settings);
    repaired |= sanitize_provider_base_url_ping_cache_ttl_seconds(&mut settings);
    repaired |= sanitize_upstream_timeouts(&mut settings);
    repaired |= sanitize_response_fixer_limits(&mut settings);
    let canonical = canonical_settings_json(&settings)?;
    repaired |= raw_settings_json != canonical;
    if repaired {
        // Best-effort: persist repaired values while keeping read semantics.
        let _ = write(app, &settings);
    }

    if let Ok(mut guard) = cache.write() {
        *guard = Some(CachedSettings {
            data: settings.clone(),
            last_updated: Instant::now(),
        });
    }

    Ok(settings)
}

pub fn log_retention_days_fail_open(app: &tauri::AppHandle) -> u32 {
    match read(app) {
        Ok(cfg) => cfg.log_retention_days,
        Err(err) => {
            if !LOG_RETENTION_DAYS_FAIL_OPEN_WARNED.swap(true, Ordering::Relaxed) {
                tracing::warn!(
                    default = DEFAULT_LOG_RETENTION_DAYS,
                    "settings read failed, using default log retention days: {}",
                    err
                );
            }
            DEFAULT_LOG_RETENTION_DAYS
        }
    }
}

pub fn write(app: &tauri::AppHandle, settings: &AppSettings) -> AppResult<AppSettings> {
    if settings.preferred_port < 1024 {
        return Err("SEC_INVALID_INPUT: preferred_port must be between 1024 and 65535".into());
    }
    if settings.log_retention_days == 0 {
        return Err("SEC_INVALID_INPUT: log_retention_days must be >= 1".into());
    }
    if settings.provider_cooldown_seconds > MAX_PROVIDER_COOLDOWN_SECONDS {
        return Err(format!(
            "SEC_INVALID_INPUT: provider_cooldown_seconds must be <= {MAX_PROVIDER_COOLDOWN_SECONDS}"
        )
        .into());
    }
    if settings.provider_base_url_ping_cache_ttl_seconds == 0 {
        return Err(
            "SEC_INVALID_INPUT: provider_base_url_ping_cache_ttl_seconds must be >= 1".into(),
        );
    }
    if settings.provider_base_url_ping_cache_ttl_seconds
        > MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
    {
        return Err(format!(
            "SEC_INVALID_INPUT: provider_base_url_ping_cache_ttl_seconds must be <= {MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS}"
        )
        .into());
    }
    if settings.upstream_first_byte_timeout_seconds > MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_first_byte_timeout_seconds must be <= {MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS}"
        )
        .into());
    }
    if settings.upstream_stream_idle_timeout_seconds > MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_stream_idle_timeout_seconds must be <= {MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS}"
        )
        .into());
    }
    if settings.upstream_request_timeout_non_streaming_seconds
        > MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS
    {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_request_timeout_non_streaming_seconds must be <= {MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS}"
        )
        .into());
    }
    if settings.response_fixer_max_json_depth == 0 {
        return Err("SEC_INVALID_INPUT: response_fixer_max_json_depth must be >= 1".into());
    }
    if settings.response_fixer_max_json_depth > MAX_RESPONSE_FIXER_MAX_JSON_DEPTH {
        return Err(format!(
            "SEC_INVALID_INPUT: response_fixer_max_json_depth must be <= {MAX_RESPONSE_FIXER_MAX_JSON_DEPTH}"
        )
        .into());
    }
    if settings.response_fixer_max_fix_size == 0 {
        return Err("SEC_INVALID_INPUT: response_fixer_max_fix_size must be >= 1".into());
    }
    if settings.response_fixer_max_fix_size > MAX_RESPONSE_FIXER_MAX_FIX_SIZE {
        return Err(format!(
            "SEC_INVALID_INPUT: response_fixer_max_fix_size must be <= {MAX_RESPONSE_FIXER_MAX_FIX_SIZE}"
        )
        .into());
    }
    if settings.failover_max_attempts_per_provider == 0 {
        return Err("SEC_INVALID_INPUT: failover_max_attempts_per_provider must be >= 1".into());
    }
    if settings.failover_max_providers_to_try == 0 {
        return Err("SEC_INVALID_INPUT: failover_max_providers_to_try must be >= 1".into());
    }
    if settings.failover_max_attempts_per_provider > MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER {
        return Err(format!(
            "failover_max_attempts_per_provider must be <= {MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER}"
        )
        .into());
    }
    if settings.failover_max_providers_to_try > MAX_FAILOVER_MAX_PROVIDERS_TO_TRY {
        return Err(format!(
            "failover_max_providers_to_try must be <= {MAX_FAILOVER_MAX_PROVIDERS_TO_TRY}"
        )
        .into());
    }
    if settings
        .failover_max_attempts_per_provider
        .saturating_mul(settings.failover_max_providers_to_try)
        > MAX_FAILOVER_TOTAL_ATTEMPTS
    {
        return Err(format!(
            "failover limits too high: failover_max_attempts_per_provider * failover_max_providers_to_try must be <= {MAX_FAILOVER_TOTAL_ATTEMPTS}"
        )
        .into());
    }

    if settings.circuit_breaker_failure_threshold == 0 {
        return Err("circuit_breaker_failure_threshold must be >= 1"
            .to_string()
            .into());
    }
    if settings.circuit_breaker_open_duration_minutes == 0 {
        return Err("circuit_breaker_open_duration_minutes must be >= 1"
            .to_string()
            .into());
    }
    if settings.circuit_breaker_failure_threshold > MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD {
        return Err(format!(
            "circuit_breaker_failure_threshold must be <= {MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD}"
        )
        .into());
    }
    if settings.circuit_breaker_open_duration_minutes > MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES {
        return Err(format!(
            "circuit_breaker_open_duration_minutes must be <= {MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES}"
        )
        .into());
    }

    let path = settings_path(app)?;
    let tmp_path = path.with_file_name("settings.json.tmp");
    let backup_path = path.with_file_name("settings.json.bak");

    let canonical = canonical_settings_json(settings)?;
    let content = serde_json::to_vec_pretty(&canonical)
        .map_err(|e| format!("failed to serialize settings: {e}"))?;

    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("failed to write temp settings file: {e}"))?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    if path.exists() {
        std::fs::rename(&path, &backup_path)
            .map_err(|e| format!("failed to create settings backup: {e}"))?;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        let _ = std::fs::rename(&backup_path, &path);
        return Err(format!("failed to finalize settings: {e}").into());
    }

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    let cache = SETTINGS_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = cache.write() {
        *guard = Some(CachedSettings {
            data: settings.clone(),
            last_updated: Instant::now(),
        });
    }

    Ok(settings.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // -- parse_settings_json --

    #[test]
    fn parse_settings_json_detects_schema_version_present() {
        let json = r#"{"schema_version": 14, "preferred_port": 37123}"#;
        let (settings, schema_version_present, _) = parse_settings_json(json).unwrap();
        assert!(schema_version_present);
        assert_eq!(settings.schema_version, 14);
        assert_eq!(settings.preferred_port, 37123);
    }

    #[test]
    fn parse_settings_json_detects_schema_version_absent() {
        let json = r#"{"preferred_port": 37123}"#;
        let (settings, schema_version_present, _) = parse_settings_json(json).unwrap();
        assert!(!schema_version_present);
        // schema_version defaults via serde
        assert_eq!(settings.preferred_port, 37123);
    }

    #[test]
    fn parse_settings_json_uses_defaults_for_missing_fields() {
        let json = r#"{}"#;
        let (settings, _, _) = parse_settings_json(json).unwrap();
        assert_eq!(settings.preferred_port, DEFAULT_GATEWAY_PORT);
        assert_eq!(settings.log_retention_days, DEFAULT_LOG_RETENTION_DAYS);
        assert!(settings.tray_enabled);
        assert!(!settings.auto_start);
    }

    #[test]
    fn parse_settings_json_rejects_invalid_json() {
        assert!(parse_settings_json("not json").is_err());
    }

    #[test]
    fn canonical_settings_json_drops_default_fields() {
        let canonical = canonical_settings_json(&AppSettings::default()).unwrap();
        assert_eq!(
            canonical,
            serde_json::json!({
                "schema_version": SCHEMA_VERSION
            })
        );
    }

    #[test]
    fn canonical_settings_json_keeps_non_default_fields() {
        let settings = AppSettings {
            auto_start: true,
            ..Default::default()
        };
        let canonical = canonical_settings_json(&settings).unwrap();
        assert_eq!(
            canonical,
            serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "auto_start": true
            })
        );
    }

    #[test]
    fn canonical_settings_json_detects_extra_default_fields() {
        let raw = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "preferred_port": DEFAULT_GATEWAY_PORT
        });
        let settings = AppSettings::default();
        let canonical = canonical_settings_json(&settings).unwrap();
        assert_ne!(raw, canonical);
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

    // -- GatewayListenMode --

    #[test]
    fn gateway_listen_mode_default_is_localhost() {
        assert_eq!(GatewayListenMode::default(), GatewayListenMode::Localhost);
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
}
