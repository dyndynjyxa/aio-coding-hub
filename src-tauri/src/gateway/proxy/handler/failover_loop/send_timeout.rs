//! Usage: Handle upstream send timeout inside `failover_loop::run`.

use super::*;

pub(super) async fn handle_timeout(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_>,
) -> LoopControl {
    let state = ctx.state;
    let cli_key = ctx.cli_key.to_string();
    let method_hint = ctx.method_hint.to_string();
    let forwarded_path = ctx.forwarded_path.to_string();
    let query = ctx.query.clone();
    let trace_id = ctx.trace_id.to_string();
    let created_at = ctx.created_at;
    let provider_cooldown_secs = ctx.provider_cooldown_secs;
    let upstream_first_byte_timeout_secs = ctx.upstream_first_byte_timeout_secs;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;

    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
    } = provider_ctx;
    let provider_name_base = provider_name_base.to_string();
    let provider_base_url_base = provider_base_url_base.to_string();

    let AttemptCtx {
        attempt_index,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_error_category,
        last_error_code,
        circuit_snapshot,
        abort_guard: _,
    } = loop_state;

    let category = ErrorCategory::SystemError;
    let error_code = "GW_UPSTREAM_TIMEOUT";
    let decision = if retry_index < max_attempts_per_provider {
        FailoverDecision::RetrySameProvider
    } else {
        FailoverDecision::SwitchProvider
    };

    let outcome = format!(
        "request_timeout: category={} code={} decision={} timeout_secs={}",
        category.as_str(),
        error_code,
        decision.as_str(),
        upstream_first_byte_timeout_secs,
    );

    attempts.push(FailoverAttempt {
        provider_id,
        provider_name: provider_name_base.clone(),
        base_url: provider_base_url_base.clone(),
        outcome: outcome.clone(),
        status: None,
        provider_index: Some(provider_index),
        retry_index: Some(retry_index),
        session_reuse,
        error_category: Some(category.as_str()),
        error_code: Some(error_code),
        decision: Some(decision.as_str()),
        reason: Some("request timeout".to_string()),
        attempt_started_ms: Some(attempt_started_ms),
        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
        circuit_state_before: Some(circuit_before.state.as_str()),
        circuit_state_after: None,
        circuit_failure_count: Some(circuit_before.failure_count),
        circuit_failure_threshold: Some(circuit_before.failure_threshold),
    });

    let attempt_event = GatewayAttemptEvent {
        trace_id: trace_id.clone(),
        cli_key: cli_key.clone(),
        method: method_hint.clone(),
        path: forwarded_path.clone(),
        query: query.clone(),
        attempt_index,
        provider_id,
        session_reuse,
        provider_name: provider_name_base.clone(),
        base_url: provider_base_url_base.clone(),
        outcome,
        status: None,
        attempt_started_ms,
        attempt_duration_ms: attempt_started.elapsed().as_millis(),
        circuit_state_before: Some(circuit_before.state.as_str()),
        circuit_state_after: None,
        circuit_failure_count: Some(circuit_before.failure_count),
        circuit_failure_threshold: Some(circuit_before.failure_threshold),
    };
    emit_attempt_event(&state.app, attempt_event.clone());
    enqueue_attempt_log_with_backpressure(
        &state.app,
        &state.db,
        &state.attempt_log_tx,
        &attempt_event,
        created_at,
    )
    .await;

    *last_error_category = Some(category.as_str());
    *last_error_code = Some(error_code);

    if provider_cooldown_secs > 0
        && matches!(
            decision,
            FailoverDecision::SwitchProvider | FailoverDecision::Abort
        )
    {
        let now_unix = now_unix_seconds() as i64;
        let snap = state
            .circuit
            .trigger_cooldown(provider_id, now_unix, provider_cooldown_secs);
        *circuit_snapshot = snap;
    }

    match decision {
        FailoverDecision::RetrySameProvider => LoopControl::ContinueRetry,
        FailoverDecision::SwitchProvider => {
            failed_provider_ids.insert(provider_id);
            LoopControl::BreakRetry
        }
        FailoverDecision::Abort => LoopControl::BreakRetry,
    }
}
