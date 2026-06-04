//! Usage: Settings file read/write, cache layer, path resolution, and JSON parsing.

use super::defaults::*;
use super::migration::{
    normalize_cli_priority_order, normalize_codex_home_override, repair_settings,
};
use super::types::{AppSettings, CodexHomeMode, GatewayListenMode, WslHostAddressMode};
use crate::app_paths;
use crate::shared::error::AppResult;
use crate::shared::fs::read_file_with_max_len;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Instant;
use tauri::Manager;

static LOG_RETENTION_DAYS_FAIL_OPEN_WARNED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct CachedSettings {
    path: PathBuf,
    data: AppSettings,
    last_updated: Instant,
}

static SETTINGS_CACHE: OnceLock<RwLock<Option<CachedSettings>>> = OnceLock::new();

fn cache_settings(path: &Path, settings: &AppSettings) {
    let cache = SETTINGS_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = cache.write() {
        *guard = Some(CachedSettings {
            path: path.to_path_buf(),
            data: settings.clone(),
            last_updated: Instant::now(),
        });
    }
}

fn settings_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join("settings.json"))
}

fn legacy_settings_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    let config_dir = app
        .path()
        .config_dir()
        .map_err(|e| format!("failed to resolve legacy config dir: {e}"))?;

    Ok(config_dir.join(LEGACY_IDENTIFIER).join("settings.json"))
}

fn invalid_settings_json(reason: impl std::fmt::Display) -> crate::shared::error::AppError {
    format!("SEC_INVALID_INPUT: invalid settings.json: {reason}").into()
}

fn read_settings_json_file(path: &Path) -> AppResult<String> {
    let bytes = read_file_with_max_len(path, SETTINGS_FILE_MAX_BYTES)
        .map_err(|e| format!("failed to read settings: {e}"))?;
    String::from_utf8(bytes).map_err(|e| invalid_settings_json(format!("expected UTF-8: {e}")))
}

