//! Usage: SQLite migration v23->v24.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;
use serde::Deserialize;

pub(super) fn migrate_v23_to_v24(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 24;
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
    .map_err(|e| format!("failed to migrate v23->v24: {e}"))?;

    tx.execute_batch(
        r#"
ALTER TABLE request_logs ADD COLUMN final_provider_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_request_logs_final_provider_id_created_at
  ON request_logs(final_provider_id, created_at);
"#,
    )
    .map_err(|e| format!("failed to migrate v23->v24: {e}"))?;

    // Backfill final_provider_id for existing request logs.
    // We use the same semantics as runtime routing:
    // - Prefer the last success attempt, otherwise fallback to the last attempt.
    #[derive(Debug, Deserialize)]
    struct AttemptRow {
        provider_id: i64,
        outcome: String,
    }

    fn final_provider_id_from_attempts_json(attempts_json: &str) -> Option<i64> {
        let attempts: Vec<AttemptRow> = serde_json::from_str(attempts_json).unwrap_or_default();
        let picked = attempts
            .iter()
            .rev()
            .find(|a| a.outcome == "success")
            .or_else(|| attempts.last());
        picked.map(|a| a.provider_id).filter(|v| *v > 0)
    }

    {
        let mut select_stmt = tx
            .prepare("SELECT id, attempts_json FROM request_logs")
            .map_err(|e| format!("failed to prepare request_logs backfill query: {e}"))?;
        let mut update_stmt = tx
            .prepare("UPDATE request_logs SET final_provider_id = ?1 WHERE id = ?2")
            .map_err(|e| format!("failed to prepare request_logs backfill update: {e}"))?;

        let mut rows = select_stmt
            .query([])
            .map_err(|e| format!("failed to run request_logs backfill query: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs backfill row: {e}"))?
        {
            let id: i64 = row
                .get("id")
                .map_err(|e| format!("failed to read request_logs.id for backfill: {e}"))?;
            let attempts_json: String = row.get("attempts_json").unwrap_or_default();
            let provider_id = final_provider_id_from_attempts_json(&attempts_json);
            if provider_id.is_none() {
                continue;
            }
            update_stmt
                .execute(rusqlite::params![provider_id, id])
                .map_err(|e| format!("failed to backfill request_logs.final_provider_id: {e}"))?;
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
