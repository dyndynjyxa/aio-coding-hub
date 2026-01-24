//! Usage: SQLite migration v15->v16.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v15_to_v16(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 16;
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
    .map_err(|e| format!("failed to migrate v15->v16: {e}"))?;

    let mut has_weight = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "weight" {
                has_weight = true;
                break;
            }
        }
    }

    if has_weight {
        tx.execute_batch(
            r#"
ALTER TABLE providers RENAME TO providers_old;

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  UNIQUE(cli_key, name)
);

INSERT INTO providers(
  id,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier
)
SELECT
  id,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier
FROM providers_old;

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
  CASE WHEN state = 'OPEN' THEN 'OPEN' ELSE 'CLOSED' END,
  failure_count,
  CASE WHEN state = 'OPEN' THEN open_until ELSE NULL END,
  updated_at
FROM provider_circuit_breakers_old;

DROP TABLE provider_circuit_breakers_old;
DROP TABLE providers_old;

CREATE INDEX IF NOT EXISTS idx_providers_cli_key ON providers(cli_key);
CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
"#,
        )
        .map_err(|e| format!("failed to migrate v15->v16: {e}"))?;
    }

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
