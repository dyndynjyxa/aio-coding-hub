//! Usage: SQLite migration v18->v19.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v18_to_v19(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 19;
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
    .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;

    let mut has_excluded_from_stats = false;
    let mut has_special_settings_json = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(request_logs)")
            .map_err(|e| format!("failed to prepare request_logs table_info: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query request_logs table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read request_logs column name: {e}"))?;
            match name.as_str() {
                "excluded_from_stats" => has_excluded_from_stats = true,
                "special_settings_json" => has_special_settings_json = true,
                _ => {}
            }
            if has_excluded_from_stats && has_special_settings_json {
                break;
            }
        }
    }

    if !has_excluded_from_stats {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN excluded_from_stats INTEGER NOT NULL DEFAULT 0;
"#,
        )
        .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;
    }

    if !has_special_settings_json {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN special_settings_json TEXT;
"#,
        )
        .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;
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
