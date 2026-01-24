//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).

mod send_timeout;
mod success_event_stream;
mod success_non_stream;
mod thinking_signature_rectifier_400;

use super::super::abort_guard::RequestAbortGuard;
use super::super::caches::CachedGatewayError;
use super::super::logging::{
    enqueue_attempt_log_with_backpressure, enqueue_request_log_with_backpressure,
};
use super::super::{
    errors::{
        classify_reqwest_error, classify_upstream_status, error_response,
        error_response_with_retry_after,
    },
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
    http_util::{
        build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
        is_event_stream, maybe_gunzip_response_body_bytes_with_limit,
    },
    model_rewrite::{replace_model_in_body_json, replace_model_in_path, replace_model_in_query},
    ErrorCategory, RequestLogEnqueueArgs,
};

use crate::{circuit_breaker, providers, usage};
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::gateway::events::{
    emit_attempt_event, emit_circuit_event, emit_circuit_transition, emit_request_event,
    FailoverAttempt, GatewayAttemptEvent, GatewayCircuitEvent,
};
use crate::gateway::manager::GatewayAppState;
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, StreamFinalizeCtx,
    TimingOnlyTeeStream, UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, ensure_cli_required_headers, inject_provider_auth,
    now_unix_seconds, strip_hop_headers, RequestedModelLocation,
};

const MAX_NON_SSE_BODY_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Copy)]
struct CommonCtx<'a> {
    state: &'a GatewayAppState,
    cli_key: &'a String,
    forwarded_path: &'a String,
    method_hint: &'a String,
    query: &'a Option<String>,
    trace_id: &'a String,
    started: Instant,
    created_at_ms: i64,
    created_at: i64,
    session_id: &'a Option<String>,
    requested_model: &'a Option<String>,
    effective_sort_mode_id: Option<i64>,
    special_settings: &'a Arc<Mutex<Vec<serde_json::Value>>>,
    provider_cooldown_secs: i64,
    upstream_first_byte_timeout_secs: u32,
    upstream_first_byte_timeout: Option<Duration>,
    upstream_stream_idle_timeout: Option<Duration>,
    upstream_request_timeout_non_streaming: Option<Duration>,
    max_attempts_per_provider: u32,
    enable_response_fixer: bool,
    response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
    introspection_body: &'a [u8],
}

#[derive(Clone, Copy)]
struct ProviderCtx<'a> {
    provider_id: i64,
    provider_name_base: &'a String,
    provider_base_url_base: &'a String,
    provider_index: u32,
    session_reuse: Option<bool>,
}

#[derive(Clone, Copy)]
struct AttemptCtx<'a> {
    attempt_index: u32,
    retry_index: u32,
    attempt_started_ms: u128,
    attempt_started: Instant,
    circuit_before: &'a circuit_breaker::CircuitSnapshot,
}

struct LoopState<'a> {
    attempts: &'a mut Vec<FailoverAttempt>,
    failed_provider_ids: &'a mut HashSet<i64>,
    last_error_category: &'a mut Option<&'static str>,
    last_error_code: &'a mut Option<&'static str>,
    circuit_snapshot: &'a mut circuit_breaker::CircuitSnapshot,
    abort_guard: &'a mut RequestAbortGuard,
}

enum LoopControl {
    ContinueRetry,
    BreakRetry,
    Return(Response),
}

