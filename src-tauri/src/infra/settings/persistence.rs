//! Usage: Settings file read/write, cache layer, path resolution, and JSON parsing.

use super::defaults::*;
use super::migration::{
    normalize_cli_priority_order, normalize_codex_home_override, repair_settings,
};
use super::types::{AppSettings, CodexHomeMode};
use crate::app_paths;
use crate::shared::error::AppResult;
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
            let repaired =
                repair_settings(&mut settings, schema_version_present, &raw_settings_json)?;
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

    let repaired = repair_settings(&mut settings, schema_version_present, &raw_settings_json)?;
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

pub fn write<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &AppSettings,
) -> AppResult<AppSettings> {
    let mut settings = settings.clone();
    settings.cli_priority_order = normalize_cli_priority_order(&settings.cli_priority_order);
    settings.codex_home_override = normalize_codex_home_override(&settings.codex_home_override);
    if settings.codex_home_mode != CodexHomeMode::Custom {
        settings.codex_home_override.clear();
    }
    if settings.codex_home_mode == CodexHomeMode::Custom && settings.codex_home_override.is_empty()
    {
        settings.codex_home_mode = CodexHomeMode::UserHomeDefault;
    }

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

    let canonical = canonical_settings_json(&settings)?;
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
}
