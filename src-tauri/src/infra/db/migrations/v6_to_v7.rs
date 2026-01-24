//! Usage: SQLite migration v6->v7.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v6_to_v7(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 7;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS request_attempt_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trace_id TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  query TEXT,
  attempt_index INTEGER NOT NULL,
  provider_id INTEGER NOT NULL,
  provider_name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  outcome TEXT NOT NULL,
  status INTEGER,
  attempt_started_ms INTEGER NOT NULL DEFAULT 0,
  attempt_duration_ms INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_request_attempt_logs_trace_attempt
  ON request_attempt_logs(trace_id, attempt_index);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_trace_id
  ON request_attempt_logs(trace_id);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_created_at
  ON request_attempt_logs(created_at);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_cli_created_at
  ON request_attempt_logs(cli_key, created_at);
"#,
    )
    .map_err(|e| format!("failed to migrate v6->v7: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}
