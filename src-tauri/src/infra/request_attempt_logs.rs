//! Usage: Attempt log persistence (sqlite buffered writer, queries, and cleanup).

use crate::{db, settings};
use rusqlite::{params, ErrorCode};
use serde::Serialize;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const WRITE_BUFFER_CAPACITY: usize = 1024;
const WRITE_BATCH_MAX: usize = 100;
const CLEANUP_MIN_INTERVAL: Duration = Duration::from_secs(10 * 60);
const INSERT_RETRY_MAX_ATTEMPTS: u32 = 8;
const INSERT_RETRY_BASE_DELAY_MS: u64 = 20;
const INSERT_RETRY_MAX_DELAY_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbWriteErrorKind {
    Busy,
    Other,
}

#[derive(Debug)]
struct DbWriteError {
    kind: DbWriteErrorKind,
    message: String,
}

impl DbWriteError {
    fn other(message: String) -> Self {
        Self {
            kind: DbWriteErrorKind::Other,
            message,
        }
    }

    fn from_rusqlite(context: &'static str, err: rusqlite::Error) -> Self {
        let kind = classify_rusqlite_error(&err);
        Self {
            kind,
            message: format!("DB_ERROR: {context}: {err}"),
        }
    }

    fn is_retryable(&self) -> bool {
        self.kind == DbWriteErrorKind::Busy
    }
}

fn classify_rusqlite_error(err: &rusqlite::Error) -> DbWriteErrorKind {
    match err {
        rusqlite::Error::SqliteFailure(e, _) => match e.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => DbWriteErrorKind::Busy,
            _ => DbWriteErrorKind::Other,
        },
        _ => DbWriteErrorKind::Other,
    }
}

fn retry_delay(attempt_index: u32) -> Duration {
    let exp = attempt_index.min(20);
    let raw = INSERT_RETRY_BASE_DELAY_MS.saturating_mul(1u64.checked_shl(exp).unwrap_or(u64::MAX));
    Duration::from_millis(raw.min(INSERT_RETRY_MAX_DELAY_MS))
}

