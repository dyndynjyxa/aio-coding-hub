use super::types::InstalledSkillSummary;
use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::workspaces;
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
        installed_commit: row.get("installed_commit")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn installed_list_for_workspace(
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<InstalledSkillSummary>> {
    let conn = db.open_connection()?;
    let _ = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      s.id,
      s.skill_key,
      s.name,
      s.description,
      s.source_git_url,
      s.source_branch,
      s.source_subdir,
      s.installed_commit,
      CASE WHEN e.skill_id IS NULL THEN 0 ELSE 1 END AS enabled,
      s.created_at,
      s.updated_at
    FROM skills s
    LEFT JOIN workspace_skill_enabled e
      ON e.workspace_id = ?1 AND e.skill_id = s.id
    ORDER BY s.updated_at DESC, s.id DESC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare installed list query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], row_to_installed)
        .map_err(|e| db_err!("failed to list skills: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read skill row: {e}"))?);
    }
    Ok(out)
}

pub(super) fn installed_source_set(
    conn: &Connection,
) -> crate::shared::error::AppResult<HashSet<String>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT source_git_url, source_branch, source_subdir
    FROM skills
    "#,
        )
        .map_err(|e| db_err!("failed to prepare installed source query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let url: String = row.get(0)?;
            let branch: String = row.get(1)?;
            let subdir: String = row.get(2)?;
            Ok(format!("{}#{}#{}", url, branch, subdir))
        })
        .map_err(|e| db_err!("failed to query installed sources: {e}"))?;

    let mut set = HashSet::new();
    for row in rows {
        set.insert(row.map_err(|e| db_err!("failed to read installed source row: {e}"))?);
    }
    Ok(set)
}

pub(super) fn get_skill_by_id(
    conn: &Connection,
    skill_id: i64,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    conn.query_row(
        r#"
SELECT
  id,
  skill_key,
  name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  installed_commit,
  0 AS enabled,
  created_at,
  updated_at
FROM skills
WHERE id = ?1
"#,
        params![skill_id],
        row_to_installed,
    )
    .optional()
    .map_err(|e| db_err!("failed to query skill: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: skill not found"))
}

pub(super) fn get_skill_by_id_for_workspace(
    conn: &Connection,
    workspace_id: i64,
    skill_id: i64,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    conn.query_row(
        r#"
SELECT
  s.id,
  s.skill_key,
  s.name,
  s.description,
  s.source_git_url,
  s.source_branch,
  s.source_subdir,
  s.installed_commit,
  CASE WHEN e.skill_id IS NULL THEN 0 ELSE 1 END AS enabled,
  s.created_at,
  s.updated_at
FROM skills s
LEFT JOIN workspace_skill_enabled e
  ON e.workspace_id = ?1 AND e.skill_id = s.id
WHERE s.id = ?2
"#,
        params![workspace_id, skill_id],
        row_to_installed,
    )
    .optional()
    .map_err(|e| db_err!("failed to query skill: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: skill not found"))
}

pub(super) fn skill_key_exists(
    conn: &Connection,
    key: &str,
) -> crate::shared::error::AppResult<bool> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM skills WHERE skill_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query skill_key: {e}"))?;
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

pub(super) fn generate_unique_skill_key(
    conn: &Connection,
    name: &str,
) -> crate::shared::error::AppResult<String> {
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
