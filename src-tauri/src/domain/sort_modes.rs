//! Usage: Sort mode persistence and provider ordering configuration helpers.

use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct SortModeSummary {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortModeActiveRow {
    pub cli_key: String,
    pub mode_id: Option<i64>,
    pub updated_at: i64,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn validate_mode_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: mode name is required".to_string());
    }

    if name.chars().count() > 32 {
        return Err("SEC_INVALID_INPUT: mode name is too long (max 32 chars)".to_string());
    }

    let lowered = name.to_ascii_lowercase();
    if lowered == "default" || name == "默认" {
        return Err("SEC_INVALID_INPUT: mode name is reserved".to_string());
    }

    Ok(name.to_string())
}

fn row_to_mode_summary(row: &rusqlite::Row<'_>) -> Result<SortModeSummary, rusqlite::Error> {
    Ok(SortModeSummary {
        id: row.get("id")?,
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn ensure_mode_exists(conn: &Connection, mode_id: i64) -> Result<(), String> {
    if mode_id <= 0 {
        return Err("SEC_INVALID_INPUT: invalid mode_id".to_string());
    }

    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM sort_modes WHERE id = ?1",
            params![mode_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query sort_mode: {e}"))?;

    if exists.is_none() {
        return Err("DB_NOT_FOUND: sort_mode not found".to_string());
    }

    Ok(())
}

fn read_active_row(conn: &Connection, cli_key: &str) -> Result<SortModeActiveRow, String> {
    conn.query_row(
        r#"
SELECT
  cli_key,
  mode_id,
  updated_at
FROM sort_mode_active
WHERE cli_key = ?1
"#,
        params![cli_key],
        |row| {
            Ok(SortModeActiveRow {
                cli_key: row.get("cli_key")?,
                mode_id: row.get("mode_id")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query sort_mode_active: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: sort_mode_active not found".to_string())
}

pub fn list_modes(db: &db::Db) -> Result<Vec<SortModeSummary>, String> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  name,
  created_at,
  updated_at
FROM sort_modes
ORDER BY id ASC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare sort_modes query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_mode_summary)
        .map_err(|e| format!("DB_ERROR: failed to list sort_modes: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read sort_mode row: {e}"))?);
    }
    Ok(items)
}

pub fn create_mode(db: &db::Db, name: &str) -> Result<SortModeSummary, String> {
    let name = validate_mode_name(name)?;
    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    conn.execute(
        r#"
INSERT INTO sort_modes(
  name,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3)
"#,
        params![name, now, now],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            format!("DB_CONSTRAINT: sort_mode already exists: name={name}")
        }
        other => format!("DB_ERROR: failed to insert sort_mode: {other}"),
    })?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        r#"
SELECT
  id,
  name,
  created_at,
  updated_at
FROM sort_modes
WHERE id = ?1
"#,
        params![id],
        row_to_mode_summary,
    )
    .map_err(|e| format!("DB_ERROR: failed to query inserted sort_mode: {e}"))
}

pub fn rename_mode(db: &db::Db, mode_id: i64, name: &str) -> Result<SortModeSummary, String> {
    let name = validate_mode_name(name)?;
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    let now = now_unix_seconds();

    conn.execute(
        "UPDATE sort_modes SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now, mode_id],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            format!("DB_CONSTRAINT: sort_mode already exists: name={name}")
        }
        other => format!("DB_ERROR: failed to update sort_mode: {other}"),
    })?;

    conn.query_row(
        r#"
SELECT
  id,
  name,
  created_at,
  updated_at
FROM sort_modes
WHERE id = ?1
"#,
        params![mode_id],
        row_to_mode_summary,
    )
    .map_err(|e| format!("DB_ERROR: failed to query sort_mode: {e}"))
}

pub fn delete_mode(db: &db::Db, mode_id: i64) -> Result<(), String> {
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;

    let changed = conn
        .execute("DELETE FROM sort_modes WHERE id = ?1", params![mode_id])
        .map_err(|e| format!("DB_ERROR: failed to delete sort_mode: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: sort_mode not found".to_string());
    }
    Ok(())
}

pub fn list_active(db: &db::Db) -> Result<Vec<SortModeActiveRow>, String> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  cli_key,
  mode_id,
  updated_at
