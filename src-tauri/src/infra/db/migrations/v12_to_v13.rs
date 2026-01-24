//! Usage: SQLite migration v12->v13.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v12_to_v13(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 13;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE providers ADD COLUMN cost_multiplier REAL NOT NULL DEFAULT 1.0;

ALTER TABLE request_logs ADD COLUMN requested_model TEXT;
ALTER TABLE request_logs ADD COLUMN cost_usd_femto INTEGER;

CREATE TABLE IF NOT EXISTS model_prices (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  model TEXT NOT NULL,
  price_json TEXT NOT NULL,
  currency TEXT NOT NULL DEFAULT 'USD',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, model)
);

CREATE INDEX IF NOT EXISTS idx_model_prices_cli_key ON model_prices(cli_key);
CREATE INDEX IF NOT EXISTS idx_model_prices_cli_key_model ON model_prices(cli_key, model);
"#,
    )
    .map_err(|e| format!("failed to migrate v12->v13: {e}"))?;

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