pub(super) struct FailoverLoopInput {
    pub(super) state: GatewayAppState,
    pub(super) cli_key: String,
    pub(super) forwarded_path: String,
    pub(super) req_method: Method,
    pub(super) method_hint: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) requested_model_location: Option<RequestedModelLocation>,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) providers: Vec<providers::ProviderForGateway>,
    pub(super) session_bound_provider_id: Option<i64>,
    pub(super) base_headers: HeaderMap,
    pub(super) body_bytes: Bytes,
    pub(super) introspection_json: Option<serde_json::Value>,
    pub(super) strip_request_content_encoding_seed: bool,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_base_url_ping_cache_ttl_seconds: u32,
    pub(super) max_attempts_per_provider: u32,
    pub(super) max_providers_to_try: u32,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_first_byte_timeout: Option<Duration>,
    pub(super) upstream_stream_idle_timeout: Option<Duration>,
    pub(super) upstream_request_timeout_non_streaming: Option<Duration>,
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
    pub(super) abort_guard: RequestAbortGuard,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
}

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
        let provider_name_base = provider.name.clone();
        let provider_base_url_display = provider
            .base_urls
            .first()
            .cloned()
            .unwrap_or_else(String::new);

        if failed_provider_ids.contains(&provider_id) {
            continue;
        }

        let now_unix = now_unix_seconds() as i64;
        let allow = state.circuit.should_allow(provider_id, now_unix);
        if let Some(t) = allow.transition.as_ref() {
            emit_circuit_transition(
                &state.app,
                &trace_id,
                &cli_key,
                provider_id,
                &provider_name_base,
                &provider_base_url_display,
                t,
                now_unix,
            );
        }
        if !allow.allow {
            let snap = allow.after;
            let reason = if snap.state == circuit_breaker::CircuitState::Open {
                skipped_open = skipped_open.saturating_add(1);
                "SKIP_OPEN"
            } else {
                skipped_cooldown = skipped_cooldown.saturating_add(1);
                "SKIP_COOLDOWN"
            };

            if let Some(until) = snap.cooldown_until.or(snap.open_until) {
                if until > now_unix {
                    earliest_available_unix = Some(match earliest_available_unix {
                        Some(cur) => cur.min(until),
                        None => until,
                    });
                }
            }

            emit_circuit_event(
                &state.app,
                GatewayCircuitEvent {
                    trace_id: trace_id.clone(),
                    cli_key: cli_key.clone(),
                    provider_id,
                    provider_name: provider_name_base.clone(),
                    base_url: provider_base_url_display.clone(),
                    prev_state: snap.state.as_str(),
                    next_state: snap.state.as_str(),
                    failure_count: snap.failure_count,
                    failure_threshold: snap.failure_threshold,
                    open_until: snap.open_until,
                    cooldown_until: snap.cooldown_until,
                    reason,
                    ts: now_unix,
                },
            );
            continue;
        }

        // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

        let provider_base_url_base = select_provider_base_url_for_request(
            &state,
            provider,
            provider_base_url_ping_cache_ttl_seconds,
        )
        .await;

        let mut circuit_snapshot = allow.after.clone();

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

        if cli_key == "claude" && provider.claude_models.has_any() {
            if let Some(requested_model) = requested_model.as_deref() {
                let has_thinking = introspection_json
                    .as_ref()
                    .and_then(|v| v.get("thinking"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("type"))
                    .and_then(|v| v.as_str())
                    == Some("enabled");

                let effective_model =
                    provider.get_effective_claude_model(requested_model, has_thinking);
                if effective_model != requested_model {
                    let location =
                        requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
                    let mut applied = false;
                    match location {
                        RequestedModelLocation::BodyJson => {
                            if let Some(root) = introspection_json.as_ref() {
                                let mut next = root.clone();
                                let replaced =
                                    replace_model_in_body_json(&mut next, &effective_model);
                                if replaced {
                                    if let Ok(bytes) = serde_json::to_vec(&next) {
                                        upstream_body_bytes = Bytes::from(bytes);
                                        strip_request_content_encoding = true;
                                        applied = true;
                                    }
                                }
                            }
                        }
                        RequestedModelLocation::Query => {
                            if let Some(q) = upstream_query.as_deref() {
                                let next = replace_model_in_query(q, &effective_model);
                                applied = next != q;
                                upstream_query = Some(next);
                            }
                        }
                        RequestedModelLocation::Path => {
                            if let Some(next_path) =
                                replace_model_in_path(&upstream_forwarded_path, &effective_model)
                            {
                                applied = next_path != upstream_forwarded_path;
                                upstream_forwarded_path = next_path;
                            }
                        }
                    }

                    let model_lower = requested_model.to_ascii_lowercase();
                    let kind = if has_thinking
                        && provider
                            .claude_models
                            .reasoning_model
                            .as_deref()
                            .is_some_and(|v| v == effective_model.as_str())
                    {
                        "reasoning"
                    } else if model_lower.contains("haiku")
                        && provider
                            .claude_models
                            .haiku_model
                            .as_deref()
                            .is_some_and(|v| v == effective_model.as_str())
                    {
                        "haiku"
                    } else if model_lower.contains("sonnet")
                        && provider
                            .claude_models
                            .sonnet_model
                            .as_deref()
                            .is_some_and(|v| v == effective_model.as_str())
                    {
                        "sonnet"
                    } else if model_lower.contains("opus")
                        && provider
                            .claude_models
                            .opus_model
                            .as_deref()
                            .is_some_and(|v| v == effective_model.as_str())
                    {
                        "opus"
                    } else {
                        "main"
                    };

                    if let Ok(mut settings) = special_settings.lock() {
                        settings.push(serde_json::json!({
                            "type": "claude_model_mapping",
                            "scope": "attempt",
                            "hit": true,
                            "applied": applied,
                            "providerId": provider_id,
                            "providerName": provider_name_base.clone(),
                            "requestedModel": requested_model,
                            "effectiveModel": effective_model,
                            "mappingKind": kind,
                            "hasThinking": has_thinking,
                            "location": match location {
                                RequestedModelLocation::BodyJson => "body",
                                RequestedModelLocation::Query => "query",
                                RequestedModelLocation::Path => "path",
                            },
                        }));
                    }
                }
            }
        }

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

                    let attempt_event = GatewayAttemptEvent {
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
                        outcome,
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.db,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
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

            enum SendResult {
                Ok(reqwest::Response),
                Err(reqwest::Error),
                Timeout,
            }

            let send_result = if let Some(timeout) = upstream_first_byte_timeout {
                let send = state
                    .client
                    .request(method.clone(), url)
                    .headers(headers)
                    .body(upstream_body_bytes.clone())
                    .send();
                match tokio::time::timeout(timeout, send).await {
                    Ok(Ok(resp)) => SendResult::Ok(resp),
                    Ok(Err(err)) => SendResult::Err(err),
                    Err(_) => SendResult::Timeout,
                }
            } else {
                match state
                    .client
                    .request(method.clone(), url)
                    .headers(headers)
                    .body(upstream_body_bytes.clone())
                    .send()
                    .await
                {
                    Ok(resp) => SendResult::Ok(resp),
                    Err(err) => SendResult::Err(err),
                }
            };

            match send_result {
                SendResult::Ok(resp) => {
                    let status = resp.status();
                    let mut response_headers = resp.headers().clone();

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

                    if cli_key == "claude"
                        && enable_thinking_signature_rectifier
                        && status.as_u16() == 400
                    {
                        let loop_state = LoopState {
                            attempts: &mut attempts,
                            failed_provider_ids: &mut failed_provider_ids,
                            last_error_category: &mut last_error_category,
                            last_error_code: &mut last_error_code,
                            circuit_snapshot: &mut circuit_snapshot,
                            abort_guard: &mut abort_guard,
                        };
                        match thinking_signature_rectifier_400::handle_thinking_signature_rectifier_400(
                            ctx,
                            provider_ctx,
                            attempt_ctx,
                            loop_state,
                            enable_thinking_signature_rectifier,
                            resp,
                            status,
                            response_headers,
                            &mut upstream_body_bytes,
                            &mut strip_request_content_encoding,
                            &mut thinking_signature_rectifier_retried,
                        )
                        .await
                        {
                            LoopControl::ContinueRetry => continue,
                            LoopControl::BreakRetry => break,
                            LoopControl::Return(resp) => return resp,
                        }
                    }

                    let (category, error_code, base_decision) = classify_upstream_status(status);
                    let mut decision = base_decision;
                    if matches!(decision, FailoverDecision::RetrySameProvider)
                        && retry_index >= max_attempts_per_provider
                    {
                        decision = FailoverDecision::SwitchProvider;
                    }

                    let mut circuit_state_before = Some(circuit_before.state.as_str());
                    let mut circuit_state_after: Option<&'static str> = None;
                    let mut circuit_failure_count = Some(circuit_before.failure_count);
                    let circuit_failure_threshold = Some(circuit_before.failure_threshold);

                    let now_unix = now_unix_seconds() as i64;
                    if matches!(category, ErrorCategory::ProviderError) {
                        let change = state.circuit.record_failure(provider_id, now_unix);
                        circuit_snapshot = change.after.clone();
                        circuit_state_before = Some(change.before.state.as_str());
                        circuit_state_after = Some(change.after.state.as_str());
                        circuit_failure_count = Some(change.after.failure_count);

                        if let Some(t) = change.transition {
                            emit_circuit_transition(
                                &state.app,
                                &trace_id,
                                &cli_key,
                                provider_id,
                                &provider_name_base,
                                &provider_base_url_base,
                                &t,
                                now_unix,
                            );
                        }

                        if change.after.state == circuit_breaker::CircuitState::Open {
                            decision = FailoverDecision::SwitchProvider;
                        }
                    }

                    if provider_cooldown_secs > 0
                        && matches!(category, ErrorCategory::ProviderError)
                        && matches!(
                            decision,
                            FailoverDecision::SwitchProvider | FailoverDecision::Abort
                        )
                    {
                        let snap = state.circuit.trigger_cooldown(
                            provider_id,
                            now_unix,
                            provider_cooldown_secs,
                        );
                        circuit_snapshot = snap;
                    }

                    let reason = format!("status={}", status.as_u16());
                    let outcome = format!(
                        "upstream_error: status={} category={} code={} decision={}",
                        status.as_u16(),
                        category.as_str(),
                        error_code,
                        decision.as_str()
                    );

                    attempts.push(FailoverAttempt {
                        provider_id,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: Some(status.as_u16()),
                        provider_index: Some(provider_index),
                        retry_index: Some(retry_index),
                        session_reuse,
                        error_category: Some(category.as_str()),
                        error_code: Some(error_code),
                        decision: Some(decision.as_str()),
                        reason: Some(reason),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before,
                        circuit_state_after,
                        circuit_failure_count,
                        circuit_failure_threshold,
                    });

                    let attempt_event = GatewayAttemptEvent {
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
                        outcome: outcome.clone(),
                        status: Some(status.as_u16()),
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before,
                        circuit_state_after,
                        circuit_failure_count,
                        circuit_failure_threshold,
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.db,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    match decision {
                        FailoverDecision::RetrySameProvider => {
                            if let Some(delay) = retry_backoff_delay(status, retry_index) {
                                tokio::time::sleep(delay).await;
                            }
                            continue;
                        }
                        FailoverDecision::SwitchProvider => {
                            failed_provider_ids.insert(provider_id);
                            break;
                        }
                        FailoverDecision::Abort => {
                            strip_hop_headers(&mut response_headers);
                            let should_gunzip = has_gzip_content_encoding(&response_headers);
                            if should_gunzip {
                                // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                                response_headers.remove(header::CONTENT_ENCODING);
                                response_headers.remove(header::CONTENT_LENGTH);
                            }
                            let attempts_json = serde_json::to_string(&attempts)
                                .unwrap_or_else(|_| "[]".to_string());
                            let ctx = StreamFinalizeCtx {
                                app: state.app.clone(),
                                db: state.db.clone(),
                                log_tx: state.log_tx.clone(),
                                circuit: state.circuit.clone(),
                                session: state.session.clone(),
                                session_id: session_id.clone(),
                                sort_mode_id: effective_sort_mode_id,
                                trace_id: trace_id.clone(),
                                cli_key: cli_key.clone(),
                                method: method_hint.clone(),
                                path: forwarded_path.clone(),
                                query: query.clone(),
                                excluded_from_stats: false,
                                special_settings: special_settings.clone(),
                                status: status.as_u16(),
                                error_category: Some(category.as_str()),
                                error_code: Some(error_code),
                                started,
                                attempts: attempts.clone(),
                                attempts_json,
                                requested_model: requested_model.clone(),
                                created_at_ms,
                                created_at,
                                provider_cooldown_secs,
                                provider_id,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_base.clone(),
                            };

                            let body = if should_gunzip {
                                let upstream = GunzipStream::new(resp.bytes_stream());
                                let stream = TimingOnlyTeeStream::new(
                                    upstream,
                                    ctx,
                                    upstream_request_timeout_non_streaming,
                                );
                                Body::from_stream(stream)
                            } else {
                                let stream = TimingOnlyTeeStream::new(
                                    resp.bytes_stream(),
                                    ctx,
                                    upstream_request_timeout_non_streaming,
                                );
                                Body::from_stream(stream)
                            };
                            abort_guard.disarm();
                            return build_response(status, &response_headers, &trace_id, body);
                        }
                    }
                }
                SendResult::Timeout => {
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
                SendResult::Err(err) => {
                    let (category, error_code) = classify_reqwest_error(&err);
                    let decision = if retry_index < max_attempts_per_provider {
                        FailoverDecision::RetrySameProvider
                    } else {
                        FailoverDecision::SwitchProvider
                    };

                    let outcome = format!(
                        "request_error: category={} code={} decision={} err={err}",
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
                        reason: Some("reqwest error".to_string()),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    });

                    let attempt_event = GatewayAttemptEvent {
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
                        outcome,
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.db,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    if provider_cooldown_secs > 0
                        && matches!(
                            decision,
                            FailoverDecision::SwitchProvider | FailoverDecision::Abort
                        )
                    {
                        let now_unix = now_unix_seconds() as i64;
                        state.circuit.trigger_cooldown(
                            provider_id,
                            now_unix,
                            provider_cooldown_secs,
                        );
                    }

                    match decision {
                        FailoverDecision::RetrySameProvider => continue,
                        FailoverDecision::SwitchProvider => {
                            failed_provider_ids.insert(provider_id);
                            break;
                        }
                        FailoverDecision::Abort => break,
                    }
                }
            }
        }
    }

    if attempts.is_empty() && !providers.is_empty() {
        let now_unix = now_unix_seconds() as i64;
        let retry_after_seconds = earliest_available_unix
            .and_then(|t| t.checked_sub(now_unix))
            .filter(|v| *v > 0)
            .map(|v| v as u64);

        let message = format!(
            "no provider available (skipped: open={skipped_open}, cooldown={skipped_cooldown}) for cli_key={cli_key}",
        );

        let resp = error_response_with_retry_after(
            StatusCode::SERVICE_UNAVAILABLE,
            trace_id.clone(),
            "GW_ALL_PROVIDERS_UNAVAILABLE",
            message.clone(),
            vec![],
            retry_after_seconds,
        );

        emit_request_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            None,
            Some("GW_ALL_PROVIDERS_UNAVAILABLE"),
            started.elapsed().as_millis(),
            None,
            vec![],
            None,
        );

        enqueue_request_log_with_backpressure(
            &state.app,
            &state.db,
            &state.log_tx,
            RequestLogEnqueueArgs {
                trace_id: trace_id.clone(),
                cli_key,
                session_id: session_id.clone(),
                method: method_hint,
                path: forwarded_path,
                query,
                excluded_from_stats: false,
                special_settings_json: response_fixer::special_settings_json(&special_settings),
                status: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                error_code: Some("GW_ALL_PROVIDERS_UNAVAILABLE"),
                duration_ms: started.elapsed().as_millis(),
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                requested_model: requested_model.clone(),
                created_at_ms,
                created_at,
                usage: None,
            },
        )
        .await;

        if let Some(retry_after_seconds) = retry_after_seconds.filter(|v| *v > 0) {
            if let Ok(mut cache) = state.recent_errors.lock() {
                cache.insert_error(
                    now_unix,
                    unavailable_fingerprint_key,
                    CachedGatewayError {
                        trace_id: trace_id.clone(),
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
                        message: message.clone(),
                        retry_after_seconds: Some(retry_after_seconds),
                        expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                        fingerprint_debug: unavailable_fingerprint_debug.clone(),
                    },
                );
                cache.insert_error(
                    now_unix,
                    fingerprint_key,
                    CachedGatewayError {
                        trace_id: trace_id.clone(),
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
                        message,
                        retry_after_seconds: Some(retry_after_seconds),
                        expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                        fingerprint_debug: fingerprint_debug.clone(),
                    },
                );
            }
        }

        abort_guard.disarm();
        return resp;
    }

    let final_error_code = last_error_code.unwrap_or("GW_UPSTREAM_ALL_FAILED");
    let final_error_category = last_error_category;

    let resp = error_response(
        StatusCode::BAD_GATEWAY,
        trace_id.clone(),
        final_error_code,
        format!("all providers failed for cli_key={cli_key}"),
        attempts.clone(),
    );

    emit_request_event(
        &state.app,
        trace_id.clone(),
        cli_key.clone(),
        method_hint.clone(),
        forwarded_path.clone(),
        query.clone(),
        Some(StatusCode::BAD_GATEWAY.as_u16()),
        final_error_category,
        Some(final_error_code),
        started.elapsed().as_millis(),
        None,
        attempts.clone(),
        None,
    );

    enqueue_request_log_with_backpressure(
        &state.app,
        &state.db,
        &state.log_tx,
        RequestLogEnqueueArgs {
            trace_id,
            cli_key,
            session_id: session_id.clone(),
            method: method_hint,
            path: forwarded_path,
            query,
            excluded_from_stats: false,
            special_settings_json: response_fixer::special_settings_json(&special_settings),
            status: Some(StatusCode::BAD_GATEWAY.as_u16()),
            error_code: Some(final_error_code),
            duration_ms: started.elapsed().as_millis(),
            ttfb_ms: None,
            attempts_json: serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string()),
            requested_model,
            created_at_ms,
            created_at,
            usage: None,
        },
    )
    .await;

    abort_guard.disarm();
    resp
}
