//! Usage: Handle successful non-SSE upstream responses inside `failover_loop::run`.

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
    let state = ctx.state;
    let cli_key = ctx.cli_key.to_string();
    let method_hint = ctx.method_hint.to_string();
    let forwarded_path = ctx.forwarded_path.to_string();
    let query = ctx.query.clone();
    let trace_id = ctx.trace_id.to_string();
    let started = ctx.started;
    let created_at_ms = ctx.created_at_ms;
    let created_at = ctx.created_at;
    let session_id = ctx.session_id.clone();
    let requested_model = ctx.requested_model.clone();
    let effective_sort_mode_id = ctx.effective_sort_mode_id;
    let special_settings = Arc::clone(ctx.special_settings);
    let provider_cooldown_secs = ctx.provider_cooldown_secs;
    let upstream_request_timeout_non_streaming = ctx.upstream_request_timeout_non_streaming;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;
    let enable_response_fixer = ctx.enable_response_fixer;
    let response_fixer_non_stream_config = ctx.response_fixer_non_stream_config;

    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
    } = provider_ctx;
    let provider_name_base = provider_name_base.to_string();
    let provider_base_url_base = provider_base_url_base.to_string();

    let AttemptCtx {
        attempt_index,
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
        let attempts_json = serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());

        let should_gunzip = has_gzip_content_encoding(&response_headers);

        match resp.content_length() {
            Some(len) if len > MAX_NON_SSE_BODY_BYTES as u64 => {
                let outcome = "success".to_string();

                attempts.push(FailoverAttempt {
                    provider_id,
                    provider_name: provider_name_base.clone(),
                    base_url: provider_base_url_base.clone(),
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
                    status: Some(status.as_u16()),
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
                    error_category: None,
                    error_code: None,
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
                        &trace_id,
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
                    &trace_id,
                    body,
                ));
            }
            None => {
                let outcome = "success".to_string();

                attempts.push(FailoverAttempt {
                    provider_id,
                    provider_name: provider_name_base.clone(),
                    base_url: provider_base_url_base.clone(),
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
                    status: Some(status.as_u16()),
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
                    error_category: None,
                    error_code: None,
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
                builder = builder.header("x-trace-id", trace_id.as_str());

                abort_guard.disarm();
                return LoopControl::Return(match builder.body(body) {
                    Ok(r) => r,
                    Err(_) => {
                        let mut fallback =
                            (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR")
                                .into_response();
                        fallback.headers_mut().insert(
                            "x-trace-id",
                            HeaderValue::from_str(&trace_id)
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
            let category = ErrorCategory::SystemError;
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
                category.as_str(),
                error_code,
                decision.as_str(),
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
                reason: Some("failed to read upstream body".to_string()),
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
                status: Some(status.as_u16()),
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

            *last_error_category = Some(category.as_str());
            *last_error_code = Some(error_code);

            if provider_cooldown_secs > 0
                && matches!(
                    decision,
                    FailoverDecision::SwitchProvider | FailoverDecision::Abort
                )
            {
                let now_unix = now_unix_seconds() as i64;
                let snap =
                    state
                        .circuit
                        .trigger_cooldown(provider_id, now_unix, provider_cooldown_secs);
                *circuit_snapshot = snap;
            }

            match decision {
                FailoverDecision::RetrySameProvider => {
                    return LoopControl::ContinueRetry;
                }
                FailoverDecision::SwitchProvider => {
                    failed_provider_ids.insert(provider_id);
                    return LoopControl::BreakRetry;
                }
                FailoverDecision::Abort => return LoopControl::BreakRetry,
            }
        }
    };

    let outcome = "success".to_string();

    attempts.push(FailoverAttempt {
        provider_id,
        provider_name: provider_name_base.clone(),
        base_url: provider_base_url_base.clone(),
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
        status: Some(status.as_u16()),
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
            if let Ok(mut settings) = special_settings.lock() {
                settings.push(setting);
            }
        }
        body_bytes = outcome.body;
    }

    let usage = usage::parse_usage_from_json_bytes(&body_bytes);
    let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
    let requested_model_for_log = requested_model.clone().or_else(|| {
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
    builder = builder.header("x-trace-id", trace_id.as_str());

    let out = match builder.body(body) {
        Ok(r) => r,
        Err(_) => {
            let mut fallback =
                (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR").into_response();
            fallback.headers_mut().insert(
                "x-trace-id",
                HeaderValue::from_str(&trace_id).unwrap_or(HeaderValue::from_static("unknown")),
            );
            fallback
        }
    };

    if out.status() == status {
        let now_unix = now_unix_seconds() as i64;
        let change = state.circuit.record_success(provider_id, now_unix);
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
        if let Some(last) = attempts.last_mut() {
            last.circuit_state_after = Some(change.after.state.as_str());
            last.circuit_failure_count = Some(change.after.failure_count);
            last.circuit_failure_threshold = Some(change.after.failure_threshold);
        }
        if (200..300).contains(&status.as_u16()) {
            if let Some(session_id) = session_id.as_deref() {
                state.session.bind_success(
                    &cli_key,
                    session_id,
                    provider_id,
                    effective_sort_mode_id,
                    now_unix,
                );
            }
        }
    }

    let attempts_json = serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());
    let duration_ms = started.elapsed().as_millis();
    emit_request_event(
        &state.app,
        trace_id.clone(),
        cli_key.clone(),
        method_hint.clone(),
        forwarded_path.clone(),
        query.clone(),
        Some(status.as_u16()),
        None,
        None,
        duration_ms,
        Some(duration_ms),
        attempts.clone(),
        usage_metrics,
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
            status: Some(status.as_u16()),
            error_code: None,
            duration_ms,
            ttfb_ms: None,
            attempts_json,
            requested_model: requested_model_for_log,
            created_at_ms,
            created_at,
            usage,
        },
    )
    .await;
    abort_guard.disarm();
    LoopControl::Return(out)
}