fn ensure_settings_file_len(bytes: &[u8]) -> AppResult<()> {
    if bytes.len() > SETTINGS_FILE_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: settings.json too large (max {SETTINGS_FILE_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

fn validate_update_releases_url(value: &str) -> AppResult<()> {
    let raw = value.trim();
    if raw.is_empty() {
        return Ok(());
    }
    if raw.len() > MAX_UPDATE_RELEASES_URL_LEN {
        return Err(format!(
            "SEC_INVALID_INPUT: update_releases_url must be <= {MAX_UPDATE_RELEASES_URL_LEN} characters"
        )
        .into());
    }

    let parsed = reqwest::Url::parse(raw)
        .map_err(|err| format!("SEC_INVALID_INPUT: invalid update_releases_url: {err}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("SEC_INVALID_INPUT: update_releases_url must use http or https".into());
    }
    if parsed.host_str().is_none() {
        return Err("SEC_INVALID_INPUT: update_releases_url must include a host".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("SEC_INVALID_INPUT: update_releases_url must not include credentials".into());
    }

    Ok(())
}

fn validate_no_control_chars(field: &str, value: &str) -> AppResult<()> {
    if value.chars().any(char::is_control) {
        return Err(
            format!("SEC_INVALID_INPUT: {field} must not contain control characters").into(),
        );
    }
    Ok(())
}

fn validate_non_empty_bounded_string(field: &str, value: &str, max_len: usize) -> AppResult<()> {
    let raw = value.trim();
    if raw.is_empty() {
        return Err(format!("SEC_INVALID_INPUT: {field} cannot be empty").into());
    }
    if raw.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {field} must be <= {max_len} characters").into());
    }
    validate_no_control_chars(field, raw)
}

fn validate_optional_bounded_string(field: &str, value: &str, max_len: usize) -> AppResult<()> {
    let raw = value.trim();
    if raw.is_empty() {
        return Ok(());
    }
    if raw.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {field} must be <= {max_len} characters").into());
    }
    validate_no_control_chars(field, raw)
}

pub(super) fn parse_settings_json(
    content: &str,
) -> AppResult<(AppSettings, bool, serde_json::Value)> {
    let raw: serde_json::Value = serde_json::from_str(content).map_err(invalid_settings_json)?;
    let schema_version_present = raw.get("schema_version").is_some();
    let settings: AppSettings =
        serde_json::from_value(raw.clone()).map_err(invalid_settings_json)?;
    Ok((settings, schema_version_present, raw))
}

pub(super) fn canonical_settings_json(settings: &AppSettings) -> AppResult<serde_json::Value> {
    let mut serialized =
        serde_json::to_value(settings).map_err(|e| format!("failed to serialize settings: {e}"))?;

    let serialized_obj = serialized.as_object_mut().ok_or_else(|| {
        "failed to serialize settings: expected settings to serialize as an object".to_string()
    })?;

    // Keep the full managed snapshot on disk so future default changes do not
    // reinterpret omitted keys as a new user choice after an upgrade.
    if !serialized_obj.contains_key("schema_version") {
        serialized_obj.insert(
            "schema_version".to_string(),
            serde_json::json!(SCHEMA_VERSION),
        );
    }

    Ok(serialized)
}

pub fn read<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<AppSettings> {
    let cache = SETTINGS_CACHE.get_or_init(|| RwLock::new(None));
    let path = settings_path(app)?;

    if let Ok(guard) = cache.read() {
        if let Some(cached) = guard.as_ref() {
            if cached.path == path && cached.last_updated.elapsed() < CACHE_TTL {
                return Ok(cached.data.clone());
            }
        }
    }

    if !path.exists() {
        let legacy_path = legacy_settings_path(app)?;
        if legacy_path.exists() {
            let content = read_settings_json_file(&legacy_path)?;
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
            let repaired =
                repair_settings(&mut settings, schema_version_present, &raw_settings_json)?;
            validate_bounds(&settings)?;
            if repaired {
                // best-effort: persist sanitized defaults
            }
            let _ = write(app, &settings);
            cache_settings(&path, &settings);
            return Ok(settings);
        }

        let settings = AppSettings::default();
        // Best-effort: create default settings.json on first read to make the config discoverable/editable.
        let _ = write(app, &settings);
        cache_settings(&path, &settings);
        return Ok(settings);
    }

    let content = read_settings_json_file(&path)?;
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

    let repaired = repair_settings(&mut settings, schema_version_present, &raw_settings_json)?;
    validate_bounds(&settings)?;
    if repaired {
        // Best-effort: persist repaired values while keeping read semantics.
        let _ = write(app, &settings);
    }
    cache_settings(&path, &settings);

    Ok(settings)
}

pub fn log_retention_days_fail_open<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> u32 {
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

pub(crate) fn validate_bounds(settings: &AppSettings) -> AppResult<()> {
    if settings.preferred_port < 1024 {
        return Err("SEC_INVALID_INPUT: preferred_port must be between 1024 and 65535".into());
    }
    if settings.gateway_listen_mode == GatewayListenMode::Custom {
        crate::shared::listen_address::parse_custom_listen_address(
            &settings.gateway_custom_listen_address,
        )?;
    }
    if settings.wsl_host_address_mode == WslHostAddressMode::Custom {
        crate::shared::listen_address::parse_custom_host_address(
            &settings.wsl_custom_host_address,
        )?;
    }
    if settings.upstream_proxy_url.len() > MAX_UPSTREAM_PROXY_URL_LEN {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_proxy_url must be <= {MAX_UPSTREAM_PROXY_URL_LEN} characters"
        )
        .into());
    }
    if settings.upstream_proxy_username.len() > MAX_UPSTREAM_PROXY_USERNAME_LEN {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_proxy_username must be <= {MAX_UPSTREAM_PROXY_USERNAME_LEN} characters"
        )
        .into());
    }
    if settings.upstream_proxy_password.len() > MAX_UPSTREAM_PROXY_PASSWORD_LEN {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_proxy_password must be <= {MAX_UPSTREAM_PROXY_PASSWORD_LEN} characters"
        )
        .into());
    }
    for (field, value) in [
        (
            "cx2cc_fallback_model_opus",
            settings.cx2cc_fallback_model_opus.as_str(),
        ),
        (
            "cx2cc_fallback_model_sonnet",
            settings.cx2cc_fallback_model_sonnet.as_str(),
        ),
        (
            "cx2cc_fallback_model_haiku",
            settings.cx2cc_fallback_model_haiku.as_str(),
        ),
        (
            "cx2cc_fallback_model_main",
            settings.cx2cc_fallback_model_main.as_str(),
        ),
    ] {
        validate_non_empty_bounded_string(field, value, MAX_CX2CC_MODEL_NAME_LEN)?;
    }
    validate_optional_bounded_string(
        "cx2cc_model_reasoning_effort",
        &settings.cx2cc_model_reasoning_effort,
        MAX_CX2CC_OPTIONAL_FIELD_LEN,
    )?;
    validate_optional_bounded_string(
        "cx2cc_service_tier",
        &settings.cx2cc_service_tier,
        MAX_CX2CC_OPTIONAL_FIELD_LEN,
    )?;
    validate_update_releases_url(&settings.update_releases_url)?;
    if settings.log_retention_days == 0 {
        return Err("SEC_INVALID_INPUT: log_retention_days must be >= 1".into());
    }
    if settings.log_retention_days > MAX_LOG_RETENTION_DAYS {
        return Err(format!(
            "SEC_INVALID_INPUT: log_retention_days must be <= {MAX_LOG_RETENTION_DAYS}"
        )
        .into());
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
    if settings.upstream_stream_idle_timeout_seconds > 0
        && settings.upstream_stream_idle_timeout_seconds < MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS
    {
        return Err(format!(
            "SEC_INVALID_INPUT: upstream_stream_idle_timeout_seconds must be 0 (disabled) or >= {MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS}"
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
            "SEC_INVALID_INPUT: failover_max_attempts_per_provider must be <= {MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER}"
        )
        .into());
    }
    if settings.failover_max_providers_to_try > MAX_FAILOVER_MAX_PROVIDERS_TO_TRY {
        return Err(format!(
            "SEC_INVALID_INPUT: failover_max_providers_to_try must be <= {MAX_FAILOVER_MAX_PROVIDERS_TO_TRY}"
        )
        .into());
    }
    if settings
        .failover_max_attempts_per_provider
        .saturating_mul(settings.failover_max_providers_to_try)
        > MAX_FAILOVER_TOTAL_ATTEMPTS
    {
        return Err(format!(
            "SEC_INVALID_INPUT: failover limits too high: failover_max_attempts_per_provider * failover_max_providers_to_try must be <= {MAX_FAILOVER_TOTAL_ATTEMPTS}"
        )
        .into());
    }

    if settings.circuit_breaker_failure_threshold == 0 {
        return Err("SEC_INVALID_INPUT: circuit_breaker_failure_threshold must be >= 1".into());
    }
    if settings.circuit_breaker_open_duration_minutes == 0 {
        return Err("SEC_INVALID_INPUT: circuit_breaker_open_duration_minutes must be >= 1".into());
    }
    if settings.circuit_breaker_failure_threshold > MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD {
        return Err(format!(
            "SEC_INVALID_INPUT: circuit_breaker_failure_threshold must be <= {MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD}"
        )
        .into());
    }
    if settings.circuit_breaker_open_duration_minutes > MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES {
        return Err(format!(
            "SEC_INVALID_INPUT: circuit_breaker_open_duration_minutes must be <= {MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES}"
        )
        .into());
    }

    Ok(())
}

