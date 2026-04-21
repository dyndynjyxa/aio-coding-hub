//! Usage: Handle successful event-stream upstream responses inside `failover_loop::run`.

use super::*;
use crate::gateway::proxy::gemini_oauth;
use crate::gateway::proxy::protocol_bridge;
use std::time::Duration;

pub(super) async fn handle_success_event_stream(
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

    let started = common.started;
    let upstream_first_byte_timeout_secs = common.upstream_first_byte_timeout_secs;
    let upstream_first_byte_timeout = common.upstream_first_byte_timeout;
    // Per-provider idle timeout overrides the global setting if configured.
    let upstream_stream_idle_timeout = provider_ctx_owned
        .stream_idle_timeout_seconds
        .and_then(|secs| {
            if secs > 0 {
                Some(Duration::from_secs(secs as u64))
            } else {
                None
            }
        })
        .or(common.upstream_stream_idle_timeout);
    let max_attempts_per_provider = common.max_attempts_per_provider;
    let enable_response_fixer = common.enable_response_fixer;
    let response_fixer_stream_config = common.response_fixer_stream_config;

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
        anthropic_stream_requested: _,
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

    if is_event_stream(&response_headers) {
        strip_hop_headers(&mut response_headers);
        tracing::info!(
            trace_id = %common.trace_id,
            provider_id,
            cx2cc_active,
            "handling successful upstream event-stream response"
        );
        if cx2cc_active {
            emit_gateway_log(
                &common.state.app,
                "info",
                "CX2CC_SUCCESS_EVENT_STREAM",
                format!(
                    "[CX2CC] handling successful upstream event-stream response trace_id={} provider_id={}",
                    common.trace_id, provider_id
                ),
            );
        }

        let mut resp = resp;

        enum FirstChunkProbe {
            Skipped,
            Ok(Option<Bytes>, Option<u128>),
            ReadError(reqwest::Error),
            Timeout,
        }

        let probe = match upstream_first_byte_timeout {
            Some(total) => {
                let elapsed = attempt_started.elapsed();
                if elapsed >= total {
                    FirstChunkProbe::Timeout
                } else {
                    let remaining = total - elapsed;
                    match tokio::time::timeout(remaining, resp.chunk()).await {
                        Ok(Ok(Some(chunk))) => {
                            FirstChunkProbe::Ok(Some(chunk), Some(started.elapsed().as_millis()))
                        }
                        Ok(Ok(None)) => FirstChunkProbe::Ok(None, None),
                        Ok(Err(err)) => FirstChunkProbe::ReadError(err),
                        Err(_) => FirstChunkProbe::Timeout,
                    }
                }
            }
            None => FirstChunkProbe::Skipped,
        };
        let probe_is_empty_event_stream = matches!(probe, FirstChunkProbe::Ok(None, None));

        let mut first_chunk: Option<Bytes> = None;
        let mut initial_first_byte_ms: Option<u128> = None;

        match probe {
            FirstChunkProbe::Ok(chunk, ttfb_ms) => {
                first_chunk = chunk;
                initial_first_byte_ms = ttfb_ms;
            }
            FirstChunkProbe::ReadError(err) => {
                let error_code = GatewayErrorCode::StreamError.as_str();
                let decision = if retry_index < max_attempts_per_provider {
                    FailoverDecision::RetrySameProvider
                } else {
                    FailoverDecision::SwitchProvider
                };

                let outcome = format!(
                    "stream_first_chunk_error: category={} code={} decision={} timeout_secs={}",
                    ErrorCategory::SystemError.as_str(),
                    error_code,
                    decision.as_str(),
                    upstream_first_byte_timeout_secs,
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
                    reason: format!("first chunk read error (event-stream): {err}"),
                })
                .await;
            }
            FirstChunkProbe::Timeout => {
                let error_code = GatewayErrorCode::UpstreamTimeout.as_str();
                let decision = if retry_index < max_attempts_per_provider {
                    FailoverDecision::RetrySameProvider
                } else {
                    FailoverDecision::SwitchProvider
                };

                let outcome = format!(
                    "stream_first_byte_timeout: category={} code={} decision={} timeout_secs={}",
                    ErrorCategory::SystemError.as_str(),
                    error_code,
                    decision.as_str(),
                    upstream_first_byte_timeout_secs,
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
                    reason: "first byte timeout (event-stream)".to_string(),
                })
                .await;
            }
            FirstChunkProbe::Skipped => {}
        }

        if upstream_first_byte_timeout.is_some()
            && first_chunk.is_none()
            && initial_first_byte_ms.is_none()
            && probe_is_empty_event_stream
        {
            let error_code = GatewayErrorCode::StreamError.as_str();
            let decision = if retry_index < max_attempts_per_provider {
                FailoverDecision::RetrySameProvider
            } else {
                FailoverDecision::SwitchProvider
            };

            let outcome = format!(
                "stream_first_chunk_eof: category={} code={} decision={} timeout_secs={}",
                ErrorCategory::SystemError.as_str(),
                error_code,
                decision.as_str(),
                upstream_first_byte_timeout_secs,
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
                reason: "upstream returned empty event-stream".to_string(),
            })
            .await;
        }

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

        let should_gunzip = has_gzip_content_encoding(&response_headers);
        if should_gunzip {
            // 上游可能无视 accept-encoding: identity 返回 gzip；
            response_headers.remove(header::CONTENT_ENCODING);
            response_headers.remove(header::CONTENT_LENGTH);
        }

        let enable_response_fixer_for_this_response =
            enable_response_fixer && !has_non_identity_content_encoding(&response_headers);

        if enable_response_fixer_for_this_response {
            response_headers.remove(header::CONTENT_LENGTH);
            response_headers.insert(
                "x-cch-response-fixer",
                HeaderValue::from_static("processed"),
            );
        }

        let use_sse_relay = common.cli_key == "codex"
            && matches!(
                common.forwarded_path.trim_end_matches('/'),
                "/v1/responses" | "/responses"
            );

        let body = match (enable_response_fixer_for_this_response, should_gunzip) {
            (true, true) => {
                let upstream =
                    GunzipStream::new(FirstChunkStream::new(first_chunk, resp.bytes_stream()));
                let upstream =
                    gemini_oauth::GeminiOAuthSseStream::new(upstream, gemini_oauth_response_mode);
                let upstream = protocol_bridge::stream::BridgeStream::for_cx2cc(
                    upstream,
                    cx2cc_active,
                    common.requested_model.clone(),
                    common.cx2cc_settings.clone(),
                );
                let upstream = response_fixer::ResponseFixerStream::new(
                    upstream,
                    response_fixer_stream_config,
                    common.special_settings.clone(),
                );
                if use_sse_relay {
                    spawn_usage_sse_relay_body(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    )
                } else {
                    let stream = UsageSseTeeStream::new(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    );
                    Body::from_stream(stream)
                }
            }
            (true, false) => {
                let upstream = FirstChunkStream::new(first_chunk, resp.bytes_stream());
                let upstream =
                    gemini_oauth::GeminiOAuthSseStream::new(upstream, gemini_oauth_response_mode);
                let upstream = protocol_bridge::stream::BridgeStream::for_cx2cc(
                    upstream,
                    cx2cc_active,
                    common.requested_model.clone(),
                    common.cx2cc_settings.clone(),
                );
                let upstream = response_fixer::ResponseFixerStream::new(
                    upstream,
                    response_fixer_stream_config,
                    common.special_settings.clone(),
                );
                if use_sse_relay {
                    spawn_usage_sse_relay_body(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    )
                } else {
                    let stream = UsageSseTeeStream::new(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    );
                    Body::from_stream(stream)
                }
            }
            (false, true) => {
                let upstream =
                    GunzipStream::new(FirstChunkStream::new(first_chunk, resp.bytes_stream()));
                let upstream =
                    gemini_oauth::GeminiOAuthSseStream::new(upstream, gemini_oauth_response_mode);
                let upstream = protocol_bridge::stream::BridgeStream::for_cx2cc(
                    upstream,
                    cx2cc_active,
                    common.requested_model.clone(),
                    common.cx2cc_settings.clone(),
                );
                if use_sse_relay {
                    spawn_usage_sse_relay_body(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    )
                } else {
                    let stream = UsageSseTeeStream::new(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    );
                    Body::from_stream(stream)
                }
            }
            (false, false) => {
                let upstream = FirstChunkStream::new(first_chunk, resp.bytes_stream());
                let upstream =
                    gemini_oauth::GeminiOAuthSseStream::new(upstream, gemini_oauth_response_mode);
                let upstream = protocol_bridge::stream::BridgeStream::for_cx2cc(
                    upstream,
                    cx2cc_active,
                    common.requested_model.clone(),
                    common.cx2cc_settings.clone(),
                );
                if use_sse_relay {
                    spawn_usage_sse_relay_body(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    )
                } else {
                    let stream = UsageSseTeeStream::new(
                        upstream,
                        ctx,
                        upstream_stream_idle_timeout,
                        initial_first_byte_ms,
                    );
                    Body::from_stream(stream)
                }
            }
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

    unreachable!("expected event-stream response")
}
