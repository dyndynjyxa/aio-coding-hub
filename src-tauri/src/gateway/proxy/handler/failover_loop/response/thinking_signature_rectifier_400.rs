//! Usage: Handle protocol-specific reactive rectifiers for upstream 400 responses.

use super::attempt_executor::grant_repair_retry_slot_if_needed;
use super::*;
use crate::gateway::proxy::provider_router;
use crate::gateway::proxy::upstream_client_error_rules;
use crate::gateway::reactive_rectifier::{self, ReactiveRectifierKind, ReactiveRectifierSettings};
use crate::gateway::streams::UpstreamResponse;
use crate::gateway::{
    gemini_function_id_rectifier, thinking_budget_rectifier, thinking_effort_conflict_rectifier,
};

pub(super) struct HandleThinkingRectifiers400Input<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) provider_ctx: ProviderCtx<'a>,
    pub(super) attempt_ctx: AttemptCtx<'a>,
    pub(super) loop_state: LoopState<'a, R>,
    pub(super) enable_thinking_effort_conflict_rectifier: bool,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) enable_gemini_function_id_rectifier: bool,
    pub(super) resp: UpstreamResponse,
    pub(super) status: StatusCode,
    pub(super) response_headers: HeaderMap,
    pub(super) upstream: super::upstream_error::UpstreamRequestState<'a>,
}

fn rectify_request(
    kind: ReactiveRectifierKind,
    message: &mut serde_json::Value,
    trigger: &'static str,
    provider_id: i64,
    provider_name: &str,
    retry_index: u32,
) -> (bool, serde_json::Value) {
    let common = || {
        serde_json::json!({
            "type": kind.as_str(),
            "scope": "attempt",
            "providerId": provider_id,
            "providerName": provider_name,
            "trigger": trigger,
            "attemptNumber": retry_index,
            "retryAttemptNumber": retry_index + 1,
        })
    };

    match kind {
        ReactiveRectifierKind::ThinkingEffortConflict => {
            let result = thinking_effort_conflict_rectifier::rectify(message);
            let mut audit = common();
            audit["hit"] = serde_json::json!(result.applied);
            audit["removedOutputConfigEffort"] =
                serde_json::json!(result.removed_output_config_effort);
            audit["removedReasoningEffort"] = serde_json::json!(result.removed_reasoning_effort);
            audit["thinkingType"] = serde_json::json!(result.thinking_type);
            (result.applied, audit)
        }
        ReactiveRectifierKind::ThinkingSignature => {
            let result = thinking_signature_rectifier::rectify_anthropic_request_message(message);
            let mut audit = common();
            audit["hit"] = serde_json::json!(result.applied);
            audit["removedThinkingBlocks"] = serde_json::json!(result.removed_thinking_blocks);
            audit["removedRedactedThinkingBlocks"] =
                serde_json::json!(result.removed_redacted_thinking_blocks);
            audit["removedSignatureFields"] = serde_json::json!(result.removed_signature_fields);
            audit["removedTopLevelThinking"] = serde_json::json!(result.removed_top_level_thinking);
            (result.applied, audit)
        }
        ReactiveRectifierKind::ThinkingBudget => {
            let result = thinking_budget_rectifier::rectify_anthropic_request_message(message);
            let mut audit = common();
            audit["hit"] = serde_json::json!(result.applied);
            audit["before"] = serde_json::json!({
                "maxTokens": result.before.max_tokens,
                "thinkingType": result.before.thinking_type,
                "thinkingBudgetTokens": result.before.thinking_budget_tokens,
            });
            audit["after"] = serde_json::json!({
                "maxTokens": result.after.max_tokens,
                "thinkingType": result.after.thinking_type,
                "thinkingBudgetTokens": result.after.thinking_budget_tokens,
            });
            (result.applied, audit)
        }
        ReactiveRectifierKind::GeminiFunctionId => {
            let result = gemini_function_id_rectifier::rectify(message);
            let mut audit = common();
            audit["hit"] = serde_json::json!(result.applied);
            audit["strippedFunctionCallIds"] = serde_json::json!(result.stripped_function_call_ids);
            audit["strippedFunctionResponseIds"] =
                serde_json::json!(result.stripped_function_response_ids);
            (result.applied, audit)
        }
    }
}

