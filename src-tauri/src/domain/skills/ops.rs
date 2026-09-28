use super::fs_ops::{
    copy_dir_recursive, create_skill_link, exists_or_is_link, has_skill_md, is_managed_dir,
    is_managed_link_to_ssot, is_symlink, is_symlink_or_junction, remove_managed_dir, remove_marker,
    skill_dir_content_hash, write_source_metadata, SkillSourceMetadata,
};
use super::installed::{generate_unique_skill_key, get_skill_by_id, get_skill_by_id_for_workspace};
use super::local::managed_marker_belongs_to_installed_skill;
use super::paths::{cli_skills_root, ensure_skills_roots, ssot_skills_root, validate_cli_key};
use super::repo_cache::ensure_repo_cache;
use super::skill_md::parse_skill_md;
use super::types::InstalledSkillSummary;
use super::util::validate_relative_subdir;
use crate::db;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use crate::workspaces;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;

fn is_external_local_skill_dir(path: &Path) -> crate::shared::error::AppResult<bool> {
    if !exists_or_is_link(path) || is_managed_dir(path) {
        return Ok(false);
    }
    if is_symlink(path)? {
        return Ok(true);
    }
    Ok(path.is_dir() && has_skill_md(path))
}

fn is_aio_managed_skill_target(
    conn: &Connection,
    path: &Path,
    ssot_root: &Path,
) -> crate::shared::error::AppResult<bool> {
    Ok(managed_marker_belongs_to_installed_skill(conn, path)?
        || is_managed_link_to_ssot(path, ssot_root))
}

fn local_source_cli_key(source_git_url: &str) -> Option<&str> {
    source_git_url
        .strip_prefix("local://")
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn ensure_ssot_dir_exists<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    skill: &InstalledSkillSummary,
    ssot_dir: &Path,
) -> crate::shared::error::AppResult<()> {
    if ssot_dir.exists() {
        return Ok(());
    }

    let Some(source_cli_key) = local_source_cli_key(&skill.source_git_url) else {
        return Err("SKILL_SSOT_MISSING: ssot skill dir not found"
            .to_string()
            .into());
    };

    validate_cli_key(source_cli_key)?;
    validate_relative_subdir(&skill.source_subdir)?;

    let local_source_dir = cli_skills_root(app, source_cli_key)?.join(skill.source_subdir.trim());
    if !local_source_dir.is_dir() || !has_skill_md(&local_source_dir) {
        return Err("SKILL_SSOT_MISSING: ssot skill dir not found"
            .to_string()
            .into());
    }

    if let Err(err) = copy_dir_recursive(&local_source_dir, ssot_dir) {
        let _ = std::fs::remove_dir_all(ssot_dir);
        return Err(err);
    }
    Ok(())
}

fn sync_to_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    skill_key: &str,
    ssot_dir: &Path,
) -> crate::shared::error::AppResult<()> {
    let cli_root = cli_skills_root(app, cli_key)?;
    std::fs::create_dir_all(&cli_root)
        .map_err(|e| format!("failed to create {}: {e}", cli_root.display()))?;
    let target = cli_root.join(skill_key);

    if exists_or_is_link(&target) {
        if is_managed_dir(&target)
            || is_managed_link_to_ssot(&target, ssot_dir.parent().unwrap_or(ssot_dir))
        {
            remove_managed_dir(&target)?;
        } else if is_external_local_skill_dir(&target)? {
            return Ok(());
        } else {
            return Err(format!("SKILL_TARGET_EXISTS_UNMANAGED: {}", target.display()).into());
        }
    }

    create_skill_link(ssot_dir, &target)?;
    Ok(())
}

fn remove_from_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    skill_key: &str,
) -> crate::shared::error::AppResult<()> {
    let cli_root = cli_skills_root(app, cli_key)?;
    let target = cli_root.join(skill_key);
    if !exists_or_is_link(&target) {
        return Ok(());
    }
    let ssot_root = ssot_skills_root(app)?;
    if is_managed_link_to_ssot(&target, &ssot_root) {
        return remove_managed_dir(&target);
    }
    if is_external_local_skill_dir(&target)? {
        // Do not remove unmanaged local skill targets owned by external tooling.
        return Ok(());
    }
    remove_managed_dir(&target)
}

