//! Usage: Best-effort drop guard to log client-aborted requests.

use crate::{db, request_logs};
use std::time::Instant;

use super::{spawn_enqueue_request_log_with_backpressure, ErrorCategory, RequestLogEnqueueArgs};
use crate::gateway::events::emit_request_event;

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
        emit_request_event(
            &self.app,
            self.trace_id.clone(),
            self.cli_key.clone(),
            self.method.clone(),
            self.path.clone(),
            self.query.clone(),
            None,
            Some(ErrorCategory::ClientAbort.as_str()),
            Some("GW_REQUEST_ABORTED"),
            duration_ms,
            None,
            vec![],
            None,
        );

        spawn_enqueue_request_log_with_backpressure(
            self.app.clone(),
            self.db.clone(),
            self.log_tx.clone(),
            RequestLogEnqueueArgs {
                trace_id: self.trace_id.clone(),
                cli_key: self.cli_key.clone(),
                session_id: None,
                method: self.method.clone(),
                path: self.path.clone(),
                query: self.query.clone(),
                excluded_from_stats: false,
                special_settings_json: None,
                status: None,
                error_code: Some("GW_REQUEST_ABORTED"),
                duration_ms,
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                requested_model: None,
                created_at_ms: self.created_at_ms,
                created_at: self.created_at,
                usage: None,
            },
        );
    }
}
