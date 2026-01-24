//! Usage: SQLite migration v9->v10.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v9_to_v10(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 10;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE mcp_servers ADD COLUMN normalized_name TEXT NOT NULL DEFAULT '';

UPDATE mcp_servers
SET normalized_name = LOWER(TRIM(name))
WHERE normalized_name = '' OR normalized_name IS NULL;

CREATE INDEX IF NOT EXISTS idx_mcp_servers_normalized_name ON mcp_servers(normalized_name);
"#,
    )
    .map_err(|e| format!("failed to migrate v9->v10: {e}"))?;

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
