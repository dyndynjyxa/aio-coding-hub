//! Usage: SQLite migration v30->v31 - Add sliding window columns to provider_circuit_breakers.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v30_to_v31(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 31;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_table: bool = tx
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='provider_circuit_breakers'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if has_table {
        let alter_statements = [
            "ALTER TABLE provider_circuit_breakers ADD COLUMN failure_timestamps_json TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE provider_circuit_breakers ADD COLUMN half_open_success_count INTEGER NOT NULL DEFAULT 0",
        ];

        for stmt in &alter_statements {
            match tx.execute_batch(stmt) {
                Ok(()) => {}
                Err(e) if e.to_string().contains("duplicate column name") => {}
                Err(e) => {
                    return Err(format!(
                        "failed to alter provider_circuit_breakers table: {e}"
                    ));
                }
            }
        }

        // Backfill existing rows: convert failure_count into a single-element timestamps array
        // using updated_at as the timestamp. This preserves the count for circuits that were
        // already tracking failures.
        tx.execute_batch(
            r#"
UPDATE provider_circuit_breakers
SET failure_timestamps_json = (
  CASE
    WHEN failure_count > 0 THEN '[' || updated_at || ']'
    ELSE '[]'
  END
)
WHERE failure_timestamps_json = '[]' AND failure_count > 0
"#,
        )
        .map_err(|e| format!("failed to backfill failure_timestamps_json: {e}"))?;
    }

    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
    )
    .map_err(|e| format!("failed to create schema_migrations table: {e}"))?;
    let now = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, ?)",
        [VERSION, now],
    )
    .map_err(|e| format!("failed to insert schema_migrations row for v{VERSION}: {e}"))?;

    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;

    Ok(())
}
