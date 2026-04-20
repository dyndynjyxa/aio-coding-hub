//! Usage: Workspace (profile) related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, workspace_switch, workspaces};

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspaces_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<workspaces::WorkspacesListResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("workspaces_list", move || {
        workspaces::list_by_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspace_create(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    name: String,
    clone_from_active: Option<bool>,
) -> Result<workspaces::WorkspaceSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("workspace_create", move || {
        workspaces::create(&db, &cli_key, &name, clone_from_active.unwrap_or(false))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspace_rename(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    name: String,
) -> Result<workspaces::WorkspaceSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("workspace_rename", move || {
        workspaces::rename(&db, workspace_id, &name)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspace_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("workspace_delete", move || {
        workspaces::delete(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspace_preview(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<workspace_switch::WorkspacePreview, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("workspace_preview", move || {
        workspace_switch::preview(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn workspace_apply(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<workspace_switch::WorkspaceApplyReport, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("workspace_apply", move || {
        workspace_switch::apply(&app, &db, workspace_id)
    })
    .await
    .map_err(Into::into)
}
