//! Usage: App data and DB disk-management helpers (reset, usage stats, cleanup).

use crate::app_paths;
use crate::db;
use rusqlite::TransactionBehavior;
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct DbDiskUsage {
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClearRequestLogsResult {
    pub request_logs_deleted: u64,
    pub request_attempt_logs_deleted: u64,
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

pub fn db_disk_usage_get(app: &tauri::AppHandle) -> Result<DbDiskUsage, String> {
    let db_path = db::db_path(app)?;
    let (wal_path, shm_path) = db_related_paths(&db_path);

    let db_bytes = file_len_or_zero(&db_path)?;
    let wal_bytes = file_len_or_zero(&wal_path)?;
    let shm_bytes = file_len_or_zero(&shm_path)?;

    Ok(DbDiskUsage {
        db_bytes,
        wal_bytes,
        shm_bytes,
        total_bytes: db_bytes.saturating_add(wal_bytes).saturating_add(shm_bytes),
    })
}

pub fn request_logs_clear_all(app: &tauri::AppHandle) -> Result<ClearRequestLogsResult, String> {
    let mut conn = db::open_connection(app)?;

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let request_attempt_logs_deleted = tx
        .execute("DELETE FROM request_attempt_logs", [])
        .map_err(|e| format!("DB_ERROR: failed to clear request_attempt_logs: {e}"))?;

    let request_logs_deleted = tx
        .execute("DELETE FROM request_logs", [])
        .map_err(|e| format!("DB_ERROR: failed to clear request_logs: {e}"))?;

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit transaction: {e}"))?;

    // Best-effort: reclaim disk usage (WAL truncate + vacuum).
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    let _ = conn.execute_batch("VACUUM;");
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    Ok(ClearRequestLogsResult {
        request_logs_deleted: request_logs_deleted as u64,
        request_attempt_logs_deleted: request_attempt_logs_deleted as u64,
    })
}

pub fn app_data_reset(app: &tauri::AppHandle) -> Result<bool, String> {
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
