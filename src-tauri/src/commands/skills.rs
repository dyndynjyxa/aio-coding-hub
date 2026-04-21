//! Usage: Skills management related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, skills};

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repos_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<skills::SkillRepoSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skill_repos_list", move || skills::repos_list(&db))
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repo_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    repo_id: Option<i64>,
    git_url: String,
    branch: String,
    enabled: bool,
) -> Result<skills::SkillRepoSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skill_repo_upsert", move || {
        skills::repo_upsert(&db, repo_id, &git_url, &branch, enabled)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_repo_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    repo_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run(
        "skill_repo_delete",
        move || -> crate::shared::error::AppResult<bool> {
            skills::repo_delete(&db, repo_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_installed_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::InstalledSkillSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("skills_installed_list", move || {
        skills::installed_list_for_workspace(&db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_discover_available(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    refresh: bool,
) -> Result<Vec<skills::AvailableSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || skills::discover_available(&app, &db, refresh))
        .await
        .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn skill_install(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    git_url: String,
    branch: String,
    source_subdir: String,
    enabled: bool,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::install(
            &app,
            &db,
            workspace_id,
            &git_url,
            &branch,
            &source_subdir,
            enabled,
        )
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_install_to_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    git_url: String,
    branch: String,
    source_subdir: String,
) -> Result<skills::LocalSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::install_to_local(&app, &db, workspace_id, &git_url, &branch, &source_subdir)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
    enabled: bool,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::set_enabled(&app, &db, workspace_id, skill_id, enabled)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_uninstall(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    skill_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || skills::uninstall(&app, &db, skill_id))
        .await
        .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))??;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_return_to_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::return_to_local(&app, &db, workspace_id, skill_id)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))??;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_local_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::LocalSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || skills::local_list(&app, &db, workspace_id))
        .await
        .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_local_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_name: String,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::delete_local(&app, &db, workspace_id, &dir_name)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))??;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_import_local(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_name: String,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::import_local(&app, &db, workspace_id, &dir_name)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_import_local_batch(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_names: Vec<String>,
) -> Result<skills::SkillImportLocalBatchReport, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::import_local_batch(&app, &db, workspace_id, dir_names)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_paths_get(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<skills::SkillsPaths, String> {
    blocking::run("skills_paths_get", move || {
        skills::paths_get(&app, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_check_updates(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::SkillUpdateInfo>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::check_updates_for_workspace(&app, &db, workspace_id)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_update(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    skill_id: i64,
) -> Result<skills::InstalledSkillSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tauri::async_runtime::spawn_blocking(move || {
        skills::update_skill(&app, &db, workspace_id, skill_id)
    })
    .await
    .map_err(|e| format!("SKILL_TASK_JOIN: {e}"))?
    .map_err(Into::into)
}
