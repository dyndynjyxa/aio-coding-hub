//! Usage: Retry decision engine (error classification -> retry same / switch / abort).
//!
//! Processes the outcome of a single attempt and decides the next action:
//! continue retrying the same provider, switch to the next provider, or
//! return a final response to the client.

use super::attempt_executor::{AttemptSendOutcome, AttemptTiming, RetryLoopState};
use super::provider_iterator::PreparedProvider;
use super::*;
use crate::gateway::proxy::request_context::RequestContext;

#[derive(Clone, Copy)]
pub(super) struct AttemptIndices {
    pub(super) retry_index: u32,
    pub(super) attempt_index: u32,
}

/// Run the inner retry loop for a single prepared provider.
///
/// Returns `Some(Response)` if a final response was produced (success or
/// terminal error); returns `None` when all retries for this provider are
/// exhausted and the outer loop should try the next provider.
pub(super) async fn run_retry_loop(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    prepared: &mut PreparedProvider,
    mut loop_state: LoopState<'_>,
) -> Option<Response> {
    let mut retry_state = RetryLoopState::new();

    for retry_index in 1..=prepared.provider_max_attempts {
        let attempt_index = loop_state.attempts.len().saturating_add(1) as u32;

        let send_outcome = attempt_executor::execute_attempt(
            ctx,
            input,
            prepared,
            &mut retry_state,
            retry_index,
            attempt_index,
            &mut loop_state,
        )
        .await;

        let ctrl = dispatch_outcome(
            ctx,
            input,
            prepared,
            &mut retry_state,
            AttemptIndices {
                retry_index,
                attempt_index,
            },
            send_outcome,
            &mut loop_state,
        )
        .await;

        match ctrl {
            LoopControl::ContinueRetry => continue,
            LoopControl::BreakRetry => break,
            LoopControl::Return(resp) => return Some(resp),
        }
    }

    None
}

/// Dispatch one attempt outcome to the appropriate handler and return
/// a `LoopControl` for the retry loop.
async fn dispatch_outcome(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    prepared: &mut PreparedProvider,
    retry_state: &mut RetryLoopState,
    indices: AttemptIndices,
    send_outcome: AttemptSendOutcome,
    loop_state: &mut LoopState<'_>,
) -> LoopControl {
    match send_outcome {
        AttemptSendOutcome::UrlBuildFailed(ctrl) => ctrl,
        AttemptSendOutcome::OAuthInjectFailed => LoopControl::BreakRetry,
        AttemptSendOutcome::Response(resp, timing) => {
            response_router::route_response(
                ctx,
                input,
                prepared,
                retry_state,
                indices,
                resp,
                timing,
                loop_state,
            )
            .await
        }
        AttemptSendOutcome::Timeout(timing) => {
            let (attempt_ctx, provider_ctx) = build_error_contexts(
                input,
                prepared,
                &timing,
                indices.attempt_index,
                indices.retry_index,
            );
            send_timeout::handle_timeout(ctx, provider_ctx, attempt_ctx, loop_state.reborrow())
                .await
        }
        AttemptSendOutcome::ReqwestError(err, timing) => {
            let (attempt_ctx, provider_ctx) = build_error_contexts(
                input,
                prepared,
                &timing,
                indices.attempt_index,
                indices.retry_index,
            );
            upstream_error::handle_reqwest_error(
                ctx,
                provider_ctx,
                attempt_ctx,
                loop_state.reborrow(),
                err,
            )
            .await
        }
    }
}

/// Build `AttemptCtx` and `ProviderCtx` for error-path handling (timeout / reqwest error).
fn build_error_contexts<'a>(
    _input: &RequestContext,
    prepared: &'a PreparedProvider,
    timing: &AttemptTiming,
    attempt_index: u32,
    retry_index: u32,
) -> (AttemptCtx<'a>, ProviderCtx<'a>) {
    let attempt_ctx = AttemptCtx {
        attempt_index,
        retry_index,
        attempt_started_ms: timing.attempt_started_ms,
        attempt_started: timing.attempt_started,
        circuit_before: &prepared.circuit_snapshot,
        gemini_oauth_response_mode: prepared.gemini_oauth_response_mode,
        cx2cc_active: prepared.cx2cc_active,
        anthropic_stream_requested: prepared.anthropic_stream_requested,
    };
    let provider_ctx = ProviderCtx {
        provider_id: prepared.provider_id,
        provider_name_base: &prepared.provider_name_base,
        provider_base_url_base: &prepared.provider_base_url_base,
        provider_index: prepared.provider_index,
        session_reuse: prepared.session_reuse,
        stream_idle_timeout_seconds: prepared.stream_idle_timeout_seconds,
    };
    (attempt_ctx, provider_ctx)
}
