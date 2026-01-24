use super::fs_ops::{copy_dir_recursive, is_managed_dir, remove_managed_dir};
use super::installed::{generate_unique_skill_key, get_skill_by_id};
use super::paths::{cli_skills_root, ensure_skills_roots, ssot_skills_root, validate_cli_key};
use super::repo_cache::ensure_repo_cache;
use super::skill_md::parse_skill_md;
use super::types::InstalledSkillSummary;
use super::util::{enabled_to_int, normalize_name, validate_relative_subdir};
use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::params;
use rusqlite::OptionalExtension;
use std::path::Path;

fn sync_to_cli(
    app: &tauri::AppHandle,
    cli_key: &str,
    skill_key: &str,
    ssot_dir: &Path,
) -> Result<(), String> {
    let cli_root = cli_skills_root(app, cli_key)?;
    std::fs::create_dir_all(&cli_root)
        .map_err(|e| format!("failed to create {}: {e}", cli_root.display()))?;
    let target = cli_root.join(skill_key);

    if target.exists() {
        if !is_managed_dir(&target) {
            return Err(format!(
                "SKILL_TARGET_EXISTS_UNMANAGED: {}",
                target.display()
            ));
        }
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("failed to remove {}: {e}", target.display()))?;
    }

    copy_dir_recursive(ssot_dir, &target)?;
    super::fs_ops::write_marker(&target)?;
    Ok(())
}

fn remove_from_cli(app: &tauri::AppHandle, cli_key: &str, skill_key: &str) -> Result<(), String> {
    let cli_root = cli_skills_root(app, cli_key)?;
    let target = cli_root.join(skill_key);
    if !target.exists() {
        return Ok(());
    }
    remove_managed_dir(&target)
}

#[allow(clippy::too_many_arguments)]
pub fn install(
    app: &tauri::AppHandle,
    db: &db::Db,
    git_url: &str,
    branch: &str,
    source_subdir: &str,
    enabled_claude: bool,
    enabled_codex: bool,
    enabled_gemini: bool,
) -> Result<InstalledSkillSummary, String> {
    ensure_skills_roots(app)?;
    validate_relative_subdir(source_subdir)?;

    let mut conn = db.open_connection()?;
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
        .map_err(|e| format!("DB_ERROR: failed to query skill by source: {e}"))?;
    if existing_id.is_some() {
        return Err("SKILL_ALREADY_INSTALLED: skill already installed".to_string());
    }

    let repo_dir = ensure_repo_cache(app, git_url, branch, true)?;
    let src_dir = repo_dir.join(source_subdir.trim());
    if !src_dir.exists() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()));
    }

    let skill_md = src_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in source_subdir".to_string());
    }

    let (name, description) = parse_skill_md(&skill_md)?;
    let normalized_name = normalize_name(&name);

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let skill_key = generate_unique_skill_key(&tx, &name)?;
    let ssot_root = ssot_skills_root(app)?;
    let ssot_dir = ssot_root.join(&skill_key);
    if ssot_dir.exists() {
        return Err("SKILL_CONFLICT: ssot dir already exists".to_string());
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
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
"#,
        params![
            skill_key,
            name.trim(),
            normalized_name,
            description,
            git_url.trim(),
            branch.trim(),
            source_subdir.trim(),
            enabled_to_int(enabled_claude),
            enabled_to_int(enabled_codex),
            enabled_to_int(enabled_gemini),
            now,
            now
        ],
    )
    .map_err(|e| format!("DB_ERROR: failed to insert skill: {e}"))?;

    let skill_id = tx.last_insert_rowid();

    // FS: copy to SSOT first.
    if let Err(err) = copy_dir_recursive(&src_dir, &ssot_dir) {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
        return Err(err);
    }

    // FS: sync to enabled CLIs.
    let sync_steps = [
        ("claude", enabled_claude),
        ("codex", enabled_codex),
        ("gemini", enabled_gemini),
    ];

    for (cli_key, enabled) in sync_steps {
        if !enabled {
            continue;
        }
        if let Err(err) = sync_to_cli(app, cli_key, &skill_key, &ssot_dir) {
            let _ = remove_from_cli(app, "claude", &skill_key);
            let _ = remove_from_cli(app, "codex", &skill_key);
            let _ = remove_from_cli(app, "gemini", &skill_key);
            let _ = std::fs::remove_dir_all(&ssot_dir);
            let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
            return Err(err);
        }
    }

    if let Err(err) = tx.commit() {
        let _ = remove_from_cli(app, "claude", &skill_key);
        let _ = remove_from_cli(app, "codex", &skill_key);
        let _ = remove_from_cli(app, "gemini", &skill_key);
        let _ = std::fs::remove_dir_all(&ssot_dir);
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    get_skill_by_id(&conn, skill_id)
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    db: &db::Db,
    skill_id: i64,
    cli_key: &str,
    enabled: bool,
) -> Result<InstalledSkillSummary, String> {
    validate_cli_key(cli_key)?;

    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    let current = get_skill_by_id(&conn, skill_id)?;
    let ssot_root = ssot_skills_root(app)?;
    let ssot_dir = ssot_root.join(&current.skill_key);
    if !ssot_dir.exists() {
        return Err("SKILL_SSOT_MISSING: ssot skill dir not found".to_string());
    }

    if enabled {
        sync_to_cli(app, cli_key, &current.skill_key, &ssot_dir)?;
    } else {
        remove_from_cli(app, cli_key, &current.skill_key)?;
    }

    let column = match cli_key {
        "claude" => "enabled_claude",
        "codex" => "enabled_codex",
        "gemini" => "enabled_gemini",
        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    };

    let sql = format!("UPDATE skills SET {column} = ?1, updated_at = ?2 WHERE id = ?3");
    conn.execute(&sql, params![enabled_to_int(enabled), now, skill_id])
        .map_err(|e| format!("DB_ERROR: failed to update skill enabled: {e}"))?;

    get_skill_by_id(&conn, skill_id)
}

pub fn uninstall(app: &tauri::AppHandle, db: &db::Db, skill_id: i64) -> Result<(), String> {
    let conn = db.open_connection()?;
    let skill = get_skill_by_id(&conn, skill_id)?;

    // Safety: ensure we will only delete managed dirs.
    let cli_roots = [
        ("claude", cli_skills_root(app, "claude")?),
        ("codex", cli_skills_root(app, "codex")?),
        ("gemini", cli_skills_root(app, "gemini")?),
    ];
    for (_cli, root) in &cli_roots {
        let target = root.join(&skill.skill_key);
        if target.exists() && !is_managed_dir(&target) {
            return Err(format!(
                "SKILL_REMOVE_BLOCKED_UNMANAGED: {}",
                target.display()
            ));
        }
    }

    remove_from_cli(app, "claude", &skill.skill_key)?;
    remove_from_cli(app, "codex", &skill.skill_key)?;
    remove_from_cli(app, "gemini", &skill.skill_key)?;

    let ssot_dir = ssot_skills_root(app)?.join(&skill.skill_key);
    if ssot_dir.exists() {
        std::fs::remove_dir_all(&ssot_dir)
            .map_err(|e| format!("failed to remove {}: {e}", ssot_dir.display()))?;
    }

    let changed = conn
        .execute("DELETE FROM skills WHERE id = ?1", params![skill_id])
        .map_err(|e| format!("DB_ERROR: failed to delete skill: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: skill not found".to_string());
    }
    Ok(())
}
