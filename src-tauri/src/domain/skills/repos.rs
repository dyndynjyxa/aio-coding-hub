use super::git_url::{canonical_git_url_key, normalize_repo_branch};
use super::types::SkillRepoSummary;
use super::util::enabled_to_int;
use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

fn row_to_repo(row: &rusqlite::Row<'_>) -> Result<SkillRepoSummary, rusqlite::Error> {
    Ok(SkillRepoSummary {
        id: row.get("id")?,
        git_url: row.get("git_url")?,
        branch: row.get("branch")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn get_repo_by_id(conn: &Connection, repo_id: i64) -> Result<SkillRepoSummary, String> {
    conn.query_row(
        r#"
SELECT
  id,
  git_url,
  branch,
  enabled,
  created_at,
  updated_at
FROM skill_repos
WHERE id = ?1
"#,
        params![repo_id],
        row_to_repo,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query repo: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: skill repo not found".to_string())
}

pub fn repos_list(db: &db::Db) -> Result<Vec<SkillRepoSummary>, String> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  git_url,
  branch,
  enabled,
  created_at,
  updated_at
FROM skill_repos
ORDER BY updated_at DESC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare repo list query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_repo)
        .map_err(|e| format!("DB_ERROR: failed to query repos: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read repo row: {e}"))?);
    }

    // De-dup repos by canonical git URL for a clearer UX.
    // Keeps the newest record (query is already ordered by updated_at DESC).
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for row in out {
        let key = canonical_git_url_key(&row.git_url);
        let key = if key.is_empty() {
            row.git_url.trim().to_ascii_lowercase()
        } else {
            key
        };
        if seen.insert(key) {
            deduped.push(row);
        }
    }

    Ok(deduped)
}

pub fn repo_upsert(
    db: &db::Db,
    repo_id: Option<i64>,
    git_url: &str,
    branch: &str,
    enabled: bool,
) -> Result<SkillRepoSummary, String> {
    let git_url = git_url.trim();
    if git_url.is_empty() {
        return Err("SEC_INVALID_INPUT: git_url is required".to_string());
    }
    let branch = normalize_repo_branch(branch);

    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    match repo_id {
        None => {
            let canonical = canonical_git_url_key(git_url);
            let canonical = if canonical.is_empty() {
                git_url.to_ascii_lowercase()
            } else {
                canonical
            };

            let mut stmt = conn
                .prepare(
                    r#"
SELECT id, git_url, branch
FROM skill_repos
ORDER BY updated_at DESC, id DESC
"#,
                )
                .map_err(|e| format!("DB_ERROR: failed to prepare repo lookup: {e}"))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| format!("DB_ERROR: failed to query repos: {e}"))?;

            let mut matches = Vec::new();
            for row in rows {
                let (id, existing_url, existing_branch) =
                    row.map_err(|e| format!("DB_ERROR: failed to read repo row: {e}"))?;
                let key = canonical_git_url_key(&existing_url);
                let key = if key.is_empty() {
                    existing_url.trim().to_ascii_lowercase()
                } else {
                    key
                };
                if key == canonical {
                    matches.push((id, existing_url, existing_branch));
                }
            }

            if !matches.is_empty() {
                let mut target_id = matches[0].0;
                for (id, existing_url, existing_branch) in &matches {
                    if existing_url.trim() == git_url
                        && normalize_repo_branch(existing_branch) == branch
                    {
                        target_id = *id;
                        break;
                    }
                }

                conn.execute(
                    r#"
UPDATE skill_repos
SET
  git_url = ?1,
  branch = ?2,
  enabled = ?3,
  updated_at = ?4
WHERE id = ?5
"#,
                    params![git_url, branch, enabled_to_int(enabled), now, target_id],
                )
                .map_err(|e| format!("DB_ERROR: failed to update skill repo: {e}"))?;

                return get_repo_by_id(&conn, target_id);
            }

            conn.execute(
                r#"
INSERT INTO skill_repos(
  git_url,
  branch,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5)
"#,
                params![git_url, branch, enabled_to_int(enabled), now, now],
            )
            .map_err(|e| format!("DB_ERROR: failed to insert skill repo: {e}"))?;

            let id = conn.last_insert_rowid();
            get_repo_by_id(&conn, id)
        }
        Some(id) => {
            conn.execute(
                r#"
UPDATE skill_repos
SET
  git_url = ?1,
  branch = ?2,
  enabled = ?3,
  updated_at = ?4
WHERE id = ?5
"#,
                params![git_url, branch, enabled_to_int(enabled), now, id],
            )
            .map_err(|e| format!("DB_ERROR: failed to update skill repo: {e}"))?;
            get_repo_by_id(&conn, id)
        }
    }
}

pub fn repo_delete(db: &db::Db, repo_id: i64) -> Result<(), String> {
    let conn = db.open_connection()?;
    let changed = conn
        .execute("DELETE FROM skill_repos WHERE id = ?1", params![repo_id])
        .map_err(|e| format!("DB_ERROR: failed to delete skill repo: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: skill repo not found".to_string());
    }
    Ok(())
}
