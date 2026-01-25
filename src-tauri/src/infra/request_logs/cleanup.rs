//! Usage: Request log retention cleanup.

use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::params;

pub fn cleanup_expired(db: &db::Db, retention_days: u32) -> Result<u64, String> {
    if retention_days == 0 {
        return Err("SEC_INVALID_INPUT: log_retention_days must be >= 1".to_string());
    }

    let now = now_unix_seconds();
    let cutoff = now.saturating_sub((retention_days as i64).saturating_mul(86400));

    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            "DELETE FROM request_logs WHERE created_at < ?1",
            params![cutoff],
        )
        .map_err(|e| format!("DB_ERROR: failed to cleanup request_logs: {e}"))?;

    Ok(changed as u64)
}
