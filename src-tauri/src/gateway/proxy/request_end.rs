//! Usage: Shared helpers to emit request-end events and enqueue request logs consistently.

use super::logging::enqueue_request_log_with_backpressure;
use super::{spawn_enqueue_request_log_with_backpressure, RequestLogEnqueueArgs};
use crate::gateway::events::{emit_request_event, FailoverAttempt};
use crate::{db, request_logs};

pub(super) struct RequestEndDeps<'a> {
    pub(super) app: &'a tauri::AppHandle,
    pub(super) db: &'a db::Db,
    pub(super) log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
}

impl<'a> RequestEndDeps<'a> {
    pub(super) fn new(
        app: &'a tauri::AppHandle,
        db: &'a db::Db,
        log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    ) -> Self {
        Self { app, db, log_tx }
    }
}

pub(super) struct RequestEndArgs<'a> {
    pub(super) deps: RequestEndDeps<'a>,
    pub(super) trace_id: &'a str,
    pub(super) cli_key: &'a str,
    pub(super) method: &'a str,
    pub(super) path: &'a str,
    pub(super) query: Option<&'a str>,
    pub(super) excluded_from_stats: bool,
    pub(super) status: Option<u16>,
    pub(super) error_category: Option<&'static str>,
    pub(super) error_code: Option<&'static str>,
    pub(super) duration_ms: u128,
    pub(super) event_ttfb_ms: Option<u128>,
    pub(super) log_ttfb_ms: Option<u128>,
    pub(super) attempts: &'a [FailoverAttempt],
    pub(super) special_settings_json: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) log_usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
}

struct PreparedRequestEnd<'a> {
    deps: RequestEndDeps<'a>,
    error_category: Option<&'static str>,
    event_ttfb_ms: Option<u128>,
    attempts: Vec<FailoverAttempt>,
    usage_metrics: Option<crate::usage::UsageMetrics>,
    log_args: RequestLogEnqueueArgs,
}

fn prepare_request_end(args: RequestEndArgs<'_>) -> PreparedRequestEnd<'_> {
    let query = args.query.map(str::to_string);
    let (attempts, attempts_json) = if args.attempts.is_empty() {
        (Vec::new(), "[]".to_string())
    } else {
        let attempts = args.attempts.to_vec();
        let attempts_json = serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());
        (attempts, attempts_json)
    };

    let log_args = RequestLogEnqueueArgs {
        trace_id: args.trace_id.to_string(),
        cli_key: args.cli_key.to_string(),
        session_id: args.session_id,
        method: args.method.to_string(),
        path: args.path.to_string(),
        query,
        excluded_from_stats: args.excluded_from_stats,
        special_settings_json: args.special_settings_json,
        status: args.status,
        error_code: args.error_code,
        duration_ms: args.duration_ms,
        ttfb_ms: args.log_ttfb_ms,
        attempts_json,
        requested_model: args.requested_model,
        created_at_ms: args.created_at_ms,
        created_at: args.created_at,
        usage_metrics: args.log_usage_metrics,
        usage: args.usage,
    };

    PreparedRequestEnd {
        deps: args.deps,
        error_category: args.error_category,
        event_ttfb_ms: args.event_ttfb_ms,
        attempts,
        usage_metrics: args.usage_metrics,
        log_args,
    }
}

pub(super) async fn emit_request_event_and_enqueue_request_log(args: RequestEndArgs<'_>) {
    let PreparedRequestEnd {
        deps,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
        log_args,
    } = prepare_request_end(args);

    emit_request_event(
        deps.app,
        log_args.trace_id.clone(),
        log_args.cli_key.clone(),
        log_args.method.clone(),
        log_args.path.clone(),
        log_args.query.clone(),
        log_args.status,
        error_category,
        log_args.error_code,
        log_args.duration_ms,
        event_ttfb_ms,
        attempts,
        usage_metrics,
    );

    enqueue_request_log_with_backpressure(deps.app, deps.db, deps.log_tx, log_args).await;
}

pub(super) fn emit_request_event_and_spawn_request_log(args: RequestEndArgs<'_>) {
    let PreparedRequestEnd {
        deps,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
        log_args,
    } = prepare_request_end(args);

    emit_request_event(
        deps.app,
        log_args.trace_id.clone(),
        log_args.cli_key.clone(),
        log_args.method.clone(),
        log_args.path.clone(),
        log_args.query.clone(),
        log_args.status,
        error_category,
        log_args.error_code,
        log_args.duration_ms,
        event_ttfb_ms,
        attempts,
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        deps.app.clone(),
        deps.db.clone(),
        deps.log_tx.clone(),
        log_args,
    );
}