fn ensure_local_target_for_return(
    local_target: &Path,
    ssot_dir: &Path,
) -> crate::shared::error::AppResult<()> {
    if exists_or_is_link(local_target) {
        let ssot_root = ssot_dir.parent().unwrap_or(ssot_dir);
        if is_managed_dir(local_target) || is_managed_link_to_ssot(local_target, ssot_root) {
            remove_managed_dir(local_target)?;
        } else if is_symlink(local_target)? || (local_target.is_dir() && has_skill_md(local_target))
        {
            return Ok(());
        } else {
            return Err(format!(
                "SKILL_RETURN_LOCAL_TARGET_EXISTS_UNMANAGED: {}",
                local_target.display()
            )
            .into());
        }
    }

    // At this point local_target does not exist (either never existed or was removed above).
    if let Err(err) = copy_dir_recursive(ssot_dir, local_target) {
        let _ = std::fs::remove_dir_all(local_target);
        return Err(err);
    }
    remove_marker(local_target);
    Ok(())
}

fn remove_managed_targets_except<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    skill_key: &str,
    keep_target: &Path,
) -> crate::shared::error::AppResult<()> {
    let ssot_root = ssot_skills_root(app)?;
    for cli_key in
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
    {
        let root = cli_skills_root(app, cli_key)?;
        let target = root.join(skill_key);
        if target == keep_target || !exists_or_is_link(&target) {
            continue;
        }
        if is_managed_dir(&target) || is_managed_link_to_ssot(&target, &ssot_root) {
            remove_managed_dir(&target)?;
            continue;
        }
        if is_external_local_skill_dir(&target)? {
            continue;
        }
        return Err(format!("SKILL_REMOVE_BLOCKED_UNMANAGED: {}", target.display()).into());
    }
    Ok(())
}

fn delete_skill_row(conn: &Connection, skill_id: i64) -> crate::shared::error::AppResult<()> {
    let changed = conn
        .execute("DELETE FROM skills WHERE id = ?1", params![skill_id])
        .map_err(|e| db_err!("failed to delete skill: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: skill not found".to_string().into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn install(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    workspace_id: i64,
    git_url: &str,
    branch: &str,
    source_subdir: &str,
    enabled: bool,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    ensure_skills_roots(app)?;
    validate_relative_subdir(source_subdir)?;

    let mut conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    let should_sync = workspaces::is_active_workspace(&conn, workspace_id)?;
    let now = now_unix_seconds();

    // Ensure source not already installed.
    let existing_id: Option<i64> = conn
        .query_row(
            r#"
SELECT id
FROM skills
WHERE source_git_url = ?1 AND source_branch = ?2 AND source_subdir = ?3
LIMIT 1
"#,
            params![git_url.trim(), branch.trim(), source_subdir.trim()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query skill by source: {e}"))?;
    if existing_id.is_some() {
        return Err("SKILL_ALREADY_INSTALLED: skill already installed"
            .to_string()
            .into());
    }

    let repo_dir = ensure_repo_cache(app, git_url, branch, true)?;
    let src_dir = repo_dir.join(source_subdir.trim());
    if !src_dir.exists() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }

    let skill_md = src_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in source_subdir"
            .to_string()
            .into());
    }

    // Try to capture the installed commit hash (best effort).
    // For GitHub snapshot mode this may fail, which is acceptable.
    let installed_commit = super::repo_cache::get_repo_head_commit(&repo_dir).ok();

    let (name, description) = parse_skill_md(&skill_md)?;
    let normalized_name = normalize_name(&name);

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let skill_key = generate_unique_skill_key(&tx, &name)?;
    let ssot_root = ssot_skills_root(app)?;
    let ssot_dir = ssot_root.join(&skill_key);
    if ssot_dir.exists() {
        return Err("SKILL_CONFLICT: ssot dir already exists".to_string().into());
    }

    tx.execute(
        r#"
INSERT INTO skills(
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  installed_commit,
  installed_content_hash,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)
"#,
        params![
            skill_key,
            name.trim(),
            normalized_name,
            description,
            git_url.trim(),
            branch.trim(),
            source_subdir.trim(),
            installed_commit,
            now,
            now
        ],
    )
    .map_err(|e| db_err!("failed to insert skill: {e}"))?;

    let skill_id = tx.last_insert_rowid();

    if enabled {
        tx.execute(
            r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, skill_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
            params![workspace_id, skill_id, now],
        )
        .map_err(|e| db_err!("failed to enable skill for workspace: {e}"))?;
    }

    // FS: copy to SSOT first.
    if let Err(err) = copy_dir_recursive(&src_dir, &ssot_dir) {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
        return Err(err);
    }

    let installed_content_hash = match skill_dir_content_hash(&ssot_dir) {
        Ok(hash) => hash,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&ssot_dir);
            let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
            return Err(err);
        }
    };
    if let Err(err) = tx.execute(
        "UPDATE skills SET installed_content_hash = ?1 WHERE id = ?2",
        params![installed_content_hash, skill_id],
    ) {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
        return Err(db_err!(
            "failed to update installed skill content hash: {err}"
        ));
    }

    // FS: sync to CLI only when enabled in the active workspace.
    if should_sync && enabled {
        if let Err(err) = sync_to_cli(app, &cli_key, &skill_key, &ssot_dir) {
            let _ = remove_from_cli(app, &cli_key, &skill_key);
            let _ = std::fs::remove_dir_all(&ssot_dir);
            let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
            return Err(err);
        }
    }

    if let Err(err) = tx.commit() {
        let _ = remove_from_cli(app, &cli_key, &skill_key);
        let _ = std::fs::remove_dir_all(&ssot_dir);
        return Err(db_err!("failed to commit: {err}"));
    }

    get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)
}

