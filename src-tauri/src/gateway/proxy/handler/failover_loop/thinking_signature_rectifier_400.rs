//! Usage: Handle Claude thinking rectifiers (signature/budget) 400 path inside `failover_loop::run`.

use super::*;
use crate::gateway::proxy::provider_router;
use crate::gateway::proxy::upstream_client_error_rules;
use crate::gateway::thinking_budget_rectifier;
use crate::shared::mutex_ext::MutexExt;

pub(super) struct HandleThinkingRectifiers400Input<'a> {
    pub(super) ctx: CommonCtx<'a>,
    pub(super) provider_ctx: ProviderCtx<'a>,
    pub(super) attempt_ctx: AttemptCtx<'a>,
    pub(super) loop_state: LoopState<'a>,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) resp: reqwest::Response,
    pub(super) status: StatusCode,
    pub(super) response_headers: HeaderMap,
    pub(super) upstream: super::upstream_error::UpstreamRequestState<'a>,
}

pub(super) async fn handle_thinking_rectifiers_400(
    input: HandleThinkingRectifiers400Input<'_>,
) -> LoopControl {
    let HandleThinkingRectifiers400Input {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        resp,
        status,
        mut response_headers,
        upstream,
    } = input;
    let upstream_body_bytes = upstream.upstream_body_bytes;
    let strip_request_content_encoding = upstream.strip_request_content_encoding;
    let thinking_signature_rectifier_retried = upstream.thinking_signature_rectifier_retried;
    let thinking_budget_rectifier_retried = upstream.thinking_budget_rectifier_retried;
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
        provider_cooldown_secs,
        max_attempts_per_provider,
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
        ..
    } = ProviderCtxOwned::from(provider_ctx);

    let AttemptCtx {
        attempt_index: _,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
        ..
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_error_category,
        last_error_code,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    if cli_key == "claude"
        && status.as_u16() == 400
        && (enable_thinking_signature_rectifier || enable_thinking_budget_rectifier)
    {
        let buffered_body = match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                let duration_ms = started.elapsed().as_millis();
                let client_attempts = if ctx.verbose_provider_error {
                    attempts.clone()
                } else {
                    vec![]
                };
                let resp = error_response(
                    StatusCode::BAD_GATEWAY,
                    trace_id.clone(),
                    GatewayErrorCode::UpstreamBodyReadError.as_str(),
                    format!("failed to read upstream error body: {err}"),
                    client_attempts,
                );
                emit_request_event_and_enqueue_request_log(RequestEndArgs {
                    deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                    trace_id: trace_id.as_str(),
                    cli_key: cli_key.as_str(),
                    method: method_hint.as_str(),
                    path: forwarded_path.as_str(),
                    observe: ctx.observe,
                    query: query.as_deref(),
                    excluded_from_stats: false,
                    status: Some(StatusCode::BAD_GATEWAY.as_u16()),
                    error_category: Some(ErrorCategory::SystemError.as_str()),
                    error_code: Some(GatewayErrorCode::UpstreamBodyReadError.as_str()),
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
        let signature_trigger = enable_thinking_signature_rectifier
            .then(|| thinking_signature_rectifier::detect_trigger(&upstream_body_text))
            .flatten();
        let budget_trigger = signature_trigger
            .is_none()
            .then(|| thinking_budget_rectifier::detect_trigger(&upstream_body_text))
            .flatten();

        let mut rectified_applied = false;
        let mut rectifier_kind: Option<&'static str> = None;
        let mut rectifier_trigger: Option<&'static str> = None;
        let mut rectifier_retried = false;

        let (base_category, error_code, base_decision) = classify_upstream_status(status);

        let mut matched_rule_id: Option<&'static str> = None;
        let mut category = base_category;
        let mut decision = base_decision;
        let mut should_record_circuit_failure = matches!(category, ErrorCategory::ProviderError);

        if let Some(trigger) = signature_trigger {
            rectifier_kind = Some("thinking_signature_rectifier");
            rectifier_trigger = Some(trigger);

            if *thinking_signature_rectifier_retried {
                rectifier_retried = true;
                should_record_circuit_failure = false;
                category = ErrorCategory::NonRetryableClientError;
                decision = FailoverDecision::Abort;
            } else {
                let mut message_value =
                    serde_json::from_slice::<serde_json::Value>(upstream_body_bytes.as_ref())
                        .or_else(|_| {
                            serde_json::from_slice::<serde_json::Value>(introspection_body)
                        })
                        .unwrap_or(serde_json::Value::Null);
                let rectified = thinking_signature_rectifier::rectify_anthropic_request_message(
                    &mut message_value,
                );

                let mut settings = special_settings.lock_or_recover();
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

                if rectified.applied {
                    if let Ok(next) = serde_json::to_vec(&message_value) {
                        *upstream_body_bytes = Bytes::from(next);
                        *strip_request_content_encoding = true;
                        *thinking_signature_rectifier_retried = true;
                        rectified_applied = true;
                        should_record_circuit_failure = false;
                        decision = FailoverDecision::RetrySameProvider;
                    }
                }

                if !rectified_applied {
                    should_record_circuit_failure = false;
                    category = ErrorCategory::NonRetryableClientError;
                    decision = FailoverDecision::Abort;
                }
            }
        } else if let Some(trigger) = budget_trigger.filter(|_| enable_thinking_budget_rectifier) {
            rectifier_kind = Some("thinking_budget_rectifier");
            rectifier_trigger = Some(trigger);

            if *thinking_budget_rectifier_retried {
                rectifier_retried = true;
                should_record_circuit_failure = false;
                category = ErrorCategory::NonRetryableClientError;
                decision = FailoverDecision::Abort;
            } else {
                let mut message_value =
                    serde_json::from_slice::<serde_json::Value>(upstream_body_bytes.as_ref())
                        .or_else(|_| {
                            serde_json::from_slice::<serde_json::Value>(introspection_body)
                        })
                        .unwrap_or(serde_json::Value::Null);
                let rectified = thinking_budget_rectifier::rectify_anthropic_request_message(
                    &mut message_value,
                );

                let mut settings = special_settings.lock_or_recover();
                settings.push(serde_json::json!({
                    "type": "thinking_budget_rectifier",
                    "scope": "request",
                    "hit": rectified.applied,
                    "providerId": provider_id,
                    "providerName": provider_name_base.clone(),
                    "trigger": trigger,
                    "attemptNumber": retry_index,
                    "retryAttemptNumber": retry_index + 1,
                    "before": {
                        "maxTokens": rectified.before.max_tokens,
                        "thinkingType": rectified.before.thinking_type,
                        "thinkingBudgetTokens": rectified.before.thinking_budget_tokens,
                    },
                    "after": {
                        "maxTokens": rectified.after.max_tokens,
                        "thinkingType": rectified.after.thinking_type,
                        "thinkingBudgetTokens": rectified.after.thinking_budget_tokens,
                    },
                }));

                if rectified.applied {
                    if let Ok(next) = serde_json::to_vec(&message_value) {
                        *upstream_body_bytes = Bytes::from(next);
                        *strip_request_content_encoding = true;
                        *thinking_budget_rectifier_retried = true;
                        rectified_applied = true;
                        should_record_circuit_failure = false;
                        decision = FailoverDecision::RetrySameProvider;
                    }
                }

                if !rectified_applied {
                    should_record_circuit_failure = false;
                    category = ErrorCategory::NonRetryableClientError;
                    decision = FailoverDecision::Abort;
                }
            }
        } else {
            // Fallback: match configured non-retryable client error rules and handle like upstream_error.
            matched_rule_id = upstream_client_error_rules::match_non_retryable_client_error(
                &cli_key,
                status,
                body_for_scan.as_ref(),
            );
            if matched_rule_id.is_some() {
                should_record_circuit_failure = false;
                category = ErrorCategory::NonRetryableClientError;
                decision = FailoverDecision::Abort;
            }
        }

        if matches!(decision, FailoverDecision::RetrySameProvider)
            && retry_index >= max_attempts_per_provider
        {
            decision = FailoverDecision::SwitchProvider;
        }

        let mut circuit_state_before = Some(circuit_before.state.as_str());
        let mut circuit_state_after: Option<&'static str> = None;
        let mut circuit_failure_count = Some(circuit_before.failure_count);
        let circuit_failure_threshold = Some(circuit_before.failure_threshold);

        if should_record_circuit_failure && !rectified_applied {
            let now_unix = now_unix_seconds() as i64;
            let change = provider_router::record_failure_and_emit_transition(
                provider_router::RecordCircuitArgs::from_state(
                    state,
                    trace_id.as_str(),
                    cli_key.as_str(),
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

            if change.after.state == crate::circuit_breaker::CircuitState::Open {
                decision = FailoverDecision::SwitchProvider;
            }

            if provider_cooldown_secs > 0
                && matches!(
                    decision,
                    FailoverDecision::SwitchProvider | FailoverDecision::Abort
                )
            {
                *circuit_snapshot = provider_router::trigger_cooldown(
                    state.circuit.as_ref(),
                    provider_id,
                    now_unix,
                    provider_cooldown_secs,
                );
            }
        }

        let reason = if let Some(rectifier_kind) = rectifier_kind {
            let trigger = rectifier_trigger.unwrap_or("unknown_trigger");
            if rectifier_retried {
                format!(
                    "status={} rectifier={rectifier_kind} trigger={trigger} retried=true",
                    status.as_u16()
                )
            } else {
                format!(
                    "status={} rectifier={rectifier_kind} trigger={trigger}",
                    status.as_u16()
                )
            }
        } else {
            match matched_rule_id {
                Some(rule_id) => format!("status={} rule={rule_id}", status.as_u16()),
                None => format!("status={}", status.as_u16()),
            }
        };
        let outcome = format!(
            "upstream_error: status={} category={} code={} decision={}",
            status.as_u16(),
            category.as_str(),
            error_code,
            decision.as_str()
        );
        let selection_method = dc::selection_method(provider_index, retry_index, session_reuse);
        let reason_code = category.reason_code();

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
            selection_method,
            reason_code: Some(reason_code),
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
                        let mut settings = special_settings.lock_or_recover();
                        settings.push(setting);
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
                    observe: ctx.observe,
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

    unreachable!("expected thinking rectifier 400 path")
}
