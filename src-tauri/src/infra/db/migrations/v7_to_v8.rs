//! Usage: SQLite migration v7->v8.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v7_to_v8(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 8;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE INDEX IF NOT EXISTS idx_prompts_cli_key ON prompts(cli_key);
CREATE INDEX IF NOT EXISTS idx_prompts_cli_key_updated_at ON prompts(cli_key, updated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_cli_key_single_enabled
  ON prompts(cli_key)
  WHERE enabled = 1;
"#,
    )
    .map_err(|e| format!("failed to migrate v7->v8: {e}"))?;

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
