//! Usage: Shared helpers to finalize stream requests (event + request log).

use super::finalize::finalize_circuit_and_session;
use super::StreamFinalizeCtx;
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

    let (log_args, attempts) = RequestLogEnqueueArgs::from_stream_request_end_parts(
        ctx.trace_id.clone(),
        ctx.cli_key.clone(),
        ctx.session_id.clone(),
        ctx.method.clone(),
        ctx.path.clone(),
        ctx.query.clone(),
        ctx.excluded_from_stats,
        response_fixer::special_settings_json(&ctx.special_settings),
        ctx.status,
        completion.error_code,
        duration_ms,
        completion.ttfb_ms,
        attempts,
        attempts_json,
        completion.requested_model,
        ctx.created_at_ms,
        ctx.created_at,
        completion.usage,
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
    );
}

#[cfg(test)]
mod tests {
    use super::StreamRequestCompletion;
    use crate::gateway::proxy::GatewayErrorCode;

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
}
