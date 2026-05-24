//! Usage: Shared helpers to record SystemError attempts and apply failover decisions.

use super::*;
use crate::circuit_breaker;
use crate::gateway::events::decision_chain as dc;
use crate::gateway::proxy::status_override;
use crate::gateway::proxy::{is_claude_count_tokens_request, provider_router};

pub(super) struct RecordSystemFailureArgs<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) provider_ctx: ProviderCtx<'a>,
    pub(super) attempt_ctx: AttemptCtx<'a>,
    pub(super) loop_state: LoopState<'a, R>,
    pub(super) status: Option<u16>,
    pub(super) error_code: &'static str,
    pub(super) decision: FailoverDecision,
    pub(super) outcome: String,
    pub(super) reason: String,
}

pub(super) async fn record_system_failure_and_decide<R: tauri::Runtime>(
    args: RecordSystemFailureArgs<'_, R>,
) -> LoopControl {
    record_system_failure_and_decide_impl(args, CooldownPolicy::Apply).await
}

pub(super) async fn record_system_failure_and_decide_no_cooldown<R: tauri::Runtime>(
    args: RecordSystemFailureArgs<'_, R>,
) -> LoopControl {
    record_system_failure_and_decide_impl(args, CooldownPolicy::Skip).await
}

#[derive(Debug, Clone, Copy)]
enum CooldownPolicy {
    Apply,
    Skip,
}

async fn record_system_failure_and_decide_impl<R: tauri::Runtime>(
    args: RecordSystemFailureArgs<'_, R>,
    cooldown_policy: CooldownPolicy,
) -> LoopControl {
    let RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        status,
        error_code,
        mut decision,
        mut outcome,
        reason,
    } = args;
    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
        ..
    } = provider_ctx;

    let AttemptCtx {
        attempt_index: _,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
        ..
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_outcome,
        circuit_snapshot,
        abort_guard: _,
    } = loop_state;

    let category = ErrorCategory::SystemError;
    let effective_status = status_override::effective_status(status, Some(error_code));

    let is_count_tokens =
        is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());
    let now_unix = now_unix_seconds() as i64;

    let mut circuit_state_before = Some(circuit_before.state.as_str());
    let mut circuit_state_after: Option<&'static str> = None;
    let mut circuit_failure_count = Some(circuit_before.failure_count);
    let circuit_failure_threshold = Some(circuit_before.failure_threshold);

    if !is_count_tokens {
        let change = provider_router::record_failure_and_emit_transition(
            provider_router::RecordCircuitArgs::from_state(
                ctx.state,
                ctx.trace_id.as_str(),
                ctx.cli_key.as_str(),
                provider_id,
                provider_name_base.as_str(),
                provider_base_url_base.as_str(),
                now_unix,
            ),
        );
        *circuit_snapshot = change.after.clone();
        circuit_state_before = Some(change.before.state.as_str());
        circuit_state_after = Some(change.after.state.as_str());
        circuit_failure_count = Some(change.after.failure_count);

        let recorded_decision = decision;
        decision =
            system_failure_decision_after_circuit_record(decision, false, Some(change.after.state));
        outcome =
            system_failure_outcome_after_decision_override(outcome, recorded_decision, decision);
    }

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
        selection_method: dc::selection_method(provider_index, retry_index, session_reuse),
        reason_code: Some(category.reason_code()),
        attempt_started_ms: Some(attempt_started_ms),
        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
        circuit_state_before,
        circuit_state_after,
        circuit_failure_count,
        circuit_failure_threshold,
    });

    emit_attempt_event_and_log_with_circuit_before(
        ctx,
        provider_ctx,
        attempt_ctx,
        outcome,
        effective_status,
    )
    .await;

    *last_outcome = Some(AttemptOutcome::new(category.as_str(), error_code));

    let should_apply_cooldown = matches!(cooldown_policy, CooldownPolicy::Apply)
        && !is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());

    if should_apply_cooldown {
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

fn system_failure_decision_after_circuit_record(
    decision: FailoverDecision,
    is_count_tokens: bool,
    circuit_state_after: Option<circuit_breaker::CircuitState>,
) -> FailoverDecision {
    if !is_count_tokens
        && matches!(decision, FailoverDecision::RetrySameProvider)
        && matches!(
            circuit_state_after,
            Some(circuit_breaker::CircuitState::Open)
        )
    {
        return FailoverDecision::SwitchProvider;
    }

    decision
}

fn system_failure_outcome_after_decision_override(
    outcome: String,
    before: FailoverDecision,
    after: FailoverDecision,
) -> String {
    if matches!(before, FailoverDecision::RetrySameProvider)
        && matches!(after, FailoverDecision::SwitchProvider)
    {
        return outcome.replace("decision=retry", "decision=switch");
    }

    outcome
}

#[cfg(test)]
mod tests {
    use super::{
        circuit_breaker, system_failure_decision_after_circuit_record,
        system_failure_outcome_after_decision_override, FailoverDecision,
    };

    #[test]
    fn system_failure_switches_when_circuit_opens() {
        let decision = system_failure_decision_after_circuit_record(
            FailoverDecision::RetrySameProvider,
            false,
            Some(circuit_breaker::CircuitState::Open),
        );

        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn system_failure_preserves_retry_when_circuit_stays_closed() {
        let decision = system_failure_decision_after_circuit_record(
            FailoverDecision::RetrySameProvider,
            false,
            Some(circuit_breaker::CircuitState::Closed),
        );

        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }

    #[test]
    fn system_failure_preserves_count_tokens_abort() {
        let decision = system_failure_decision_after_circuit_record(
            FailoverDecision::Abort,
            true,
            Some(circuit_breaker::CircuitState::Open),
        );

        assert!(matches!(decision, FailoverDecision::Abort));
    }

    #[test]
    fn system_failure_outcome_tracks_retry_to_switch_override() {
        let outcome = system_failure_outcome_after_decision_override(
            "request_timeout: category=SYSTEM_ERROR code=GW_UPSTREAM_TIMEOUT decision=retry timeout_secs=30"
                .to_string(),
            FailoverDecision::RetrySameProvider,
            FailoverDecision::SwitchProvider,
        );

        assert!(outcome.contains("decision=switch"));
        assert!(!outcome.contains("decision=retry"));
    }
}
