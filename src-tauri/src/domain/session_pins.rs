//! Usage: Persistent (disk-backed) per-session sort_mode pins.
//!
//! A persistent pin survives gateway restarts and the in-memory session TTL.
//! It is re-applied when a Claude Code session resumes (same `(cli_key,
//! session_id)`). Distinct from the in-memory ephemeral pin (5-min sliding TTL).
//!
//! Row semantics (mirrors the in-memory `Option<Option<i64>>`):
//! - no row            => not persistently pinned
//! - row, `sort_mode_id` NULL => pinned to Default
//! - row, `sort_mode_id` = id => pinned to that custom mode

use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct PersistentPinRow {
    pub cli_key: String,
    pub session_id: String,
    /// `None` = pinned to Default; `Some(id)` = pinned to that custom mode.
    pub sort_mode_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn ensure_mode_exists(conn: &Connection, mode_id: i64) -> crate::shared::error::AppResult<()> {
    if mode_id <= 0 {
        return Err("SEC_INVALID_INPUT: invalid mode_id".into());
    }
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM sort_modes WHERE id = ?1",
            params![mode_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query sort_mode: {e}"))?;
    if exists.is_none() {
        return Err("SEC_INVALID_INPUT: sort_mode does not exist".into());
    }
    Ok(())
}

/// Read the persistent pin for a `(cli_key, session_id)`.
///
/// Returns `None` if no persistent pin; `Some(None)` if pinned to Default;
/// `Some(Some(id))` if pinned to a custom mode. Shaped for the routing
/// resolution chain.
pub(crate) fn get_persistent_pin(
    db: &db::Db,
    cli_key: &str,
    session_id: &str,
) -> crate::shared::error::AppResult<Option<Option<i64>>> {
    let conn = db.open_connection()?;
    let row: Option<Option<i64>> = conn
        .query_row(
            "SELECT sort_mode_id FROM gateway_session_persistent_pins WHERE cli_key = ?1 AND session_id = ?2",
            params![cli_key, session_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query persistent pin: {e}"))?;
    Ok(row)
}

/// Create or update a persistent pin. `sort_mode_id == None` pins to Default.
/// Validates that a `Some(id)` references an existing sort_mode.
pub(crate) fn upsert_persistent_pin(
    db: &db::Db,
    cli_key: &str,
    session_id: &str,
    sort_mode_id: Option<i64>,
) -> crate::shared::error::AppResult<()> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("SEC_INVALID_INPUT: session_id is empty".into());
    }

    let conn = db.open_connection()?;
    if let Some(mode_id) = sort_mode_id {
        ensure_mode_exists(&conn, mode_id)?;
    }
    let now = now_unix_seconds();

    conn.execute(
        r#"
INSERT INTO gateway_session_persistent_pins(
  cli_key,
  session_id,
  sort_mode_id,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?4)
ON CONFLICT(cli_key, session_id) DO UPDATE SET
  sort_mode_id = excluded.sort_mode_id,
  updated_at = excluded.updated_at
"#,
        params![cli_key, session_id, sort_mode_id, now],
    )
    .map_err(|e| db_err!("failed to upsert persistent pin: {e}"))?;
    Ok(())
}

/// Remove a persistent pin. Returns `true` if a row was deleted.
pub(crate) fn delete_persistent_pin(
    db: &db::Db,
    cli_key: &str,
    session_id: &str,
) -> crate::shared::error::AppResult<bool> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("SEC_INVALID_INPUT: session_id is empty".into());
    }

    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            "DELETE FROM gateway_session_persistent_pins WHERE cli_key = ?1 AND session_id = ?2",
            params![cli_key, session_id],
        )
        .map_err(|e| db_err!("failed to delete persistent pin: {e}"))?;
    Ok(changed > 0)
}

/// List all persistent pins (for UI回显 / management).
pub(crate) fn list_persistent_pins(
    db: &db::Db,
) -> crate::shared::error::AppResult<Vec<PersistentPinRow>> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT cli_key, session_id, sort_mode_id, created_at, updated_at
FROM gateway_session_persistent_pins
ORDER BY updated_at DESC
"#,
        )
        .map_err(|e| db_err!("failed to prepare persistent pins query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(PersistentPinRow {
                cli_key: row.get("cli_key")?,
                session_id: row.get("session_id")?,
                sort_mode_id: row.get("sort_mode_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| db_err!("failed to list persistent pins: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read persistent pin row: {e}"))?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests;
