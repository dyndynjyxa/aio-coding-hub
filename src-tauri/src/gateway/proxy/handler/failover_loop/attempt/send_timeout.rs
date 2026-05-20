//! Usage: Handle upstream send timeout inside `failover_loop::run`.

use super::*;
use crate::gateway::proxy::is_claude_count_tokens_request;

fn timeout_decision(is_count_tokens: bool) -> FailoverDecision {
    if is_count_tokens {
        return FailoverDecision::Abort;
    }

    FailoverDecision::SwitchProvider
}

pub(super) async fn handle_timeout<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_, R>,
) -> LoopControl {
    let is_count_tokens =
        is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());
    let error_code = GatewayErrorCode::UpstreamTimeout.as_str();
    let decision = timeout_decision(is_count_tokens);

    let outcome = format!(
        "request_timeout: category={} code={} decision={} timeout_secs={}",
        ErrorCategory::SystemError.as_str(),
        error_code,
        decision.as_str(),
        ctx.upstream_first_byte_timeout_secs,
    );

    if is_count_tokens {
        return record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
            ctx,
            provider_ctx,
            attempt_ctx,
            loop_state,
            status: None,
            error_code,
            decision,
            outcome,
            reason: "request timeout".to_string(),
        })
        .await;
    }

    record_system_failure_and_decide(RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        status: None,
        error_code,
        decision,
        outcome,
        reason: "request timeout".to_string(),
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{timeout_decision, FailoverDecision};

    #[test]
    fn timeout_decision_aborts_for_count_tokens() {
        let decision = timeout_decision(true);
        assert!(matches!(decision, FailoverDecision::Abort));
    }

    #[test]
    fn timeout_decision_switches_for_regular_requests_before_retry_limit() {
        let decision = timeout_decision(false);
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn timeout_decision_switches_for_regular_requests_after_retry_limit() {
        let decision = timeout_decision(false);
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }
}
