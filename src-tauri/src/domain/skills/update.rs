//! Usage: Skill update detection and execution.

use super::git_url::{normalize_repo_branch, parse_github_owner_repo};
use super::installed::{get_skill_by_id_for_workspace, installed_list_for_workspace};
use super::ops::{install, uninstall};
use super::repo_cache::{ensure_repo_cache, get_repo_head_commit, github_get_branch_commit};
use super::types::{InstalledSkillSummary, SkillUpdateInfo};
use crate::db;
use rusqlite::params;

/// Check if a source_git_url is a remote market skill (http/https).
/// Local skills (local://) are not eligible for update checks.
fn is_market_skill(source_git_url: &str) -> bool {
    let url = source_git_url.trim().to_lowercase();
    url.starts_with("http://") || url.starts_with("https://")
}

/// Get the latest commit for a skill from its source repository.
/// For GitHub repos, uses the API to get the branch commit.
/// For git repos, refreshes the cache and reads HEAD.
fn get_latest_commit_for_skill(
    app: &tauri::AppHandle,
    git_url: &str,
    branch: &str,
) -> crate::shared::error::AppResult<String> {
    let normalized_branch = normalize_repo_branch(branch);

    // For GitHub repos, try using the API first (works for snapshot mode).
    if let Some((owner, repo)) = parse_github_owner_repo(git_url) {
        // Determine the effective branch. If "auto", try common defaults.
        let effective_branch = if normalized_branch == "auto" {
            // Try main first, then master. If both fail, return error.
            match github_get_branch_commit(&owner, &repo, "main") {
                Ok(commit) => return Ok(commit),
                Err(_) => match github_get_branch_commit(&owner, &repo, "master") {
                    Ok(commit) => return Ok(commit),
                    Err(e) => return Err(e),
                },
            }
        } else {
            normalized_branch.clone()
        };

        return github_get_branch_commit(&owner, &repo, &effective_branch);
    }

    // For non-GitHub repos, refresh the cache and read HEAD.
    let repo_dir = ensure_repo_cache(app, git_url, &normalized_branch, true)?;
    get_repo_head_commit(&repo_dir)
}

/// Check for updates for all market skills in a workspace.
pub fn check_updates_for_workspace(
    app: &tauri::AppHandle,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<SkillUpdateInfo>> {
    use std::collections::HashMap;

    let skills = installed_list_for_workspace(db, workspace_id)?;
    let mut results = Vec::new();

    // Cache latest commits by (git_url, branch) to avoid redundant API calls
    // when multiple skills share the same source repository.
    let mut commit_cache: HashMap<(String, String), Option<String>> = HashMap::new();

    for skill in skills {
        if !is_market_skill(&skill.source_git_url) {
            continue;
        }

        let cache_key = (skill.source_git_url.clone(), skill.source_branch.clone());
        let latest_commit = commit_cache
            .entry(cache_key)
            .or_insert_with(|| {
                get_latest_commit_for_skill(app, &skill.source_git_url, &skill.source_branch).ok()
            })
            .clone();

        let installed_commit = skill.installed_commit.clone();
        let has_update = match (&installed_commit, &latest_commit) {
            (Some(installed), Some(latest)) => installed != latest,
            _ => false,
        };

        results.push(SkillUpdateInfo {
            skill_id: skill.id,
            has_update,
            installed_commit,
            latest_commit,
        });
    }

    Ok(results)
}

/// Update a skill by uninstalling and re-installing it.
/// Preserves the enabled state across the update.
pub fn update_skill(
    app: &tauri::AppHandle,
    db: &db::Db,
    workspace_id: i64,
    skill_id: i64,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    // Get the current skill info before uninstalling.
    let conn = db.open_connection()?;
    let skill = get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)?;
    drop(conn);

    // Validate this is a market skill.
    if !is_market_skill(&skill.source_git_url) {
        return Err("SKILL_UPDATE_NOT_SUPPORTED: only market skills can be updated".into());
    }

    // Capture the current enabled state.
    let was_enabled = skill.enabled;
    let git_url = skill.source_git_url.clone();
    let branch = skill.source_branch.clone();
    let source_subdir = skill.source_subdir.clone();

    // Uninstall the skill.
    uninstall(app, db, skill_id)?;

    // Re-install the skill with the same source info, preserving enabled state.
    install(
        app,
        db,
        workspace_id,
        &git_url,
        &branch,
        &source_subdir,
        was_enabled,
    )
}

/// Update the installed_commit for a skill in the database.
#[allow(dead_code)]
pub(super) fn update_installed_commit(
    db: &db::Db,
    skill_id: i64,
    commit: Option<&str>,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    conn.execute(
        "UPDATE skills SET installed_commit = ?1, updated_at = ?2 WHERE id = ?3",
        params![commit, now, skill_id],
    )
    .map_err(|e| crate::shared::error::db_err!("failed to update installed_commit: {e}"))?;
    Ok(())
}
