//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).

mod attempt_record;
mod claude_metadata_user_id_injection;
mod claude_model_mapping;
mod context;
mod event_helpers;
mod finalize;
mod provider_gate;
mod provider_limits;
mod request_end_helpers;
mod send;
mod send_timeout;
mod success_event_stream;
mod success_non_stream;
mod thinking_signature_rectifier_400;
mod upstream_error;

use super::super::request_context::RequestContext;
use attempt_record::{
    record_system_failure_and_decide, record_system_failure_and_decide_no_cooldown,
    RecordSystemFailureArgs,
};
use event_helpers::{
    emit_attempt_event_and_log, emit_attempt_event_and_log_with_circuit_before,
    AttemptCircuitFields,
};
use request_end_helpers::{
    emit_request_event_and_enqueue_request_log, RequestEndArgs, RequestEndDeps,
};

use super::super::{
    errors::{classify_upstream_status, error_response},
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
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
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gateway::events::{
    decision_chain as dc, emit_attempt_event, FailoverAttempt, GatewayAttemptEvent,
};
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, TimingOnlyTeeStream,
    UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, ensure_cli_required_headers, inject_provider_auth,
    now_unix_seconds, strip_hop_headers,
};

use context::{
    build_stream_finalize_ctx, AttemptCtx, CommonCtx, CommonCtxArgs, CommonCtxOwned, LoopControl,
    LoopState, ProviderCtx, ProviderCtxOwned, MAX_NON_SSE_BODY_BYTES,
};

struct FinalizeOwnedCommon {
    cli_key: String,
    method_hint: String,
    forwarded_path: String,
    query: Option<String>,
    trace_id: String,
    session_id: Option<String>,
    requested_model: Option<String>,
    special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
}

fn finalize_owned_from_input(input: &RequestContext) -> FinalizeOwnedCommon {
    FinalizeOwnedCommon {
        cli_key: input.cli_key.clone(),
        method_hint: input.method_hint.clone(),
        forwarded_path: input.forwarded_path.clone(),
        query: input.query.clone(),
        trace_id: input.trace_id.clone(),
        session_id: input.session_id.clone(),
        requested_model: input.requested_model.clone(),
        special_settings: input.special_settings.clone(),
    }
}

