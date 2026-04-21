//! Usage: Single attempt execution (build request, send upstream, return result).
//!
//! Encapsulates URL construction, header assembly, auth injection, body
//! cleaning, and the upstream send for one retry attempt.

use super::provider_iterator::PreparedProvider;
use super::*;
use crate::gateway::proxy::abort_guard::RequestAbortGuard;
use crate::gateway::proxy::request_context::RequestContext;

/// Mutable per-provider state that persists across retries within one provider.
pub(super) struct RetryLoopState {
    pub(super) claude_api_key_bearer_fallback: bool,
    pub(super) oauth_reactive_refreshed_once: bool,
    pub(super) thinking_signature_rectifier_retried: bool,
    pub(super) thinking_budget_rectifier_retried: bool,
}

impl RetryLoopState {
    pub(super) fn new() -> Self {
        Self {
            claude_api_key_bearer_fallback: false,
            oauth_reactive_refreshed_once: false,
            thinking_signature_rectifier_retried: false,
            thinking_budget_rectifier_retried: false,
        }
    }
}

/// Timing captured at the start of an attempt, before the upstream send.
pub(super) struct AttemptTiming {
    pub(super) attempt_started_ms: u128,
    pub(super) attempt_started: Instant,
}

/// Result of building + sending one attempt.
pub(super) enum AttemptSendOutcome {
    Response(reqwest::Response, AttemptTiming),
    Timeout(AttemptTiming),
    ReqwestError(reqwest::Error, AttemptTiming),
    /// URL build failure already recorded; caller should apply the returned LoopControl.
    UrlBuildFailed(LoopControl),
    /// OAuth adapter injection failed; break out of retry loop for this provider.
    OAuthInjectFailed,
}

/// Build request headers, inject auth, clean body, send upstream, and return
/// the raw outcome. The caller (retry engine / response router) handles the
/// result.
pub(super) async fn execute_attempt(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    prepared: &mut PreparedProvider,
    retry_state: &mut RetryLoopState,
    retry_index: u32,
    attempt_index: u32,
    loop_state: &mut LoopState<'_>,
) -> AttemptSendOutcome {
    let attempt_started_ms = input.started.elapsed().as_millis();
    let circuit_before = prepared.circuit_snapshot.clone();

    // --- Build URL ---
    let url = match try_build_url(prepared) {
        Ok(u) => u,
        Err(err) => {
            let attempt_ctx = build_attempt_ctx(
                attempt_index,
                retry_index,
                attempt_started_ms,
                &circuit_before,
                prepared,
            );
            let provider_ctx = build_provider_ctx(prepared);
            let ctrl =
                handle_url_build_failure(ctx, input, attempt_ctx, provider_ctx, err, loop_state)
                    .await;
            return AttemptSendOutcome::UrlBuildFailed(ctrl);
        }
    };

    // --- Emit "started" attempt event ---
    emit_started_event(
        input,
        prepared,
        attempt_index,
        retry_index,
        attempt_started_ms,
        &circuit_before,
        loop_state.abort_guard,
    );

    // --- Build headers + inject auth ---
    let mut headers = input.base_headers.clone();
    ensure_cli_required_headers(&input.cli_key, &mut headers);
    if input.cli_key == "claude" {
        mark_internal_forwarded_request(&mut headers);
    }
    codex_session_id_completion::inject_session_headers_if_needed(
        &mut headers,
        prepared.cx2cc_codex_session_id.as_deref(),
    );

    if let Err(failed_attempt) = attempt_auth::inject_auth(
        ctx,
        input,
        prepared,
        retry_state,
        &attempt_auth::AuthErrorCtx {
            attempt_index,
            retry_index,
            attempt_started_ms,
            circuit_before: &circuit_before,
        },
        &mut headers,
    ) {
        loop_state.attempts.push(*failed_attempt);
        return AttemptSendOutcome::OAuthInjectFailed;
    }

    // --- Clean body + send upstream ---
    let cleaned_body = request_sanitizer::clean_body(input, prepared);

    let timing = AttemptTiming {
        attempt_started_ms,
        attempt_started: Instant::now(),
    };

    let send_result =
        send::send_upstream(ctx, input.req_method.clone(), url, headers, cleaned_body).await;

    match send_result {
        send::SendResult::Ok(resp) => AttemptSendOutcome::Response(resp, timing),
        send::SendResult::Timeout => AttemptSendOutcome::Timeout(timing),
        send::SendResult::Err(err) => AttemptSendOutcome::ReqwestError(err, timing),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn try_build_url(prepared: &PreparedProvider) -> Result<reqwest::Url, String> {
    build_target_url(
        &prepared.provider_base_url_base,
        &prepared.upstream_forwarded_path,
        prepared.upstream_query.as_deref(),
    )
    .map_err(|e| e.to_string())
}

async fn handle_url_build_failure(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    attempt_ctx: AttemptCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    err: String,
    loop_state: &mut LoopState<'_>,
) -> LoopControl {
    tracing::warn!(
        trace_id = %input.trace_id,
        cli_key = %input.cli_key,
        provider_id = provider_ctx.provider_id,
        provider_name = %provider_ctx.provider_name_base,
        base_url = %provider_ctx.provider_base_url_base,
        "build_target_url failed: {err}"
    );
    let error_code = GatewayErrorCode::InternalError.as_str();
    let decision = FailoverDecision::SwitchProvider;
    let outcome = format!(
        "build_target_url_error: category={} code={} decision={} err={err}",
        ErrorCategory::SystemError.as_str(),
        error_code,
        decision.as_str(),
    );
    record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state: loop_state.reborrow(),
        status: None,
        error_code,
        decision,
        outcome,
        reason: format!("invalid base_url: {err}"),
    })
    .await
}

