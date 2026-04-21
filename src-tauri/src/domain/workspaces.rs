//! Usage: Workspace (profile) persistence and active workspace resolution.

use crate::db;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceSummary {
    pub id: i64,
    pub cli_key: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacesListResult {
    pub active_id: Option<i64>,
    pub items: Vec<WorkspaceSummary>,
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<WorkspaceSummary, rusqlite::Error> {
    Ok(WorkspaceSummary {
        id: row.get("id")?,
        cli_key: row.get("cli_key")?,
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn get_by_id(conn: &Connection, workspace_id: i64) -> Result<WorkspaceSummary, String> {
    conn.query_row(
        r#"
SELECT
  id,
  cli_key,
  name,
  created_at,
  updated_at
FROM workspaces
WHERE id = ?1
"#,
        params![workspace_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| db_err!("failed to query workspace: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: workspace not found".to_string())
}

pub fn get_cli_key_by_id(
    conn: &Connection,
    workspace_id: i64,
) -> crate::shared::error::AppResult<String> {
    conn.query_row(
        "SELECT cli_key FROM workspaces WHERE id = ?1",
        params![workspace_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| db_err!("failed to query workspace cli_key: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: workspace not found".to_string().into())
}

pub fn active_id_by_cli(
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<i64>> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let id = conn
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = ?1",
            params![cli_key],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query workspace_active: {e}"))?;

    Ok(id.flatten())
}

pub fn is_active_workspace(
    conn: &Connection,
    workspace_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let cli_key = get_cli_key_by_id(conn, workspace_id)?;
    let active_id = active_id_by_cli(conn, &cli_key)?;
    Ok(active_id == Some(workspace_id))
}

pub fn list_by_cli(
    db: &db::Db,
    cli_key: &str,
) -> crate::shared::error::AppResult<WorkspacesListResult> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let conn = db.open_connection()?;
    let active_id = active_id_by_cli(&conn, cli_key)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      id,
      cli_key,
      name,
      created_at,
      updated_at
    FROM workspaces
    WHERE cli_key = ?1
    ORDER BY updated_at DESC, id DESC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare workspaces query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| db_err!("failed to list workspaces: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read workspace row: {e}"))?);
    }

    Ok(WorkspacesListResult { active_id, items })
}

pub fn create(
    db: &db::Db,
    cli_key: &str,
    name: &str,
    clone_from_active: bool,
) -> crate::shared::error::AppResult<WorkspaceSummary> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: workspace name is required"
            .to_string()
            .into());
    }

    let normalized_name = normalize_name(name);
    let now = now_unix_seconds();

    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    tx.execute(
        r#"
INSERT INTO workspaces(
  cli_key,
  name,
  normalized_name,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5)
"#,
        params![cli_key, name, normalized_name, now, now],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            crate::shared::error::AppError::new(
                "DB_CONSTRAINT",
                format!("workspace already exists for cli_key={cli_key}, name={name}"),
            )
        }
        other => db_err!("failed to insert workspace: {other}"),
    })?;

    let id = tx.last_insert_rowid();

    if clone_from_active {
        if let Some(from_workspace_id) = active_id_by_cli(&tx, cli_key)? {
            tx.execute(
                r#"
INSERT INTO prompts(
  workspace_id,
  name,
  content,
  enabled,
  created_at,
  updated_at
)
SELECT
  ?1,
  name,
  content,
  enabled,
  ?3,
  ?3
FROM prompts
WHERE workspace_id = ?2
"#,
                params![id, from_workspace_id, now],
            )
            .map_err(|e| db_err!("failed to clone prompts: {e}"))?;

            tx.execute(
                r#"
INSERT OR IGNORE INTO workspace_mcp_enabled(
  workspace_id,
  server_id,
  created_at,
  updated_at
)
SELECT
  ?1,
  server_id,
  ?3,
  ?3
FROM workspace_mcp_enabled
WHERE workspace_id = ?2
"#,
                params![id, from_workspace_id, now],
            )
            .map_err(|e| db_err!("failed to clone mcp enabled: {e}"))?;

            tx.execute(
                r#"
INSERT OR IGNORE INTO workspace_skill_enabled(
  workspace_id,
  skill_id,
  created_at,
  updated_at
)
SELECT
  ?1,
  skill_id,
  ?3,
  ?3
FROM workspace_skill_enabled
WHERE workspace_id = ?2
"#,
                params![id, from_workspace_id, now],
            )
            .map_err(|e| db_err!("failed to clone skills enabled: {e}"))?;
        }
    } else {
        tx.execute(
            r#"
INSERT INTO prompts(
  workspace_id,
  name,
  content,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, 1, ?4, ?4)
"#,
            params![id, "默认", "", now],
        )
        .map_err(|e| db_err!("failed to seed default prompt: {e}"))?;
    }

    if let Err(err) = tx.commit() {
        return Err(db_err!("failed to commit: {err}"));
    }

    Ok(get_by_id(&conn, id)?)
}

pub fn rename(
    db: &db::Db,
    workspace_id: i64,
    name: &str,
) -> crate::shared::error::AppResult<WorkspaceSummary> {
    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: workspace name is required"
            .to_string()
            .into());
    }

    let normalized_name = normalize_name(name);
    let now = now_unix_seconds();

    let conn = db.open_connection()?;
    let before = get_by_id(&conn, workspace_id)?;

    conn.execute(
        r#"
UPDATE workspaces
SET
  name = ?1,
  normalized_name = ?2,
  updated_at = ?3
WHERE id = ?4
"#,
        params![name, normalized_name, now, workspace_id],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            crate::shared::error::AppError::new(
                "DB_CONSTRAINT",
                format!(
                    "workspace already exists for cli_key={}, name={}",
                    before.cli_key, name
                ),
            )
        }
        other => db_err!("failed to rename workspace: {other}"),
    })?;

    Ok(get_by_id(&conn, workspace_id)?)
}

pub fn delete(db: &db::Db, workspace_id: i64) -> crate::shared::error::AppResult<bool> {
    let mut conn = db.open_connection()?;
    let before = get_by_id(&conn, workspace_id)?;

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let active_id = active_id_by_cli(&tx, &before.cli_key)?;
    if active_id == Some(workspace_id) {
        return Err(
            "SEC_INVALID_INPUT: cannot delete active workspace; switch first"
                .to_string()
                .into(),
        );
    }

    let changed = tx
        .execute(
            "DELETE FROM workspaces WHERE id = ?1",
            params![workspace_id],
        )
        .map_err(|e| db_err!("failed to delete workspace: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: workspace not found".to_string().into());
    }

    if let Err(err) = tx.commit() {
        return Err(db_err!("failed to commit: {err}"));
    }

    Ok(true)
}

pub fn set_active(db: &db::Db, workspace_id: i64) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let cli_key = get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;

    let now = now_unix_seconds();
    conn.execute(
        r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
        params![cli_key, workspace_id, now],
    )
    .map_err(|e| db_err!("failed to update workspace_active: {e}"))?;

    Ok(())
}
