//! Usage: SQLite migration v21->v22.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v21_to_v22(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 22;
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
    .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;

    let mut has_base_urls_json = false;
    let mut has_base_url_mode = false;
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
            if name == "base_urls_json" {
                has_base_urls_json = true;
            }
            if name == "base_url_mode" {
                has_base_url_mode = true;
            }
            if has_base_urls_json && has_base_url_mode {
                break;
            }
        }
    }

    if !has_base_urls_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN base_urls_json TEXT NOT NULL DEFAULT '[]';
"#,
        )
        .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;
    }

    if !has_base_url_mode {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN base_url_mode TEXT NOT NULL DEFAULT 'order';
"#,
        )
        .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;
    }

    // Backfill `base_urls_json` from legacy `base_url` if needed.
    {
        let mut stmt = tx
            .prepare("SELECT id, base_url, base_urls_json FROM providers")
            .map_err(|e| format!("failed to prepare providers backfill query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers for backfill: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers backfill row: {e}"))?
        {
            let id: i64 = row
                .get(0)
                .map_err(|e| format!("failed to read provider id: {e}"))?;
            let base_url: String = row
                .get(1)
                .map_err(|e| format!("failed to read provider base_url: {e}"))?;
            let base_urls_json: String = row
                .get(2)
                .map_err(|e| format!("failed to read provider base_urls_json: {e}"))?;

            let should_backfill = base_urls_json.trim().is_empty() || base_urls_json.trim() == "[]";
            let base_url = base_url.trim();
            if !should_backfill || base_url.is_empty() {
                continue;
            }

            let json = serde_json::to_string(&vec![base_url.to_string()])
                .unwrap_or_else(|_| "[]".to_string());
            tx.execute(
                "UPDATE providers SET base_urls_json = ?1 WHERE id = ?2",
                (json, id),
            )
            .map_err(|e| format!("failed to backfill providers.base_urls_json: {e}"))?;
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
