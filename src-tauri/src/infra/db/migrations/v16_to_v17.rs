//! Usage: SQLite migration v16->v17.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v16_to_v17(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 17;
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
    .map_err(|e| format!("failed to migrate v16->v17: {e}"))?;

    tx.execute_batch(
        r#"
ALTER TABLE provider_circuit_breakers RENAME TO provider_circuit_breakers_old;

CREATE TABLE provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

INSERT INTO provider_circuit_breakers(
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
)
SELECT
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
FROM provider_circuit_breakers_old;

DROP TABLE provider_circuit_breakers_old;

CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
"#,
    )
    .map_err(|e| format!("failed to migrate v16->v17: {e}"))?;

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
