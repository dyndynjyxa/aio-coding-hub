//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).
//!
//! Submodules are organised into physical subdirectories by responsibility while
//! staying flat in the Rust module tree (via `#[path]`) so that existing
//! `use super::` imports inside each file continue to resolve against
//! `failover_loop` itself.
//!
//! - `prepare/`  — provider selection, gating, credential resolution, protocol bridging
//! - `attempt/`  — single-attempt execution, auth injection, retry decisions
//! - `response/` — response routing, stream/non-stream handling, error/finalize

// --- shared (stay in root) ---
mod context;
mod event_helpers;
mod loop_helpers;
mod request_end_helpers;

// --- prepare/ : provider selection & request shaping ---
#[path = "prepare/claude_metadata_user_id_injection.rs"]
mod claude_metadata_user_id_injection;
#[path = "prepare/claude_model_mapping.rs"]
mod claude_model_mapping;
#[path = "prepare/codex_chatgpt.rs"]
mod codex_chatgpt;
#[path = "prepare/codex_service_tier.rs"]
mod codex_service_tier;
#[path = "prepare/codex_session_id_completion.rs"]
mod codex_session_id_completion;
#[path = "prepare/cx2cc_preparation.rs"]
mod cx2cc_preparation;
#[path = "prepare/oauth.rs"]
mod oauth;
#[path = "prepare/provider_checks.rs"]
mod provider_checks;
#[path = "prepare/provider_gate.rs"]
mod provider_gate;
#[path = "prepare/provider_iterator.rs"]
mod provider_iterator;
#[path = "prepare/provider_limits.rs"]
mod provider_limits;
#[path = "prepare/request_sanitizer.rs"]
mod request_sanitizer;

// --- attempt/ : single-attempt execution & retry ---
#[path = "attempt/attempt_auth.rs"]
mod attempt_auth;
#[path = "attempt/attempt_executor.rs"]
mod attempt_executor;
#[path = "attempt/attempt_record.rs"]
mod attempt_record;
#[path = "attempt/retry_engine.rs"]
mod retry_engine;
#[path = "attempt/send.rs"]
mod send;
#[path = "attempt/send_timeout.rs"]
mod send_timeout;

// --- response/ : upstream response handling & finalization ---
#[path = "response/finalize.rs"]
mod finalize;
#[path = "response/response_router.rs"]
mod response_router;
#[path = "response/success_event_stream.rs"]
mod success_event_stream;
#[path = "response/success_non_stream.rs"]
mod success_non_stream;
#[path = "response/thinking_signature_rectifier_400.rs"]
mod thinking_signature_rectifier_400;
#[path = "response/upstream_error.rs"]
mod upstream_error;

use crate::gateway::proxy::request_context::RequestContext;
use attempt_record::{
    record_system_failure_and_decide, record_system_failure_and_decide_no_cooldown,
    RecordSystemFailureArgs,
};
use codex_chatgpt::{
    is_codex_chatgpt_backend, maybe_apply_codex_chatgpt_request_compat,
    maybe_inject_codex_chatgpt_headers, original_anthropic_stream_requested,
    parse_codex_chatgpt_account_id, should_apply_claude_model_mapping,
    strip_incompatible_protocol_headers,
};
use event_helpers::{
    emit_attempt_event_and_log, emit_attempt_event_and_log_with_circuit_before,
    AttemptCircuitFields,
};
use loop_helpers::{
    apply_cx2cc_request_settings, finalize_owned_from_input, push_skipped_provider_attempt,
    should_finalize_as_all_providers_unavailable, SkippedProviderAttempt,
};
use oauth::{
    refresh_oauth_credential_after_401, resolve_effective_credential,
    resolve_oauth_adapter_for_provider,
};
use request_end_helpers::{
    emit_request_event_and_enqueue_request_log, RequestCompletion, RequestEndArgs,
    RequestEndContextArgs, RequestEndDeps,
};

use crate::gateway::proxy::{
    errors::{classify_upstream_status, error_response},
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
    gemini_oauth,
    http_util::{
        build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
        is_event_stream, maybe_gunzip_response_body_bytes_with_limit,
    },
    ErrorCategory, GatewayErrorCode,
};

