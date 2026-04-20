//! Usage: Data reset / disk usage related Tauri commands.

use crate::app_state::{ensure_db_ready, prepare_db_reset, DbInitState, GatewayState};
use crate::{app_paths, blocking, data_management};

#[tauri::command]
#[specta::specta]
pub(crate) async fn app_data_dir_get(app: tauri::AppHandle) -> Result<String, String> {
    blocking::run(
        "app_data_dir_get",
        move || -> crate::shared::error::AppResult<String> {
            let dir = app_paths::app_data_dir(&app)?;
            Ok(dir.to_string_lossy().to_string())
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn db_disk_usage_get(
    app: tauri::AppHandle,
) -> Result<data_management::DbDiskUsage, String> {
    blocking::run("db_disk_usage_get", move || {
        data_management::db_disk_usage_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_clear_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<data_management::ClearRequestLogsResult, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("request_logs_clear_all", move || {
        data_management::request_logs_clear_all(&db)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn app_data_reset(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    state: tauri::State<'_, GatewayState>,
) -> Result<bool, String> {
    // Best-effort: stop gateway first to avoid concurrent writes locking sqlite files.
    let _ = super::gateway_stop(app.clone(), state).await;
    let _db_reset_guard = prepare_db_reset(db_state.inner()).await;
    blocking::run("app_data_reset", move || {
        data_management::app_data_reset(&app)
    })
    .await
    .map_err(Into::into)
}