pub(super) async fn run(mut input: RequestContext) -> Response {
    let method = input.req_method.clone();
    let started = input.started;
    let created_at_ms = input.created_at_ms;
    let created_at = input.created_at;

    let introspection_body = body_for_introspection(&input.base_headers, input.body_bytes.as_ref());
    let ctx = CommonCtx::from(CommonCtxArgs {
        state: &input.state,
        cli_key: &input.cli_key,
        forwarded_path: &input.forwarded_path,
        method_hint: &input.method_hint,
        query: &input.query,
        trace_id: &input.trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: &input.session_id,
        requested_model: &input.requested_model,
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
    let mut providers_tried: usize = 0;
    let mut earliest_available_unix: Option<i64> = None;
    let mut skipped_open: usize = 0;
    let mut skipped_cooldown: usize = 0;
    let mut skipped_limits: usize = 0;

    for provider in input.providers.iter() {
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

        let skipped_open_before = skipped_open;
        let skipped_cooldown_before = skipped_cooldown;
        let Some(gate_allow) = provider_gate::gate_provider(provider_gate::ProviderGateInput {
            ctx,
            provider_id,
            provider_name_base: &provider_name_base,
            provider_base_url_display: &provider_base_url_display,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
        }) else {
            let (reason_code, reason_label) = if skipped_open > skipped_open_before {
                (Some(dc::REASON_CIRCUIT_OPEN), "open")
            } else if skipped_cooldown > skipped_cooldown_before {
                (Some(dc::REASON_CIRCUIT_COOLDOWN), "cooldown")
            } else {
                (None, "unknown")
            };

            // Record skipped provider (circuit breaker gate)
            attempts.push(FailoverAttempt {
                provider_id,
                provider_name: provider_name_base.clone(),
                base_url: provider_base_url_display.clone(),
                outcome: "skipped".to_string(),
                status: None,
                provider_index: None,
                retry_index: None,
                session_reuse: None,
                error_category: Some("circuit_breaker"),
                error_code: Some(GatewayErrorCode::ProviderCircuitOpen.as_str()),
                decision: Some("skip"),
                reason: Some(format!(
                    "provider skipped by circuit breaker ({reason_label})"
                )),
                selection_method: Some(dc::SELECTION_METHOD_FILTERED),
                reason_code,
                attempt_started_ms: Some(started.elapsed().as_millis()),
                attempt_duration_ms: Some(0),
                circuit_state_before: None,
                circuit_state_after: None,
                circuit_failure_count: None,
                circuit_failure_threshold: None,
            });
            continue;
        };

        if !provider_limits::gate_provider(provider_limits::ProviderLimitsInput {
            ctx,
            provider,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_limits: &mut skipped_limits,
        }) {
            // Record skipped provider (rate limit gate)
            attempts.push(FailoverAttempt {
                provider_id,
                provider_name: provider_name_base.clone(),
                base_url: provider_base_url_display.clone(),
                outcome: "skipped".to_string(),
                status: None,
                provider_index: None,
                retry_index: None,
                session_reuse: None,
                error_category: Some("rate_limit"),
                error_code: Some(GatewayErrorCode::ProviderRateLimited.as_str()),
                decision: Some("skip"),
                reason: Some("provider skipped by rate limit".to_string()),
                selection_method: Some(dc::SELECTION_METHOD_FILTERED),
                reason_code: Some(dc::REASON_RATE_LIMITED),
                attempt_started_ms: Some(started.elapsed().as_millis()),
                attempt_duration_ms: Some(0),
                circuit_state_before: None,
                circuit_state_after: None,
                circuit_failure_count: None,
                circuit_failure_threshold: None,
            });
            continue;
        }

        // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

        let provider_base_url_base = select_provider_base_url_for_request(
            &input.state,
            provider,
            input.provider_base_url_ping_cache_ttl_seconds,
        )
        .await;

        let mut circuit_snapshot = gate_allow.circuit_after;

        providers_tried = providers_tried.saturating_add(1);
        let provider_index = providers_tried as u32;
        let session_reuse = match input.session_bound_provider_id {
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

        let mut upstream_forwarded_path = input.forwarded_path.clone();
        let mut upstream_query = input.query.clone();
        let mut upstream_body_bytes = input.body_bytes.clone();
        let mut strip_request_content_encoding = input.strip_request_content_encoding_seed;
        let mut thinking_signature_rectifier_retried = false;
        let mut thinking_budget_rectifier_retried = false;

        claude_model_mapping::apply_if_needed(
            ctx,
            provider,
            provider_ctx,
            input.requested_model_location,
            input.introspection_json.as_ref(),
            claude_model_mapping::UpstreamRequestMut {
                forwarded_path: &mut upstream_forwarded_path,
                query: &mut upstream_query,
                body_bytes: &mut upstream_body_bytes,
                strip_request_content_encoding: &mut strip_request_content_encoding,
            },
        );

        claude_metadata_user_id_injection::apply_if_needed(
            claude_metadata_user_id_injection::ApplyClaudeMetadataUserIdInjectionInput {
                ctx,
                provider_id,
                enabled: input.enable_claude_metadata_user_id_injection,
                session_id: input.session_id.as_deref(),
                base_headers: &input.base_headers,
                forwarded_path: upstream_forwarded_path.as_str(),
                upstream_body_bytes: &mut upstream_body_bytes,
                strip_request_content_encoding: &mut strip_request_content_encoding,
            },
        );

        for retry_index in 1..=input.max_attempts_per_provider {
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
                    let error_code = GatewayErrorCode::InternalError.as_str();
                    let decision = FailoverDecision::SwitchProvider;

                    let outcome = format!(
                        "build_target_url_error: category={} code={} decision={} err={err}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
                    );
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
                        ctx,
                        provider_ctx,
                        attempt_ctx,
                        loop_state,
                        status: None,
                        error_code,
                        decision,
                        outcome,
                        reason: format!("invalid base_url: {err}"),
                    })
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
            };

            // Realtime routing UX: emit an attempt event as soon as a provider is selected (before awaiting upstream).
            //
            // Note: do NOT enqueue attempt_logs for this "started" event (avoid DB noise/IO); completion events still get persisted.
            emit_attempt_event(
                &input.state.app,
                GatewayAttemptEvent {
                    trace_id: input.trace_id.clone(),
                    cli_key: input.cli_key.clone(),
                    method: input.method_hint.clone(),
                    path: input.forwarded_path.clone(),
                    query: input.query.clone(),
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

            let mut headers = input.base_headers.clone();
            ensure_cli_required_headers(&input.cli_key, &mut headers);

            // Always override auth headers to avoid leaking any official OAuth tokens to a third-party relay base_url.
            let effective_auth_key = match provider.auth_type {
                crate::providers::AuthType::Oauth => provider
                    .oauth_access_token
                    .as_deref()
                    .unwrap_or("")
                    .to_string(),
                crate::providers::AuthType::ApiKey => provider.api_key_plaintext.clone(),
            };
            inject_provider_auth(&input.cli_key, effective_auth_key.trim(), &mut headers);
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
                            let loop_state = LoopState::new(
                                &mut attempts,
                                &mut failed_provider_ids,
                                &mut last_error_category,
                                &mut last_error_code,
                                &mut circuit_snapshot,
                                &mut input.abort_guard,
                            );
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

                        let loop_state = LoopState::new(
                            &mut attempts,
                            &mut failed_provider_ids,
                            &mut last_error_category,
                            &mut last_error_code,
                            &mut circuit_snapshot,
                            &mut input.abort_guard,
                        );
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

                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match upstream_error::handle_non_success_response(
                        upstream_error::HandleNonSuccessResponseInput {
                            ctx,
                            provider_ctx,
                            attempt_ctx,
                            loop_state,
                            enable_thinking_signature_rectifier: input
                                .enable_thinking_signature_rectifier,
                            enable_thinking_budget_rectifier: input
                                .enable_thinking_budget_rectifier,
                            resp,
                            upstream: upstream_error::UpstreamRequestState {
                                upstream_body_bytes: &mut upstream_body_bytes,
                                strip_request_content_encoding: &mut strip_request_content_encoding,
                                thinking_signature_rectifier_retried:
                                    &mut thinking_signature_rectifier_retried,
                                thinking_budget_rectifier_retried:
                                    &mut thinking_budget_rectifier_retried,
                            },
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
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match send_timeout::handle_timeout(ctx, provider_ctx, attempt_ctx, loop_state)
                        .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
                send::SendResult::Err(err) => {
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
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

    if attempts.is_empty() && !input.providers.is_empty() {
        let owned = finalize_owned_from_input(&input);
        return finalize::all_providers_unavailable(finalize::AllUnavailableInput {
            state: &input.state,
            abort_guard: &mut input.abort_guard,
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
            earliest_available_unix,
            skipped_open,
            skipped_cooldown,
            skipped_limits,
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
        abort_guard: &mut input.abort_guard,
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
