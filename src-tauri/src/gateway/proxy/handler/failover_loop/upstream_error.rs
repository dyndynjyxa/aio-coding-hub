//! Usage: Handle upstream non-success responses and reqwest errors inside `failover_loop::run`.

use super::super::super::errors::{
    classify_reqwest_error, classify_upstream_status, error_response,
};
use super::super::super::failover::{retry_backoff_delay, FailoverDecision};
use super::super::super::http_util::{
    build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
    maybe_gunzip_response_body_bytes_with_limit,
};
use super::super::super::is_claude_count_tokens_request;
use super::super::super::provider_router;
use super::super::super::upstream_client_error_rules;
use super::super::super::ErrorCategory;
use super::attempt_record::{
    record_system_failure_and_decide, record_system_failure_and_decide_no_cooldown,
    RecordSystemFailureArgs,
};
use super::context::{
    AttemptCtx, CommonCtx, CommonCtxOwned, LoopControl, LoopState, ProviderCtx,
    MAX_NON_SSE_BODY_BYTES,
};
use super::thinking_signature_rectifier_400;
use super::{emit_attempt_event_and_log, AttemptCircuitFields};
use super::{emit_request_event_and_enqueue_request_log, RequestEndArgs, RequestEndDeps};
use crate::circuit_breaker;
use crate::gateway::events::FailoverAttempt;
use crate::gateway::response_fixer;
use crate::gateway::streams::GunzipStream;
use crate::gateway::util::{now_unix_seconds, strip_hop_headers};
use axum::body::{Body, Bytes};
use axum::http::{header, HeaderValue};

pub(super) struct UpstreamRequestState<'a> {
    pub(super) upstream_body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
    pub(super) thinking_signature_rectifier_retried: &'a mut bool,
}

