//! Usage: App data and DB disk-management helpers (reset, usage stats, cleanup).

use crate::app_paths;
use crate::db;
use crate::shared::error::db_err;
use rusqlite::TransactionBehavior;
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DbDiskUsage {
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClearRequestLogsResult {
    pub request_logs_deleted: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DbCompactResult {
    pub before_bytes: u64,
    pub after_bytes: u64,
}

fn file_len_or_zero(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(format!("failed to stat {}: {err}", path.to_string_lossy())),
    }
}

fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(format!(
            "failed to remove {}: {err}",
            path.to_string_lossy()
        )),
    }
}

fn db_related_paths(db_path: &Path) -> (PathBuf, PathBuf) {
    let wal_path = {
        let mut out = db_path.to_path_buf().into_os_string();
        out.push("-wal");
        PathBuf::from(out)
    };
    let shm_path = {
        let mut out = db_path.to_path_buf().into_os_string();
        out.push("-shm");
        PathBuf::from(out)
    };
    (wal_path, shm_path)
}

fn disk_usage_at(db_path: &Path) -> Result<DbDiskUsage, String> {
    let (wal_path, shm_path) = db_related_paths(db_path);

    let db_bytes = file_len_or_zero(db_path)?;
    let wal_bytes = file_len_or_zero(&wal_path)?;
    let shm_bytes = file_len_or_zero(&shm_path)?;

    Ok(DbDiskUsage {
        db_bytes,
        wal_bytes,
        shm_bytes,
        total_bytes: db_bytes.saturating_add(wal_bytes).saturating_add(shm_bytes),
    })
}

pub fn db_disk_usage_get(app: &tauri::AppHandle) -> crate::shared::error::AppResult<DbDiskUsage> {
    let db_path = db::db_path(app)?;
    Ok(disk_usage_at(&db_path)?)
}

pub fn db_compact(
    app: &tauri::AppHandle,
    db: &db::Db,
) -> crate::shared::error::AppResult<DbCompactResult> {
    let db_path = db::db_path(app)?;
    db_compact_at(&db_path, db)
}

fn db_compact_at(db_path: &Path, db: &db::Db) -> crate::shared::error::AppResult<DbCompactResult> {
    tracing::info!("compacting database (user-initiated)");

    let before_bytes = disk_usage_at(db_path)?.total_bytes;

    let conn = db.open_connection()?;

    // Checkpoints stay best-effort (same sequence as request_logs_clear_all),
    // but VACUUM failures must surface: this is a user-initiated action.
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    conn.execute_batch("VACUUM;")
        .map_err(|e| db_err!("failed to vacuum database: {e}"))?;
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    let after_bytes = disk_usage_at(db_path)?.total_bytes;

    tracing::info!(before_bytes, after_bytes, "database compacted");

    Ok(DbCompactResult {
        before_bytes,
        after_bytes,
    })
}

pub fn request_logs_clear_all(
    db: &db::Db,
) -> crate::shared::error::AppResult<ClearRequestLogsResult> {
    tracing::warn!("clearing all request logs (user-initiated)");

    let mut conn = db.open_connection()?;

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let request_logs_deleted = tx
        .execute("DELETE FROM request_logs", [])
        .map_err(|e| db_err!("failed to clear request_logs: {e}"))?;

    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;

    tracing::warn!(
        request_logs_deleted = request_logs_deleted,
        "request logs cleared"
    );

    // Best-effort: reclaim disk usage (WAL truncate + vacuum).
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    let _ = conn.execute_batch("VACUUM;");
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    Ok(ClearRequestLogsResult {
        request_logs_deleted: request_logs_deleted as u64,
    })
}

pub fn app_data_reset<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<bool> {
    tracing::error!(
        "app data reset initiated (destructive operation: deleting settings and database)"
    );

    // Ensure the app data dir exists.
    let dir = app_paths::app_data_dir(app)?;

    // settings.json (+ temp artifacts)
    let settings_path = dir.join("settings.json");
    let settings_tmp_path = dir.join("settings.json.tmp");
    let settings_bak_path = dir.join("settings.json.bak");
    let _ = remove_file_if_exists(&settings_tmp_path)?;
    let _ = remove_file_if_exists(&settings_bak_path)?;
    let _ = remove_file_if_exists(&settings_path)?;

    // sqlite db (+ wal/shm)
    let db_path = db::db_path(app)?;
    let (wal_path, shm_path) = db_related_paths(&db_path);
    let _ = remove_file_if_exists(&wal_path)?;
    let _ = remove_file_if_exists(&shm_path)?;
    let _ = remove_file_if_exists(&db_path)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::db_compact_at;
    use rusqlite::params;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn init_test_db() -> (crate::db::Db, PathBuf, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("data-management.sqlite");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        (db, db_path, dir)
    }

    fn insert_request_log_rows(db: &crate::db::Db, count: usize) {
        let conn = db.open_connection().expect("open connection");
        // Bulky payload so deletions leave measurable free pages behind.
        let attempts_json = format!("[\"{}\"]", "x".repeat(4096));
        for idx in 0..count {
            conn.execute(
                r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, duration_ms, attempts_json,
  created_at, created_at_ms, excluded_from_stats
) VALUES (?1, 'claude', 'POST', '/v1/messages', 200, 10, ?2, 1770000000, 1770000000000, 0)
"#,
                params![format!("trace-compact-{idx}"), attempts_json],
            )
            .expect("insert request log row");
        }
    }

    fn count_request_logs(db: &crate::db::Db) -> i64 {
        let conn = db.open_connection().expect("open connection");
        conn.query_row("SELECT COUNT(1) FROM request_logs", [], |row| row.get(0))
            .expect("count request logs")
    }

    #[test]
    fn db_compact_keeps_rows_and_reclaims_space() {
        let (db, db_path, _dir) = init_test_db();

        insert_request_log_rows(&db, 300);
        {
            let conn = db.open_connection().expect("open connection");
            conn.execute("DELETE FROM request_logs WHERE rowid % 4 != 0", [])
                .expect("delete rows");
        }
        let rows_before = count_request_logs(&db);
        assert!(rows_before > 0, "expected surviving rows before compact");

        let result = db_compact_at(&db_path, &db).expect("compact db");

        assert_eq!(
            count_request_logs(&db),
            rows_before,
            "compact must not delete data"
        );
        assert!(
            result.after_bytes <= result.before_bytes,
            "after_bytes {} must not exceed before_bytes {}",
            result.after_bytes,
            result.before_bytes
        );
    }

    #[test]
    fn db_compact_surfaces_vacuum_failure_and_keeps_rows() {
        let (db, db_path, _dir) = init_test_db();
        insert_request_log_rows(&db, 4);

        // Hold the write lock on a separate connection so VACUUM cannot acquire it.
        let blocker = rusqlite::Connection::open(&db_path).expect("open blocker connection");
        blocker
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("begin immediate");

        let err = db_compact_at(&db_path, &db).expect_err("vacuum must fail while db is locked");
        assert!(
            err.to_string().contains("failed to vacuum database"),
            "unexpected error: {err}"
        );

        blocker.execute_batch("ROLLBACK;").expect("rollback");
        assert_eq!(
            count_request_logs(&db),
            4,
            "rows must survive failed compact"
        );
    }
}
