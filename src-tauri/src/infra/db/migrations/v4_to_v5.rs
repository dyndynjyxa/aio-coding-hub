//! Usage: SQLite migration v4->v5.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v4_to_v5(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 5;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE request_logs ADD COLUMN input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN output_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN total_tokens INTEGER;

ALTER TABLE request_logs ADD COLUMN cache_read_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_5m_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_1h_input_tokens INTEGER;

ALTER TABLE request_logs ADD COLUMN usage_json TEXT;
"#,
    )
    .map_err(|e| format!("failed to migrate v4->v5: {e}"))?;

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