pub(super) async fn handle_thinking_rectifiers_400<R: tauri::Runtime>(
    input: HandleThinkingRectifiers400Input<'_, R>,
) -> LoopControl {
    let HandleThinkingRectifiers400Input {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        enable_thinking_effort_conflict_rectifier,
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        enable_gemini_function_id_rectifier,
        resp,
        status,
        mut response_headers,
        upstream,
    } = input;
    let upstream_body_bytes = upstream.upstream_body_bytes;
    let strip_request_content_encoding = upstream.strip_request_content_encoding;
    let thinking_effort_conflict_rectifier_retried =
        upstream.thinking_effort_conflict_rectifier_retried;
    let thinking_signature_rectifier_retried = upstream.thinking_signature_rectifier_retried;
    let thinking_budget_rectifier_retried = upstream.thinking_budget_rectifier_retried;
    let gemini_function_id_rectifier_retried = upstream.gemini_function_id_rectifier_retried;
    let additional_repair_retry_slots = upstream.additional_repair_retry_slots;
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
        provider_health_neutral,
        provider_cooldown_secs,
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
        provider_max_attempts,
        attempt_started_ms,
        attempt_started,
        circuit_before,
        ..
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_outcome,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    if status.as_u16() == 400 {
        let buffered_body =
            match super::upstream_error::read_response_body_for_error_scan(resp).await {
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
                    emit_request_event_and_enqueue_request_log(
                        RequestEndArgs::from_context(RequestEndContextArgs {
                            deps: RequestEndDeps::new(
                                &state.app,
                                &state.db,
                                &state.log_tx,
                                &state.plugin_pipeline,
                                &state.active_requests,
                            ),
                            trace_id: trace_id.as_str(),
                            cli_key: cli_key.as_str(),
                            method: method_hint.as_str(),
                            path: forwarded_path.as_str(),
                            observe: ctx.observe,
                            query: query.as_deref(),
                            excluded_from_stats: false,
                            duration_ms,
                            attempts: attempts.as_slice(),
                            special_settings_json: None,
                            session_id,
                            requested_model,
                            created_at_ms,
                            created_at,
                        })
                        .with_completion(RequestCompletion::failure(
                            StatusCode::BAD_GATEWAY.as_u16(),
                            Some(ErrorCategory::SystemError.as_str()),
                            GatewayErrorCode::UpstreamBodyReadError.as_str(),
                        )),
                    )
                    .await;
                    abort_guard.disarm();
                    return LoopControl::Return(resp);
                }
            };

        let mut headers_for_scan = response_headers.clone();
        let body_for_scan = maybe_gunzip_response_body_bytes_with_limit(
            buffered_body.clone(),
            &mut headers_for_scan,
            super::upstream_error::error_body_scan_limit_usize(),
        );
        let upstream_body_text = String::from_utf8_lossy(body_for_scan.as_ref()).to_string();
        let matched_rectifier = reactive_rectifier::detect(
            cli_key.as_str(),
            &upstream_body_text,
            ReactiveRectifierSettings {
                thinking_effort_conflict: enable_thinking_effort_conflict_rectifier,
                thinking_signature: enable_thinking_signature_rectifier,
                thinking_budget: enable_thinking_budget_rectifier,
                gemini_function_id: enable_gemini_function_id_rectifier,
            },
        );

        let mut rectified_applied = false;
        let mut rectifier_kind: Option<&'static str> = None;
        let mut rectifier_trigger: Option<&'static str> = None;
        let mut rectifier_retried = false;

        let (base_category, error_code, base_decision) = classify_upstream_status(status);

        let mut matched_rule_id: Option<&'static str> = None;
        let mut category = base_category;
        let mut decision = base_decision;
        let mut should_record_circuit_failure = matches!(category, ErrorCategory::ProviderError);

        if let Some(matched) = matched_rectifier.filter(|matched| matched.enabled) {
            rectifier_kind = Some(matched.kind.as_str());
            rectifier_trigger = Some(matched.trigger);
            let already_retried = match matched.kind {
                ReactiveRectifierKind::ThinkingEffortConflict => {
                    *thinking_effort_conflict_rectifier_retried
                }
                ReactiveRectifierKind::ThinkingSignature => *thinking_signature_rectifier_retried,
                ReactiveRectifierKind::ThinkingBudget => *thinking_budget_rectifier_retried,
                ReactiveRectifierKind::GeminiFunctionId => *gemini_function_id_rectifier_retried,
            };

            if already_retried {
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
                let (mutation_applied, mut audit) = rectify_request(
                    matched.kind,
                    &mut message_value,
                    matched.trigger,
                    provider_id,
                    provider_name_base.as_str(),
                    retry_index,
                );

                if mutation_applied {
                    if let Ok(next) = serde_json::to_vec(&message_value) {
                        *upstream_body_bytes = Bytes::from(next);
                        *strip_request_content_encoding = true;
                        match matched.kind {
                            ReactiveRectifierKind::ThinkingEffortConflict => {
                                *thinking_effort_conflict_rectifier_retried = true;
                            }
                            ReactiveRectifierKind::ThinkingSignature => {
                                *thinking_signature_rectifier_retried = true;
                            }
                            ReactiveRectifierKind::ThinkingBudget => {
                                *thinking_budget_rectifier_retried = true;
                            }
                            ReactiveRectifierKind::GeminiFunctionId => {
                                *gemini_function_id_rectifier_retried = true;
                            }
                        }
                        let granted_retry_slot = grant_repair_retry_slot_if_needed(
                            additional_repair_retry_slots,
                            retry_index,
                            provider_max_attempts,
                        );
                        audit["grantedRetrySlot"] = serde_json::json!(granted_retry_slot);
                        rectified_applied = true;
                        should_record_circuit_failure = false;
                        decision = FailoverDecision::RetrySameProvider;
                    }
                }

                response_fixer::push_special_setting(&special_settings, audit);

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
            if matched_rule_id.is_some()
                || upstream_client_error_rules::should_abort_unmatched_client_error(
                    status,
                    matched_rule_id,
                )
            {
                should_record_circuit_failure = false;
                category = ErrorCategory::NonRetryableClientError;
                decision = FailoverDecision::Abort;
            }
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
                )
                .with_provider_health_neutral(provider_health_neutral),
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
                    provider_health_neutral,
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
            circuit_recover_at_unix: None,
            circuit_trigger_error_code: None,
            provider_bridged: Some(provider_ctx.provider_bridged),
            timeout_secs: None,
            reasoning_effort: attempt_ctx.reasoning_effort.map(str::to_string),
            upstream_sent: attempt_ctx.upstream_sent,
            claude_model_mapping: provider_ctx.claude_model_mapping.cloned(),
            model_redirect: provider_ctx.model_redirect.cloned(),
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

        *last_outcome = Some(AttemptOutcome::new(category.as_str(), error_code));

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
                        response_fixer::push_special_setting(&special_settings, setting);
                    }
                    body_to_return = outcome.body;
                }

                let special_settings_json =
                    response_fixer::special_settings_json(&special_settings);
                let duration_ms = started.elapsed().as_millis();

                emit_request_event_and_enqueue_request_log(
                    RequestEndArgs::from_context(RequestEndContextArgs {
                        deps: RequestEndDeps::new(
                            &state.app,
                            &state.db,
                            &state.log_tx,
                            &state.plugin_pipeline,
                            &state.active_requests,
                        ),
                        trace_id: trace_id.as_str(),
                        cli_key: cli_key.as_str(),
                        method: method_hint.as_str(),
                        path: forwarded_path.as_str(),
                        observe: ctx.observe,
                        query: query.as_deref(),
                        excluded_from_stats: false,
                        duration_ms,
                        attempts: attempts.as_slice(),
                        special_settings_json,
                        session_id,
                        requested_model,
                        created_at_ms,
                        created_at,
                    })
                    .with_completion(RequestCompletion::failure_with_ttfb(
                        status.as_u16(),
                        Some(category.as_str()),
                        error_code,
                        duration_ms,
                    )),
                )
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