pub(super) async fn handle_non_success_response(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_>,
    enable_thinking_signature_rectifier: bool,
    resp: reqwest::Response,
    upstream: UpstreamRequestState<'_>,
) -> LoopControl {
    let status = resp.status();
    let response_headers = resp.headers().clone();
    let is_count_tokens =
        is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());

    if !is_count_tokens
        && ctx.cli_key == "claude"
        && enable_thinking_signature_rectifier
        && status.as_u16() == 400
    {
        return thinking_signature_rectifier_400::handle_thinking_signature_rectifier_400(
            ctx,
            provider_ctx,
            attempt_ctx,
            loop_state,
            enable_thinking_signature_rectifier,
            resp,
            status,
            response_headers,
            upstream.upstream_body_bytes,
            upstream.strip_request_content_encoding,
            upstream.thinking_signature_rectifier_retried,
        )
        .await;
    }

    let mut resp = Some(resp);

    let state = ctx.state;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;
    let provider_cooldown_secs = ctx.provider_cooldown_secs;

    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
    } = provider_ctx;

    let AttemptCtx {
        attempt_index: _,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_error_category,
        last_error_code,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    let (base_category, error_code, base_decision) = classify_upstream_status(status);
    let mut category = base_category;
    let mut decision = if is_count_tokens {
        FailoverDecision::Abort
    } else {
        base_decision
    };
    if matches!(decision, FailoverDecision::RetrySameProvider)
        && retry_index >= max_attempts_per_provider
    {
        decision = FailoverDecision::SwitchProvider;
    }

    let mut abort_body_bytes: Option<Bytes> = None;
    let mut abort_response_headers: Option<axum::http::HeaderMap> = None;
    let mut matched_rule_id: Option<&'static str> = None;
    if !is_count_tokens
        && upstream_client_error_rules::should_attempt_non_retryable_match(
            status,
            resp.as_ref().and_then(|r| r.content_length()),
        )
    {
        if let Some(resp) = resp.take() {
            if let Ok(bytes) = resp.bytes().await {
                let mut headers_for_scan = response_headers.clone();
                strip_hop_headers(&mut headers_for_scan);
                let body_for_scan = maybe_gunzip_response_body_bytes_with_limit(
                    bytes,
                    &mut headers_for_scan,
                    MAX_NON_SSE_BODY_BYTES,
                );
                matched_rule_id = upstream_client_error_rules::match_non_retryable_client_error(
                    ctx.cli_key.as_str(),
                    status,
                    body_for_scan.as_ref(),
                );
                if matched_rule_id.is_some() {
                    category = ErrorCategory::NonRetryableClientError;
                    decision = FailoverDecision::Abort;
                    abort_body_bytes = Some(body_for_scan);
                    abort_response_headers = Some(headers_for_scan);
                }
            }
        }
    }

    let mut circuit_state_before = Some(circuit_before.state.as_str());
    let mut circuit_state_after: Option<&'static str> = None;
    let mut circuit_failure_count = Some(circuit_before.failure_count);
    let circuit_failure_threshold = Some(circuit_before.failure_threshold);

    let now_unix = now_unix_seconds() as i64;
    if !is_count_tokens && matches!(category, ErrorCategory::ProviderError) {
        let change = provider_router::record_failure_and_emit_transition(
            provider_router::RecordCircuitArgs::from_state(
                state,
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

        if change.after.state == circuit_breaker::CircuitState::Open {
            decision = FailoverDecision::SwitchProvider;
        }
    }

    if !is_count_tokens
        && provider_cooldown_secs > 0
        && matches!(category, ErrorCategory::ProviderError)
        && matches!(
            decision,
            FailoverDecision::SwitchProvider | FailoverDecision::Abort
        )
    {
        let snap = provider_router::trigger_cooldown(
            state.circuit.as_ref(),
            provider_id,
            now_unix,
            provider_cooldown_secs,
        );
        *circuit_snapshot = snap;
    }

    let reason = match matched_rule_id {
        Some(rule_id) => format!("status={} rule={rule_id}", status.as_u16()),
        None => format!("status={}", status.as_u16()),
    };
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

    emit_attempt_event_and_log(
        ctx,
        provider_ctx,
        attempt_ctx,
        outcome,
        Some(status.as_u16()),
        AttemptCircuitFields {
            state_before: circuit_state_before,
            state_after: circuit_state_after,
            failure_count: circuit_failure_count,
            failure_threshold: circuit_failure_threshold,
        },
    )
    .await;

    *last_error_category = Some(category.as_str());
    *last_error_code = Some(error_code);

    match decision {
        FailoverDecision::RetrySameProvider => {
            if let Some(delay) = retry_backoff_delay(status, retry_index) {
                tokio::time::sleep(delay).await;
            }
            LoopControl::ContinueRetry
        }
        FailoverDecision::SwitchProvider => {
            failed_provider_ids.insert(provider_id);
            LoopControl::BreakRetry
        }
        FailoverDecision::Abort => {
            // On abort, we intentionally do NOT use stream tee finalizers, to avoid triggering
            // provider cooldown on non-retryable client input errors (align with claude-code-hub).

            let CommonCtxOwned {
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
                enable_response_fixer,
                response_fixer_non_stream_config,
                ..
            } = CommonCtxOwned::from(ctx);

            if let (Some(mut response_headers), Some(mut body_bytes)) =
                (abort_response_headers, abort_body_bytes)
            {
                let enable_response_fixer_for_this_response =
                    enable_response_fixer && !has_non_identity_content_encoding(&response_headers);
                if enable_response_fixer_for_this_response {
                    response_headers.remove(header::CONTENT_LENGTH);
                    let outcome = response_fixer::process_non_stream(
                        body_bytes,
                        response_fixer_non_stream_config,
                    );
                    response_headers.insert(
                        "x-cch-response-fixer",
                        HeaderValue::from_static(outcome.header_value),
                    );
                    if let Some(setting) = outcome.special_setting {
                        if let Ok(mut settings) = special_settings.lock() {
                            settings.push(setting);
                        }
                    }
                    body_bytes = outcome.body;
                }

                let special_settings_json =
                    response_fixer::special_settings_json(&special_settings);
                let duration_ms = started.elapsed().as_millis();

                emit_request_event_and_enqueue_request_log(RequestEndArgs {
                    deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                    trace_id: trace_id.as_str(),
                    cli_key: cli_key.as_str(),
                    method: method_hint.as_str(),
                    path: forwarded_path.as_str(),
                    query: query.as_deref(),
                    excluded_from_stats: false,
                    status: Some(status.as_u16()),
                    error_category: Some(category.as_str()),
                    error_code: Some(error_code),
                    duration_ms,
                    event_ttfb_ms: Some(duration_ms),
                    log_ttfb_ms: Some(duration_ms),
                    attempts: attempts.as_slice(),
                    special_settings_json,
                    session_id,
                    requested_model,
                    created_at_ms,
                    created_at,
                    usage_metrics: None,
                    log_usage_metrics: None,
                    usage: None,
                })
                .await;

                abort_guard.disarm();

                return LoopControl::Return(build_response(
                    status,
                    &response_headers,
                    trace_id.as_str(),
                    Body::from(body_bytes),
                ));
            }

            let special_settings_json = response_fixer::special_settings_json(&special_settings);
            let duration_ms = started.elapsed().as_millis();

            emit_request_event_and_enqueue_request_log(RequestEndArgs {
                deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                trace_id: trace_id.as_str(),
                cli_key: cli_key.as_str(),
                method: method_hint.as_str(),
                path: forwarded_path.as_str(),
                query: query.as_deref(),
                excluded_from_stats: false,
                status: Some(status.as_u16()),
                error_category: Some(category.as_str()),
                error_code: Some(error_code),
                duration_ms,
                event_ttfb_ms: Some(duration_ms),
                log_ttfb_ms: Some(duration_ms),
                attempts: attempts.as_slice(),
                special_settings_json,
                session_id,
                requested_model,
                created_at_ms,
                created_at,
                usage_metrics: None,
                log_usage_metrics: None,
                usage: None,
            })
            .await;

            abort_guard.disarm();

            let mut response_headers = response_headers;
            strip_hop_headers(&mut response_headers);
            let should_gunzip = has_gzip_content_encoding(&response_headers);
            if should_gunzip {
                // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                response_headers.remove(header::CONTENT_ENCODING);
                response_headers.remove(header::CONTENT_LENGTH);
            }

            let Some(resp) = resp else {
                return LoopControl::Return(error_response(
                    axum::http::StatusCode::BAD_GATEWAY,
                    trace_id.clone(),
                    "GW_UPSTREAM_READ_ERROR",
                    "failed to stream upstream error body".to_string(),
                    attempts.clone(),
                ));
            };
            let body = if should_gunzip {
                let upstream = GunzipStream::new(resp.bytes_stream());
                Body::from_stream(upstream)
            } else {
                Body::from_stream(resp.bytes_stream())
            };

            LoopControl::Return(build_response(
                status,
                &response_headers,
                trace_id.as_str(),
                body,
            ))
        }
    }
}

pub(super) async fn handle_reqwest_error(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_>,
    err: reqwest::Error,
) -> LoopControl {
    if is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str()) {
        let (_, error_code) = classify_reqwest_error(&err);
        let decision = FailoverDecision::Abort;
        let outcome = format!(
            "request_error: category={} code={} decision={} err={err}",
            ErrorCategory::SystemError.as_str(),
            error_code,
            decision.as_str(),
        );
        return record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
            ctx,
            provider_ctx,
            attempt_ctx,
            loop_state,
            status: None,
            error_code,
            decision,
            outcome,
            reason: "reqwest error".to_string(),
        })
        .await;
    }

    let max_attempts_per_provider = ctx.max_attempts_per_provider;

    let (_, error_code) = classify_reqwest_error(&err);
    let retry_index = attempt_ctx.retry_index;
    let decision = if retry_index < max_attempts_per_provider {
        FailoverDecision::RetrySameProvider
    } else {
        FailoverDecision::SwitchProvider
    };

    let outcome = format!(
        "request_error: category={} code={} decision={} err={err}",
        ErrorCategory::SystemError.as_str(),
        error_code,
        decision.as_str(),
    );

    record_system_failure_and_decide(RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        status: None,
        error_code,
        decision,
        outcome,
        reason: "reqwest error".to_string(),
    })
    .await
}
