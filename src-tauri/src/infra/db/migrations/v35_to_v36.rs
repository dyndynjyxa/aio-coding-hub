//! Usage: SQLite migration v35->v36 - Persistent per-session sort_mode pins.
//!
//! Adds `gateway_session_persistent_pins`: a disk-backed pin that survives
//! gateway restarts and the in-memory session TTL. Re-applied when a Claude Code
//! session resumes (same `(cli_key, session_id)`). Distinct from the in-memory
//! ephemeral pin (5-min sliding TTL). Row presence = pinned; `sort_mode_id NULL`
//! = pinned to Default; non-null = pinned to that custom mode.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v35_to_v36(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 36;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;
    let now = now_unix_seconds();

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS gateway_session_persistent_pins (
  cli_key TEXT NOT NULL,
  session_id TEXT NOT NULL,
  sort_mode_id INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, session_id),
  FOREIGN KEY(sort_mode_id) REFERENCES sort_modes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_gw_session_persistent_pins_sort_mode_id
  ON gateway_session_persistent_pins(sort_mode_id);
"#,
    )
    .map_err(|e| format!("failed to create gateway_session_persistent_pins table: {e}"))?;

    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
    )
    .map_err(|e| format!("failed to create schema_migrations table: {e}"))?;
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, ?)",
        [VERSION, now],
    )
    .map_err(|e| format!("failed to insert schema_migrations row for v{VERSION}: {e}"))?;

    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;

    Ok(())
}
