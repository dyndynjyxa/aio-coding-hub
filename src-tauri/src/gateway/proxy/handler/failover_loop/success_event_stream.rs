//! Usage: Handle successful event-stream upstream responses inside `failover_loop::run`.

use super::*;

pub(super) async fn handle_success_event_stream(
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
    let upstream_first_byte_timeout_secs = ctx.upstream_first_byte_timeout_secs;
    let upstream_first_byte_timeout = ctx.upstream_first_byte_timeout;
    let upstream_stream_idle_timeout = ctx.upstream_stream_idle_timeout;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;
    let enable_response_fixer = ctx.enable_response_fixer;
    let response_fixer_stream_config = ctx.response_fixer_stream_config;

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

    if is_event_stream(&response_headers) {
        strip_hop_headers(&mut response_headers);

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
                let category = ErrorCategory::SystemError;
                let error_code = "GW_STREAM_ERROR";
                let decision = if retry_index < max_attempts_per_provider {
                    FailoverDecision::RetrySameProvider
                } else {
                    FailoverDecision::SwitchProvider
                };

                let outcome = format!(
                    "stream_first_chunk_error: category={} code={} decision={} timeout_secs={}",
                    category.as_str(),
                    error_code,
                    decision.as_str(),
                    upstream_first_byte_timeout_secs,
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
                    reason: Some(format!("first chunk read error (event-stream): {err}")),
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
                    let snap = state.circuit.trigger_cooldown(
                        provider_id,
                        now_unix,
                        provider_cooldown_secs,
                    );
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
            FirstChunkProbe::Timeout => {
                let category = ErrorCategory::SystemError;
                let error_code = "GW_UPSTREAM_TIMEOUT";
                let decision = if retry_index < max_attempts_per_provider {
                    FailoverDecision::RetrySameProvider
                } else {
                    FailoverDecision::SwitchProvider
                };

                let outcome = format!(
                    "stream_first_byte_timeout: category={} code={} decision={} timeout_secs={}",
                    category.as_str(),
                    error_code,
                    decision.as_str(),
                    upstream_first_byte_timeout_secs,
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
                    reason: Some("first byte timeout (event-stream)".to_string()),
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
                    let snap = state.circuit.trigger_cooldown(
                        provider_id,
                        now_unix,
                        provider_cooldown_secs,
                    );
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
            FirstChunkProbe::Skipped => {}
        }

        if upstream_first_byte_timeout.is_some()
            && first_chunk.is_none()
            && initial_first_byte_ms.is_none()
            && probe_is_empty_event_stream
        {
            let category = ErrorCategory::SystemError;
            let error_code = "GW_STREAM_ERROR";
            let decision = if retry_index < max_attempts_per_provider {
                FailoverDecision::RetrySameProvider
            } else {
                FailoverDecision::SwitchProvider
            };

            let outcome = format!(
                "stream_first_chunk_eof: category={} code={} decision={} timeout_secs={}",
                category.as_str(),
                error_code,
                decision.as_str(),
                upstream_first_byte_timeout_secs,
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
                reason: Some("upstream returned empty event-stream".to_string()),
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

        let attempts_json = serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());
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

        let should_gunzip = has_gzip_content_encoding(&response_headers);
        if should_gunzip {
            // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
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

        let use_sse_relay = cli_key == "codex" && forwarded_path == "/v1/responses";

        let body = match (enable_response_fixer_for_this_response, should_gunzip) {
            (true, true) => {
                let upstream =
                    GunzipStream::new(FirstChunkStream::new(first_chunk, resp.bytes_stream()));
                let upstream = response_fixer::ResponseFixerStream::new(
                    upstream,
                    response_fixer_stream_config,
                    special_settings.clone(),
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
                let upstream = response_fixer::ResponseFixerStream::new(
                    upstream,
                    response_fixer_stream_config,
                    special_settings.clone(),
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
        builder = builder.header("x-trace-id", trace_id.as_str());

        abort_guard.disarm();
        return LoopControl::Return(match builder.body(body) {
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
        });
    }

    unreachable!("expected event-stream response")
}
