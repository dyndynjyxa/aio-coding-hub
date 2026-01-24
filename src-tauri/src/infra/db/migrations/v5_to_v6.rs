//! Usage: SQLite migration v5->v6.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v5_to_v6(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 6;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE request_logs ADD COLUMN ttfb_ms INTEGER;

UPDATE request_logs
SET ttfb_ms = duration_ms
WHERE ttfb_ms IS NULL;
"#,
    )
    .map_err(|e| format!("failed to migrate v5->v6: {e}"))?;

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