pub fn set_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    skill_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    let mut conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    let should_sync = workspaces::is_active_workspace(&conn, workspace_id)?;
    let now = now_unix_seconds();

    let current = get_skill_by_id(&conn, skill_id)?;
    let was_enabled: bool = conn
        .query_row(
            "SELECT 1 FROM workspace_skill_enabled WHERE workspace_id = ?1 AND skill_id = ?2",
            params![workspace_id, skill_id],
            |_row| Ok(()),
        )
        .optional()
        .map_err(|e| db_err!("failed to query workspace_skill_enabled: {e}"))?
        .is_some();

    if was_enabled == enabled {
        return get_skill_by_id_for_workspace(&conn, workspace_id, skill_id);
    }

    let ssot_root = ssot_skills_root(app)?;
    let ssot_dir = ssot_root.join(&current.skill_key);
    ensure_ssot_dir_exists(app, &current, &ssot_dir)?;

    if should_sync {
        if enabled {
            sync_to_cli(app, &cli_key, &current.skill_key, &ssot_dir)?;
        } else {
            remove_from_cli(app, &cli_key, &current.skill_key)?;
        }
    }

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    if enabled {
        tx.execute(
            r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, skill_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
            params![workspace_id, skill_id, now],
        )
        .map_err(|e| db_err!("failed to enable skill: {e}"))?;
    } else {
        tx.execute(
            "DELETE FROM workspace_skill_enabled WHERE workspace_id = ?1 AND skill_id = ?2",
            params![workspace_id, skill_id],
        )
        .map_err(|e| db_err!("failed to disable skill: {e}"))?;
    }

    if let Err(err) = tx.commit() {
        if should_sync {
            if enabled {
                let _ = remove_from_cli(app, &cli_key, &current.skill_key);
            } else if was_enabled {
                let _ = sync_to_cli(app, &cli_key, &current.skill_key, &ssot_dir);
            }
        }
        return Err(db_err!("failed to commit: {err}"));
    }

    get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)
}

