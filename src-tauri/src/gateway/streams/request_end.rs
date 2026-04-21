//! Usage: Shared helpers to finalize stream requests (event + request log).

use super::finalize::finalize_circuit_and_session;
use super::StreamFinalizeCtx;
use crate::gateway::proxy::{spawn_enqueue_request_log_with_backpressure, RequestLogEnqueueArgs};
use crate::gateway::response_fixer;

pub(super) fn emit_request_event_and_spawn_request_log(
    ctx: &StreamFinalizeCtx,
    error_code: Option<&'static str>,
    ttfb_ms: Option<u128>,
    requested_model: Option<String>,
    usage_metrics: Option<crate::usage::UsageMetrics>,
    usage: Option<crate::usage::UsageExtract>,
) {
    let duration_ms = ctx.started.elapsed().as_millis();
    let effective_error_category = finalize_circuit_and_session(ctx, error_code);
    if !ctx.observe {
        return;
    }

    // When a stream error occurs, update the last attempt's outcome to reflect
    // the actual error instead of keeping the stale "success" recorded when the
    // stream initially started.
    let (attempts, attempts_json) = if error_code.is_some() {
        let mut attempts = ctx.attempts.clone();
        if let Some(last) = attempts.last_mut() {
            if last.outcome == "success" {
                last.outcome = format!("stream_error: code={}", error_code.unwrap_or("unknown"));
                last.error_code = error_code;
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
        error_code,
        duration_ms,
        ttfb_ms,
        attempts,
        attempts_json,
        requested_model,
        ctx.created_at_ms,
        ctx.created_at,
        usage,
    );

    log_args.emit_gateway_request_event(
        &ctx.app,
        effective_error_category,
        ttfb_ms,
        attempts,
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        ctx.app.clone(),
        ctx.db.clone(),
        ctx.log_tx.clone(),
        log_args,
    );
}
