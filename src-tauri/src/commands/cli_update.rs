//! Usage: Tauri commands for checking and updating CLI installations.

use crate::cli_update as cli_update_infra;

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_check_latest_version(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<cli_update_infra::CliVersionCheck, String> {
    Ok(cli_update_infra::cli_check_latest_version(&app, cli_key).await)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_update(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<cli_update_infra::CliUpdateResult, String> {
    Ok(cli_update_infra::cli_update(&app, cli_key).await)
}
