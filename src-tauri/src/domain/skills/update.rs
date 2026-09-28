//! Usage: Skill update detection and execution.

use super::fs_ops::{copy_dir_recursive, skill_dir_content_hash};
use super::git_url::{normalize_repo_branch, parse_github_owner_repo};
use super::installed::{get_skill_by_id_for_workspace, installed_list_for_workspace};
use super::ops::sync_one_cli;
use super::paths::ssot_skills_root;
use super::repo_cache::{ensure_repo_cache, get_repo_head_commit, github_get_branch_commit};
use super::skill_md::parse_skill_md;
use super::types::{InstalledSkillSummary, SkillUpdateInfo};
use super::util::validate_relative_subdir;
use crate::db;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static SKILL_UPDATE_LOCKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();
static UPDATE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct SkillUpdateGuard {
    skill_id: i64,
}

impl Drop for SkillUpdateGuard {
    fn drop(&mut self) {
        if let Some(locks) = SKILL_UPDATE_LOCKS.get() {
            if let Ok(mut locked) = locks.lock() {
                locked.remove(&self.skill_id);
            }
        }
    }
}

fn acquire_skill_update_lock(skill_id: i64) -> crate::shared::error::AppResult<SkillUpdateGuard> {
    let locks = SKILL_UPDATE_LOCKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut locked = locks
        .lock()
        .map_err(|_| "SKILL_UPDATE_LOCK_POISONED: failed to acquire update lock")?;
    if !locked.insert(skill_id) {
        return Err("SKILL_UPDATE_IN_PROGRESS: skill is already being updated".into());
    }
    Ok(SkillUpdateGuard { skill_id })
}

/// Local-only skills (local://) are not eligible for remote update checks.
fn is_updatable_skill_source(source_git_url: &str) -> bool {
    let url = source_git_url.trim().to_lowercase();
    !url.is_empty() && !url.starts_with("local://")
}

/// Get the latest commit for a skill from its source repository.
/// For GitHub repos, uses the API to get the branch commit.
/// For git repos, refreshes the cache and reads HEAD.
fn get_latest_commit_for_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
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

fn get_latest_content_hash_for_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
    source_subdir: &str,
) -> crate::shared::error::AppResult<String> {
    validate_relative_subdir(source_subdir)?;
    let normalized_branch = normalize_repo_branch(branch);
    let repo_dir = ensure_repo_cache(app, git_url, &normalized_branch, true)?;
    let src_dir = repo_dir.join(source_subdir.trim());
    if !src_dir.is_dir() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }
    skill_dir_content_hash(&src_dir)
}

fn installed_content_hash_for_skill(
    db: &db::Db,
    skill_id: i64,
) -> crate::shared::error::AppResult<Option<String>> {
    let conn = db.open_connection()?;
    installed_content_hash_for_conn(&conn, skill_id)
}

fn installed_content_hash_for_conn(
    conn: &Connection,
    skill_id: i64,
) -> crate::shared::error::AppResult<Option<String>> {
    conn.query_row(
        "SELECT installed_content_hash FROM skills WHERE id = ?1",
        params![skill_id],
        |row| row.get(0),
    )
    .optional()
    .map(|value| value.flatten())
    .map_err(|e| db_err!("failed to query installed_content_hash: {e}"))
}

fn set_installed_content_hash(
    db: &db::Db,
    skill_id: i64,
    hash: &str,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    conn.execute(
        "UPDATE skills SET installed_content_hash = ?1 WHERE id = ?2",
        params![hash, skill_id],
    )
    .map_err(|e| db_err!("failed to update installed_content_hash: {e}"))?;
    Ok(())
}

fn get_or_backfill_installed_content_hash(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    skill: &InstalledSkillSummary,
) -> Option<String> {
    if let Ok(Some(hash)) = installed_content_hash_for_skill(db, skill.id) {
        return Some(hash);
    }

    let ssot_dir = ssot_skills_root(app).ok()?.join(&skill.skill_key);
    let hash = skill_dir_content_hash(&ssot_dir).ok()?;
    let _ = set_installed_content_hash(db, skill.id, &hash);
    Some(hash)
}

