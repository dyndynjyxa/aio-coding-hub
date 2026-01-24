//! Usage: SQLite migration v20->v21.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v20_to_v21(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 21;
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
    .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;

    let mut has_session_id = false;
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
            if name == "session_id" {
                has_session_id = true;
                break;
            }
        }
    }

    if !has_session_id {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN session_id TEXT;
"#,
        )
        .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;
    }

    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_session_id ON request_logs(session_id);
"#,
    )
    .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;

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