pub fn uninstall<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    skill_id: i64,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let skill = get_skill_by_id(&conn, skill_id)?;

    // Safety: ensure we will only delete managed dirs.
    let ssot_root = ssot_skills_root(app)?;
    for cli_key in
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
    {
        let root = cli_skills_root(app, cli_key)?;
        let target = root.join(&skill.skill_key);
        if exists_or_is_link(&target)
            && !is_managed_dir(&target)
            && !is_managed_link_to_ssot(&target, &ssot_root)
            && !is_external_local_skill_dir(&target)?
        {
            return Err(format!("SKILL_REMOVE_BLOCKED_UNMANAGED: {}", target.display()).into());
        }
    }

    for cli_key in
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
    {
        remove_from_cli(app, cli_key, &skill.skill_key)?;
    }

    let ssot_dir = ssot_root.join(&skill.skill_key);
    if ssot_dir.exists() {
        std::fs::remove_dir_all(&ssot_dir)
            .map_err(|e| format!("failed to remove {}: {e}", ssot_dir.display()))?;
    }

    delete_skill_row(&conn, skill_id)
}

pub fn return_to_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    skill_id: i64,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    if !workspaces::is_active_workspace(&conn, workspace_id)? {
        return Err(
            "SKILL_RETURN_LOCAL_REQUIRES_ACTIVE_WORKSPACE: switch to the target workspace before returning"
                .to_string()
                .into(),
        );
    }

    let skill = get_skill_by_id(&conn, skill_id)?;
    let ssot_dir = ssot_skills_root(app)?.join(&skill.skill_key);
    ensure_ssot_dir_exists(app, &skill, &ssot_dir)?;

    let cli_root = cli_skills_root(app, &cli_key)?;
    std::fs::create_dir_all(&cli_root)
        .map_err(|e| format!("failed to create {}: {e}", cli_root.display()))?;
    let local_target = cli_root.join(&skill.skill_key);
    ensure_local_target_for_return(&local_target, &ssot_dir)?;
    write_source_metadata(
        &local_target,
        &SkillSourceMetadata {
            source_git_url: skill.source_git_url.clone(),
            source_branch: skill.source_branch.clone(),
            source_subdir: skill.source_subdir.clone(),
        },
    )?;
    remove_managed_targets_except(app, &skill.skill_key, &local_target)?;

    std::fs::remove_dir_all(&ssot_dir)
        .map_err(|e| format!("failed to remove {}: {e}", ssot_dir.display()))?;

    delete_skill_row(&conn, skill_id)
}

fn sync_enabled_skill_keys_for_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    enabled_list: Vec<String>,
) -> crate::shared::error::AppResult<()> {
    ensure_skills_roots(app)?;
    validate_cli_key(cli_key)?;

    let _grok_sync_guard = if cli_key == "grok" {
        Some(super::sync_manifest::lock()?)
    } else {
        None
    };
    let previous_grok_manifest = if cli_key == "grok" {
        super::sync_manifest::read(app)?
    } else {
        None
    };

    let enabled_set: HashSet<String> = enabled_list.iter().cloned().collect();

    let cli_root = cli_skills_root(app, cli_key)?;
    std::fs::create_dir_all(&cli_root)
        .map_err(|e| format!("failed to create {}: {e}", cli_root.display()))?;

    let ssot_root = ssot_skills_root(app)?;

    if let Ok(entries) = std::fs::read_dir(&cli_root) {
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("failed to read dir entry {}: {e}", cli_root.display()))?;
            let path = entry.path();
            if !path.is_dir() && !is_symlink_or_junction(&path) {
                continue;
            }
            if !is_aio_managed_skill_target(conn, &path, &ssot_root)? {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_string();
            if dir_name.is_empty() {
                continue;
            }
            if enabled_set.contains(&dir_name) {
                continue;
            }
            remove_managed_dir(&path)?;
        }
    }

    for skill_key in &enabled_list {
        let ssot_dir = ssot_root.join(skill_key);
        if !ssot_dir.exists() {
            return Err(format!("SKILL_SSOT_MISSING: {}", ssot_dir.display()).into());
        }
        sync_to_cli(app, cli_key, skill_key, &ssot_dir)?;
    }

    if cli_key == "grok" {
        if let Some(previous) = previous_grok_manifest {
            let previous_root = Path::new(&previous.root_path);
            if !crate::grok_config::paths_equivalent(previous_root, &cli_root)? {
                for skill_key in previous.managed_keys {
                    let target = previous_root.join(skill_key);
                    if exists_or_is_link(&target)
                        && is_aio_managed_skill_target(conn, &target, &ssot_root)?
                    {
                        remove_managed_dir(&target)?;
                    }
                }
            }
        }

        let mut managed_keys = Vec::new();
        for skill_key in &enabled_list {
            let target = cli_root.join(skill_key);
            if exists_or_is_link(&target) && is_aio_managed_skill_target(conn, &target, &ssot_root)?
            {
                managed_keys.push(skill_key.clone());
            }
        }
        super::sync_manifest::write(app, &cli_root, managed_keys)?;
    }

    Ok(())
}

