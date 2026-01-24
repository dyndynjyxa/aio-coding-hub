//! Usage: SQLite migration v26->v27.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v26_to_v27(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 27;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

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

    if !has_provider_mode {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN provider_mode TEXT NOT NULL DEFAULT 'relay';
"#,
        )
        .map_err(|e| format!("failed to migrate v26->v27: {e}"))?;
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
