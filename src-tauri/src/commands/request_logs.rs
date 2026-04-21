//! Usage: Request logs and trace detail related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::commands::limit::normalize_limit;
use crate::{blocking, request_attempt_logs, request_logs};

const REQUEST_LOGS_DEFAULT_LIMIT: u32 = 50;
const REQUEST_LOGS_MAX_LIMIT: u32 = 500;
const REQUEST_ATTEMPT_LOGS_MAX_LIMIT: u32 = 200;

fn request_logs_limit(limit: Option<u32>) -> usize {
    normalize_limit(limit, REQUEST_LOGS_DEFAULT_LIMIT, 1, REQUEST_LOGS_MAX_LIMIT)
}

fn request_attempt_logs_limit(limit: Option<u32>) -> usize {
    normalize_limit(
        limit,
        REQUEST_LOGS_DEFAULT_LIMIT,
        1,
        REQUEST_ATTEMPT_LOGS_MAX_LIMIT,
    )
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list", move || {
        request_logs::list_recent(&db, &cli_key, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_all", move || {
        request_logs::list_recent_all(&db, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_after_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    after_id: i64,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_after_id", move || {
        request_logs::list_after_id(&db, &cli_key, after_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_after_id_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    after_id: i64,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_after_id_all", move || {
        request_logs::list_after_id_all(&db, after_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_log_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    log_id: i64,
) -> Result<request_logs::RequestLogDetail, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("request_log_get", move || {
        request_logs::get_by_id(&db, log_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_log_get_by_trace_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    trace_id: String,
) -> Result<Option<request_logs::RequestLogDetail>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("request_log_get_by_trace_id", move || {
        request_logs::get_by_trace_id(&db, &trace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_attempt_logs_by_trace_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    trace_id: String,
    limit: Option<u32>,
) -> Result<Vec<request_attempt_logs::RequestAttemptLog>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_attempt_logs_limit(limit);
    blocking::run("request_attempt_logs_by_trace_id", move || {
        request_attempt_logs::list_by_trace_id(&db, &trace_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{request_attempt_logs_limit, request_logs_limit};

    #[test]
    fn request_logs_limit_uses_default_and_clamps() {
        assert_eq!(request_logs_limit(None), 50);
        assert_eq!(request_logs_limit(Some(0)), 1);
        assert_eq!(request_logs_limit(Some(999)), 500);
        assert_eq!(request_logs_limit(Some(200)), 200);
    }

    #[test]
    fn request_attempt_logs_limit_uses_default_and_clamps() {
        assert_eq!(request_attempt_logs_limit(None), 50);
        assert_eq!(request_attempt_logs_limit(Some(0)), 1);
        assert_eq!(request_attempt_logs_limit(Some(999)), 200);
        assert_eq!(request_attempt_logs_limit(Some(88)), 88);
    }
}
