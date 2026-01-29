//! Usage: Shared helpers to finalize stream requests (event + request log).

use super::finalize::finalize_circuit_and_session;
use super::StreamFinalizeCtx;
use crate::gateway::events::emit_request_event;
use crate::gateway::proxy::{
    spawn_enqueue_request_log_with_backpressure, status_override, RequestLogEnqueueArgs,
};
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
    let effective_status = status_override::effective_status(Some(ctx.status), error_code);
    let effective_excluded_from_stats =
        ctx.excluded_from_stats || status_override::is_client_abort(error_code);

    let trace_id = ctx.trace_id.clone();
    let cli_key = ctx.cli_key.clone();
    let method = ctx.method.clone();
    let path = ctx.path.clone();
    let query = ctx.query.clone();

    emit_request_event(
        &ctx.app,
        trace_id.clone(),
        cli_key.clone(),
        method.clone(),
        path.clone(),
        query.clone(),
        effective_status,
        effective_error_category,
        error_code,
        duration_ms,
        ttfb_ms,
        ctx.attempts.clone(),
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        ctx.app.clone(),
        ctx.db.clone(),
        ctx.log_tx.clone(),
        RequestLogEnqueueArgs {
            trace_id,
            cli_key,
            session_id: ctx.session_id.clone(),
            method,
            path,
            query,
            excluded_from_stats: effective_excluded_from_stats,
            special_settings_json: response_fixer::special_settings_json(&ctx.special_settings),
            status: effective_status,
            error_code,
            duration_ms,
            ttfb_ms,
            attempts_json: ctx.attempts_json.clone(),
            requested_model,
            created_at_ms: ctx.created_at_ms,
            created_at: ctx.created_at,
            usage_metrics: None,
            usage,
        },
    );
}
