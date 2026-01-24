//! Usage: SQLite migration v17->v18.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v17_to_v18(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 18;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sort_modes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id),
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE CASCADE,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sort_mode_active (
  cli_key TEXT PRIMARY KEY,
  mode_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_mode_cli_sort_order
  ON sort_mode_providers(mode_id, cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_provider_id
  ON sort_mode_providers(provider_id);
CREATE INDEX IF NOT EXISTS idx_sort_mode_active_mode_id
  ON sort_mode_active(mode_id);
"#,
    )
    .map_err(|e| format!("failed to migrate v17->v18: {e}"))?;

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
