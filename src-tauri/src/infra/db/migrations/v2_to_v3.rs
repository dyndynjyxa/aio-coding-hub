//! Usage: SQLite migration v2->v3.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v2_to_v3(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 3;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE providers ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);
"#,
    )
    .map_err(|e| format!("failed to migrate v2->v3: {e}"))?;

    let mut cli_keys = Vec::new();
    {
        let mut stmt = tx
            .prepare("SELECT DISTINCT cli_key FROM providers")
            .map_err(|e| format!("failed to prepare cli_key query: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("failed to query cli_key list: {e}"))?;
        for row in rows {
            cli_keys.push(row.map_err(|e| format!("failed to read cli_key row: {e}"))?);
        }
    }

    for cli_key in cli_keys {
        let mut stmt = tx
            .prepare("SELECT id FROM providers WHERE cli_key = ?1 ORDER BY id DESC")
            .map_err(|e| format!("failed to prepare providers list for {cli_key}: {e}"))?;
        let rows = stmt
            .query_map([&cli_key], |row| row.get::<_, i64>(0))
            .map_err(|e| format!("failed to query providers list for {cli_key}: {e}"))?;

        for (sort_order, row) in (0_i64..).zip(rows) {
            let provider_id = row.map_err(|e| format!("failed to read provider id row: {e}"))?;
            tx.execute(
                "UPDATE providers SET sort_order = ?1 WHERE id = ?2",
                (sort_order, provider_id),
            )
            .map_err(|e| {
                format!("failed to backfill sort_order for provider {provider_id}: {e}")
            })?;
        }
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
