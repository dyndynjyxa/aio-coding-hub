//! WebSocket-specific decisions inside the existing provider retry loop.

use super::attempt_executor::AttemptTiming;
use super::provider_iterator::PreparedProvider;
use super::retry_engine::AttemptIndices;
use super::*;
use crate::gateway::proxy::request_context::RequestContext;
use crate::gateway::responses_ws::protocol;
use crate::shared::mutex_ext::MutexExt;

pub(super) fn marker<R: tauri::Runtime>(
    input: &RequestContext<R>,
    provider: i64,
    transport: &str,
    action: &str,
    failure: Option<&str>,
    reason: Option<&str>,
) {
    response_fixer::push_special_setting(
        &input.special_settings,
        serde_json::json!({
            "type":"codex_responses_transport", "scope":"attempt", "providerId":provider,
            "client_transport": if input.ws_request.as_ref().is_some_and(|r| !r.client_ws) { "http" } else { "responses_ws" },
            "upstream_transport":transport, "transport_action":action, "failure_class":failure,
            "reason_code":reason, "output_committed":false,
        }),
    );
}

pub(super) async fn transport_failure<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &PreparedProvider,
    indices: AttemptIndices,
    timing: AttemptTiming,
    reason: &'static str,
    loop_state: &mut LoopState<'_, R>,
) -> LoopControl {
    record_attempt(prepared, indices, &timing, reason, loop_state);
    let circuit = prepared.circuit_snapshot.clone();
    let mut attempt = attempt_executor::build_attempt_ctx(
        indices.attempt_index,
        indices.retry_index,
        timing.attempt_started_ms,
        &circuit,
        prepared,
    );
    attempt.attempt_started = timing.attempt_started;
    attempt.upstream_sent = timing.upstream_sent;
    emit_attempt_event_and_log_with_circuit_before(
        ctx,
        attempt_executor::build_provider_ctx(prepared),
        attempt,
        "transport_fallback".into(),
        None,
    )
    .await;
    marker(
        input,
        prepared.provider_id,
        "responses_ws",
        "http_fallback",
        Some("transport"),
        Some(reason),
    );
    let Some(request) = &input.ws_request else {
        if let Some(connection) = &input.ws_connection {
            if let Some(session) = input
                .base_headers
                .get("session-id")
                .and_then(|value| value.to_str().ok())
            {
                connection.runtime.force_http(session);
            }
        }
        return finish_error(
            ctx,
            input,
            loop_state,
            "prewarm_http_required",
            "WebSocket prewarm unavailable; reconnect using HTTP",
        )
        .await;
    };
    let previous = {
        let mut generation = request.generation.lock_or_recover();
        generation
            .budget
            .http_provider_ids
            .insert(prepared.provider_id);
        generation.previous.is_some()
    };
    if previous {
        recover(ctx, input, prepared, indices, timing, loop_state).await
    } else {
        LoopControl::RetryTransport
    }
}

pub(super) async fn recover<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &PreparedProvider,
    _indices: AttemptIndices,
    _timing: AttemptTiming,
    loop_state: &mut LoopState<'_, R>,
) -> LoopControl {
    let Some(request) = &input.ws_request else {
        return finish_error(
            ctx,
            input,
            loop_state,
            "invalid_request",
            "Cannot restore an unidentified generation",
        )
        .await;
    };
    {
        let mut generation = request.generation.lock_or_recover();
        generation
            .budget
            .failed_providers
            .extend(loop_state.failed_provider_ids.iter().copied());
    }
    match request.connection.runtime.suspend(request) {
        Ok(()) => {
            marker(
                input,
                prepared.provider_id,
                "responses_ws",
                "full_input_retry",
                Some("context"),
                Some("previous_response_not_found"),
            );
            finish_error(
                ctx,
                input,
                loop_state,
                "previous_response_not_found",
                "Response context is unavailable; resend the full input",
            )
            .await
        }
        Err(_) => {
            finish_error(
                ctx,
                input,
                loop_state,
                "invalid_request",
                "Response context cannot be safely restored",
            )
            .await
        }
    }
}

pub(super) async fn finish_error<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    loop_state: &mut LoopState<'_, R>,
    code: &str,
    message: &str,
) -> LoopControl {
    emit_request_event_and_enqueue_request_log(
        RequestEndArgs::from_context(RequestEndContextArgs {
            deps: RequestEndDeps::new(
                &ctx.state.app,
                &ctx.state.db,
                &ctx.state.log_tx,
                &ctx.state.plugin_pipeline,
                &ctx.state.active_requests,
            ),
            trace_id: &input.trace_id,
            cli_key: &input.cli_key,
            method: &input.method_hint,
            path: &input.forwarded_path,
            observe: input.observe_request,
            query: input.query.as_deref(),
            excluded_from_stats: input.provider_health_neutral,
            duration_ms: input.started.elapsed().as_millis(),
            attempts: loop_state.attempts.as_slice(),
            special_settings_json: response_fixer::special_settings_json(&input.special_settings),
            session_id: input.session_id.clone(),
            requested_model: input.requested_model.clone(),
            created_at_ms: input.created_at_ms,
            created_at: input.created_at,
        })
        .with_completion(RequestCompletion::failure_with_ttfb(
            400,
            Some(if code == "previous_response_not_found" {
                "context"
            } else {
                "local"
            }),
            GatewayErrorCode::StreamError.as_str(),
            input.started.elapsed().as_millis(),
        )),
    )
    .await;
    loop_state.abort_guard.disarm();
    let mut event = protocol::error_event(code, message);
    event["trace_id"] = serde_json::Value::String(input.trace_id.clone());
    LoopControl::Return((StatusCode::BAD_REQUEST, axum::Json(event)).into_response())
}

fn record_attempt<R: tauri::Runtime>(
    prepared: &PreparedProvider,
    indices: AttemptIndices,
    timing: &AttemptTiming,
    reason: &str,
    loop_state: &mut LoopState<'_, R>,
) {
    loop_state.attempts.push(FailoverAttempt {
        provider_id: prepared.provider_id,
        provider_name: prepared.provider_name_base.clone(),
        base_url: prepared.provider_base_url_base.clone(),
        outcome: "transport_fallback".into(),
        status: None,
        provider_index: Some(prepared.provider_index),
        retry_index: Some(indices.retry_index),
        session_reuse: prepared.session_reuse,
        error_category: Some("transport"),
        error_code: Some(GatewayErrorCode::StreamError.as_str()),
        decision: Some("retry_same_provider"),
        reason: Some(reason.into()),
        selection_method: None,
        reason_code: None,
        attempt_started_ms: Some(timing.attempt_started_ms),
        attempt_duration_ms: Some(timing.attempt_started.elapsed().as_millis()),
        circuit_state_before: Some(prepared.circuit_snapshot.state.as_str()),
        circuit_state_after: None,
        circuit_failure_count: Some(prepared.circuit_snapshot.failure_count),
        circuit_failure_threshold: Some(prepared.circuit_snapshot.failure_threshold),
        circuit_recover_at_unix: None,
        circuit_trigger_error_code: None,
        provider_bridged: Some(false),
        timeout_secs: None,
        reasoning_effort: timing.reasoning_effort.clone(),
        upstream_sent: timing.upstream_sent,
        claude_model_mapping: None,
        model_redirect: prepared.model_redirect.clone(),
    });
}