#[cfg(test)]
mod tests {
    use super::{upstream_client_error_rules, ErrorCategory, FailoverDecision};

    fn thinking_rectifier_fallback_classification(
        status: reqwest::StatusCode,
        matched_rule_id: Option<&'static str>,
    ) -> (ErrorCategory, FailoverDecision, bool) {
        let mut category = ErrorCategory::ProviderError;
        let mut decision = FailoverDecision::RetrySameProvider;
        let mut should_record_circuit_failure = true;

        if matched_rule_id.is_some()
            || upstream_client_error_rules::should_abort_unmatched_client_error(
                status,
                matched_rule_id,
            )
        {
            should_record_circuit_failure = false;
            category = ErrorCategory::NonRetryableClientError;
            decision = FailoverDecision::Abort;
        }

        (category, decision, should_record_circuit_failure)
    }

    #[test]
    fn unmatched_400_does_not_record_circuit_failure() {
        let (category, decision, should_record) =
            thinking_rectifier_fallback_classification(reqwest::StatusCode::BAD_REQUEST, None);

        assert!(matches!(category, ErrorCategory::NonRetryableClientError));
        assert!(matches!(decision, FailoverDecision::Abort));
        assert!(!should_record);
    }

    #[test]
    fn provider_side_402_still_records_circuit_failure() {
        let (category, decision, should_record) =
            thinking_rectifier_fallback_classification(reqwest::StatusCode::PAYMENT_REQUIRED, None);

        assert!(matches!(category, ErrorCategory::ProviderError));
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
        assert!(should_record);
    }
}
