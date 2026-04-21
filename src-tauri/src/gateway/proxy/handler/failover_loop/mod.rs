//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).

mod attempt_auth;
mod attempt_executor;
mod attempt_record;
mod claude_metadata_user_id_injection;
mod claude_model_mapping;
mod codex_chatgpt;
mod codex_session_id_completion;
mod context;
mod cx2cc_preparation;
mod event_helpers;
mod finalize;
mod loop_helpers;
mod oauth;
mod provider_checks;
mod provider_gate;
mod provider_iterator;
mod provider_limits;
mod request_end_helpers;
mod request_sanitizer;
mod response_router;
mod retry_engine;
mod send;
mod send_timeout;
mod success_event_stream;
mod success_non_stream;
mod thinking_signature_rectifier_400;
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
    emit_request_event_and_enqueue_request_log, RequestEndArgs, RequestEndDeps,
};

use crate::gateway::proxy::{
    errors::{classify_upstream_status, error_response},
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
    gemini_oauth,
    http_util::{
        build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
        is_event_stream, maybe_gunzip_response_body_bytes_with_limit,
    },
    mark_internal_forwarded_request, ErrorCategory, GatewayErrorCode,
};

use crate::usage;
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gateway::events::{
    decision_chain as dc, emit_attempt_event, emit_gateway_log, FailoverAttempt,
    GatewayAttemptEvent,
};
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, TimingOnlyTeeStream,
    UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, clear_all_auth_headers, ensure_cli_required_headers,
    inject_provider_auth, now_unix_seconds, strip_hop_headers,
};
use crate::shared::mutex_ext::MutexExt;

use context::{
    build_stream_finalize_ctx, AttemptCtx, CommonCtx, CommonCtxArgs, CommonCtxOwned, LoopControl,
    LoopState, ProviderCtx, ProviderCtxOwned, MAX_NON_SSE_BODY_BYTES,
};

/// Main failover loop: iterate providers, retry attempts, handle responses.
///
/// This is a thin orchestrator that delegates to:
/// - `provider_iterator` for provider preparation (gate, credential, CX2CC)
/// - `retry_engine` for the per-provider retry loop
/// - `finalize` for terminal states (all unavailable / all failed)
pub(super) async fn run(mut input: RequestContext) -> Response {
    let started = input.started;
    let created_at_ms = input.created_at_ms;
    let created_at = input.created_at;

    let mut abort_guard = input.abort_guard.take();

    let introspection_body =
        body_for_introspection(&input.base_headers, input.body_bytes.as_ref()).into_owned();
    let ctx = CommonCtx::from(CommonCtxArgs {
        state: &input.state,
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

    let mut attempts: Vec<FailoverAttempt> = Vec::new();
    let mut failed_provider_ids: HashSet<i64> = HashSet::new();
    let mut last_error_category: Option<&'static str> = None;
    let mut last_error_code: Option<&'static str> = None;

    let max_providers_to_try = (input.max_providers_to_try as usize).max(1);
    let mut counters = provider_iterator::IterationCounters::new();
    let anthropic_stream_requested =
        original_anthropic_stream_requested(input.introspection_json.as_ref());

    let providers: Vec<_> = input.providers.clone();

    for provider in providers.iter() {
        if counters.providers_tried >= max_providers_to_try {
            break;
        }

        let preparation = provider_iterator::prepare_provider(
            ctx,
            &input,
            provider,
            &mut counters,
            &mut attempts,
            &failed_provider_ids,
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
                &mut attempts,
                &mut failed_provider_ids,
                &mut last_error_category,
                &mut last_error_code,
                &mut circuit_snapshot,
                &mut abort_guard,
            ),
        )
        .await
        {
            return resp;
        }
    }

    // --- Finalization ---
    if should_finalize_as_all_providers_unavailable(&attempts) && !input.providers.is_empty() {
        let owned = finalize_owned_from_input(&input);
        return finalize::all_providers_unavailable(finalize::AllUnavailableInput {
            state: &input.state,
            abort_guard: &mut abort_guard,
            observe: input.observe_request,
            attempts,
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
        attempts,
        last_error_category,
        last_error_code,
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
