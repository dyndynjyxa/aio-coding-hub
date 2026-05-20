use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::infra::config_migrate;
use crate::shared::error::AppError;
use crate::shared::fs::read_file_with_max_len;
use crate::shared::ipc_confirm::RiskyIpcConfirm;
use std::path::Path;

fn map_config_import_read_error(err: AppError) -> String {
    let message = err.to_string();
    if message.starts_with("SEC_INVALID_INPUT:") {
        message
    } else if let Some(message) = message.strip_prefix("INTERNAL_ERROR: ") {
        format!("SYSTEM_ERROR: failed to read config import file: {message}")
    } else {
        format!("SYSTEM_ERROR: failed to read config import file: {message}")
    }
}

fn read_config_import_bundle_with_max_len(
    file_path: &str,
    max_len: usize,
) -> Result<config_migrate::ConfigBundle, String> {
    let bytes = read_file_with_max_len(Path::new(file_path), max_len)
        .map_err(map_config_import_read_error)?;
    let raw = String::from_utf8(bytes)
        .map_err(|err| format!("SEC_INVALID_INPUT: config import file must be UTF-8: {err}"))?;
    serde_json::from_str(&raw)
        .map_err(|err| format!("SEC_INVALID_INPUT: invalid config import json: {err}"))
}

fn read_config_import_bundle(file_path: &str) -> Result<config_migrate::ConfigBundle, String> {
    read_config_import_bundle_with_max_len(file_path, config_migrate::CONFIG_IMPORT_FILE_MAX_BYTES)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn config_export(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    file_path: String,
) -> Result<bool, String> {
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("SEC_INVALID_INPUT: file_path is required".to_string());
    }
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("config_export", move || {
        let bundle = config_migrate::config_export(&app, &db)?;
        let content = serde_json::to_string_pretty(&bundle)
            .map_err(|err| format!("SYSTEM_ERROR: failed to serialize config export: {err}"))?;
        std::fs::write(&file_path, content)
            .map_err(|err| format!("SYSTEM_ERROR: failed to write config export file: {err}"))?;
        Ok::<bool, String>(true)
    })
    .await;
    result.map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn config_import(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    file_path: String,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<config_migrate::ConfigImportResult, String> {
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("SEC_INVALID_INPUT: file_path is required".to_string());
    }
    RiskyIpcConfirm::require(confirm, "config_import", file_path.clone())?;
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("config_import", move || {
        let bundle = read_config_import_bundle(&file_path)?;
        config_migrate::config_import(&app, &db, bundle)
    })
    .await
    .map_err(|err| -> String { err.into() })?;

    #[cfg(windows)]
    super::wsl::wsl_sync_trigger::trigger(app_for_wsl);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_file(name: &str, bytes: &[u8]) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).expect("write temp file");
        (dir, path.to_string_lossy().to_string())
    }

    #[test]
    fn read_config_import_bundle_accepts_valid_json() {
        let raw = serde_json::json!({
            "schema_version": config_migrate::CONFIG_BUNDLE_SCHEMA_VERSION,
            "exported_at": "2026-05-19T00:00:00.000Z",
            "app_version": "0.0.0-test",
            "settings": "{}",
            "providers": [],
            "sort_modes": [],
            "sort_mode_active": {},
            "workspaces": [],
            "mcp_servers": [],
            "skill_repos": [],
            "installed_skills": [],
            "local_skills": []
        })
        .to_string();
        let (_dir, path) = write_temp_file("config.json", raw.as_bytes());

        let bundle = read_config_import_bundle_with_max_len(&path, 4096).expect("bundle");

        assert_eq!(
            bundle.schema_version,
            config_migrate::CONFIG_BUNDLE_SCHEMA_VERSION
        );
    }

    #[test]
    fn read_config_import_bundle_rejects_oversized_file() {
        let (_dir, path) = write_temp_file("config.json", b"{\"schema_version\":2}");

        let err = read_config_import_bundle_with_max_len(&path, 4)
            .err()
            .expect("oversized import file should fail");

        assert!(err.contains("SEC_INVALID_INPUT:"));
        assert!(err.contains("too large"));
    }

    #[test]
    fn read_config_import_bundle_rejects_invalid_utf8() {
        let (_dir, path) = write_temp_file("config.json", &[0xff]);

        let err = read_config_import_bundle_with_max_len(&path, 16)
            .err()
            .expect("invalid utf8 should fail");

        assert!(err.contains("SEC_INVALID_INPUT: config import file must be UTF-8"));
    }
}
