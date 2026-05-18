//! Usage: WebDAV sync Tauri commands (test, upload, download).

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::infra::config_migrate;
use crate::webdav::{WebDavConfig, WebDavDownloadResult, WebDavTestResult, WebDavUploadResult};
use crate::{blocking, db, settings};

#[tauri::command]
#[specta::specta]
pub(crate) async fn webdav_test(config: WebDavConfig) -> Result<WebDavTestResult, String> {
    crate::webdav::webdav_test_connection(&config)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn webdav_upload_sync(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    config: WebDavConfig,
) -> Result<WebDavUploadResult, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    // Export config bundle (same as file export)
    let bundle = blocking::run("webdav_upload_export", {
        let app = app.clone();
        move || config_migrate::config_export(&app, &db)
    })
    .await
    .map_err(|e| -> String { e.into() })?;

    let data = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("SYSTEM_ERROR: failed to serialize config: {e}"))?;

    crate::webdav::webdav_upload(&config, &data)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn webdav_download_sync(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    config: WebDavConfig,
) -> Result<config_migrate::ConfigImportResult, String> {
    let download_result = crate::webdav::webdav_download(&config)
        .await
        .map_err(|e| -> String { e.into() })?;

    if !download_result.success {
        return Err(download_result.message);
    }

    let raw = download_result
        .data
        .ok_or_else(|| "SYSTEM_ERROR: download succeeded but no data returned".to_string())?;

    let bundle: config_migrate::ConfigBundle = serde_json::from_str(&raw)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid sync data: {e}"))?;

    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    blocking::run("webdav_download_import", move || {
        config_migrate::config_import(&app, &db, bundle)
    })
    .await
    .map_err(Into::into)
}