/// Check for updates for all remotely sourced skills in a workspace.
pub fn check_updates_for_workspace(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<SkillUpdateInfo>> {
    use std::collections::HashMap;

    let skills = installed_list_for_workspace(db, workspace_id)?;
    let mut results = Vec::new();

    // Cache latest commits by (git_url, branch) to avoid redundant API calls
    // when multiple skills share the same source repository.
    let mut commit_cache: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut content_hash_cache: HashMap<(String, String, String), Option<String>> = HashMap::new();

    for skill in skills {
        if !is_updatable_skill_source(&skill.source_git_url) {
            continue;
        }

        let content_cache_key = (
            skill.source_git_url.clone(),
            skill.source_branch.clone(),
            skill.source_subdir.clone(),
        );
        let latest_content_hash = content_hash_cache
            .entry(content_cache_key)
            .or_insert_with(|| {
                get_latest_content_hash_for_skill(
                    app,
                    &skill.source_git_url,
                    &skill.source_branch,
                    &skill.source_subdir,
                )
                .ok()
            })
            .clone();

        let installed_content_hash = get_or_backfill_installed_content_hash(app, db, &skill);

        let cache_key = (skill.source_git_url.clone(), skill.source_branch.clone());
        let latest_commit = commit_cache
            .entry(cache_key)
            .or_insert_with(|| {
                get_latest_commit_for_skill(app, &skill.source_git_url, &skill.source_branch).ok()
            })
            .clone();

        let installed_commit = skill.installed_commit.clone();
        let has_update = match (&installed_content_hash, &latest_content_hash) {
            (Some(installed), Some(latest)) => installed != latest,
            _ => match (&installed_commit, &latest_commit) {
                (Some(installed), Some(latest)) => installed != latest,
                _ => false,
            },
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

/// Update a skill by replacing the SSOT directory in place.
/// Preserves all workspace enablements.
pub fn update_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    workspace_id: i64,
    skill_id: i64,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    let mut conn = db.open_connection()?;
    let _guard = acquire_skill_update_lock(skill_id)?;
    let skill = get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)?;
    let previous_content_hash = installed_content_hash_for_conn(&conn, skill_id)?;

    // Local-only imports do not have a remote source to refresh from.
    if !is_updatable_skill_source(&skill.source_git_url) {
        return Err("SKILL_UPDATE_NOT_SUPPORTED: local skills cannot be updated".into());
    }
    validate_relative_subdir(&skill.source_subdir)?;

    let normalized_branch = normalize_repo_branch(&skill.source_branch);
    let repo_dir = ensure_repo_cache(app, &skill.source_git_url, &normalized_branch, true)?;
    let src_dir = repo_dir.join(skill.source_subdir.trim());
    if !src_dir.is_dir() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }
    let skill_md = src_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in source_subdir"
            .to_string()
            .into());
    }

    let (name, description) = parse_skill_md(&skill_md)?;
    let normalized_name = normalize_name(&name);
    let installed_commit = get_repo_head_commit(&repo_dir).ok().or_else(|| {
        get_latest_commit_for_skill(app, &skill.source_git_url, &skill.source_branch).ok()
    });

    let ssot_root = ssot_skills_root(app)?;
    let ssot_dir = ssot_root.join(&skill.skill_key);
    let staging_dir = unique_update_path(&ssot_root, &skill.skill_key, "staging");
    let backup_dir = unique_update_path(&ssot_root, &skill.skill_key, "old");
    replace_skill_dir(&src_dir, &ssot_dir, &staging_dir, &backup_dir)?;

    let installed_content_hash = match skill_dir_content_hash(&ssot_dir) {
        Ok(hash) => hash,
        Err(err) => {
            let _ = restore_replaced_skill_dir(&ssot_dir, &backup_dir);
            return Err(err);
        }
    };

    let now = now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;
    let updated_rows = match tx.execute(
        r#"
UPDATE skills
SET
  name = ?1,
  normalized_name = ?2,
  description = ?3,
  installed_commit = ?4,
  installed_content_hash = ?5,
  updated_at = ?6
WHERE id = ?7
"#,
        params![
            name.trim(),
            normalized_name,
            description,
            installed_commit,
            installed_content_hash,
            now,
            skill_id
        ],
    ) {
        Ok(rows) => rows,
        Err(err) => {
            let _ = tx.rollback();
            let _ = restore_replaced_skill_dir(&ssot_dir, &backup_dir);
            return Err(db_err!("failed to update skill metadata: {err}"));
        }
    };
    if updated_rows != 1 {
        let _ = tx.rollback();
        let _ = restore_replaced_skill_dir(&ssot_dir, &backup_dir);
        return Err("SKILL_UPDATE_CONFLICT: skill no longer exists".into());
    }

    if let Err(err) = tx.commit() {
        let _ = restore_replaced_skill_dir(&ssot_dir, &backup_dir);
        return Err(db_err!("failed to commit: {err}"));
    }

    let skill_cli_keys: Vec<_> =
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
            .collect();
    for cli_key in skill_cli_keys.iter().copied() {
        if let Err(err) = sync_one_cli(app, &conn, cli_key) {
            let rollback_suffix = restore_committed_update(
                &conn,
                &skill,
                previous_content_hash.as_deref(),
                &ssot_dir,
                &backup_dir,
            )
            .map(|message| format!("; rollback failed: {message}"))
            .unwrap_or_default();
            for repair_cli_key in skill_cli_keys.iter().copied() {
                if let Err(repair_err) = sync_one_cli(app, &conn, repair_cli_key) {
                    tracing::warn!(
                        cli_key = %repair_cli_key,
                        "skill update rollback sync skipped: {repair_err}"
                    );
                }
            }
            return Err(format!(
                "SKILL_UPDATE_SYNC_FAILED: failed to sync {cli_key}: {err}{rollback_suffix}"
            )
            .into());
        }
    }

    let _ = std::fs::remove_dir_all(&backup_dir);

    get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)
}