pub fn write<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &AppSettings,
) -> AppResult<AppSettings> {
    let mut settings = settings.clone();
    settings.cli_priority_order = normalize_cli_priority_order(&settings.cli_priority_order);
    settings.update_releases_url = settings.update_releases_url.trim().to_string();
    settings.upstream_proxy_url = settings.upstream_proxy_url.trim().to_string();
    settings.upstream_proxy_username = settings.upstream_proxy_username.trim().to_string();
    settings.cx2cc_fallback_model_opus = settings.cx2cc_fallback_model_opus.trim().to_string();
    settings.cx2cc_fallback_model_sonnet = settings.cx2cc_fallback_model_sonnet.trim().to_string();
    settings.cx2cc_fallback_model_haiku = settings.cx2cc_fallback_model_haiku.trim().to_string();
    settings.cx2cc_fallback_model_main = settings.cx2cc_fallback_model_main.trim().to_string();
    settings.cx2cc_model_reasoning_effort =
        settings.cx2cc_model_reasoning_effort.trim().to_string();
    settings.cx2cc_service_tier = settings.cx2cc_service_tier.trim().to_string();
    settings.codex_home_override = normalize_codex_home_override(&settings.codex_home_override);
    if settings.codex_home_mode != CodexHomeMode::Custom {
        settings.codex_home_override.clear();
    }
    if settings.codex_home_mode == CodexHomeMode::Custom && settings.codex_home_override.is_empty()
    {
        settings.codex_home_mode = CodexHomeMode::UserHomeDefault;
    }

    validate_bounds(&settings)?;

    let path = settings_path(app)?;
    let tmp_path = path.with_file_name("settings.json.tmp");
    let backup_path = path.with_file_name("settings.json.bak");

    let canonical = canonical_settings_json(&settings)?;
    let content = serde_json::to_vec_pretty(&canonical)
        .map_err(|e| format!("failed to serialize settings: {e}"))?;
    ensure_settings_file_len(&content)?;

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

    cache_settings(&path, &settings);

    Ok(settings)
}

