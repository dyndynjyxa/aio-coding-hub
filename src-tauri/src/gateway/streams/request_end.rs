//! Usage: Shared helpers to finalize stream requests (event + request log).

use super::finalize::finalize_circuit_and_session;
use super::StreamFinalizeCtx;
use crate::gateway::active_requests::ActiveRequestFinishReason;
use crate::gateway::proxy::{
    spawn_enqueue_request_log_with_backpressure, GatewayErrorCode, RequestLogEnqueueArgs,
};
use crate::gateway::response_fixer;

pub(super) struct StreamRequestCompletion {
    pub(super) error_code: Option<&'static str>,
    pub(super) ttfb_ms: Option<u128>,
    pub(super) requested_model: Option<String>,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
    pub(super) terminal_signal: Option<&'static str>,
}

impl StreamRequestCompletion {
    pub(super) fn success(
        ttfb_ms: Option<u128>,
        requested_model: Option<String>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
        usage: Option<crate::usage::UsageExtract>,
    ) -> Self {
        Self {
            error_code: None,
            ttfb_ms,
            requested_model,
            usage_metrics,
            usage,
            terminal_signal: Some("completed"),
        }
    }

    pub(super) fn failure(
        error_code: &'static str,
        ttfb_ms: Option<u128>,
        requested_model: Option<String>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
        usage: Option<crate::usage::UsageExtract>,
    ) -> Self {
        Self {
            error_code: Some(error_code),
            ttfb_ms,
            requested_model,
            usage_metrics,
            usage,
            terminal_signal: Some("error"),
        }
    }

    pub(super) fn from_error_code(
        error_code: Option<&'static str>,
        ttfb_ms: Option<u128>,
        requested_model: Option<String>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
        usage: Option<crate::usage::UsageExtract>,
    ) -> Self {
        match error_code {
            Some(code) => Self::failure(code, ttfb_ms, requested_model, usage_metrics, usage),
            None => Self::success(ttfb_ms, requested_model, usage_metrics, usage),
        }
    }

    pub(super) fn with_terminal_signal(mut self, terminal_signal: Option<&'static str>) -> Self {
        self.terminal_signal = terminal_signal;
        self
    }
}

fn ensure_stream_client_abort_setting<R: tauri::Runtime>(
    ctx: &StreamFinalizeCtx<R>,
    duration_ms: u128,
    ttfb_ms: Option<u128>,
    error_code: Option<&'static str>,
) {
    if error_code != Some(GatewayErrorCode::StreamAborted.as_str()) {
        return;
    }

    let already_recorded = ctx
        .special_settings
        .lock()
        .map(|guard| {
            guard.iter().any(|entry| {
                entry.get("type").and_then(serde_json::Value::as_str) == Some("client_abort")
                    && entry.get("scope").and_then(serde_json::Value::as_str) == Some("stream")
            })
        })
        .unwrap_or(false);

    if already_recorded {
        return;
    }

    let duration_ms_i64 = duration_ms.min(i64::MAX as u128) as i64;
    let ttfb_ms_i64 = ttfb_ms.and_then(|value| {
        if value >= duration_ms {
            return None;
        }
        Some(value.min(i64::MAX as u128) as i64)
    });

    response_fixer::push_special_setting(
        &ctx.special_settings,
        serde_json::json!({
            "type": "client_abort",
            "scope": "stream",
            "reason": "stream_finalized_aborted",
            "detected_by": "stream_finalize",
            "duration_ms": duration_ms_i64,
            "ttfb_ms": ttfb_ms_i64,
            "ts": crate::gateway::util::now_unix_seconds() as i64,
        }),
    );
}

fn status_for_stream_request_log(status: u16, error_code: Option<&'static str>) -> u16 {
    match error_code {
        Some(code)
            if code == GatewayErrorCode::StreamError.as_str()
                || code == GatewayErrorCode::Fake200.as_str() =>
        {
            if (200..400).contains(&status) {
                502
            } else {
                status
            }
        }
        Some(code)
            if code == GatewayErrorCode::StreamAborted.as_str()
                || code == GatewayErrorCode::RequestAborted.as_str() =>
        {
            499
        }
        _ => status,
    }
}

