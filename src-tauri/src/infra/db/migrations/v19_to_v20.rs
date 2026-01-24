//! Usage: SQLite migration v19->v20.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v19_to_v20(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 20;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;

    let mut has_created_at_ms = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(request_logs)")
            .map_err(|e| format!("failed to prepare request_logs table_info: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query request_logs table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read request_logs column name: {e}"))?;
            if name == "created_at_ms" {
                has_created_at_ms = true;
                break;
            }
        }
    }

    if !has_created_at_ms {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0;
"#,
        )
        .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;
    }

    tx.execute(
        "UPDATE request_logs SET created_at_ms = created_at * 1000 WHERE created_at_ms = 0",
        [],
    )
    .map_err(|e| format!("failed to backfill request_logs.created_at_ms: {e}"))?;

    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at_ms ON request_logs(created_at_ms);
CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at_ms
  ON request_logs(cli_key, created_at_ms);
"#,
    )
    .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;

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