FROM sort_mode_active
ORDER BY cli_key ASC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare sort_mode_active query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SortModeActiveRow {
                cli_key: row.get("cli_key")?,
                mode_id: row.get("mode_id")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to list sort_mode_active: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read sort_mode_active row: {e}"))?);
    }
    Ok(items)
}

pub fn set_active(
    db: &db::Db,
    cli_key: &str,
    mode_id: Option<i64>,
) -> Result<SortModeActiveRow, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let conn = db.open_connection()?;
    if let Some(mode_id) = mode_id {
        ensure_mode_exists(&conn, mode_id)?;
    }
    let now = now_unix_seconds();

    conn.execute(
        r#"
INSERT INTO sort_mode_active(
  cli_key,
  mode_id,
  updated_at
) VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  mode_id = excluded.mode_id,
  updated_at = excluded.updated_at
"#,
        params![cli_key, mode_id, now],
    )
    .map_err(|e| format!("DB_ERROR: failed to upsert sort_mode_active: {e}"))?;

    read_active_row(&conn, cli_key)
}

pub fn list_mode_providers(db: &db::Db, mode_id: i64, cli_key: &str) -> Result<Vec<i64>, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  provider_id
FROM sort_mode_providers
WHERE mode_id = ?1
  AND cli_key = ?2
ORDER BY sort_order ASC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare sort_mode_providers query: {e}"))?;

    let rows = stmt
        .query_map(params![mode_id, cli_key], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("DB_ERROR: failed to list sort_mode_providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read provider_id row: {e}"))?);
    }
    Ok(items)
}

fn ensure_providers_belong_to_cli(
    conn: &Connection,
    cli_key: &str,
    provider_ids: &[i64],
) -> Result<(), String> {
    if provider_ids.is_empty() {
        return Ok(());
    }

    let mut unique_ids = HashSet::new();
    for id in provider_ids {
        if *id <= 0 {
            return Err(format!("SEC_INVALID_INPUT: invalid provider_id={id}"));
        }
        if !unique_ids.insert(*id) {
            return Err(format!("SEC_INVALID_INPUT: duplicate provider_id={id}"));
        }
    }

    let placeholders = db::sql_placeholders(unique_ids.len());
    let sql = format!("SELECT id FROM providers WHERE cli_key = ?1 AND id IN ({placeholders})");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare provider validation query: {e}"))?;

    let mut params_vec: Vec<rusqlite::types::Value> = Vec::with_capacity(unique_ids.len() + 1);
    params_vec.push(rusqlite::types::Value::from(cli_key.to_string()));
    params_vec.extend(unique_ids.iter().map(|id| (*id).into()));

    let rows = stmt
        .query_map(params_from_iter(params_vec), |row| row.get::<_, i64>(0))
        .map_err(|e| format!("DB_ERROR: failed to query provider validation: {e}"))?;

    let mut found = HashSet::new();
    for row in rows {
        found.insert(row.map_err(|e| format!("DB_ERROR: failed to read provider id: {e}"))?);
    }

    if found.len() != unique_ids.len() {
        let missing: Vec<i64> = unique_ids.difference(&found).copied().collect();
        return Err(format!(
            "SEC_INVALID_INPUT: provider_id does not belong to cli_key={cli_key}: {missing:?}"
        ));
    }

    Ok(())
}

pub fn set_mode_providers_order(
    db: &db::Db,
    mode_id: i64,
    cli_key: &str,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<i64>, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let mut conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    ensure_providers_belong_to_cli(&conn, cli_key, &ordered_provider_ids)?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;
    tx.execute(
        "DELETE FROM sort_mode_providers WHERE mode_id = ?1 AND cli_key = ?2",
        params![mode_id, cli_key],
    )
    .map_err(|e| format!("DB_ERROR: failed to clear sort_mode_providers: {e}"))?;

    let now = now_unix_seconds();
    for (idx, provider_id) in ordered_provider_ids.iter().enumerate() {
        tx.execute(
            r#"
INSERT INTO sort_mode_providers(
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
"#,
            params![mode_id, cli_key, provider_id, idx as i64, now, now],
        )
        .map_err(|e| format!("DB_ERROR: failed to insert sort_mode_provider: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit transaction: {e}"))?;

    list_mode_providers(db, mode_id, cli_key)
}
