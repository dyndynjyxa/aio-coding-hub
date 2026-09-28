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
pub(super) async fn run_retry_loop<R>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &mut PreparedProvider,
    mut loop_state: LoopState<'_, R>,
) -> Option<Response>
where
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    let mut retry_state = RetryLoopState::new();

    let mut retry_index = input.ws_request.as_ref().map_or(1, |request| {
        use crate::shared::mutex_ext::MutexExt;
        let generation = request.generation.lock_or_recover();
        if generation.budget.provider_id == Some(prepared.provider_id) {
            generation.budget.retry_index.max(1)
        } else {
            1
        }
    });
    while retry_index <= retry_state.effective_attempt_limit(prepared.provider_max_attempts) {
        if let Some(request) = &input.ws_request {
            use crate::shared::mutex_ext::MutexExt;
            let mut generation = request.generation.lock_or_recover();
            if generation.budget.provider_id != Some(prepared.provider_id)
                || generation.budget.retry_index != retry_index
            {
                generation.budget.provider_id = Some(prepared.provider_id);
                generation.budget.retry_index = retry_index;
                generation.budget.deadline = input
                    .upstream_first_byte_timeout
                    .and_then(|duration| std::time::Instant::now().checked_add(duration));
            }
        }
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
            LoopControl::RetryTransport => continue,
            LoopControl::ContinueRetry => {
                let Some(next_retry_index) = retry_index.checked_add(1) else {
                    break;
                };
                retry_index = next_retry_index;
                continue;
            }
            LoopControl::BreakRetry => break,
            LoopControl::Return(resp) => return Some(resp),
        }
    }

    None
}

/// Dispatch one attempt outcome to the appropriate handler and return
/// a `LoopControl` for the retry loop.
async fn dispatch_outcome<R>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &mut PreparedProvider,
    retry_state: &mut RetryLoopState,
    indices: AttemptIndices,
    send_outcome: AttemptSendOutcome,
    loop_state: &mut LoopState<'_, R>,
) -> LoopControl
where
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    match send_outcome {
        AttemptSendOutcome::UrlBuildFailed(ctrl) => ctrl,
        AttemptSendOutcome::OAuthInjectFailed => LoopControl::BreakRetry,
        AttemptSendOutcome::PluginBlocked(reason) => LoopControl::Return(error_response(
            StatusCode::FORBIDDEN,
            input.trace_id.clone(),
            GatewayErrorCode::InternalError.as_str(),
            reason,
            loop_state.attempts.clone(),
        )),
        AttemptSendOutcome::WsTransport(reason, timing) => {
            ws_attempt::transport_failure(ctx, input, prepared, indices, timing, reason, loop_state)
                .await
        }
        AttemptSendOutcome::ContextLost(timing) => {
            ws_attempt::recover(ctx, input, prepared, indices, timing, loop_state).await
        }
        AttemptSendOutcome::LocalProtocol(reason, _timing) => {
            if reason == "prewarm_http_required" {
                if let Some(connection) = &input.ws_connection {
                    if let Some(session) = input
                        .base_headers
                        .get("session-id")
                        .and_then(|v| v.to_str().ok())
                    {
                        connection.runtime.force_http(session);
                    }
                }
            }
            ws_attempt::finish_error(ctx, input, loop_state, "invalid_request", reason).await
        }
        AttemptSendOutcome::StreamResponse(resp, timing) => {
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
        AttemptSendOutcome::Response(resp, timing) => {
            response_router::route_response(
                ctx,
                input,
                prepared,
                retry_state,
                indices,
                resp.into(),
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
fn build_error_contexts<'a, R: tauri::Runtime>(
    _input: &RequestContext<R>,
    prepared: &'a PreparedProvider,
    timing: &'a AttemptTiming,
    attempt_index: u32,
    retry_index: u32,
) -> (AttemptCtx<'a>, ProviderCtx<'a>) {
    let attempt_ctx = AttemptCtx {
        attempt_index,
        retry_index,
        provider_max_attempts: prepared.provider_max_attempts,
        attempt_started_ms: timing.attempt_started_ms,
        attempt_started: timing.attempt_started,
        circuit_before: &prepared.circuit_snapshot,
        gemini_oauth_response_mode: prepared.gemini_oauth_response_mode,
        cx2cc_active: prepared.cx2cc_active,
        anthropic_stream_requested: prepared.anthropic_stream_requested,
        reasoning_effort: timing.reasoning_effort.as_deref(),
        upstream_sent: timing.upstream_sent,
    };
    let provider_ctx = ProviderCtx {
        provider_id: prepared.provider_id,
        provider_name_base: &prepared.provider_name_base,
        provider_base_url_base: &prepared.provider_base_url_base,
        auth_mode: prepared.auth_mode.as_str(),
        provider_index: prepared.provider_index,
        provider_bridged: prepared.provider_bridged,
        session_reuse: prepared.session_reuse,
        stream_idle_timeout_seconds: prepared.stream_idle_timeout_seconds,
        claude_model_mapping: prepared.claude_model_mapping.as_ref(),
        model_redirect: prepared.model_redirect.as_ref(),
    };
    (attempt_ctx, provider_ctx)
}

#[cfg(test)]
mod tests {
    use super::attempt_executor::grant_repair_retry_slot_if_needed;
    use super::RetryLoopState;

    #[test]
    fn repair_retry_slot_extends_only_at_the_current_boundary() {
        let mut slots = 0;

        assert!(!grant_repair_retry_slot_if_needed(&mut slots, 1, 2));
        let state = RetryLoopState {
            additional_repair_retry_slots: slots,
            ..RetryLoopState::new()
        };
        assert_eq!(state.effective_attempt_limit(2), 2);

        assert!(grant_repair_retry_slot_if_needed(&mut slots, 2, 2));
        let state = RetryLoopState {
            additional_repair_retry_slots: slots,
            ..RetryLoopState::new()
        };
        assert_eq!(state.effective_attempt_limit(2), 3);
    }

    #[test]
    fn later_repair_can_extend_the_new_boundary_once_more() {
        let mut slots = 0;

        assert!(grant_repair_retry_slot_if_needed(&mut slots, 1, 1));
        let state = RetryLoopState {
            additional_repair_retry_slots: slots,
            ..RetryLoopState::new()
        };
        assert_eq!(state.effective_attempt_limit(1), 2);
        assert!(grant_repair_retry_slot_if_needed(&mut slots, 2, 1));
        let state = RetryLoopState {
            additional_repair_retry_slots: slots,
            ..RetryLoopState::new()
        };
        assert_eq!(state.effective_attempt_limit(1), 3);
    }

    #[test]
    fn ordinary_attempt_limit_is_unchanged_without_a_repair() {
        let state = RetryLoopState::new();
        assert_eq!(state.effective_attempt_limit(1), 1);
        assert_eq!(state.effective_attempt_limit(5), 5);
    }
}