pub(crate) fn sync_one_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<()> {
    ensure_skills_roots(app)?;
    validate_cli_key(cli_key)?;

    let Some(workspace_id) = workspaces::active_id_by_cli(conn, cli_key)? else {
        return sync_enabled_skill_keys_for_cli(app, conn, cli_key, Vec::new());
    };

    sync_cli_for_workspace(app, conn, workspace_id)
}

pub fn sync_cli_for_workspace<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    ensure_skills_roots(app)?;

    let cli_key = workspaces::get_cli_key_by_id(conn, workspace_id)?;
    validate_cli_key(&cli_key)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.skill_key
    FROM skills s
    JOIN workspace_skill_enabled e
      ON e.skill_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.skill_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled skills query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled skills: {e}"))?;

    let mut enabled_set = HashSet::new();
    let mut enabled_list: Vec<String> = Vec::new();
    for row in rows {
        let key = row.map_err(|e| db_err!("failed to read enabled skill row: {e}"))?;
        if enabled_set.insert(key.clone()) {
            enabled_list.push(key);
        }
    }
    enabled_list.sort();

    sync_enabled_skill_keys_for_cli(app, conn, &cli_key, enabled_list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[derive(Default)]
    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn set(&mut self, key: &'static str, value: impl Into<OsString>) {
            if !self.0.iter().any(|(saved, _)| *saved == key) {
                self.0.push((key, std::env::var_os(key)));
            }
            std::env::set_var(key, value.into());
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn grok_skill_sync_rebinds_home_and_preserves_unmanaged_old_targets() {
        let _lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().expect("tempdir");
        let old_home = temp.path().join("grok-old");
        let new_home = temp.path().join("grok-new");
        let mut env = EnvRestore::default();
        env.set(
            "AIO_CODING_HUB_HOME_DIR",
            temp.path().as_os_str().to_os_string(),
        );
        env.set("AIO_CODING_HUB_DOTDIR_NAME", ".aio-skills-rebind-test");
        env.set("GROK_HOME", old_home.as_os_str().to_os_string());
        let app = tauri::test::mock_app();
        let conn = Connection::open_in_memory().expect("open in-memory database");

        let ssot_dir = ssot_skills_root(app.handle())
            .expect("resolve SSOT root")
            .join("demo");
        std::fs::create_dir_all(&ssot_dir).expect("create SSOT skill");
        std::fs::write(ssot_dir.join("SKILL.md"), "---\nname: Demo\n---\n")
            .expect("write SSOT skill");

        sync_enabled_skill_keys_for_cli(app.handle(), &conn, "grok", vec!["demo".to_string()])
            .expect("sync old Grok skills root");
        let old_skills_root = old_home.join("skills");
        let old_managed = old_skills_root.join("demo");
        assert!(exists_or_is_link(&old_managed));

        let old_unmanaged = old_skills_root.join("local-only");
        std::fs::create_dir_all(&old_unmanaged).expect("create unmanaged old skill");
        std::fs::write(
            old_unmanaged.join("SKILL.md"),
            "---\nname: Local only\n---\n",
        )
        .expect("write unmanaged old skill");
        env.set("GROK_HOME", new_home.as_os_str().to_os_string());

        sync_enabled_skill_keys_for_cli(app.handle(), &conn, "grok", vec!["demo".to_string()])
            .expect("sync rebound Grok skills root");

        assert!(!exists_or_is_link(&old_managed));
        assert!(old_unmanaged.is_dir());
        assert!(exists_or_is_link(&new_home.join("skills").join("demo")));
    }
}