fn build_attempt_ctx<'a>(
    attempt_index: u32,
    retry_index: u32,
    attempt_started_ms: u128,
    circuit_before: &'a crate::circuit_breaker::CircuitSnapshot,
    prepared: &PreparedProvider,
) -> AttemptCtx<'a> {
    AttemptCtx {
        attempt_index,
        retry_index,
        attempt_started_ms,
        attempt_started: Instant::now(),
        circuit_before,
        gemini_oauth_response_mode: prepared.gemini_oauth_response_mode,
        cx2cc_active: prepared.cx2cc_active,
        anthropic_stream_requested: prepared.anthropic_stream_requested,
    }
}

fn build_provider_ctx(prepared: &PreparedProvider) -> ProviderCtx<'_> {
    ProviderCtx {
        provider_id: prepared.provider_id,
        provider_name_base: &prepared.provider_name_base,
        provider_base_url_base: &prepared.provider_base_url_base,
        provider_index: prepared.provider_index,
        session_reuse: prepared.session_reuse,
        stream_idle_timeout_seconds: prepared.stream_idle_timeout_seconds,
    }
}

fn emit_started_event(
    input: &RequestContext,
    prepared: &PreparedProvider,
    attempt_index: u32,
    retry_index: u32,
    attempt_started_ms: u128,
    circuit_before: &crate::circuit_breaker::CircuitSnapshot,
    abort_guard: &mut RequestAbortGuard,
) {
    let started_attempt = FailoverAttempt {
        provider_id: prepared.provider_id,
        provider_name: prepared.provider_name_base.clone(),
        base_url: prepared.provider_base_url_base.clone(),
        outcome: "started".to_string(),
        status: None,
        provider_index: Some(prepared.provider_index),
        retry_index: Some(retry_index),
        session_reuse: prepared.session_reuse,
        error_category: None,
        error_code: None,
        decision: None,
        reason: None,
        selection_method: dc::selection_method(
            prepared.provider_index,
            retry_index,
            prepared.session_reuse,
        ),
        reason_code: None,
        attempt_started_ms: Some(attempt_started_ms),
        attempt_duration_ms: Some(0),
        circuit_state_before: Some(circuit_before.state.as_str()),
        circuit_state_after: None,
        circuit_failure_count: Some(circuit_before.failure_count),
        circuit_failure_threshold: Some(circuit_before.failure_threshold),
    };
    abort_guard.capture_in_flight_attempt(&started_attempt);
    if input.observe_request {
        emit_attempt_event(
            &input.state.app,
            GatewayAttemptEvent {
                trace_id: input.trace_id.clone(),
                cli_key: input.cli_key.clone(),
                session_id: input.session_id.clone(),
                method: input.method_hint.clone(),
                path: input.forwarded_path.clone(),
                query: input.query.clone(),
                requested_model: input.requested_model.clone(),
                attempt_index,
                provider_id: prepared.provider_id,
                session_reuse: prepared.session_reuse,
                provider_name: prepared.provider_name_base.clone(),
                base_url: prepared.provider_base_url_base.clone(),
                outcome: "started".to_string(),
                status: None,
                attempt_started_ms,
                attempt_duration_ms: 0,
                circuit_state_before: Some(circuit_before.state.as_str()),
                circuit_state_after: None,
                circuit_failure_count: Some(circuit_before.failure_count),
                circuit_failure_threshold: Some(circuit_before.failure_threshold),
            },
        );
    }
}