#[derive(Debug, Clone)]
pub struct RequestAttemptLogInsert {
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub attempt_index: i64,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub outcome: String,
    pub status: Option<i64>,
    pub attempt_started_ms: i64,
    pub attempt_duration_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestAttemptLog {
    pub id: i64,
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub attempt_index: i64,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub outcome: String,
    pub status: Option<i64>,
    pub attempt_started_ms: i64,
    pub attempt_duration_ms: i64,
    pub created_at: i64,
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

pub fn start_buffered_writer(
    app: tauri::AppHandle,
) -> (
    mpsc::Sender<RequestAttemptLogInsert>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel::<RequestAttemptLogInsert>(WRITE_BUFFER_CAPACITY);
    let task = tauri::async_runtime::spawn_blocking(move || {
        writer_loop(app, rx);
    });
    (tx, task)
}

pub fn spawn_write_through(app: tauri::AppHandle, item: RequestAttemptLogInsert) {
    tauri::async_runtime::spawn_blocking(move || {
        let items = [item];
        if let Err(err) = insert_batch_with_retries(&app, &items) {
            eprintln!(
                "request_attempt_logs write-through insert error: {}",
                err.message
            );
        }
    });
}

fn writer_loop(app: tauri::AppHandle, mut rx: mpsc::Receiver<RequestAttemptLogInsert>) {
    let mut buffer: Vec<RequestAttemptLogInsert> = Vec::with_capacity(WRITE_BATCH_MAX);
    let now = Instant::now();
    let mut last_cleanup = now.checked_sub(CLEANUP_MIN_INTERVAL).unwrap_or(now);
    let mut cleanup_due = last_cleanup == now;

    while let Some(item) = rx.blocking_recv() {
        buffer.push(item);

        while buffer.len() < WRITE_BATCH_MAX {
            match rx.try_recv() {
                Ok(next) => buffer.push(next),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        if let Err(err) = insert_batch_with_retries(&app, &buffer) {
            eprintln!("request_attempt_logs insert_batch error: {}", err.message);
        }
        buffer.clear();

        if cleanup_due || last_cleanup.elapsed() >= CLEANUP_MIN_INTERVAL {
            let retention_days = settings::log_retention_days_fail_open(&app);
            if let Err(err) = cleanup_expired(&app, retention_days) {
                eprintln!("request_attempt_logs cleanup error: {err}");
            }
            cleanup_due = false;
            last_cleanup = Instant::now();
        }
    }

    if !buffer.is_empty() {
        if let Err(err) = insert_batch_with_retries(&app, &buffer) {
            eprintln!(
                "request_attempt_logs final insert_batch error: {}",
                err.message
            );
        }
    }
}

fn insert_batch_with_retries(
    app: &tauri::AppHandle,
    items: &[RequestAttemptLogInsert],
) -> Result<(), DbWriteError> {
    let mut attempt: u32 = 0;
    loop {
        match insert_batch_once(app, items) {
            Ok(()) => return Ok(()),
            Err(err) => {
                attempt = attempt.saturating_add(1);
                if !err.is_retryable() || attempt >= INSERT_RETRY_MAX_ATTEMPTS {
                    return Err(err);
                }
                std::thread::sleep(retry_delay(attempt.saturating_sub(1)));
            }
        }
    }
}

fn insert_batch_once(
    app: &tauri::AppHandle,
    items: &[RequestAttemptLogInsert],
) -> Result<(), DbWriteError> {
    if items.is_empty() {
        return Ok(());
    }

    let mut conn = db::open_connection(app).map_err(DbWriteError::other)?;
    let tx = conn
        .transaction()
        .map_err(|e| DbWriteError::from_rusqlite("failed to start transaction", e))?;

    {
        let mut stmt = tx
            .prepare(
                r#"
INSERT INTO request_attempt_logs (
  trace_id,
  cli_key,
  method,
  path,
  query,
  attempt_index,
  provider_id,
  provider_name,
  base_url,
  outcome,
  status,
  attempt_started_ms,
  attempt_duration_ms,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
ON CONFLICT(trace_id, attempt_index) DO UPDATE SET
  method = excluded.method,
  path = excluded.path,
  query = excluded.query,
  provider_id = excluded.provider_id,
  provider_name = excluded.provider_name,
  base_url = excluded.base_url,
  outcome = excluded.outcome,
  status = excluded.status,
  attempt_started_ms = excluded.attempt_started_ms,
  attempt_duration_ms = excluded.attempt_duration_ms
"#,
            )
            .map_err(|e| DbWriteError::from_rusqlite("failed to prepare attempt insert", e))?;

        for item in items {
            validate_cli_key(&item.cli_key).map_err(DbWriteError::other)?;
            stmt.execute(params![
                item.trace_id,
                item.cli_key,
                item.method,
                item.path,
                item.query,
                item.attempt_index,
                item.provider_id,
                item.provider_name,
                item.base_url,
                item.outcome,
                item.status,
                item.attempt_started_ms,
                item.attempt_duration_ms,
                item.created_at
            ])
            .map_err(|e| DbWriteError::from_rusqlite("failed to insert request_attempt_log", e))?;
        }
    }

    tx.commit()
        .map_err(|e| DbWriteError::from_rusqlite("failed to commit transaction", e))?;

    Ok(())
}

pub fn cleanup_expired(app: &tauri::AppHandle, retention_days: u32) -> Result<u64, String> {
    if retention_days == 0 {
        return Err("SEC_INVALID_INPUT: log_retention_days must be >= 1".to_string());
    }

    let now = now_unix_seconds();
    let cutoff = now.saturating_sub((retention_days as i64).saturating_mul(86400));

    let conn = db::open_connection(app)?;
    let changed = conn
        .execute(
            "DELETE FROM request_attempt_logs WHERE created_at < ?1",
            params![cutoff],
        )
        .map_err(|e| format!("DB_ERROR: failed to cleanup request_attempt_logs: {e}"))?;

    Ok(changed as u64)
}

fn row_to_log(row: &rusqlite::Row<'_>) -> Result<RequestAttemptLog, rusqlite::Error> {
    Ok(RequestAttemptLog {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        method: row.get("method")?,
        path: row.get("path")?,
        query: row.get("query")?,
        attempt_index: row.get("attempt_index")?,
        provider_id: row.get("provider_id")?,
        provider_name: row.get("provider_name")?,
        base_url: row.get("base_url")?,
        outcome: row.get("outcome")?,
        status: row.get("status")?,
        attempt_started_ms: row.get("attempt_started_ms")?,
        attempt_duration_ms: row.get("attempt_duration_ms")?,
        created_at: row.get("created_at")?,
    })
}

pub fn list_by_trace_id(
    app: &tauri::AppHandle,
    trace_id: &str,
    limit: usize,
) -> Result<Vec<RequestAttemptLog>, String> {
    let trace_id = trace_id.trim();
    if trace_id.is_empty() {
        return Err("SEC_INVALID_INPUT: trace_id is required".to_string());
    }

    let limit = limit.clamp(1, 200);
    let conn = db::open_connection(app)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  trace_id,
  cli_key,
  method,
  path,
  query,
  attempt_index,
  provider_id,
  provider_name,
  base_url,
  outcome,
  status,
  attempt_started_ms,
  attempt_duration_ms,
  created_at
FROM request_attempt_logs
WHERE trace_id = ?1
ORDER BY attempt_index ASC, id ASC
LIMIT ?2
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare attempt query: {e}"))?;

    let rows = stmt
        .query_map(params![trace_id, limit as i64], row_to_log)
        .map_err(|e| format!("DB_ERROR: failed to query request_attempt_logs: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read attempt row: {e}"))?);
    }
    Ok(out)
}
