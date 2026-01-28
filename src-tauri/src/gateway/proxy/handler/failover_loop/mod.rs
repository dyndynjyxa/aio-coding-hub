//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).

mod claude_model_mapping;
mod context;
mod event_helpers;
mod finalize;
mod provider_gate;
mod send;
mod send_timeout;
mod success_event_stream;
mod success_non_stream;
mod thinking_signature_rectifier_400;
mod upstream_error;

pub(super) use context::FailoverLoopInput;
use event_helpers::{
    emit_attempt_event_and_log, emit_attempt_event_and_log_with_circuit_before,
    AttemptCircuitFields,
};

use super::super::logging::enqueue_request_log_with_backpressure;
use super::super::{
    errors::{classify_upstream_status, error_response},
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
    http_util::{
        build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
        is_event_stream, maybe_gunzip_response_body_bytes_with_limit,
    },
    ErrorCategory, RequestLogEnqueueArgs,
};

use crate::usage;
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use crate::gateway::events::{
    emit_attempt_event, emit_circuit_transition, emit_request_event, FailoverAttempt,
    GatewayAttemptEvent,
};
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, StreamFinalizeCtx,
    TimingOnlyTeeStream, UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, ensure_cli_required_headers, inject_provider_auth,
    now_unix_seconds, strip_hop_headers,
};

use context::{AttemptCtx, CommonCtx, LoopControl, LoopState, ProviderCtx, MAX_NON_SSE_BODY_BYTES};

