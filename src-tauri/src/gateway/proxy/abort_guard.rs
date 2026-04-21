//! Usage: Best-effort drop guard to log client-aborted requests.

use crate::gateway::events::FailoverAttempt;
use crate::{db, request_logs};
use std::time::Instant;

use super::request_end::{
    emit_request_event_and_spawn_request_log, RequestEndArgs, RequestEndDeps,
};
use super::{ErrorCategory, GatewayErrorCode};

pub(super) struct RequestAbortGuard {
    app: tauri::AppHandle,
    db: db::Db,
    log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    trace_id: String,
    cli_key: String,
    method: String,
    path: String,
    observe: bool,
    query: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
    in_flight_attempt: Option<FailoverAttempt>,
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
        observe: bool,
        query: Option<String>,
        session_id: Option<String>,
        requested_model: Option<String>,
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
            observe,
            query,
            session_id,
            requested_model,
            in_flight_attempt: None,
            created_at_ms,
            created_at,
            started,
            armed: true,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }

    /// Take ownership of this guard, leaving a disarmed placeholder behind.
    /// This is useful when you need to pass the guard to a sub-function while
    /// keeping the parent struct borrowable.
    pub(super) fn take(&mut self) -> Self {
        let taken = Self {
            app: self.app.clone(),
            db: self.db.clone(),
            log_tx: self.log_tx.clone(),
            trace_id: std::mem::take(&mut self.trace_id),
            cli_key: std::mem::take(&mut self.cli_key),
            method: std::mem::take(&mut self.method),
            path: std::mem::take(&mut self.path),
            observe: self.observe,
            query: self.query.take(),
            session_id: self.session_id.take(),
            requested_model: self.requested_model.take(),
            in_flight_attempt: self.in_flight_attempt.take(),
            created_at_ms: self.created_at_ms,
            created_at: self.created_at,
            started: self.started,
            armed: self.armed,
        };
        self.armed = false; // disarm the leftover shell
        taken
    }

    pub(super) fn capture_in_flight_attempt(&mut self, attempt: &FailoverAttempt) {
        self.in_flight_attempt = Some(attempt.clone());
    }
}

impl Drop for RequestAbortGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if !self.observe {
            return;
        }

        let duration_ms = self.started.elapsed().as_millis();
        let abort_attempts: Vec<FailoverAttempt> = self.in_flight_attempt.iter().cloned().collect();
        emit_request_event_and_spawn_request_log(RequestEndArgs {
            deps: RequestEndDeps::new(&self.app, &self.db, &self.log_tx),
            trace_id: self.trace_id.as_str(),
            cli_key: self.cli_key.as_str(),
            method: self.method.as_str(),
            path: self.path.as_str(),
            observe: self.observe,
            query: self.query.as_deref(),
            excluded_from_stats: false,
            status: None,
            error_category: Some(ErrorCategory::ClientAbort.as_str()),
            error_code: Some(GatewayErrorCode::RequestAborted.as_str()),
            duration_ms,
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            attempts: abort_attempts.as_slice(),
            special_settings_json: None,
            session_id: self.session_id.clone(),
            requested_model: self.requested_model.clone(),
            created_at_ms: self.created_at_ms,
            created_at: self.created_at,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloned_abort_attempt_keeps_provider_context() {
        let attempt = FailoverAttempt {
            provider_id: 12,
            provider_name: "Claude Bridge".to_string(),
            base_url: "https://example.com".to_string(),
            outcome: "started".to_string(),
            status: None,
            provider_index: Some(1),
            retry_index: Some(1),
            session_reuse: Some(true),
            error_category: None,
            error_code: None,
            decision: None,
            reason: None,
            selection_method: Some("session_reuse"),
            reason_code: None,
            attempt_started_ms: Some(123),
            attempt_duration_ms: Some(0),
            circuit_state_before: Some("CLOSED"),
            circuit_state_after: None,
            circuit_failure_count: Some(0),
            circuit_failure_threshold: Some(5),
        };

        let logged_attempts: Vec<FailoverAttempt> = Some(attempt.clone()).iter().cloned().collect();
        assert_eq!(logged_attempts.len(), 1);
        assert_eq!(logged_attempts[0].provider_id, 12);
        assert_eq!(logged_attempts[0].provider_name, "Claude Bridge");
        assert_eq!(logged_attempts[0].outcome, "started");
    }
}
