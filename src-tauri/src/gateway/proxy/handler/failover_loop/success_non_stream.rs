//! Usage: Handle successful non-SSE upstream responses inside `failover_loop::run`.

use super::*;
use crate::gateway::proxy::{gemini_oauth, protocol_bridge, provider_router, GatewayErrorCode};
use crate::shared::mutex_ext::MutexExt;

fn buffer_cx2cc_event_stream_as_json(
    cx2cc_active: bool,
    response_headers: &mut HeaderMap,
    body_bytes: Bytes,
) -> Result<Bytes, String> {
    if !cx2cc_active
        || !(is_event_stream(response_headers) || looks_like_sse_payload(body_bytes.as_ref()))
    {
        return Ok(body_bytes);
    }

    let response = protocol_bridge::stream::aggregate_responses_event_stream(body_bytes.as_ref())?;
    let encoded = serde_json::to_vec(&response)
        .map_err(|err| format!("failed to serialize aggregated response: {err}"))?;

    response_headers.remove(header::CONTENT_LENGTH);
    response_headers.remove(header::CONTENT_ENCODING);
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok(Bytes::from(encoded))
}

fn looks_like_sse_payload(body_bytes: &[u8]) -> bool {
    let trimmed = body_bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .map(|i| &body_bytes[i..])
        .unwrap_or(b"");
    trimmed.starts_with(b"event:") || trimmed.starts_with(b"data:") || trimmed.starts_with(b":")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cx2ccSuccessPayloadKind {
    NonStreamJson,
    HeaderlessEventStream,
}

struct ClassifiedCx2ccSuccessPayload {
    kind: Cx2ccSuccessPayloadKind,
    body_bytes: Bytes,
}

fn classify_cx2cc_success_payload(
    cx2cc_active: bool,
    response_headers: &mut HeaderMap,
    body_bytes: Bytes,
) -> Result<ClassifiedCx2ccSuccessPayload, String> {
    let headerless_event_stream = cx2cc_active
        && !is_event_stream(response_headers)
        && looks_like_sse_payload(body_bytes.as_ref());
    let body_bytes = buffer_cx2cc_event_stream_as_json(cx2cc_active, response_headers, body_bytes)?;

    Ok(ClassifiedCx2ccSuccessPayload {
        kind: if headerless_event_stream {
            Cx2ccSuccessPayloadKind::HeaderlessEventStream
        } else {
            Cx2ccSuccessPayloadKind::NonStreamJson
        },
        body_bytes,
    })
}

fn summarize_openai_response_json(body: &serde_json::Value) -> String {
    let model = body
        .get("model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let status = body
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let output = body
        .get("output")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let output_types = output
        .iter()
        .take(6)
        .filter_map(|item| item.get("type").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join(",");
    let text_lengths = output
        .iter()
        .filter(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("message"))
        .flat_map(|item| {
            item.get("content")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(
            |block| match block.get("type").and_then(serde_json::Value::as_str) {
                Some("output_text") => block.get("text").and_then(serde_json::Value::as_str),
                Some("refusal") => block.get("refusal").and_then(serde_json::Value::as_str),
                _ => None,
            },
        )
        .map(|text| text.len().to_string())
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "json model={model:?} status={status:?} output_count={} output_types=[{}] text_lengths=[{}]",
        output.len(),
        output_types,
        text_lengths
    )
}

fn summarize_translated_anthropic_sse(body_bytes: &[u8]) -> String {
    let text = std::str::from_utf8(body_bytes).unwrap_or("");
    let event_names = text
        .lines()
        .filter_map(|line| line.strip_prefix("event:"))
        .map(str::trim)
        .take(12)
        .collect::<Vec<_>>()
        .join(",");
    let message_start_model = text
        .split("\n\n")
        .find_map(|frame| {
            let mut is_message_start = false;
            let mut data_parts = Vec::new();
            for line in frame.lines() {
                let line = line.trim_end_matches('\r');
                if let Some(rest) = line.strip_prefix("event:") {
                    is_message_start = rest.trim() == "message_start";
                } else if let Some(rest) = line.strip_prefix("data:") {
                    data_parts.push(rest.trim_start());
                }
            }
            if !is_message_start || data_parts.is_empty() {
                return None;
            }
            let payload = data_parts.join("\n");
            serde_json::from_str::<serde_json::Value>(&payload)
                .ok()
                .and_then(|value| {
                    value
                        .pointer("/message/model")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
        })
        .unwrap_or_default();

    format!(
        "sse len={} model={message_start_model:?} events=[{}] has_message_start={} has_content_block_delta={} has_message_stop={}",
        body_bytes.len(),
        event_names,
        text.contains("event: message_start"),
        text.contains("event: content_block_delta"),
        text.contains("event: message_stop")
    )
}

fn summarize_json_keys(body_bytes: &[u8]) -> String {
    match serde_json::from_slice::<serde_json::Value>(body_bytes) {
        Ok(serde_json::Value::Object(obj)) => {
            let keys = obj.keys().take(12).cloned().collect::<Vec<_>>().join(",");
            let model = obj
                .get("model")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let stop_reason = obj
                .get("stop_reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            format!(
                "json len={} keys=[{}] model={model:?} stop_reason={stop_reason:?}",
                body_bytes.len(),
                keys
            )
        }
        Ok(_) => format!("json len={} root=non_object", body_bytes.len()),
        Err(err) => format!("json len={} parse_err={}", body_bytes.len(), err),
    }
}

fn should_passthrough_non_stream_success(
    gemini_oauth_response_mode: Option<gemini_oauth::GeminiOAuthResponseMode>,
    cx2cc_buffered_event_stream: bool,
    cx2cc_active: bool,
) -> bool {
    gemini_oauth_response_mode.is_none() && !cx2cc_buffered_event_stream && !cx2cc_active
}

fn translate_cx2cc_non_stream_body(
    cx2cc_active: bool,
    anthropic_stream_requested: bool,
    requested_model: Option<&str>,
    cx2cc_settings: &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
    response_headers: &mut HeaderMap,
    body_bytes: Bytes,
) -> Result<Bytes, String> {
    if !cx2cc_active {
        return Ok(body_bytes);
    }

    let openai_body: serde_json::Value = serde_json::from_slice(body_bytes.as_ref())
        .map_err(|err| format!("failed to parse cx2cc response JSON: {err}"))?;

    let bridge = protocol_bridge::get_bridge("cx2cc")
        .ok_or_else(|| "cx2cc bridge not registered".to_string())?;
    let bridge_ctx = protocol_bridge::BridgeContext {
        claude_models: crate::domain::providers::ClaudeModels::default(),
        cx2cc_settings: cx2cc_settings.clone(),
        requested_model: requested_model.filter(|m| !m.is_empty()).map(String::from),
        mapped_model: None,
        stream_requested: anthropic_stream_requested,
        is_chatgpt_backend: false,
    };

    if anthropic_stream_requested {
        let sse_body = bridge
            .translate_response_to_sse(openai_body, &bridge_ctx)
            .map_err(|e| e.to_string())?;
        response_headers.remove(header::CONTENT_LENGTH);
        response_headers.remove(header::CONTENT_ENCODING);
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream; charset=utf-8"),
        );
        return Ok(sse_body);
    }

    let anthropic_body = bridge
        .translate_response(openai_body, &bridge_ctx)
        .map_err(|e| e.to_string())?;
    let encoded = serde_json::to_vec(&anthropic_body)
        .map_err(|err| format!("failed to serialize anthropic response JSON: {err}"))?;
    response_headers.remove(header::CONTENT_LENGTH);
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok(Bytes::from(encoded))
}

pub(super) async fn handle_success_non_stream(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_>,
    resp: reqwest::Response,
    status: StatusCode,
    mut response_headers: HeaderMap,
) -> LoopControl {
    let common = CommonCtxOwned::from(ctx);
    let provider_ctx_owned = ProviderCtxOwned::from(provider_ctx);
    tracing::debug!(
        trace_id = %common.trace_id,
        provider_id = provider_ctx_owned.provider_id,
        cx2cc_active = attempt_ctx.cx2cc_active,
        anthropic_stream_requested = attempt_ctx.anthropic_stream_requested,
        "handling successful upstream response, awaiting body classification"
    );

    let state = common.state;
    let started = common.started;
    let created_at_ms = common.created_at_ms;
    let created_at = common.created_at;
    let upstream_request_timeout_non_streaming = common.upstream_request_timeout_non_streaming;
    let max_attempts_per_provider = common.max_attempts_per_provider;
    let enable_response_fixer = common.enable_response_fixer;
    let response_fixer_non_stream_config = common.response_fixer_non_stream_config;

    let provider_id = provider_ctx_owned.provider_id;
    let provider_index = provider_ctx_owned.provider_index;
    let session_reuse = provider_ctx_owned.session_reuse;

    let AttemptCtx {
        attempt_index: _,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
        gemini_oauth_response_mode,
        cx2cc_active,
        anthropic_stream_requested,
    } = attempt_ctx;
    let selection_method = dc::selection_method(provider_index, retry_index, session_reuse);
    let reason_code = dc::success_reason_code(provider_index, retry_index);

    let LoopState {
        attempts,
        failed_provider_ids,
        last_error_category,
        last_error_code,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    strip_hop_headers(&mut response_headers);
    let cx2cc_buffered_event_stream = cx2cc_active && is_event_stream(&response_headers);
    if should_passthrough_non_stream_success(
        gemini_oauth_response_mode,
        cx2cc_buffered_event_stream,
        cx2cc_active,
    ) {
        let should_gunzip = has_gzip_content_encoding(&response_headers);

        match resp.content_length() {
            Some(len) if len > MAX_NON_SSE_BODY_BYTES as u64 => {
                let outcome = "success".to_string();

                attempts.push(FailoverAttempt {
                    provider_id,
                    provider_name: provider_ctx_owned.provider_name_base.clone(),
                    base_url: provider_ctx_owned.provider_base_url_base.clone(),
                    outcome: outcome.clone(),
                    status: Some(status.as_u16()),
                    provider_index: Some(provider_index),
                    retry_index: Some(retry_index),
                    session_reuse,
                    error_category: None,
                    error_code: None,
                    decision: Some("success"),
                    reason: None,
                    selection_method,
                    reason_code: Some(reason_code),
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
                    Some(status.as_u16()),
                )
                .await;

                let ctx = build_stream_finalize_ctx(
                    &common,
                    &provider_ctx_owned,
                    attempts.as_slice(),
                    status.as_u16(),
                    None,
                    None,
                );

                if should_gunzip {
                    // 上游可能无视 accept-encoding: identity 返回 gzip；
                    response_headers.remove(header::CONTENT_ENCODING);
                    response_headers.remove(header::CONTENT_LENGTH);
                }

                if should_gunzip {
                    let upstream = GunzipStream::new(resp.bytes_stream());
                    let stream = TimingOnlyTeeStream::new(
                        upstream,
                        ctx,
                        upstream_request_timeout_non_streaming,
                    );
                    let body = Body::from_stream(stream);
                    abort_guard.disarm();
                    return LoopControl::Return(build_response(
                        status,
                        &response_headers,
                        common.trace_id.as_str(),
                        body,
                    ));
                }

                let stream = TimingOnlyTeeStream::new(
                    resp.bytes_stream(),
                    ctx,
                    upstream_request_timeout_non_streaming,
                );
                let body = Body::from_stream(stream);
                abort_guard.disarm();
                return LoopControl::Return(build_response(
                    status,
                    &response_headers,
                    common.trace_id.as_str(),
                    body,
                ));
            }
            None => {
                let outcome = "success".to_string();

                attempts.push(FailoverAttempt {
                    provider_id,
                    provider_name: provider_ctx_owned.provider_name_base.clone(),
                    base_url: provider_ctx_owned.provider_base_url_base.clone(),
                    outcome: outcome.clone(),
                    status: Some(status.as_u16()),
                    provider_index: Some(provider_index),
                    retry_index: Some(retry_index),
                    session_reuse,
                    error_category: None,
                    error_code: None,
                    decision: Some("success"),
                    reason: None,
                    selection_method,
                    reason_code: Some(reason_code),
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
                    Some(status.as_u16()),
                )
                .await;

                let ctx = build_stream_finalize_ctx(
                    &common,
                    &provider_ctx_owned,
                    attempts.as_slice(),
                    status.as_u16(),
                    None,
                    None,
                );

                if should_gunzip {
                    // 上游可能无视 accept-encoding: identity 返回 gzip；
                    response_headers.remove(header::CONTENT_ENCODING);
                    response_headers.remove(header::CONTENT_LENGTH);
                }

                let body = if should_gunzip {
                    let upstream = GunzipStream::new(resp.bytes_stream());
                    let stream = UsageBodyBufferTeeStream::new(
                        upstream,
                        ctx,
                        MAX_NON_SSE_BODY_BYTES,
                        upstream_request_timeout_non_streaming,
                    );
                    Body::from_stream(stream)
                } else {
                    let stream = UsageBodyBufferTeeStream::new(
                        resp.bytes_stream(),
                        ctx,
                        MAX_NON_SSE_BODY_BYTES,
                        upstream_request_timeout_non_streaming,
                    );
                    Body::from_stream(stream)
                };

                let mut builder = Response::builder().status(status);
                for (k, v) in response_headers.iter() {
                    builder = builder.header(k, v);
                }
                builder = builder.header("x-trace-id", common.trace_id.as_str());

                abort_guard.disarm();
                return LoopControl::Return(match builder.body(body) {
                    Ok(r) => r,
                    Err(_) => {
                        let mut fallback = (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            GatewayErrorCode::ResponseBuildError.as_str(),
                        )
                            .into_response();
                        fallback.headers_mut().insert(
                            "x-trace-id",
                            HeaderValue::from_str(common.trace_id.as_str())
                                .unwrap_or(HeaderValue::from_static("unknown")),
                        );
                        fallback
                    }
                });
            }
            _ => {}
        }
    }

    let remaining_total =
        upstream_request_timeout_non_streaming.and_then(|t| t.checked_sub(started.elapsed()));
    let bytes_result = match remaining_total {
        Some(remaining) => {
            if remaining.is_zero() {
                Err("timeout")
            } else {
                match tokio::time::timeout(remaining, resp.bytes()).await {
                    Ok(Ok(b)) => Ok(b),
                    Ok(Err(_)) => Err("read_error"),
                    Err(_) => Err("timeout"),
                }
            }
        }
        None => match resp.bytes().await {
            Ok(b) => Ok(b),
            Err(_) => Err("read_error"),
        },
    };

    let mut body_bytes = match bytes_result {
        Ok(b) => b,
        Err(kind) => {
            let error_code = if kind == "timeout" {
                GatewayErrorCode::UpstreamTimeout.as_str()
            } else {
                GatewayErrorCode::UpstreamReadError.as_str()
            };
            let decision = if retry_index < max_attempts_per_provider {
                FailoverDecision::RetrySameProvider
            } else {
                FailoverDecision::SwitchProvider
            };

            let outcome = format!(
                "upstream_body_error: category={} code={} decision={} kind={kind}",
                ErrorCategory::SystemError.as_str(),
                error_code,
                decision.as_str(),
            );

            return record_system_failure_and_decide(RecordSystemFailureArgs {
                ctx,
                provider_ctx,
                attempt_ctx,
                loop_state: LoopState {
                    attempts,
                    failed_provider_ids,
                    last_error_category,
                    last_error_code,
                    circuit_snapshot,
                    abort_guard,
                },
                status: Some(status.as_u16()),
                error_code,
                decision,
                outcome,
                reason: "failed to read upstream body".to_string(),
            })
            .await;
        }
    };

    let outcome = "success".to_string();

    attempts.push(FailoverAttempt {
        provider_id,
        provider_name: provider_ctx_owned.provider_name_base.clone(),
        base_url: provider_ctx_owned.provider_base_url_base.clone(),
        outcome: outcome.clone(),
        status: Some(status.as_u16()),
        provider_index: Some(provider_index),
        retry_index: Some(retry_index),
        session_reuse,
        error_category: None,
        error_code: None,
        decision: Some("success"),
        reason: None,
        selection_method,
        reason_code: Some(reason_code),
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
        Some(status.as_u16()),
    )
    .await;

    body_bytes = maybe_gunzip_response_body_bytes_with_limit(
        body_bytes,
        &mut response_headers,
        MAX_NON_SSE_BODY_BYTES,
    );

    let classified_payload =
        match classify_cx2cc_success_payload(cx2cc_active, &mut response_headers, body_bytes) {
            Ok(classified) => classified,
            Err(err) => {
                tracing::warn!("cx2cc: non-stream event-stream aggregation failed: {err}");
                emit_gateway_log(
                    &state.app,
                    "warn",
                    "CX2CC_RESPONSE_AGGREGATE_FAILED",
                    format!("[CX2CC] non-stream event-stream aggregation failed: {err}"),
                );

                let error_code = GatewayErrorCode::InternalError.as_str();
                let decision = FailoverDecision::SwitchProvider;
                let outcome = format!(
                    "cx2cc_event_stream_aggregate_error: category={} code={} decision={} err={err}",
                    ErrorCategory::SystemError.as_str(),
                    error_code,
                    decision.as_str(),
                );

                return record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
                    ctx,
                    provider_ctx,
                    attempt_ctx,
                    loop_state: LoopState {
                        attempts,
                        failed_provider_ids,
                        last_error_category,
                        last_error_code,
                        circuit_snapshot,
                        abort_guard,
                    },
                    status: Some(status.as_u16()),
                    error_code,
                    decision,
                    outcome,
                    reason: format!("cx2cc event-stream aggregation failed: {err}"),
                })
                .await;
            }
        };

    body_bytes = classified_payload.body_bytes;

    match classified_payload.kind {
        Cx2ccSuccessPayloadKind::NonStreamJson => {
            tracing::info!(
                trace_id = %common.trace_id,
                provider_id,
                cx2cc_active,
                anthropic_stream_requested,
                "handling successful upstream non-stream response"
            );
            if cx2cc_active {
                emit_gateway_log(
                    &state.app,
                    "info",
                    "CX2CC_SUCCESS_NON_STREAM",
                    format!(
                        "[CX2CC] handling successful upstream non-stream response trace_id={} provider_id={} anthropic_stream_requested={}",
                        common.trace_id,
                        provider_id,
                        anthropic_stream_requested
                    ),
                );
            }
        }
        Cx2ccSuccessPayloadKind::HeaderlessEventStream => {
            tracing::info!(
                trace_id = %common.trace_id,
                provider_id,
                anthropic_stream_requested,
                "cx2cc: recovered headerless SSE payload on successful upstream response"
            );
            emit_gateway_log(
                &state.app,
                "info",
                "CX2CC_SSE_HEADER_MISSING",
                format!(
                    "[CX2CC] recovered headerless SSE payload trace_id={} provider_id={} anthropic_stream_requested={}",
                    common.trace_id,
                    provider_id,
                    anthropic_stream_requested
                ),
            );
        }
    }

    if cx2cc_active {
        match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
            Ok(openai_body) => emit_gateway_log(
                &state.app,
                "info",
                "CX2CC_UPSTREAM_BODY_SUMMARY",
                format!(
                    "[CX2CC] upstream body summary trace_id={} provider_id={} {}",
                    common.trace_id,
                    provider_id,
                    summarize_openai_response_json(&openai_body)
                ),
            ),
            Err(err) => emit_gateway_log(
                &state.app,
                "warn",
                "CX2CC_UPSTREAM_BODY_PARSE_FAILED",
                format!(
                    "[CX2CC] upstream body parse failed trace_id={} provider_id={} len={} err={}",
                    common.trace_id,
                    provider_id,
                    body_bytes.len(),
                    err
                ),
            ),
        }
    }

    body_bytes = gemini_oauth::translate_response_body(body_bytes, gemini_oauth_response_mode);
    if gemini_oauth_response_mode.is_some() {
        response_headers.remove(header::CONTENT_LENGTH);
    }

    // CX2CC: translate OpenAI Responses API response → Anthropic Messages API.
    match translate_cx2cc_non_stream_body(
        cx2cc_active,
        anthropic_stream_requested,
        common.requested_model.as_deref(),
        &common.cx2cc_settings,
        &mut response_headers,
        body_bytes,
    ) {
        Ok(translated_body) => {
            body_bytes = translated_body;
            if cx2cc_active {
                tracing::debug!(
                    anthropic_stream_requested,
                    "cx2cc: non-stream response translated OpenAI → Anthropic"
                );
                emit_gateway_log(
                    &state.app,
                    "info",
                    "CX2CC_TRANSLATED_BODY_SUMMARY",
                    format!(
                        "[CX2CC] translated body summary trace_id={} provider_id={} content_type={:?} {}",
                        common.trace_id,
                        provider_id,
                        response_headers
                            .get(header::CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .unwrap_or(""),
                        if anthropic_stream_requested {
                            summarize_translated_anthropic_sse(body_bytes.as_ref())
                        } else {
                            summarize_json_keys(body_bytes.as_ref())
                        }
                    ),
                );
            }
        }
        Err(err) => {
            tracing::warn!("cx2cc: response translation failed: {err}");
            emit_gateway_log(
                &state.app,
                "warn",
                "CX2CC_RESPONSE_TRANSLATE_FAILED",
                format!("[CX2CC] response translation failed: {err}"),
            );

            let error_code = GatewayErrorCode::InternalError.as_str();
            let decision = FailoverDecision::SwitchProvider;
            let outcome = format!(
                "cx2cc_response_translate_error: category={} code={} decision={} err={err}",
                ErrorCategory::SystemError.as_str(),
                error_code,
                decision.as_str(),
            );

            return record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
                ctx,
                provider_ctx,
                attempt_ctx,
                loop_state: LoopState {
                    attempts,
                    failed_provider_ids,
                    last_error_category,
                    last_error_code,
                    circuit_snapshot,
                    abort_guard,
                },
                status: Some(status.as_u16()),
                error_code,
                decision,
                outcome,
                reason: format!("cx2cc response translation failed: {err}"),
            })
            .await;
        }
    }

    let enable_response_fixer_for_this_response = enable_response_fixer
        && !is_event_stream(&response_headers)
        && !has_non_identity_content_encoding(&response_headers);
    if enable_response_fixer_for_this_response {
        response_headers.remove(header::CONTENT_LENGTH);
        let outcome =
            response_fixer::process_non_stream(body_bytes, response_fixer_non_stream_config);
        response_headers.insert(
            "x-cch-response-fixer",
            HeaderValue::from_static(outcome.header_value),
        );
        if let Some(setting) = outcome.special_setting {
            let mut settings = common.special_settings.lock_or_recover();
            settings.push(setting);
        }
        body_bytes = outcome.body;
    }

    let usage = usage::parse_usage_from_json_or_sse_bytes(common.cli_key.as_str(), &body_bytes);
    let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
    let requested_model_for_log = common.requested_model.clone().or_else(|| {
        if body_bytes.is_empty() {
            None
        } else {
            usage::parse_model_from_json_or_sse_bytes(common.cli_key.as_str(), &body_bytes)
        }
    });

    let body = Body::from(body_bytes);
    let mut builder = Response::builder().status(status);
    for (k, v) in response_headers.iter() {
        builder = builder.header(k, v);
    }
    builder = builder.header("x-trace-id", common.trace_id.as_str());

    let out = match builder.body(body) {
        Ok(r) => r,
        Err(_) => {
            let mut fallback = (
                StatusCode::INTERNAL_SERVER_ERROR,
                GatewayErrorCode::ResponseBuildError.as_str(),
            )
                .into_response();
            fallback.headers_mut().insert(
                "x-trace-id",
                HeaderValue::from_str(common.trace_id.as_str())
                    .unwrap_or(HeaderValue::from_static("unknown")),
            );
            fallback
        }
    };

    if out.status() == status {
        let now_unix = now_unix_seconds() as i64;
        let change = provider_router::record_success_and_emit_transition(
            provider_router::RecordCircuitArgs::from_state(
                state,
                common.trace_id.as_str(),
                common.cli_key.as_str(),
                provider_id,
                provider_ctx_owned.provider_name_base.as_str(),
                provider_ctx_owned.provider_base_url_base.as_str(),
                now_unix,
            ),
        );
        if let Some(last) = attempts.last_mut() {
            last.circuit_state_after = Some(change.after.state.as_str());
            last.circuit_failure_count = Some(change.after.failure_count);
            last.circuit_failure_threshold = Some(change.after.failure_threshold);
        }
        if (200..300).contains(&status.as_u16()) {
            if let Some(session_id) = common.session_id.as_deref() {
                state.session.bind_success(
                    &common.cli_key,
                    session_id,
                    provider_id,
                    common.effective_sort_mode_id,
                    now_unix,
                );
            }
        }
    }

    let duration_ms = started.elapsed().as_millis();
    emit_request_event_and_enqueue_request_log(RequestEndArgs {
        deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
        trace_id: common.trace_id.as_str(),
        cli_key: common.cli_key.as_str(),
        method: common.method_hint.as_str(),
        path: common.forwarded_path.as_str(),
        observe: common.observe,
        query: common.query.as_deref(),
        excluded_from_stats: false,
        status: Some(status.as_u16()),
        error_category: None,
        error_code: None,
        duration_ms,
        event_ttfb_ms: Some(duration_ms),
        log_ttfb_ms: Some(duration_ms),
        attempts: attempts.as_slice(),
        special_settings_json: response_fixer::special_settings_json(&common.special_settings),
        session_id: common.session_id.clone(),
        requested_model: requested_model_for_log,
        created_at_ms,
        created_at,
        usage_metrics,
        log_usage_metrics: None,
        usage,
    })
    .await;
    abort_guard.disarm();
    LoopControl::Return(out)
}

#[cfg(test)]
mod tests {
    use super::{
        buffer_cx2cc_event_stream_as_json, classify_cx2cc_success_payload,
        should_passthrough_non_stream_success, translate_cx2cc_non_stream_body,
        Cx2ccSuccessPayloadKind,
    };
    use crate::domain::usage;
    use axum::body::Bytes;
    use axum::http::{header, HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn buffers_cx2cc_event_stream_into_json_response() {
        let raw = concat!(
            "event: response.created\n",
            "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "event: response.output_item.done\n",
            "data: {\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n",
            "event: response.completed\n",
            "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream; charset=utf-8"),
        );
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("999"));

        let body = buffer_cx2cc_event_stream_as_json(
            true,
            &mut headers,
            Bytes::from_static(raw.as_bytes()),
        )
        .unwrap();
        let json: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();

        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert!(headers.get(header::CONTENT_LENGTH).is_none());
        assert_eq!(json["id"], "resp_123");
        assert_eq!(json["status"], "completed");
        assert_eq!(json["output"][0]["content"][0]["text"], "Hello");
    }

    #[test]
    fn buffers_cx2cc_event_stream_without_content_type_header_into_json_response() {
        let raw = concat!(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "data: {\"type\":\"response.output_item.done\",\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello without content-type\"}]}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
        );
        let mut headers = HeaderMap::new();

        let body = buffer_cx2cc_event_stream_as_json(
            true,
            &mut headers,
            Bytes::from_static(raw.as_bytes()),
        )
        .unwrap();
        let json: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();

        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(json["id"], "resp_123");
        assert_eq!(json["status"], "completed");
        assert_eq!(
            json["output"][0]["content"][0]["text"],
            "Hello without content-type"
        );
    }

    #[test]
    fn cx2cc_non_stream_success_never_uses_passthrough_shortcut() {
        assert!(!should_passthrough_non_stream_success(None, false, true));
        assert!(!should_passthrough_non_stream_success(None, true, true));
    }

    #[test]
    fn plain_non_stream_success_can_still_use_passthrough_shortcut() {
        assert!(should_passthrough_non_stream_success(None, false, false));
        assert!(!should_passthrough_non_stream_success(
            Some(crate::gateway::proxy::gemini_oauth::GeminiOAuthResponseMode::GenerateContent),
            false,
            false,
        ));
    }

    #[test]
    fn classifies_headerless_cx2cc_sse_payload_before_logging_non_stream_success() {
        let raw = concat!(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
        );
        let mut headers = HeaderMap::new();

        let classified =
            classify_cx2cc_success_payload(true, &mut headers, Bytes::from_static(raw.as_bytes()))
                .unwrap();

        assert_eq!(
            classified.kind,
            Cx2ccSuccessPayloadKind::HeaderlessEventStream
        );
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let json: serde_json::Value =
            serde_json::from_slice(classified.body_bytes.as_ref()).unwrap();
        assert_eq!(json["id"], "resp_123");
        assert_eq!(json["status"], "completed");
    }

    #[test]
    fn classifies_plain_cx2cc_json_as_non_stream_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let body = Bytes::from_static(br#"{"id":"resp_123","status":"completed"}"#);

        let classified = classify_cx2cc_success_payload(true, &mut headers, body.clone()).unwrap();

        assert_eq!(classified.kind, Cx2ccSuccessPayloadKind::NonStreamJson);
        assert_eq!(classified.body_bytes, body);
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn wraps_cx2cc_non_stream_json_as_anthropic_sse_when_claude_requested_streaming() {
        let openai_body = json!({
            "id": "resp_123",
            "status": "completed",
            "model": "gpt-5",
            "output": [
                {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello from buffered JSON"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7
            }
        });
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("321"));

        let body = translate_cx2cc_non_stream_body(
            true,
            true,
            None,
            &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
            &mut headers,
            Bytes::from(serde_json::to_vec(&openai_body).unwrap()),
        )
        .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream; charset=utf-8"
        );
        assert!(headers.get(header::CONTENT_LENGTH).is_none());
        assert!(text.contains("event: message_start"));
        assert!(text.contains("event: content_block_delta"));
        assert!(text.contains("Hello from buffered JSON"));
        assert!(text.contains("event: message_stop"));
    }

    #[test]
    fn wraps_cx2cc_non_stream_json_as_anthropic_sse_with_requested_model() {
        let openai_body = json!({
            "id": "resp_123",
            "status": "completed",
            "model": "gpt-5",
            "output": [
                {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello from buffered JSON"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7
            }
        });
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let body = translate_cx2cc_non_stream_body(
            true,
            true,
            Some("claude-sonnet-4-5"),
            &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
            &mut headers,
            Bytes::from(serde_json::to_vec(&openai_body).unwrap()),
        )
        .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("\"model\":\"claude-sonnet-4-5\""));
        assert!(!text.contains("\"model\":\"gpt-5\""));
    }

    #[test]
    fn translated_cx2cc_non_stream_sse_preserves_usage_and_model_for_logging() {
        let openai_body = json!({
            "id": "resp_123",
            "status": "completed",
            "model": "gpt-5.3-codex",
            "output": [
                {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello from buffered JSON"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7
            }
        });
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let body = translate_cx2cc_non_stream_body(
            true,
            true,
            Some("claude-opus-4-6"),
            &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
            &mut headers,
            Bytes::from(serde_json::to_vec(&openai_body).unwrap()),
        )
        .unwrap();

        let usage = usage::parse_usage_from_json_or_sse_bytes("claude", body.as_ref())
            .expect("translated SSE should retain usage for request logging");
        let model = usage::parse_model_from_json_or_sse_bytes("claude", body.as_ref())
            .expect("translated SSE should retain model for request logging");

        assert_eq!(usage.metrics.input_tokens, Some(11));
        assert_eq!(usage.metrics.output_tokens, Some(7));
        assert_eq!(model, "claude-opus-4-6");
    }
}