fn active_request_finish_reason(error_code: Option<&'static str>) -> ActiveRequestFinishReason {
    match error_code {
        Some(code)
            if code == GatewayErrorCode::StreamAborted.as_str()
                || code == GatewayErrorCode::RequestAborted.as_str() =>
        {
            ActiveRequestFinishReason::ClientAborted
        }
        Some(_) => ActiveRequestFinishReason::Failed,
        None => ActiveRequestFinishReason::Completed,
    }
}

pub(super) fn emit_request_event_and_spawn_request_log<R: tauri::Runtime>(
    ctx: &StreamFinalizeCtx<R>,
    completion: StreamRequestCompletion,
) {
    let duration_ms = ctx.started.elapsed().as_millis();
    let effective_error_category = finalize_circuit_and_session(ctx, completion.error_code);
    if !ctx.observe {
        return;
    }
    ensure_stream_client_abort_setting(ctx, duration_ms, completion.ttfb_ms, completion.error_code);

    // When a stream error occurs, update the last attempt's outcome to reflect
    // the actual error instead of keeping the stale "success" recorded when the
    // stream initially started.
    let (attempts, attempts_json) = if completion.error_code.is_some() {
        let mut attempts = ctx.attempts.clone();
        if let Some(last) = attempts.last_mut() {
            if last.outcome == "success" {
                last.outcome = format!(
                    "stream_error: code={}",
                    completion.error_code.unwrap_or("unknown")
                );
                last.error_code = completion.error_code;
                last.error_category = effective_error_category.or(Some(
                    crate::gateway::proxy::ErrorCategory::SystemError.as_str(),
                ));
                // Update duration to the full stream duration instead of the initial value.
                last.attempt_duration_ms = Some(duration_ms);
            }
        }
        let json = serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());
        (attempts, json)
    } else {
        (ctx.attempts.clone(), ctx.attempts_json.clone())
    };

    let (last_activity_ms, activity_details_json) = ctx
        .activity
        .lock()
        .map(|activity| {
            (
                Some(activity.last_activity_ms()),
                activity.details_json(completion.terminal_signal),
            )
        })
        .unwrap_or((None, None));

    let (log_args, attempts) = RequestLogEnqueueArgs::from_stream_request_end_parts(
        ctx.trace_id.clone(),
        ctx.cli_key.clone(),
        ctx.session_id.clone(),
        ctx.method.clone(),
        ctx.path.clone(),
        ctx.query.clone(),
        ctx.excluded_from_stats,
        response_fixer::special_settings_json(&ctx.special_settings),
        status_for_stream_request_log(ctx.status, completion.error_code),
        completion.error_code,
        duration_ms,
        completion.ttfb_ms,
        attempts,
        attempts_json,
        completion.requested_model,
        ctx.created_at_ms,
        last_activity_ms,
        activity_details_json,
        ctx.created_at,
        completion.usage,
    );

    ctx.active_requests.finish(
        ctx.trace_id.as_str(),
        active_request_finish_reason(completion.error_code),
    );

    log_args.emit_gateway_request_event(
        &ctx.app,
        effective_error_category,
        completion.ttfb_ms,
        attempts,
        completion.usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        ctx.app.clone(),
        ctx.db.clone(),
        ctx.log_tx.clone(),
        log_args,
        Some(ctx.plugin_pipeline.clone()),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        emit_request_event_and_spawn_request_log, status_for_stream_request_log,
        StreamRequestCompletion,
    };
    use crate::gateway::active_requests::{ActiveRequestRegistry, ActiveRequestStart};
    use crate::gateway::proxy::GatewayErrorCode;
    use crate::gateway::streams::{StreamActivityTracker, StreamFinalizeCtx};
    use crate::{circuit_breaker, db, request_logs, session_manager};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    fn active_request_start(trace_id: &str) -> ActiveRequestStart {
        ActiveRequestStart {
            trace_id: trace_id.to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            query: None,
            session_id: Some("sess-stream-end".to_string()),
            requested_model: Some("gpt-5".to_string()),
            created_at_ms: 1_700_000_000_000,
        }
    }

    fn test_stream_finalize_ctx(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        active_requests: Arc<ActiveRequestRegistry>,
    ) -> StreamFinalizeCtx<tauri::test::MockRuntime> {
        StreamFinalizeCtx {
            app,
            db,
            log_tx,
            plugin_pipeline: crate::gateway::plugins::pipeline::GatewayPluginPipeline::empty_shared(
            ),
            circuit: Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            session: Arc::new(session_manager::SessionManager::new()),
            session_id: Some("sess-stream-end".to_string()),
            sort_mode_id: None,
            trace_id: "trace-stream-end".to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            observe: true,
            query: None,
            excluded_from_stats: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            provider_health_neutral: false,
            status: 200,
            error_category: None,
            error_code: None,
            started: Instant::now(),
            attempts: Vec::new(),
            attempts_json: "[]".to_string(),
            requested_model: None,
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            provider_cooldown_secs: 0,
            provider_id: 1,
            provider_name: "test-provider".to_string(),
            base_url: "https://upstream.example".to_string(),
            auth_mode: "api_key".to_string(),
            fake_200_detected: false,
            fake_200_quota_exhausted: false,
            activity: Arc::new(Mutex::new(StreamActivityTracker::new(
                "trace-stream-end",
                "codex",
                1_700_000_000_000,
            ))),
            active_requests,
        }
    }

    #[test]
    fn stream_request_completion_builds_success_without_error_code() {
        let completion =
            StreamRequestCompletion::success(Some(8), Some("gpt-5".to_string()), None, None);

        assert!(completion.error_code.is_none());
        assert_eq!(completion.ttfb_ms, Some(8));
        assert_eq!(completion.requested_model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn stream_request_completion_keeps_terminal_fields_together() {
        let usage_metrics = crate::usage::UsageMetrics::default();
        let completion = StreamRequestCompletion::failure(
            GatewayErrorCode::StreamError.as_str(),
            Some(12),
            Some("gpt-5".to_string()),
            Some(usage_metrics),
            None,
        );

        assert_eq!(
            completion.error_code,
            Some(GatewayErrorCode::StreamError.as_str())
        );
        assert_eq!(completion.ttfb_ms, Some(12));
        assert_eq!(completion.requested_model.as_deref(), Some("gpt-5"));
        assert!(completion.usage_metrics.is_some());
        assert!(completion.usage.is_none());
    }

    #[test]
    fn stream_error_status_for_log_maps_http_200_to_502() {
        assert_eq!(
            status_for_stream_request_log(200, Some(GatewayErrorCode::StreamError.as_str())),
            502
        );
        assert_eq!(
            status_for_stream_request_log(200, Some(GatewayErrorCode::Fake200.as_str())),
            502
        );
        assert_eq!(
            status_for_stream_request_log(499, Some(GatewayErrorCode::StreamAborted.as_str())),
            499
        );
    }

    #[test]
    fn observed_stream_request_end_finishes_active_request() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("stream-request-end.sqlite"))
            .expect("init test db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-stream-end"));
        let ctx =
            test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests.clone());

        emit_request_event_and_spawn_request_log(
            &ctx,
            StreamRequestCompletion::success(None, Some("gpt-5".to_string()), None, None),
        );

        assert!(active_requests.snapshot().is_empty());
    }

    #[test]
    fn observed_stream_abort_finishes_active_request() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("stream-request-abort.sqlite"))
            .expect("init test db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-stream-end"));
        let ctx =
            test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests.clone());

        emit_request_event_and_spawn_request_log(
            &ctx,
            StreamRequestCompletion::failure(
                GatewayErrorCode::StreamAborted.as_str(),
                None,
                Some("gpt-5".to_string()),
                None,
                None,
            ),
        );

        assert!(active_requests.snapshot().is_empty());
    }
}
