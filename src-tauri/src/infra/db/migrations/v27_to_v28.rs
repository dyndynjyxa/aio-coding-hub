//! Usage: SQLite migration v27->v28.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v27_to_v28(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 28;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v27->v28: {e}"))?;

    let mut has_provider_mode = false;
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
            if name == "provider_mode" {
                has_provider_mode = true;
                break;
            }
        }
    }

    if has_provider_mode {
        // Drop official providers entirely (official provider mode is no longer supported).
        tx.execute("DELETE FROM providers WHERE provider_mode = 'official'", [])
            .map_err(|e| format!("failed to delete official providers: {e}"))?;

        tx.execute_batch(
            r#"
ALTER TABLE providers RENAME TO providers_old;
ALTER TABLE provider_circuit_breakers RENAME TO provider_circuit_breakers_old;
ALTER TABLE sort_mode_providers RENAME TO sort_mode_providers_old;
ALTER TABLE claude_model_validation_runs RENAME TO claude_model_validation_runs_old;

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);

INSERT INTO providers(
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json
)
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json
FROM providers_old;

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

CREATE TABLE sort_mode_providers (
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

INSERT INTO sort_mode_providers(
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
)
SELECT
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
FROM sort_mode_providers_old;

CREATE TABLE claude_model_validation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  request_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

INSERT INTO claude_model_validation_runs(
  id,
  provider_id,
  created_at,
  request_json,
  result_json
)
SELECT
  id,
  provider_id,
  created_at,
  request_json,
  result_json
FROM claude_model_validation_runs_old;

DROP TABLE provider_circuit_breakers_old;
DROP TABLE sort_mode_providers_old;
DROP TABLE claude_model_validation_runs_old;
DROP TABLE providers_old;

CREATE INDEX IF NOT EXISTS idx_providers_cli_key ON providers(cli_key);
CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_mode_cli_sort_order
  ON sort_mode_providers(mode_id, cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_provider_id
  ON sort_mode_providers(provider_id);
CREATE INDEX IF NOT EXISTS idx_claude_model_validation_runs_provider_id_id
  ON claude_model_validation_runs(provider_id, id);
"#,
        )
        .map_err(|e| format!("failed to migrate v27->v28: {e}"))?;
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