use crate::usage;
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::gateway::events::{
    bound_attempt_event, decision_chain as dc, emit_attempt_event, emit_gateway_debug_log_lazy,
    emit_gateway_log, FailoverAttempt, GatewayAttemptEvent,
};
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, MaybePluginChunkStream,
    TimingOnlyTeeStream, UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, clear_all_auth_headers, ensure_cli_required_headers,
    inject_provider_auth, lossy_utf8_preview, now_unix_seconds, redacted_headers_for_debug,
    strip_hop_headers, MAX_DEBUG_BODY_PREVIEW_BYTES,
};

use context::{
    build_stream_finalize_ctx, AttemptCtx, AttemptOutcome, CommonCtx, CommonCtxArgs,
    CommonCtxOwned, FailoverRunState, LoopControl, LoopState, ProviderCtx, ProviderCtxOwned,
    MAX_NON_SSE_BODY_BYTES,
};

/// Fallback stream detection from raw body bytes when introspection_json
/// parsing failed (e.g. gzip decompression exceeded limit). Looks for the
/// `"stream":true` pattern in the first 2 KB of the body.
fn stream_flag_from_raw_body(body: &[u8]) -> bool {
    let search_window = &body[..body.len().min(2048)];
    let haystack = match std::str::from_utf8(search_window) {
        Ok(s) => s,
        Err(_) => return false,
    };
    haystack.contains("\"stream\":true") || haystack.contains("\"stream\": true")
}

/// Backoff between full provider sweeps in router mode. Short enough to feel
/// aggressive ("keep hammering until it connects") while preventing a hot spin
/// against providers that fail instantly (e.g. connection refused).
const ROUTER_MODE_ROUND_BACKOFF: Duration = Duration::from_millis(500);

