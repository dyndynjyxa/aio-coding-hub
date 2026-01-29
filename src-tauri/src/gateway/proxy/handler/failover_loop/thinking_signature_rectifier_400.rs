//! Usage: Handle Claude thinking-signature rectifier (400) path inside `failover_loop::run`.

use super::super::super::upstream_client_error_rules;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_thinking_signature_rectifier_400(
    ctx: CommonCtx<'_>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_>,
    enable_thinking_signature_rectifier: bool,
    resp: reqwest::Response,
    status: StatusCode,
    mut response_headers: HeaderMap,
    upstream_body_bytes: &mut Bytes,
    strip_request_content_encoding: &mut bool,
    thinking_signature_rectifier_retried: &mut bool,
) -> LoopControl {
    let introspection_body = ctx.introspection_body;

    let CommonCtxOwned {
        state,
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

    let ProviderCtxOwned {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
    } = ProviderCtxOwned::from(provider_ctx);

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
        abort_guard,
    } = loop_state;

    if cli_key == "claude" && enable_thinking_signature_rectifier && status.as_u16() == 400 {
        let buffered_body = match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                let duration_ms = started.elapsed().as_millis();
                let resp = error_response(
                    StatusCode::BAD_GATEWAY,
                    trace_id.clone(),
                    "GW_UPSTREAM_BODY_READ_ERROR",
                    format!("failed to read upstream error body: {err}"),
                    attempts.clone(),
                );
                emit_request_event_and_enqueue_request_log(RequestEndArgs {
                    deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                    trace_id: trace_id.as_str(),
                    cli_key: cli_key.as_str(),
                    method: method_hint.as_str(),
                    path: forwarded_path.as_str(),
                    query: query.as_deref(),
                    excluded_from_stats: false,
                    status: Some(StatusCode::BAD_GATEWAY.as_u16()),
                    error_category: Some(ErrorCategory::SystemError.as_str()),
                    error_code: Some("GW_UPSTREAM_BODY_READ_ERROR"),
                    duration_ms,
                    event_ttfb_ms: None,
                    log_ttfb_ms: None,
                    attempts: attempts.as_slice(),
                    special_settings_json: None,
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
                return LoopControl::Return(resp);
            }
        };

        let mut headers_for_scan = response_headers.clone();
        let body_for_scan = maybe_gunzip_response_body_bytes_with_limit(
            buffered_body.clone(),
            &mut headers_for_scan,
            MAX_NON_SSE_BODY_BYTES,
        );
        let upstream_body_text = String::from_utf8_lossy(body_for_scan.as_ref()).to_string();
        let trigger = thinking_signature_rectifier::detect_trigger(&upstream_body_text);

        let mut rectified_applied = false;
        if let Some(trigger) = trigger {
            if !*thinking_signature_rectifier_retried {
                let mut message_value =
                    match serde_json::from_slice::<serde_json::Value>(introspection_body) {
                        Ok(v) => v,
                        Err(_) => serde_json::Value::Null,
                    };

                let rectified = thinking_signature_rectifier::rectify_anthropic_request_message(
                    &mut message_value,
                );

                if let Ok(mut settings) = special_settings.lock() {
                    settings.push(serde_json::json!({
                        "type": "thinking_signature_rectifier",
                        "scope": "request",
                        "hit": rectified.applied,
                        "providerId": provider_id,
                        "providerName": provider_name_base.clone(),
                        "trigger": trigger,
                        "attemptNumber": retry_index,
                        "retryAttemptNumber": retry_index + 1,
                        "removedThinkingBlocks": rectified.removed_thinking_blocks,
                        "removedRedactedThinkingBlocks": rectified.removed_redacted_thinking_blocks,
                        "removedSignatureFields": rectified.removed_signature_fields,
                        "removedTopLevelThinking": rectified.removed_top_level_thinking,
                    }));
                }

                if rectified.applied {
                    if let Ok(next) = serde_json::to_vec(&message_value) {
                        *upstream_body_bytes = Bytes::from(next);
                        *strip_request_content_encoding = true;
                        *thinking_signature_rectifier_retried = true;
                        rectified_applied = true;
                    }
                }
            }
        }

        let (base_category, error_code, _base_decision) = classify_upstream_status(status);

        let mut matched_rule_id: Option<&'static str> = None;
        let mut category = base_category;
        let mut decision = if rectified_applied {
            FailoverDecision::RetrySameProvider
        } else {
            // Align with claude-code-hub: if it's not a known non-retryable client error,
            // allow switching providers instead of aborting the whole request.
            FailoverDecision::SwitchProvider
        };

        if !rectified_applied {
            matched_rule_id = upstream_client_error_rules::match_non_retryable_client_error(
                &cli_key,
                status,
                body_for_scan.as_ref(),
            );
            if matched_rule_id.is_some() {
                category = ErrorCategory::NonRetryableClientError;
                decision = FailoverDecision::Abort;
            }
        }

        let circuit_state_before = Some(circuit_before.state.as_str());
        let circuit_state_after: Option<&'static str> = None;
        let circuit_failure_count = Some(circuit_before.failure_count);
        let circuit_failure_threshold = Some(circuit_before.failure_threshold);

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
                return LoopControl::ContinueRetry;
            }
            FailoverDecision::SwitchProvider => {
                failed_provider_ids.insert(provider_id);
                return LoopControl::BreakRetry;
            }
            FailoverDecision::Abort => {
                strip_hop_headers(&mut response_headers);
                let mut body_to_return = buffered_body;

                body_to_return = maybe_gunzip_response_body_bytes_with_limit(
                    body_to_return,
                    &mut response_headers,
                    MAX_NON_SSE_BODY_BYTES,
                );

                let enable_response_fixer_for_this_response =
                    enable_response_fixer && !has_non_identity_content_encoding(&response_headers);
                if enable_response_fixer_for_this_response {
                    response_headers.remove(header::CONTENT_LENGTH);
                    let outcome = response_fixer::process_non_stream(
                        body_to_return,
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
                    body_to_return = outcome.body;
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
                    Body::from(body_to_return),
                ));
            }
        }
    }

    unreachable!("expected thinking-signature rectifier path")
}
