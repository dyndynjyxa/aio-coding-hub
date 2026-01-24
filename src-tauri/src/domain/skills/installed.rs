use super::types::InstalledSkillSummary;
use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

fn row_to_installed(row: &rusqlite::Row<'_>) -> Result<InstalledSkillSummary, rusqlite::Error> {
    Ok(InstalledSkillSummary {
        id: row.get("id")?,
        skill_key: row.get("skill_key")?,
        name: row.get("name")?,
        description: row.get("description")?,
        source_git_url: row.get("source_git_url")?,
        source_branch: row.get("source_branch")?,
        source_subdir: row.get("source_subdir")?,
        enabled_claude: row.get::<_, i64>("enabled_claude")? != 0,
        enabled_codex: row.get::<_, i64>("enabled_codex")? != 0,
        enabled_gemini: row.get::<_, i64>("enabled_gemini")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn installed_list(db: &db::Db) -> Result<Vec<InstalledSkillSummary>, String> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
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
FROM skills
ORDER BY updated_at DESC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare installed list query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_installed)
        .map_err(|e| format!("DB_ERROR: failed to list skills: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read skill row: {e}"))?);
    }
    Ok(out)
}

pub(super) fn installed_source_set(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
SELECT source_git_url, source_branch, source_subdir
FROM skills
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare installed source query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let url: String = row.get(0)?;
            let branch: String = row.get(1)?;
            let subdir: String = row.get(2)?;
            Ok(format!("{}#{}#{}", url, branch, subdir))
        })
        .map_err(|e| format!("DB_ERROR: failed to query installed sources: {e}"))?;

    let mut set = HashSet::new();
    for row in rows {
        set.insert(row.map_err(|e| format!("DB_ERROR: failed to read installed source row: {e}"))?);
    }
    Ok(set)
}

pub(super) fn get_skill_by_id(
    conn: &Connection,
    skill_id: i64,
) -> Result<InstalledSkillSummary, String> {
    conn.query_row(
        r#"
SELECT
  id,
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
FROM skills
WHERE id = ?1
"#,
        params![skill_id],
        row_to_installed,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query skill: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: skill not found".to_string())
}

pub(super) fn skill_key_exists(conn: &Connection, key: &str) -> Result<bool, String> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM skills WHERE skill_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query skill_key: {e}"))?;
    Ok(exists.is_some())
}

fn suggest_key(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
            continue;
        }
        if lower == '_' || lower == '-' {
            if !out.is_empty() && !prev_dash {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }
        if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "skill".to_string()
    } else {
        out
    }
}

pub(super) fn generate_unique_skill_key(conn: &Connection, name: &str) -> Result<String, String> {
    let base = suggest_key(name);
    if !skill_key_exists(conn, &base)? {
        return Ok(base);
    }
    for idx in 2..1000 {
        let candidate = format!("{base}-{idx}");
        if !skill_key_exists(conn, &candidate)? {
            return Ok(candidate);
        }
    }
    Ok(format!("skill-{}", now_unix_seconds()))
}
