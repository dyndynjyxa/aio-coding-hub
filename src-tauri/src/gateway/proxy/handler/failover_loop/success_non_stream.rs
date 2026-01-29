//! Usage: Handle successful non-SSE upstream responses inside `failover_loop::run`.

use super::super::super::provider_router;
use super::*;

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
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_error_category,
        last_error_code,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    {
        strip_hop_headers(&mut response_headers);

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
                    // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
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
                    // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
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
                        let mut fallback =
                            (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR")
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
                "GW_UPSTREAM_TIMEOUT"
            } else {
                "GW_UPSTREAM_READ_ERROR"
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

    let enable_response_fixer_for_this_response =
        enable_response_fixer && !has_non_identity_content_encoding(&response_headers);
    if enable_response_fixer_for_this_response {
        response_headers.remove(header::CONTENT_LENGTH);
        let outcome =
            response_fixer::process_non_stream(body_bytes, response_fixer_non_stream_config);
        response_headers.insert(
            "x-cch-response-fixer",
            HeaderValue::from_static(outcome.header_value),
        );
        if let Some(setting) = outcome.special_setting {
            if let Ok(mut settings) = common.special_settings.lock() {
                settings.push(setting);
            }
        }
        body_bytes = outcome.body;
    }

    let usage = usage::parse_usage_from_json_bytes(&body_bytes);
    let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
    let requested_model_for_log = common.requested_model.clone().or_else(|| {
        if body_bytes.is_empty() {
            None
        } else {
            usage::parse_model_from_json_bytes(&body_bytes)
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
            let mut fallback =
                (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR").into_response();
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
        query: common.query.as_deref(),
        excluded_from_stats: false,
        status: Some(status.as_u16()),
        error_category: None,
        error_code: None,
        duration_ms,
        event_ttfb_ms: Some(duration_ms),
        log_ttfb_ms: None,
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