pub(super) async fn run(input: FailoverLoopInput) -> Response {
    let FailoverLoopInput {
        state,
        cli_key,
        forwarded_path,
        req_method: method,
        method_hint,
        query,
        trace_id,
        started,
        created_at_ms,
        created_at,
        session_id,
        requested_model,
        requested_model_location,
        effective_sort_mode_id,
        providers,
        session_bound_provider_id,
        base_headers,
        body_bytes,
        introspection_json,
        strip_request_content_encoding_seed,
        special_settings,
        provider_base_url_ping_cache_ttl_seconds,
        max_attempts_per_provider,
        max_providers_to_try,
        provider_cooldown_secs,
        upstream_first_byte_timeout_secs,
        upstream_first_byte_timeout,
        upstream_stream_idle_timeout,
        upstream_request_timeout_non_streaming,
        fingerprint_key,
        fingerprint_debug,
        unavailable_fingerprint_key,
        unavailable_fingerprint_debug,
        mut abort_guard,
        enable_thinking_signature_rectifier,
        enable_response_fixer,
        response_fixer_stream_config,
        response_fixer_non_stream_config,
    } = input;

    let introspection_body = body_for_introspection(&base_headers, body_bytes.as_ref());
    let ctx = CommonCtx {
        state: &state,
        cli_key: &cli_key,
        forwarded_path: &forwarded_path,
        method_hint: &method_hint,
        query: &query,
        trace_id: &trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: &session_id,
        requested_model: &requested_model,
        effective_sort_mode_id,
        special_settings: &special_settings,
        provider_cooldown_secs,
        upstream_first_byte_timeout_secs,
        upstream_first_byte_timeout,
        upstream_stream_idle_timeout,
        upstream_request_timeout_non_streaming,
        max_attempts_per_provider,
        enable_response_fixer,
        response_fixer_stream_config,
        response_fixer_non_stream_config,
        introspection_body: introspection_body.as_ref(),
    };
    let mut attempts: Vec<FailoverAttempt> = Vec::new();
    let mut failed_provider_ids: HashSet<i64> = HashSet::new();
    let mut last_error_category: Option<&'static str> = None;
    let mut last_error_code: Option<&'static str> = None;

    let max_providers_to_try = (max_providers_to_try as usize).max(1);
    let mut providers_tried: usize = 0;
    let mut earliest_available_unix: Option<i64> = None;
    let mut skipped_open: usize = 0;
    let mut skipped_cooldown: usize = 0;

    for provider in providers.iter() {
        if providers_tried >= max_providers_to_try {
            break;
        }

        let provider_id = provider.id;
        let provider_name_base = if provider.name.trim().is_empty() {
            format!("Provider #{} (auto-fixed)", provider.id)
        } else {
            provider.name.clone()
        };
        let provider_base_url_display = provider
            .base_urls
            .first()
            .cloned()
            .unwrap_or_else(String::new);

        if failed_provider_ids.contains(&provider_id) {
            continue;
        }

        let Some(gate_allow) = provider_gate::gate_provider(provider_gate::ProviderGateInput {
            ctx,
            provider_id,
            provider_name_base: &provider_name_base,
            provider_base_url_display: &provider_base_url_display,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
        }) else {
            continue;
        };

        // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

        let provider_base_url_base = select_provider_base_url_for_request(
            &state,
            provider,
            provider_base_url_ping_cache_ttl_seconds,
        )
        .await;

        let mut circuit_snapshot = gate_allow.circuit_after;

        providers_tried = providers_tried.saturating_add(1);
        let provider_index = providers_tried as u32;
        let session_reuse = match session_bound_provider_id {
            Some(id) => (id == provider_id && provider_index == 1).then_some(true),
            None => None,
        };
        let provider_ctx = ProviderCtx {
            provider_id,
            provider_name_base: &provider_name_base,
            provider_base_url_base: &provider_base_url_base,
            provider_index,
            session_reuse,
        };

        let mut upstream_forwarded_path = forwarded_path.clone();
        let mut upstream_query = query.clone();
        let mut upstream_body_bytes = body_bytes.clone();
        let mut strip_request_content_encoding = strip_request_content_encoding_seed;
        let mut thinking_signature_rectifier_retried = false;

        claude_model_mapping::apply_if_needed(
            ctx,
            provider,
            provider_ctx,
            requested_model_location,
            introspection_json.as_ref(),
            claude_model_mapping::UpstreamRequestMut {
                forwarded_path: &mut upstream_forwarded_path,
                query: &mut upstream_query,
                body_bytes: &mut upstream_body_bytes,
                strip_request_content_encoding: &mut strip_request_content_encoding,
            },
        );

        for retry_index in 1..=max_attempts_per_provider {
            let attempt_index = attempts.len().saturating_add(1) as u32;
            let attempt_started_ms = started.elapsed().as_millis();
            let attempt_started = Instant::now();
            let circuit_before = circuit_snapshot.clone();
            let attempt_ctx = AttemptCtx {
                attempt_index,
                retry_index,
                attempt_started_ms,
                attempt_started,
                circuit_before: &circuit_before,
            };

            let url = match build_target_url(
                &provider_base_url_base,
                &upstream_forwarded_path,
                upstream_query.as_deref(),
            ) {
                Ok(u) => u,
                Err(err) => {
                    let category = ErrorCategory::SystemError;
                    let error_code = "GW_INTERNAL_ERROR";
                    let decision = FailoverDecision::SwitchProvider;

                    let outcome = format!(
                        "build_target_url_error: category={} code={} decision={} err={err}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
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
                        reason: Some("invalid base_url".to_string()),
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
                        None,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    failed_provider_ids.insert(provider_id);
                    break;
                }
            };

            // Realtime routing UX: emit an attempt event as soon as a provider is selected (before awaiting upstream).
            // This enables the Home page to display the current routed provider immediately, similar to claude-code-hub.
            //
            // Note: do NOT enqueue attempt_logs for this "started" event (avoid DB noise/IO); completion events still get persisted.
            emit_attempt_event(
                &state.app,
                GatewayAttemptEvent {
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

            let mut headers = base_headers.clone();
            ensure_cli_required_headers(&cli_key, &mut headers);

            // Always override auth headers to avoid leaking any official OAuth tokens to a third-party relay base_url.
            inject_provider_auth(&cli_key, provider.api_key_plaintext.trim(), &mut headers);
            if strip_request_content_encoding {
                headers.remove(header::CONTENT_ENCODING);
            }

            let send_result = send::send_upstream(
                ctx,
                method.clone(),
                url,
                headers,
                upstream_body_bytes.clone(),
            )
            .await;

            match send_result {
                send::SendResult::Ok(resp) => {
                    let status = resp.status();
                    let response_headers = resp.headers().clone();

                    if status.is_success() {
                        if is_event_stream(&response_headers) {
                            let loop_state = LoopState {
                                attempts: &mut attempts,
                                failed_provider_ids: &mut failed_provider_ids,
                                last_error_category: &mut last_error_category,
                                last_error_code: &mut last_error_code,
                                circuit_snapshot: &mut circuit_snapshot,
                                abort_guard: &mut abort_guard,
                            };
                            match success_event_stream::handle_success_event_stream(
                                ctx,
                                provider_ctx,
                                attempt_ctx,
                                loop_state,
                                resp,
                                status,
                                response_headers,
                            )
                            .await
                            {
                                LoopControl::ContinueRetry => continue,
                                LoopControl::BreakRetry => break,
                                LoopControl::Return(resp) => return resp,
                            }
                        }

                        let loop_state = LoopState {
                            attempts: &mut attempts,
                            failed_provider_ids: &mut failed_provider_ids,
                            last_error_category: &mut last_error_category,
                            last_error_code: &mut last_error_code,
                            circuit_snapshot: &mut circuit_snapshot,
                            abort_guard: &mut abort_guard,
                        };
                        match success_non_stream::handle_success_non_stream(
                            ctx,
                            provider_ctx,
                            attempt_ctx,
                            loop_state,
                            resp,
                            status,
                            response_headers,
                        )
                        .await
                        {
                            LoopControl::ContinueRetry => continue,
                            LoopControl::BreakRetry => break,
                            LoopControl::Return(resp) => return resp,
                        }
                    }

                    let loop_state = LoopState {
                        attempts: &mut attempts,
                        failed_provider_ids: &mut failed_provider_ids,
                        last_error_category: &mut last_error_category,
                        last_error_code: &mut last_error_code,
                        circuit_snapshot: &mut circuit_snapshot,
                        abort_guard: &mut abort_guard,
                    };
                    match upstream_error::handle_non_success_response(
                        ctx,
                        provider_ctx,
                        attempt_ctx,
                        loop_state,
                        enable_thinking_signature_rectifier,
                        resp,
                        upstream_error::UpstreamRequestState {
                            upstream_body_bytes: &mut upstream_body_bytes,
                            strip_request_content_encoding: &mut strip_request_content_encoding,
                            thinking_signature_rectifier_retried:
                                &mut thinking_signature_rectifier_retried,
                        },
                    )
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
                send::SendResult::Timeout => {
                    let loop_state = LoopState {
                        attempts: &mut attempts,
                        failed_provider_ids: &mut failed_provider_ids,
                        last_error_category: &mut last_error_category,
                        last_error_code: &mut last_error_code,
                        circuit_snapshot: &mut circuit_snapshot,
                        abort_guard: &mut abort_guard,
                    };
                    match send_timeout::handle_timeout(ctx, provider_ctx, attempt_ctx, loop_state)
                        .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
                send::SendResult::Err(err) => {
                    let loop_state = LoopState {
                        attempts: &mut attempts,
                        failed_provider_ids: &mut failed_provider_ids,
                        last_error_category: &mut last_error_category,
                        last_error_code: &mut last_error_code,
                        circuit_snapshot: &mut circuit_snapshot,
                        abort_guard: &mut abort_guard,
                    };
                    match upstream_error::handle_reqwest_error(
                        ctx,
                        provider_ctx,
                        attempt_ctx,
                        loop_state,
                        err,
                    )
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
            }
        }
    }

    if attempts.is_empty() && !providers.is_empty() {
        return finalize::all_providers_unavailable(finalize::AllUnavailableInput {
            state: &state,
            abort_guard: &mut abort_guard,
            cli_key,
            method_hint,
            forwarded_path,
            query,
            trace_id,
            started,
            created_at_ms,
            created_at,
            session_id,
            requested_model,
            special_settings,
            earliest_available_unix,
            skipped_open,
            skipped_cooldown,
            fingerprint_key,
            fingerprint_debug,
            unavailable_fingerprint_key,
            unavailable_fingerprint_debug,
        })
        .await;
    }

    finalize::all_providers_failed(finalize::AllFailedInput {
        state: &state,
        abort_guard: &mut abort_guard,
        attempts,
        last_error_category,
        last_error_code,
        cli_key,
        method_hint,
        forwarded_path,
        query,
        trace_id,
        started,
        created_at_ms,
        created_at,
        session_id,
        requested_model,
        special_settings,
    })
    .await
}
