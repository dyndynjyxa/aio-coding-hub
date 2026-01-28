//! Usage: Small helpers to build/emit attempt events consistently across failover_loop.

use super::super::super::logging::enqueue_attempt_log_with_backpressure;
use super::context::{AttemptCtx, CommonCtx, ProviderCtx};
use crate::gateway::events::{emit_attempt_event, GatewayAttemptEvent};

#[derive(Clone, Copy)]
pub(super) struct AttemptCircuitFields {
    pub(super) state_before: Option<&'static str>,
    pub(super) state_after: Option<&'static str>,
    pub(super) failure_count: Option<u32>,
    pub(super) failure_threshold: Option<u32>,
}

pub(super) async fn emit_attempt_event_and_log(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    outcome: String,
    status: Option<u16>,
    circuit: AttemptCircuitFields,
) {
    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index: _,
        session_reuse,
    } = provider_ctx;
    let AttemptCtx {
        attempt_index,
        retry_index: _,
        attempt_started_ms,
        attempt_started,
        circuit_before: _,
    } = attempt_ctx;

    let attempt_event = GatewayAttemptEvent {
        trace_id: ctx.trace_id.clone(),
        cli_key: ctx.cli_key.clone(),
        method: ctx.method_hint.clone(),
        path: ctx.forwarded_path.clone(),
        query: ctx.query.clone(),
        attempt_index,
        provider_id,
        session_reuse,
        provider_name: provider_name_base.clone(),
        base_url: provider_base_url_base.clone(),
        outcome,
        status,
        attempt_started_ms,
        attempt_duration_ms: attempt_started.elapsed().as_millis(),
        circuit_state_before: circuit.state_before,
        circuit_state_after: circuit.state_after,
        circuit_failure_count: circuit.failure_count,
        circuit_failure_threshold: circuit.failure_threshold,
    };

    let state = ctx.state;
    emit_attempt_event(&state.app, attempt_event.clone());
    enqueue_attempt_log_with_backpressure(
        &state.app,
        &state.db,
        &state.attempt_log_tx,
        &attempt_event,
        ctx.created_at,
    )
    .await;
}

pub(super) async fn emit_attempt_event_and_log_with_circuit_before(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    outcome: String,
    status: Option<u16>,
) {
    let circuit_before = attempt_ctx.circuit_before;
    emit_attempt_event_and_log(
        ctx,
        provider_ctx,
        attempt_ctx,
        outcome,
        status,
        AttemptCircuitFields {
            state_before: Some(circuit_before.state.as_str()),
            state_after: None,
            failure_count: Some(circuit_before.failure_count),
            failure_threshold: Some(circuit_before.failure_threshold),
        },
    )
    .await;
}
