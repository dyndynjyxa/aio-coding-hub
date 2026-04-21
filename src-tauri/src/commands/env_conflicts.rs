//! Usage: Environment variable conflict detection related Tauri commands.

use crate::{blocking, env_conflicts};

#[tauri::command]
#[specta::specta]
pub(crate) async fn env_conflicts_check(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<Vec<env_conflicts::EnvConflict>, String> {
    blocking::run("env_conflicts_check", move || {
        env_conflicts::check_env_conflicts(&app, &cli_key)
    })
    .await
    .map_err(Into::into)
}
