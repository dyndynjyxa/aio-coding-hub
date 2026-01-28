//! Usage: Handle upstream non-success responses and reqwest errors inside `failover_loop::run`.

use super::super::super::errors::{classify_reqwest_error, classify_upstream_status};
use super::super::super::failover::{retry_backoff_delay, FailoverDecision};
use super::super::super::http_util::{build_response, has_gzip_content_encoding};
use super::super::super::ErrorCategory;
use super::context::{AttemptCtx, CommonCtx, LoopControl, LoopState, ProviderCtx};
use super::thinking_signature_rectifier_400;
use super::{
    emit_attempt_event_and_log, emit_attempt_event_and_log_with_circuit_before,
    AttemptCircuitFields,
};
use crate::circuit_breaker;
use crate::gateway::events::{emit_circuit_transition, FailoverAttempt};
use crate::gateway::streams::{GunzipStream, StreamFinalizeCtx, TimingOnlyTeeStream};
use crate::gateway::util::{now_unix_seconds, strip_hop_headers};
use axum::body::{Body, Bytes};
use axum::http::header;
use std::sync::Arc;

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

    if ctx.cli_key == "claude" && enable_thinking_signature_rectifier && status.as_u16() == 400 {
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

    let state = ctx.state;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;
    let provider_cooldown_secs = ctx.provider_cooldown_secs;
    let upstream_request_timeout_non_streaming = ctx.upstream_request_timeout_non_streaming;

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
        *circuit_snapshot = change.after.clone();
        circuit_state_before = Some(change.before.state.as_str());
        circuit_state_after = Some(change.after.state.as_str());
        circuit_failure_count = Some(change.after.failure_count);

        if let Some(t) = change.transition {
            emit_circuit_transition(
                &state.app,
                ctx.trace_id,
                ctx.cli_key,
                provider_id,
                provider_name_base,
                provider_base_url_base,
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
        let snap = state
            .circuit
            .trigger_cooldown(provider_id, now_unix, provider_cooldown_secs);
        *circuit_snapshot = snap;
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
            let mut response_headers = response_headers;
            strip_hop_headers(&mut response_headers);
            let should_gunzip = has_gzip_content_encoding(&response_headers);
            if should_gunzip {
                // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                response_headers.remove(header::CONTENT_ENCODING);
                response_headers.remove(header::CONTENT_LENGTH);
            }
            let attempts_json =
                serde_json::to_string(attempts).unwrap_or_else(|_| "[]".to_string());
            let finalize_ctx = StreamFinalizeCtx {
                app: state.app.clone(),
                db: state.db.clone(),
                log_tx: state.log_tx.clone(),
                circuit: state.circuit.clone(),
                session: state.session.clone(),
                session_id: ctx.session_id.clone(),
                sort_mode_id: ctx.effective_sort_mode_id,
                trace_id: ctx.trace_id.clone(),
                cli_key: ctx.cli_key.clone(),
                method: ctx.method_hint.clone(),
                path: ctx.forwarded_path.clone(),
                query: ctx.query.clone(),
                excluded_from_stats: false,
                special_settings: Arc::clone(ctx.special_settings),
                status: status.as_u16(),
                error_category: Some(category.as_str()),
                error_code: Some(error_code),
                started: ctx.started,
                attempts: attempts.clone(),
                attempts_json,
                requested_model: ctx.requested_model.clone(),
                created_at_ms: ctx.created_at_ms,
                created_at: ctx.created_at,
                provider_cooldown_secs,
                provider_id,
                provider_name: provider_name_base.clone(),
                base_url: provider_base_url_base.clone(),
            };

            let body = if should_gunzip {
                let upstream = GunzipStream::new(resp.bytes_stream());
                let stream = TimingOnlyTeeStream::new(
                    upstream,
                    finalize_ctx,
                    upstream_request_timeout_non_streaming,
                );
                Body::from_stream(stream)
            } else {
                let stream = TimingOnlyTeeStream::new(
                    resp.bytes_stream(),
                    finalize_ctx,
                    upstream_request_timeout_non_streaming,
                );
                Body::from_stream(stream)
            };
            abort_guard.disarm();
            LoopControl::Return(build_response(
                status,
                &response_headers,
                ctx.trace_id.as_str(),
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
    let state = ctx.state;
    let provider_cooldown_secs = ctx.provider_cooldown_secs;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;

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
        circuit_snapshot: _,
        abort_guard: _,
    } = loop_state;

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

    emit_attempt_event_and_log_with_circuit_before(ctx, provider_ctx, attempt_ctx, outcome, None)
        .await;

    *last_error_category = Some(category.as_str());
    *last_error_code = Some(error_code);

    if provider_cooldown_secs > 0
        && matches!(
            decision,
            FailoverDecision::SwitchProvider | FailoverDecision::Abort
        )
    {
        let now_unix = now_unix_seconds() as i64;
        state
            .circuit
            .trigger_cooldown(provider_id, now_unix, provider_cooldown_secs);
    }

    match decision {
        FailoverDecision::RetrySameProvider => LoopControl::ContinueRetry,
        FailoverDecision::SwitchProvider => {
            failed_provider_ids.insert(provider_id);
            LoopControl::BreakRetry
        }
        FailoverDecision::Abort => LoopControl::BreakRetry,
    }
}
