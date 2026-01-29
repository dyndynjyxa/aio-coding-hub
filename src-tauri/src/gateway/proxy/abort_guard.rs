//! Usage: Best-effort drop guard to log client-aborted requests.

use crate::{db, request_logs};
use std::time::Instant;

use super::request_end::{
    emit_request_event_and_spawn_request_log, RequestEndArgs, RequestEndDeps,
};
use super::ErrorCategory;

pub(super) struct RequestAbortGuard {
    app: tauri::AppHandle,
    db: db::Db,
    log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    trace_id: String,
    cli_key: String,
    method: String,
    path: String,
    query: Option<String>,
    created_at_ms: i64,
    created_at: i64,
    started: Instant,
    armed: bool,
}

impl RequestAbortGuard {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        app: tauri::AppHandle,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        trace_id: String,
        cli_key: String,
        method: String,
        path: String,
        query: Option<String>,
        created_at_ms: i64,
        created_at: i64,
        started: Instant,
    ) -> Self {
        Self {
            app,
            db,
            log_tx,
            trace_id,
            cli_key,
            method,
            path,
            query,
            created_at_ms,
            created_at,
            started,
            armed: true,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RequestAbortGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let duration_ms = self.started.elapsed().as_millis();
        emit_request_event_and_spawn_request_log(RequestEndArgs {
            deps: RequestEndDeps::new(&self.app, &self.db, &self.log_tx),
            trace_id: self.trace_id.as_str(),
            cli_key: self.cli_key.as_str(),
            method: self.method.as_str(),
            path: self.path.as_str(),
            query: self.query.as_deref(),
            excluded_from_stats: false,
            status: None,
            error_category: Some(ErrorCategory::ClientAbort.as_str()),
            error_code: Some("GW_REQUEST_ABORTED"),
            duration_ms,
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            attempts: &[],
            special_settings_json: None,
            session_id: None,
            requested_model: None,
            created_at_ms: self.created_at_ms,
            created_at: self.created_at,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        });
    }
}
