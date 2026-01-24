use super::fs_ops::{copy_dir_recursive, is_managed_dir, remove_marker, write_marker};
use super::installed::{get_skill_by_id, skill_key_exists};
use super::paths::{cli_skills_root, ensure_skills_roots, ssot_skills_root, validate_cli_key};
use super::skill_md::parse_skill_md;
use super::types::{InstalledSkillSummary, LocalSkillSummary};
use super::util::{enabled_to_int, normalize_name, validate_dir_name};
use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::params;

pub fn local_list(app: &tauri::AppHandle, cli_key: &str) -> Result<Vec<LocalSkillSummary>, String> {
    validate_cli_key(cli_key)?;
    let root = cli_skills_root(app, cli_key)?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&root)
        .map_err(|e| format!("failed to read dir {}: {e}", root.display()))?;

    let mut out = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", root.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if is_managed_dir(&path) {
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

        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let (name, description) = match parse_skill_md(&skill_md) {
            Ok((name, description)) => (name, description),
            Err(_) => (dir_name.clone(), String::new()),
        };

        out.push(LocalSkillSummary {
            dir_name,
            path: path.to_string_lossy().to_string(),
            name,
            description,
        });
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn import_local(
    app: &tauri::AppHandle,
    db: &db::Db,
    cli_key: &str,
    dir_name: &str,
) -> Result<InstalledSkillSummary, String> {
    validate_cli_key(cli_key)?;
    ensure_skills_roots(app)?;

    let dir_name = validate_dir_name(dir_name)?;

    let cli_root = cli_skills_root(app, cli_key)?;
    let local_dir = cli_root.join(&dir_name);
    if !local_dir.exists() {
        return Err(format!("SKILL_LOCAL_NOT_FOUND: {}", local_dir.display()));
    }
    if !local_dir.is_dir() {
        return Err("SEC_INVALID_INPUT: local skill path is not a directory".to_string());
    }
    if is_managed_dir(&local_dir) {
        return Err("SKILL_ALREADY_MANAGED: skill already managed by aio-coding-hub".to_string());
    }

    let skill_md = local_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in local skill dir".to_string());
    }

    let (name, description) = match parse_skill_md(&skill_md) {
        Ok(v) => v,
        Err(_) => (dir_name.clone(), String::new()),
    };
    let normalized_name = normalize_name(&name);

    let mut conn = db.open_connection()?;
    if skill_key_exists(&conn, &dir_name)? {
        return Err("SKILL_IMPORT_CONFLICT: same skill_key already exists".to_string());
    }

    let now = now_unix_seconds();
    let ssot_dir = ssot_skills_root(app)?.join(&dir_name);
    if ssot_dir.exists() {
        return Err("SKILL_IMPORT_CONFLICT: ssot dir already exists".to_string());
    }

    let enabled_flags = match cli_key {
        "claude" => (true, false, false),
        "codex" => (false, true, false),
        "gemini" => (false, false, true),
        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    };

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

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
            dir_name,
            name.trim(),
            normalized_name,
            description,
            format!("local://{cli_key}"),
            "local",
            dir_name,
            enabled_to_int(enabled_flags.0),
            enabled_to_int(enabled_flags.1),
            enabled_to_int(enabled_flags.2),
            now,
            now
        ],
    )
    .map_err(|e| format!("DB_ERROR: failed to insert imported skill: {e}"))?;

    let skill_id = tx.last_insert_rowid();

    if let Err(err) = copy_dir_recursive(&local_dir, &ssot_dir) {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
        return Err(err);
    }

    if let Err(err) = write_marker(&local_dir) {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        let _ = tx.execute("DELETE FROM skills WHERE id = ?1", params![skill_id]);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        let _ = std::fs::remove_dir_all(&ssot_dir);
        remove_marker(&local_dir);
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    get_skill_by_id(&conn, skill_id)
}