/// Main failover loop: iterate providers, retry attempts, handle responses.
///
/// This is a thin orchestrator that delegates to:
/// - `provider_iterator` for provider preparation (gate, credential, CX2CC)
/// - `retry_engine` for the per-provider retry loop
/// - `finalize` for terminal states (all unavailable / all failed)
pub(super) async fn run<R>(mut input: RequestContext<R>) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let started = input.started;
    let created_at_ms = input.created_at_ms;
    let created_at = input.created_at;

    let mut abort_guard = input.abort_guard.take();

    let introspection_body =
        body_for_introspection(&input.base_headers, input.body_bytes.as_ref()).into_owned();
    let ctx = CommonCtx::from(CommonCtxArgs {
        state: &input.state,
        router_mode: input.router_mode_enabled,
        cli_key: &input.cli_key,
        forwarded_path: &input.forwarded_path,
        observe: input.observe_request,
        method_hint: &input.method_hint,
        query: &input.query,
        trace_id: &input.trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: &input.session_id,
        requested_model: &input.requested_model,
        cx2cc_settings: &input.cx2cc_settings,
        effective_sort_mode_id: input.effective_sort_mode_id,
        special_settings: &input.special_settings,
        provider_health_neutral: input.provider_health_neutral,
        provider_cooldown_secs: input.provider_cooldown_secs,
        upstream_first_byte_timeout_secs: input.upstream_first_byte_timeout_secs,
        upstream_first_byte_timeout: input.upstream_first_byte_timeout,
        upstream_stream_idle_timeout: input.upstream_stream_idle_timeout,
        upstream_request_timeout_non_streaming: input.upstream_request_timeout_non_streaming,
        verbose_provider_error: input.verbose_provider_error,
        max_attempts_per_provider: input.max_attempts_per_provider,
        enable_response_fixer: input.enable_response_fixer,
        response_fixer_stream_config: input.response_fixer_stream_config,
        response_fixer_non_stream_config: input.response_fixer_non_stream_config,
        introspection_body: introspection_body.as_ref(),
    });

    let mut run_state = FailoverRunState::new();

    let providers: Vec<_> = input.providers.clone();

    // Router mode keeps sweeping the whole provider list until one connects,
    // ignoring both the per-request provider cap and the circuit breaker
    // (the gate bypass lives in `provider_router::gate_provider`). It is bounded
    // by `router_mode_max_rounds` full sweeps with a short backoff between them.
    let router_mode = input.router_mode_enabled && !providers.is_empty();
    let max_providers_to_try = if router_mode {
        usize::MAX
    } else {
        (input.max_providers_to_try as usize).max(1)
    };
    let max_rounds: u32 = if router_mode {
        input.router_mode_max_rounds.max(1)
    } else {
        1
    };

    let mut counters = provider_iterator::IterationCounters::new();
    let anthropic_stream_requested =
        original_anthropic_stream_requested(input.introspection_json.as_ref())
            || stream_flag_from_raw_body(&introspection_body);

    let mut round: u32 = 0;
    loop {
        round = round.saturating_add(1);

        // Each router-mode round re-tries every provider from scratch, so reset
        // the per-round skip counters and the "already failed this pass" set.
        if router_mode {
            counters = provider_iterator::IterationCounters::new();
            run_state.failed_provider_ids.clear();
        }

        for provider in providers.iter() {
            if counters.providers_tried >= max_providers_to_try {
                break;
            }

            let preparation = provider_iterator::prepare_provider(
                ctx,
                &input,
                provider,
                &mut counters,
                &mut run_state.attempts,
                &run_state.failed_provider_ids,
                anthropic_stream_requested,
            )
            .await;

            let mut prepared = match preparation {
                provider_iterator::PreparationOutcome::Ready(p) => *p,
                provider_iterator::PreparationOutcome::Skipped => continue,
            };

            let mut circuit_snapshot = prepared.circuit_snapshot.clone();

            if let Some(resp) = retry_engine::run_retry_loop(
                ctx,
                &input,
                &mut prepared,
                LoopState::new(
                    &mut run_state.attempts,
                    &mut run_state.failed_provider_ids,
                    &mut run_state.last_outcome,
                    &mut circuit_snapshot,
                    &mut abort_guard,
                ),
            )
            .await
            {
                return resp;
            }
        }

        if !router_mode || round >= max_rounds {
            break;
        }

        // Bound the attempts log across many rounds, then back off before the
        // next full sweep so we do not hot-spin against instantly-failing
        // providers. A client disconnect cancels this future during the sleep.
        loop_helpers::trim_router_mode_attempts(&mut run_state.attempts);
        tokio::time::sleep(ROUTER_MODE_ROUND_BACKOFF).await;
    }

    // --- Finalization ---
    if should_finalize_as_all_providers_unavailable(&run_state.attempts)
        && !input.providers.is_empty()
    {
        let owned = finalize_owned_from_input(&input);
        return finalize::all_providers_unavailable(finalize::AllUnavailableInput {
            state: &input.state,
            abort_guard: &mut abort_guard,
            observe: input.observe_request,
            attempts: std::mem::take(&mut run_state.attempts),
            cli_key: owned.cli_key,
            method_hint: owned.method_hint,
            forwarded_path: owned.forwarded_path,
            query: owned.query,
            trace_id: owned.trace_id,
            started,
            created_at_ms,
            created_at,
            session_id: owned.session_id,
            requested_model: owned.requested_model,
            special_settings: owned.special_settings,
            verbose_provider_error: input.verbose_provider_error,
            earliest_available_unix: counters.earliest_available_unix,
            skipped_open: counters.skipped_open,
            skipped_cooldown: counters.skipped_cooldown,
            skipped_limits: counters.skipped_limits,
            fingerprint_key: input.fingerprint_key,
            fingerprint_debug: input.fingerprint_debug.clone(),
            unavailable_fingerprint_key: input.unavailable_fingerprint_key,
            unavailable_fingerprint_debug: input.unavailable_fingerprint_debug.clone(),
        })
        .await;
    }

    let owned = finalize_owned_from_input(&input);
    finalize::all_providers_failed(finalize::AllFailedInput {
        state: &input.state,
        abort_guard: &mut abort_guard,
        observe: input.observe_request,
        attempts: std::mem::take(&mut run_state.attempts),
        last_outcome: run_state.last_outcome,
        cli_key: owned.cli_key,
        method_hint: owned.method_hint,
        forwarded_path: owned.forwarded_path,
        query: owned.query,
        trace_id: owned.trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: owned.session_id,
        requested_model: owned.requested_model,
        special_settings: owned.special_settings,
        verbose_provider_error: input.verbose_provider_error,
    })
    .await
}

#[cfg(test)]
mod tests;
