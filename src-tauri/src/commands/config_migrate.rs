use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::infra::config_migrate;

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
) -> Result<config_migrate::ConfigImportResult, String> {
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("SEC_INVALID_INPUT: file_path is required".to_string());
    }
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("config_import", move || {
        let raw = std::fs::read_to_string(&file_path)
            .map_err(|err| format!("SYSTEM_ERROR: failed to read config import file: {err}"))?;
        let bundle: config_migrate::ConfigBundle = serde_json::from_str(&raw)
            .map_err(|err| format!("SEC_INVALID_INPUT: invalid config import json: {err}"))?;
        config_migrate::config_import(&app, &db, bundle)
    })
    .await
    .map_err(|err| -> String { err.into() })?;

    #[cfg(windows)]
    super::wsl::wsl_sync_trigger::trigger(app_for_wsl);

    Ok(result)
}
