//! Usage: SQLite migration v31->v32 - Add installed_commit column to skills table.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v31_to_v32(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 32;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_table: bool = tx
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skills'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if has_table {
        let alter_stmt = "ALTER TABLE skills ADD COLUMN installed_commit TEXT DEFAULT NULL";

        match tx.execute_batch(alter_stmt) {
            Ok(()) => {}
            Err(e) if e.to_string().contains("duplicate column name") => {}
            Err(e) => {
                return Err(format!("failed to alter skills table: {e}"));
            }
        }
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
