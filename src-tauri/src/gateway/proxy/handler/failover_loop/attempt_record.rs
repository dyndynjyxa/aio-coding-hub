//! Usage: Shared helpers to record SystemError attempts and apply failover decisions.

use super::super::super::provider_router;
use super::super::super::status_override;
use super::*;

pub(super) struct RecordSystemFailureArgs<'a> {
    pub(super) ctx: CommonCtx<'a>,
    pub(super) provider_ctx: ProviderCtx<'a>,
    pub(super) attempt_ctx: AttemptCtx<'a>,
    pub(super) loop_state: LoopState<'a>,
    pub(super) status: Option<u16>,
    pub(super) error_code: &'static str,
    pub(super) decision: FailoverDecision,
    pub(super) outcome: String,
    pub(super) reason: String,
}

pub(super) async fn record_system_failure_and_decide(
    args: RecordSystemFailureArgs<'_>,
) -> LoopControl {
    record_system_failure_and_decide_impl(args, CooldownPolicy::Apply).await
}

pub(super) async fn record_system_failure_and_decide_no_cooldown(
    args: RecordSystemFailureArgs<'_>,
) -> LoopControl {
    record_system_failure_and_decide_impl(args, CooldownPolicy::Skip).await
}

#[derive(Debug, Clone, Copy)]
enum CooldownPolicy {
    Apply,
    Skip,
}

async fn record_system_failure_and_decide_impl(
    args: RecordSystemFailureArgs<'_>,
    cooldown_policy: CooldownPolicy,
) -> LoopControl {
    let RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        status,
        error_code,
        decision,
        outcome,
        reason,
    } = args;
    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
    } = provider_ctx;

    let AttemptCtx {
        attempt_index: _,
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
    let effective_status = status_override::effective_status(status, Some(error_code));

    attempts.push(FailoverAttempt {
        provider_id,
        provider_name: provider_name_base.clone(),
        base_url: provider_base_url_base.clone(),
        outcome: outcome.clone(),
        status: effective_status,
        provider_index: Some(provider_index),
        retry_index: Some(retry_index),
        session_reuse,
        error_category: Some(category.as_str()),
        error_code: Some(error_code),
        decision: Some(decision.as_str()),
        reason: Some(reason),
        attempt_started_ms: Some(attempt_started_ms),
        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
        circuit_state_before: Some(circuit_before.state.as_str()),
        circuit_state_after: None,
        circuit_failure_count: Some(circuit_before.failure_count),
        circuit_failure_threshold: Some(circuit_before.failure_threshold),
    });

    emit_attempt_event_and_log_with_circuit_before(
        ctx,
        provider_ctx,
        attempt_ctx,
        outcome,
        effective_status,
    )
    .await;

    *last_error_category = Some(category.as_str());
    *last_error_code = Some(error_code);

    if matches!(cooldown_policy, CooldownPolicy::Apply) {
        let provider_cooldown_secs = ctx.provider_cooldown_secs;
        if provider_cooldown_secs > 0
            && matches!(
                decision,
                FailoverDecision::SwitchProvider | FailoverDecision::Abort
            )
        {
            let now_unix = now_unix_seconds() as i64;
            let snap = provider_router::trigger_cooldown(
                ctx.state.circuit.as_ref(),
                provider_id,
                now_unix,
                provider_cooldown_secs,
            );
            *circuit_snapshot = snap;
        }
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
