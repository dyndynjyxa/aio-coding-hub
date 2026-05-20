//! Usage: Skills management related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::shared::cli_key::CliKey;
use crate::shared::ipc_confirm::RiskyIpcConfirm;
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
    blocking::run("skills_discover_available", move || {
        skills::discover_available(&app, &db, refresh)
    })
    .await
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
    blocking::run("skill_install", move || {
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
    blocking::run("skill_install_to_local", move || {
        skills::install_to_local(&app, &db, workspace_id, &git_url, &branch, &source_subdir)
    })
    .await
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
    blocking::run("skill_set_enabled", move || {
        skills::set_enabled(&app, &db, workspace_id, skill_id, enabled)
    })
    .await
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
    blocking::run(
        "skill_uninstall",
        move || -> crate::shared::error::AppResult<bool> {
            skills::uninstall(&app, &db, skill_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
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
    blocking::run(
        "skill_return_to_local",
        move || -> crate::shared::error::AppResult<bool> {
            skills::return_to_local(&app, &db, workspace_id, skill_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_local_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::LocalSkillSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skills_local_list", move || {
        skills::local_list(&app, &db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_local_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
    dir_name: String,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<bool, String> {
    RiskyIpcConfirm::require(
        confirm,
        "skill_local_delete",
        format!("workspace:{workspace_id}:skill-local:{dir_name}"),
    )?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run(
        "skill_local_delete",
        move || -> crate::shared::error::AppResult<bool> {
            skills::delete_local(&app, &db, workspace_id, &dir_name)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into)
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
    blocking::run("skill_import_local", move || {
        skills::import_local(&app, &db, workspace_id, &dir_name)
    })
    .await
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
    blocking::run("skills_import_local_batch", move || {
        skills::import_local_batch(&app, &db, workspace_id, dir_names)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skills_paths_get(
    app: tauri::AppHandle,
    cli_key: String,
) -> Result<skills::SkillsPaths, String> {
    let cli_key = normalize_skills_cli_key(&cli_key)?;

    blocking::run("skills_paths_get", move || {
        skills::paths_get(&app, &cli_key)
    })
    .await
    .map_err(Into::into)
}

fn normalize_skills_cli_key(cli_key: &str) -> Result<String, String> {
    Ok(CliKey::parse(cli_key.trim())
        .map_err(String::from)?
        .to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn skill_check_updates(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    workspace_id: i64,
) -> Result<Vec<skills::SkillUpdateInfo>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("skill_check_updates", move || {
        skills::check_updates_for_workspace(&app, &db, workspace_id)
    })
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_skills_cli_key_trims_supported_keys() {
        assert_eq!(
            normalize_skills_cli_key(" claude ").expect("valid cli key"),
            "claude"
        );
    }

    #[test]
    fn normalize_skills_cli_key_rejects_invalid_keys() {
        let err = normalize_skills_cli_key(" opencode ").expect_err("invalid cli key");
        assert_eq!(err, "SEC_INVALID_INPUT: unknown cli_key=opencode");
    }
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
    blocking::run("skill_update", move || {
        skills::update_skill(&app, &db, workspace_id, skill_id)
    })
    .await
    .map_err(Into::into)
}