/// Clear the in-process settings cache.  Only available for integration tests
/// where each `TestApp` uses a distinct temp directory.
pub fn clear_cache() {
    let cache = SETTINGS_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = cache.write() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn canonical_settings_json_keeps_default_fields() {
        let canonical = canonical_settings_json(&AppSettings::default()).unwrap();
        let serialized_defaults = serde_json::to_value(AppSettings::default()).unwrap();
        assert_eq!(canonical, serialized_defaults);
    }

    #[test]
    fn canonical_settings_json_keeps_non_default_fields() {
        let settings = AppSettings {
            auto_start: true,
            ..Default::default()
        };
        let canonical = canonical_settings_json(&settings).unwrap();
        let expected = serde_json::to_value(settings).unwrap();
        assert_eq!(canonical, expected);
    }

    #[test]
    fn canonical_settings_json_detects_truncated_snapshots() {
        let raw = serde_json::json!({
            "schema_version": SCHEMA_VERSION
        });
        let settings = AppSettings::default();
        let canonical = canonical_settings_json(&settings).unwrap();
        assert_ne!(raw, canonical);
    }

    #[test]
    fn validate_bounds_rejects_log_retention_above_cap() {
        let settings = AppSettings {
            log_retention_days: MAX_LOG_RETENTION_DAYS + 1,
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("log_retention_days must be <="));
    }

    #[test]
    fn validate_bounds_rejects_excessive_failover_product() {
        let settings = AppSettings {
            failover_max_attempts_per_provider: 20,
            failover_max_providers_to_try: 20,
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("failover limits too high"));
    }

    #[test]
    fn validate_bounds_rejects_invalid_custom_listen_address() {
        let settings = AppSettings {
            gateway_listen_mode: GatewayListenMode::Custom,
            gateway_custom_listen_address: "http://127.0.0.1:37123".to_string(),
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("custom listen address must be host or host:port"));
    }

    #[test]
    fn validate_bounds_rejects_invalid_wsl_custom_host_address() {
        let settings = AppSettings {
            wsl_host_address_mode: WslHostAddressMode::Custom,
            wsl_custom_host_address: "127.0.0.1:37123".to_string(),
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("custom host address"));
    }

    #[test]
    fn validate_bounds_rejects_non_http_update_releases_url() {
        let settings = AppSettings {
            update_releases_url: "file:///tmp/releases.json".to_string(),
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("update_releases_url must use http or https"));
    }

    #[test]
    fn validate_bounds_rejects_credentialed_update_releases_url() {
        let settings = AppSettings {
            update_releases_url: "https://user:secret@example.invalid/releases".to_string(),
            ..Default::default()
        };

        let err = validate_bounds(&settings).unwrap_err().to_string();

        assert!(err.contains("update_releases_url must not include credentials"));
    }

    #[test]
    fn validate_bounds_rejects_oversized_upstream_proxy_fields() {
        let settings = AppSettings {
            upstream_proxy_url: "x".repeat(MAX_UPSTREAM_PROXY_URL_LEN + 1),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("upstream_proxy_url must be <="));

        let settings = AppSettings {
            upstream_proxy_username: "x".repeat(MAX_UPSTREAM_PROXY_USERNAME_LEN + 1),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("upstream_proxy_username must be <="));

        let settings = AppSettings {
            upstream_proxy_password: "x".repeat(MAX_UPSTREAM_PROXY_PASSWORD_LEN + 1),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("upstream_proxy_password must be <="));
    }

    #[test]
    fn validate_bounds_rejects_invalid_cx2cc_strings() {
        let settings = AppSettings {
            cx2cc_fallback_model_main: " ".to_string(),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("cx2cc_fallback_model_main cannot be empty"));

        let settings = AppSettings {
            cx2cc_fallback_model_main: "x".repeat(MAX_CX2CC_MODEL_NAME_LEN + 1),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("cx2cc_fallback_model_main must be <="));

        let settings = AppSettings {
            cx2cc_service_tier: "priority\ninjected".to_string(),
            ..Default::default()
        };
        let err = validate_bounds(&settings).unwrap_err().to_string();
        assert!(err.contains("cx2cc_service_tier must not contain control characters"));
    }
}