fn unique_update_path(root: &Path, skill_key: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let counter = UPDATE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    root.join(format!(".{skill_key}.update-{suffix}-{nanos}-{counter}"))
}

fn replace_skill_dir(
    src_dir: &Path,
    ssot_dir: &Path,
    staging_dir: &Path,
    backup_dir: &Path,
) -> crate::shared::error::AppResult<()> {
    let _ = std::fs::remove_dir_all(staging_dir);
    let _ = std::fs::remove_dir_all(backup_dir);

    if let Err(err) = copy_dir_recursive(src_dir, staging_dir) {
        let _ = std::fs::remove_dir_all(staging_dir);
        return Err(err);
    }

    if ssot_dir.exists() {
        std::fs::rename(ssot_dir, backup_dir).map_err(|e| {
            let _ = std::fs::remove_dir_all(staging_dir);
            format!(
                "SKILL_UPDATE_REPLACE_FAILED: failed to move {} to {}: {e}",
                ssot_dir.display(),
                backup_dir.display()
            )
        })?;
    }

    if let Err(err) = std::fs::rename(staging_dir, ssot_dir) {
        let _ = restore_replaced_skill_dir(ssot_dir, backup_dir);
        return Err(format!(
            "SKILL_UPDATE_REPLACE_FAILED: failed to activate {}: {err}",
            ssot_dir.display()
        )
        .into());
    }

    Ok(())
}

fn restore_replaced_skill_dir(ssot_dir: &Path, backup_dir: &Path) -> std::io::Result<()> {
    if ssot_dir.exists() {
        std::fs::remove_dir_all(ssot_dir)?;
    }
    if backup_dir.exists() {
        std::fs::rename(backup_dir, ssot_dir)?;
    }
    Ok(())
}

fn restore_committed_update(
    conn: &Connection,
    skill: &InstalledSkillSummary,
    previous_content_hash: Option<&str>,
    ssot_dir: &Path,
    backup_dir: &Path,
) -> Option<String> {
    let mut errors = Vec::new();
    if let Err(err) = restore_replaced_skill_dir(ssot_dir, backup_dir) {
        errors.push(format!("files: {err}"));
    }
    if let Err(err) = restore_skill_metadata(conn, skill, previous_content_hash) {
        errors.push(format!("db: {err}"));
    }
    if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    }
}

fn restore_skill_metadata(
    conn: &Connection,
    skill: &InstalledSkillSummary,
    previous_content_hash: Option<&str>,
) -> crate::shared::error::AppResult<()> {
    let rows = conn
        .execute(
            r#"
UPDATE skills
SET
  name = ?1,
  normalized_name = ?2,
  description = ?3,
  installed_commit = ?4,
  installed_content_hash = ?5,
  updated_at = ?6
WHERE id = ?7
"#,
            params![
                skill.name.trim(),
                normalize_name(&skill.name),
                skill.description.as_str(),
                skill.installed_commit.as_deref(),
                previous_content_hash,
                skill.updated_at,
                skill.id
            ],
        )
        .map_err(|e| db_err!("failed to restore skill metadata: {e}"))?;
    if rows != 1 {
        return Err("SKILL_UPDATE_ROLLBACK_FAILED: skill no longer exists".into());
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_update_path_changes_between_calls() {
        let root = Path::new("/tmp");

        let first = unique_update_path(root, "context7", "staging");
        let second = unique_update_path(root, "context7", "staging");

        assert_ne!(first, second);
    }

    #[test]
    fn skill_update_lock_rejects_concurrent_same_skill() {
        let guard = acquire_skill_update_lock(i64::MIN).expect("first lock");

        let err = acquire_skill_update_lock(i64::MIN)
            .expect_err("second lock should fail")
            .to_string();
        assert!(err.contains("SKILL_UPDATE_IN_PROGRESS"));

        drop(guard);
        acquire_skill_update_lock(i64::MIN).expect("lock released");
    }
}
