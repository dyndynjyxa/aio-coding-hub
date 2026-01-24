//! Usage: SQLite migration v24->v25.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v24_to_v25(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 25;
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
    .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;

    let mut has_supported_models_json = false;
    let mut has_model_mapping_json = false;
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
            if name == "supported_models_json" {
                has_supported_models_json = true;
            }
            if name == "model_mapping_json" {
                has_model_mapping_json = true;
            }
            if has_supported_models_json && has_model_mapping_json {
                break;
            }
        }
    }

    if !has_supported_models_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN supported_models_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;
    }

    if !has_model_mapping_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN model_mapping_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;
    }

    // Backfill invalid/empty values to keep JSON parsing stable even if DB gets partially corrupted.
    tx.execute_batch(
        r#"
UPDATE providers
SET supported_models_json = '{}'
WHERE supported_models_json IS NULL OR TRIM(supported_models_json) = '';

UPDATE providers
SET model_mapping_json = '{}'
WHERE model_mapping_json IS NULL OR TRIM(model_mapping_json) = '';
"#,
    )
    .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;

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
