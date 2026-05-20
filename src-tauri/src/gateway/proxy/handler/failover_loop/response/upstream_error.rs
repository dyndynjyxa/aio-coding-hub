//! Usage: Handle upstream non-success responses and reqwest errors inside `failover_loop::run`.

use super::attempt_record::{
    record_system_failure_and_decide, record_system_failure_and_decide_no_cooldown,
    RecordSystemFailureArgs,
};
use super::context::{
    AttemptCtx, AttemptOutcome, CommonCtx, CommonCtxOwned, LoopControl, LoopState, ProviderCtx,
    MAX_NON_SSE_BODY_BYTES,
};
use super::thinking_signature_rectifier_400;
use super::{emit_attempt_event_and_log, AttemptCircuitFields};
use super::{
    emit_gateway_log, emit_request_event_and_enqueue_request_log, RequestCompletion,
    RequestEndArgs, RequestEndContextArgs, RequestEndDeps,
};
use crate::circuit_breaker;
use crate::gateway::events::decision_chain as dc;
use crate::gateway::events::FailoverAttempt;
use crate::gateway::proxy::errors::{
    classify_reqwest_error, classify_upstream_status, error_response,
};
use crate::gateway::proxy::failover::{retry_backoff_delay, FailoverDecision};
use crate::gateway::proxy::http_util::{
    build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
    maybe_gunzip_response_body_bytes_with_limit,
};
use crate::gateway::proxy::is_claude_count_tokens_request;
use crate::gateway::proxy::provider_router;
use crate::gateway::proxy::upstream_client_error_rules;
use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};
use crate::gateway::response_fixer;
use crate::gateway::streams::GunzipStream;
use crate::gateway::util::{now_unix_seconds, strip_hop_headers};
use crate::shared::mutex_ext::MutexExt;
use axum::body::{Body, Bytes};
use axum::http::{header, HeaderValue};

fn upstream_error_decision(
    is_count_tokens: bool,
    base_decision: FailoverDecision,
    retry_index: u32,
    max_attempts_per_provider: u32,
) -> FailoverDecision {
    if is_count_tokens {
        return FailoverDecision::Abort;
    }

    if matches!(base_decision, FailoverDecision::RetrySameProvider)
        && retry_index >= max_attempts_per_provider
    {
        return FailoverDecision::SwitchProvider;
    }

    base_decision
}

fn reqwest_error_decision(
    is_count_tokens: bool,
    is_connect: bool,
    retry_index: u32,
    max_attempts_per_provider: u32,
) -> FailoverDecision {
    if is_count_tokens {
        return FailoverDecision::Abort;
    }

    if is_connect {
        return FailoverDecision::SwitchProvider;
    }

    if retry_index < max_attempts_per_provider {
        FailoverDecision::RetrySameProvider
    } else {
        FailoverDecision::SwitchProvider
    }
}

async fn read_response_body_with_limit(
    mut resp: reqwest::Response,
    max_bytes: u64,
) -> Result<Bytes, reqwest::Error> {
    let limit = max_bytes.min(usize::MAX as u64) as usize;
    if limit == 0 {
        return Ok(Bytes::new());
    }

    let mut out = Vec::with_capacity(limit.min(16 * 1024));

    loop {
        let Some(chunk) = resp.chunk().await? else {
            break;
        };

        if out.len() >= limit {
            break;
        }

        let remaining = limit - out.len();
        if chunk.len() > remaining {
            out.extend_from_slice(&chunk[..remaining]);
            break;
        }

        out.extend_from_slice(&chunk);
    }

    Ok(Bytes::from(out))
}

fn error_body_scan_limit_bytes() -> u64 {
    upstream_client_error_rules::max_body_read_bytes().min(MAX_NON_SSE_BODY_BYTES as u64)
}

pub(super) fn error_body_scan_limit_usize() -> usize {
    error_body_scan_limit_bytes().min(usize::MAX as u64) as usize
}

pub(super) async fn read_response_body_for_error_scan(
    resp: reqwest::Response,
) -> Result<Bytes, reqwest::Error> {
    read_response_body_with_limit(resp, error_body_scan_limit_bytes()).await
}

pub(super) struct UpstreamRequestState<'a> {
    pub(super) upstream_body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
    pub(super) codex_previous_response_id_rectifier_retried: &'a mut bool,
    pub(super) thinking_signature_rectifier_retried: &'a mut bool,
    pub(super) thinking_budget_rectifier_retried: &'a mut bool,
}

fn codex_request_has_previous_response_id(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|root| {
            root.get("previous_response_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .map(str::to_string)
        })
        .is_some_and(|value| !value.is_empty())
}

fn should_scan_codex_previous_response_id_error(
    cli_key: &str,
    status: reqwest::StatusCode,
    already_retried: bool,
    upstream_body: &[u8],
) -> bool {
    cli_key == "codex"
        && !already_retried
        && matches!(
            status,
            reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::NOT_FOUND
        )
        && codex_request_has_previous_response_id(upstream_body)
}

fn matches_codex_previous_response_id_error(status: reqwest::StatusCode, body: &[u8]) -> bool {
    if !matches!(
        status,
        reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::NOT_FOUND
    ) {
        return false;
    }
    if body.is_empty() {
        return false;
    }

    let haystack = String::from_utf8_lossy(body).to_ascii_lowercase();
    let mentions_previous_response = haystack.contains("previous_response_id")
        || haystack.contains("previous response")
        || haystack.contains("previous response id");
    let says_missing = haystack.contains("not found")
        || haystack.contains("no response")
        || haystack.contains("could not find")
        || haystack.contains("does not exist")
        || haystack.contains("unknown")
        || haystack.contains("invalid");

    mentions_previous_response && says_missing
}

fn remove_codex_previous_response_id(body: &mut Bytes) -> bool {
    let Ok(mut root) = serde_json::from_slice::<serde_json::Value>(body) else {
        return false;
    };

    let Some(obj) = root.as_object_mut() else {
        return false;
    };
    if obj.remove("previous_response_id").is_none() {
        return false;
    }

    match serde_json::to_vec(&root) {
        Ok(next) => {
            *body = Bytes::from(next);
            true
        }
        Err(_) => false,
    }
}

pub(super) struct HandleNonSuccessResponseInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) provider_ctx: ProviderCtx<'a>,
    pub(super) attempt_ctx: AttemptCtx<'a>,
    pub(super) loop_state: LoopState<'a, R>,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) resp: reqwest::Response,
    pub(super) upstream: UpstreamRequestState<'a>,
}

pub(super) async fn handle_non_success_response<R: tauri::Runtime>(
    input: HandleNonSuccessResponseInput<'_, R>,
) -> LoopControl {
    let HandleNonSuccessResponseInput {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        resp,
        upstream,
    } = input;
    let status = resp.status();
    let response_headers = resp.headers().clone();
    let is_count_tokens =
        is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());

    if !is_count_tokens
        && ctx.cli_key == "claude"
        && status.as_u16() == 400
        && !attempt_ctx.cx2cc_active
        && (enable_thinking_signature_rectifier || enable_thinking_budget_rectifier)
    {
        return thinking_signature_rectifier_400::handle_thinking_rectifiers_400(
            thinking_signature_rectifier_400::HandleThinkingRectifiers400Input {
                ctx,
                provider_ctx,
                attempt_ctx,
                loop_state,
                enable_thinking_signature_rectifier,
                enable_thinking_budget_rectifier,
                resp,
                status,
                response_headers,
                upstream,
            },
        )
        .await;
    }

    let mut resp = Some(resp);

    let state = ctx.state;
    let max_attempts_per_provider = ctx.max_attempts_per_provider;
    let provider_cooldown_secs = ctx.provider_cooldown_secs;

    let ProviderCtx {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_index,
        session_reuse,
        ..
    } = provider_ctx;

    let AttemptCtx {
        attempt_index: _,
        retry_index,
        attempt_started_ms,
        attempt_started,
        circuit_before,
        cx2cc_active,
        ..
    } = attempt_ctx;

    let LoopState {
        attempts,
        failed_provider_ids,
        last_outcome,
        circuit_snapshot,
        abort_guard,
    } = loop_state;

    let (base_category, error_code, base_decision) = classify_upstream_status(status);
    let mut category = base_category;
    let mut decision = upstream_error_decision(
        is_count_tokens,
        base_decision,
        retry_index,
        max_attempts_per_provider,
    );

    let mut abort_body_bytes: Option<Bytes> = None;
    let mut abort_response_headers: Option<axum::http::HeaderMap> = None;
    let mut matched_rule_id: Option<&'static str> = None;
    let mut matched_429_concurrency_limit = false;
    // Body preview for errors where preserving the upstream diagnostic text matters.
    let mut upstream_body_preview: Option<String> = None;
    let need_client_error_scan = !is_count_tokens
        && (upstream_client_error_rules::should_attempt_non_retryable_match(
            status,
            resp.as_ref().and_then(|r| r.content_length()),
        ) || matches!(status.as_u16(), 402 | 429));
    let need_5xx_body_preview =
        !is_count_tokens && status.is_server_error() && !need_client_error_scan;
    let need_codex_previous_response_id_scan = !is_count_tokens
        && should_scan_codex_previous_response_id_error(
            ctx.cli_key.as_str(),
            status,
            *upstream.codex_previous_response_id_rectifier_retried,
            upstream.upstream_body_bytes,
        );
    if need_client_error_scan || need_5xx_body_preview || need_codex_previous_response_id_scan {
        if let Some(r) = resp.take() {
            let read_result = read_response_body_for_error_scan(r).await;
            if let Ok(bytes) = read_result {
                let mut headers_for_scan = response_headers.clone();
                strip_hop_headers(&mut headers_for_scan);
                let body_for_scan = maybe_gunzip_response_body_bytes_with_limit(
                    bytes,
                    &mut headers_for_scan,
                    error_body_scan_limit_usize(),
                );
                // CX2CC: log upstream error body to console for debugging.
                if cx2cc_active && retry_index == 1 {
                    let preview = String::from_utf8_lossy(&body_for_scan);
                    let truncated: String = preview.chars().take(500).collect();
                    emit_gateway_log(
                        &state.app,
                        "warn",
                        "CX2CC_UPSTREAM_ERROR",
                        format!(
                            "[CX2CC] upstream {}: {} (provider={})",
                            status.as_u16(),
                            truncated,
                            provider_name_base,
                        ),
                    );
                }
                // Extract body preview for diagnostics on 5xx and catch-all 4xx.
                if status.is_server_error() || status.is_client_error() {
                    let preview = String::from_utf8_lossy(&body_for_scan);
                    let truncated: String = preview.chars().take(500).collect();
                    if !truncated.is_empty() {
                        upstream_body_preview = Some(truncated);
                    }
                }
                if need_client_error_scan {
                    if matches!(status.as_u16(), 402 | 429)
                        && upstream_client_error_rules::match_quota_exhausted(
                            body_for_scan.as_ref(),
                        )
                    {
                        category = ErrorCategory::ProviderError;
                        decision = FailoverDecision::SwitchProvider;
                        matched_rule_id = Some("quota_exhausted");
                    }
                    if status.as_u16() == 429 {
                        matched_429_concurrency_limit =
                            upstream_client_error_rules::match_429_concurrency_limit(
                                body_for_scan.as_ref(),
                            );
                    }
                    let matched_non_retryable_rule =
                        upstream_client_error_rules::match_non_retryable_client_error(
                            ctx.cli_key.as_str(),
                            status,
                            body_for_scan.as_ref(),
                        );
                    if matched_non_retryable_rule.is_some() {
                        matched_rule_id = matched_non_retryable_rule;
                    }
                    if matched_non_retryable_rule.is_some() || matched_429_concurrency_limit {
                        category = ErrorCategory::NonRetryableClientError;
                        decision = FailoverDecision::Abort;
                    }
                }
                // Preserve consumed body/headers so downstream (e.g. Abort
                // pass-through) can still use them after resp.take().
                if abort_body_bytes.is_none() {
                    abort_body_bytes = Some(body_for_scan);
                    abort_response_headers = Some(headers_for_scan);
                }
            }
        }
    }

    if need_codex_previous_response_id_scan {
        if let Some(body) = abort_body_bytes.as_deref() {
            if matches_codex_previous_response_id_error(status, body)
                && remove_codex_previous_response_id(upstream.upstream_body_bytes)
            {
                *upstream.codex_previous_response_id_rectifier_retried = true;
                *upstream.strip_request_content_encoding = true;
                ctx.special_settings
                    .lock_or_recover()
                    .push(serde_json::json!({
                        "type": "codex_previous_response_id_rectifier",
                        "scope": "attempt",
                        "hit": true,
                        "action": "remove_previous_response_id_and_retry",
                        "providerId": provider_id,
                        "providerName": provider_name_base,
                        "status": status.as_u16(),
                        "retryAttemptNumber": retry_index,
                        "retryAttemptNumberNext": retry_index + 1,
                    }));
                return LoopControl::ContinueRetry;
            }
        }
    }

    if !is_count_tokens
        && upstream_client_error_rules::should_abort_unmatched_client_error(status, matched_rule_id)
    {
        category = ErrorCategory::NonRetryableClientError;
        decision = FailoverDecision::Abort;
        // Extract body preview for diagnostic logging when aborting unmatched 4xx.
        if upstream_body_preview.is_none() {
            if let Some(ref bytes) = abort_body_bytes {
                let preview = String::from_utf8_lossy(bytes);
                let truncated: String = preview.chars().take(500).collect();
                if !truncated.is_empty() {
                    upstream_body_preview = Some(truncated);
                }
            }
        }
    }

    let mut circuit_state_before = Some(circuit_before.state.as_str());
    let mut circuit_state_after: Option<&'static str> = None;
    let mut circuit_failure_count = Some(circuit_before.failure_count);
    let circuit_failure_threshold = Some(circuit_before.failure_threshold);

    let now_unix = now_unix_seconds() as i64;
    if !is_count_tokens && matches!(category, ErrorCategory::ProviderError) {
        let change = provider_router::record_failure_and_emit_transition(
            provider_router::RecordCircuitArgs::from_state(
                state,
                ctx.trace_id.as_str(),
                ctx.cli_key.as_str(),
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

        if change.after.state == circuit_breaker::CircuitState::Open {
            decision = FailoverDecision::SwitchProvider;
        }
    }

    if !is_count_tokens
        && provider_cooldown_secs > 0
        && matches!(category, ErrorCategory::ProviderError)
        && matches!(
            decision,
            FailoverDecision::SwitchProvider | FailoverDecision::Abort
        )
    {
        let snap = provider_router::trigger_cooldown(
            state.circuit.as_ref(),
            provider_id,
            now_unix,
            provider_cooldown_secs,
        );
        *circuit_snapshot = snap;
    }

    let reason = if matched_429_concurrency_limit {
        format!("status={} rule=429_concurrency_limit", status.as_u16())
    } else {
        let base = match matched_rule_id {
            Some(rule_id) => format!("status={} rule={rule_id}", status.as_u16()),
            None => format!("status={}", status.as_u16()),
        };
        match upstream_body_preview {
            Some(ref preview) => format!("{base}, upstream_body={preview}"),
            None => base,
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

    *last_outcome = Some(AttemptOutcome::new(category.as_str(), error_code));

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
            // On abort, we intentionally do NOT use stream tee finalizers, to avoid triggering

            let CommonCtxOwned {
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

            if let (Some(mut response_headers), Some(mut body_bytes)) =
                (abort_response_headers, abort_body_bytes)
            {
                let enable_response_fixer_for_this_response =
                    enable_response_fixer && !has_non_identity_content_encoding(&response_headers);
                if enable_response_fixer_for_this_response {
                    response_headers.remove(header::CONTENT_LENGTH);
                    let outcome = response_fixer::process_non_stream(
                        body_bytes,
                        response_fixer_non_stream_config,
                    );
                    response_headers.insert(
                        "x-cch-response-fixer",
                        HeaderValue::from_static(outcome.header_value),
                    );
                    if let Some(setting) = outcome.special_setting {
                        response_fixer::push_special_setting(&special_settings, setting);
                    }
                    body_bytes = outcome.body;
                }

                let special_settings_json =
                    response_fixer::special_settings_json(&special_settings);
                let duration_ms = started.elapsed().as_millis();

                emit_request_event_and_enqueue_request_log(
                    RequestEndArgs::from_context(RequestEndContextArgs {
                        deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
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
                    Body::from(body_bytes),
                ));
            }

            let special_settings_json = response_fixer::special_settings_json(&special_settings);
            let duration_ms = started.elapsed().as_millis();

            emit_request_event_and_enqueue_request_log(
                RequestEndArgs::from_context(RequestEndContextArgs {
                    deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
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

            let mut response_headers = response_headers;
            strip_hop_headers(&mut response_headers);
            let should_gunzip = has_gzip_content_encoding(&response_headers);
            if should_gunzip {
                // 上游可能无视 accept-encoding: identity 返回 gzip；
                response_headers.remove(header::CONTENT_ENCODING);
                response_headers.remove(header::CONTENT_LENGTH);
            }

            let Some(resp) = resp else {
                let client_attempts = if ctx.verbose_provider_error {
                    attempts.clone()
                } else {
                    vec![]
                };
                return LoopControl::Return(error_response(
                    axum::http::StatusCode::BAD_GATEWAY,
                    trace_id.clone(),
                    GatewayErrorCode::UpstreamReadError.as_str(),
                    "failed to stream upstream error body".to_string(),
                    client_attempts,
                ));
            };
            let body = if should_gunzip {
                let upstream = GunzipStream::new(resp.bytes_stream());
                Body::from_stream(upstream)
            } else {
                Body::from_stream(resp.bytes_stream())
            };

            LoopControl::Return(build_response(
                status,
                &response_headers,
                trace_id.as_str(),
                body,
            ))
        }
    }
}

pub(super) async fn handle_reqwest_error<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    provider_ctx: ProviderCtx<'_>,
    attempt_ctx: AttemptCtx<'_>,
    loop_state: LoopState<'_, R>,
    err: reqwest::Error,
) -> LoopControl {
    tracing::warn!(
        trace_id = %ctx.trace_id,
        cli_key = %ctx.cli_key,
        provider_id = provider_ctx.provider_id,
        provider_name = %provider_ctx.provider_name_base,
        base_url = %provider_ctx.provider_base_url_base,
        is_connect = err.is_connect(),
        is_timeout = err.is_timeout(),
        is_request = err.is_request(),
        "reqwest upstream error: {err}"
    );
    let is_count_tokens =
        is_claude_count_tokens_request(ctx.cli_key.as_str(), ctx.forwarded_path.as_str());
    let is_connect = err.is_connect();
    let (_, error_code) = classify_reqwest_error(&err);
    let decision = reqwest_error_decision(
        is_count_tokens,
        is_connect,
        attempt_ctx.retry_index,
        ctx.max_attempts_per_provider,
    );
    let outcome = format!(
        "request_error: category={} code={} decision={} err={err}",
        ErrorCategory::SystemError.as_str(),
        error_code,
        decision.as_str(),
    );
    let reason = if is_connect {
        "reqwest connect error"
    } else {
        "reqwest error"
    };

    if is_count_tokens {
        return record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
            ctx,
            provider_ctx,
            attempt_ctx,
            loop_state,
            status: None,
            error_code,
            decision,
            outcome,
            reason: reason.to_string(),
        })
        .await;
    }

    record_system_failure_and_decide(RecordSystemFailureArgs {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state,
        status: None,
        error_code,
        decision,
        outcome,
        reason: reason.to_string(),
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        error_body_scan_limit_usize, matches_codex_previous_response_id_error,
        read_response_body_for_error_scan, remove_codex_previous_response_id,
        reqwest_error_decision, should_scan_codex_previous_response_id_error,
        upstream_error_decision, FailoverDecision,
    };
    use axum::body::Bytes;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn known_length_response(
        body: Vec<u8>,
    ) -> (reqwest::Response, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test upstream");
        let addr = listener.local_addr().expect("local addr");
        let task = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut request_buf = [0u8; 1024];
            let _ = socket.read(&mut request_buf).await;
            let headers = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(headers.as_bytes()).await;
            let _ = socket.write_all(&body).await;
        });
        let response = reqwest::Client::new()
            .get(format!("http://{addr}/error"))
            .send()
            .await
            .expect("fetch test response");
        (response, task)
    }

    #[test]
    fn upstream_error_decision_aborts_for_count_tokens() {
        let decision = upstream_error_decision(true, FailoverDecision::RetrySameProvider, 1, 5);
        assert!(matches!(decision, FailoverDecision::Abort));
    }

    #[test]
    fn upstream_error_decision_keeps_base_decision_before_retry_limit() {
        let decision = upstream_error_decision(false, FailoverDecision::RetrySameProvider, 1, 5);
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }

    #[test]
    fn upstream_error_decision_switches_after_retry_limit() {
        let decision = upstream_error_decision(false, FailoverDecision::RetrySameProvider, 5, 5);
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn upstream_error_decision_keeps_switch_and_abort_decisions() {
        let switch_decision =
            upstream_error_decision(false, FailoverDecision::SwitchProvider, 1, 5);
        assert!(matches!(switch_decision, FailoverDecision::SwitchProvider));

        let abort_decision = upstream_error_decision(false, FailoverDecision::Abort, 1, 5);
        assert!(matches!(abort_decision, FailoverDecision::Abort));
    }

    #[tokio::test]
    async fn error_scan_body_reader_truncates_known_length_bodies() {
        let limit = error_body_scan_limit_usize();
        let payload = vec![b'x'; limit + 4096];
        let (response, server_task) = known_length_response(payload).await;

        assert_eq!(response.content_length(), Some((limit + 4096) as u64));
        let body = read_response_body_for_error_scan(response)
            .await
            .expect("read limited body");
        server_task.abort();

        assert_eq!(body.len(), limit);
        assert!(body.iter().all(|byte| *byte == b'x'));
    }

    #[test]
    fn codex_previous_response_id_scan_requires_codex_400_or_404_with_previous_response_id() {
        let body = br#"{"previous_response_id":"resp_old"}"#;

        assert!(should_scan_codex_previous_response_id_error(
            "codex",
            reqwest::StatusCode::BAD_REQUEST,
            false,
            body,
        ));
        assert!(should_scan_codex_previous_response_id_error(
            "codex",
            reqwest::StatusCode::NOT_FOUND,
            false,
            body,
        ));
        assert!(!should_scan_codex_previous_response_id_error(
            "claude",
            reqwest::StatusCode::BAD_REQUEST,
            false,
            body,
        ));
        assert!(!should_scan_codex_previous_response_id_error(
            "codex",
            reqwest::StatusCode::BAD_REQUEST,
            true,
            body,
        ));
        assert!(!should_scan_codex_previous_response_id_error(
            "codex",
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            false,
            body,
        ));
    }

    #[test]
    fn codex_previous_response_id_error_match_is_specific_to_missing_previous_response() {
        assert!(matches_codex_previous_response_id_error(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":{"message":"No response found for previous_response_id resp_old"}}"#,
        ));
        assert!(matches_codex_previous_response_id_error(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":{"message":"No response found with id 'resp_old'","param":"previous_response_id"}}"#,
        ));
        assert!(matches_codex_previous_response_id_error(
            reqwest::StatusCode::NOT_FOUND,
            br#"Previous response id does not exist"#,
        ));
        assert!(!matches_codex_previous_response_id_error(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":{"message":"model is required"}}"#,
        ));
    }

    #[test]
    fn remove_codex_previous_response_id_keeps_other_body_fields() {
        let mut body = Bytes::from_static(
            br#"{"model":"gpt-5","previous_response_id":"resp_old","input":"hello"}"#,
        );

        assert!(remove_codex_previous_response_id(&mut body));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");

        assert_eq!(json.get("previous_response_id"), None);
        assert_eq!(json["model"], "gpt-5");
        assert_eq!(json["input"], "hello");
    }

    #[test]
    fn reqwest_error_decision_aborts_count_tokens_even_for_connect_errors() {
        let decision = reqwest_error_decision(true, true, 1, 5);
        assert!(matches!(decision, FailoverDecision::Abort));
    }

    #[test]
    fn reqwest_error_decision_switches_non_count_tokens_connect_errors() {
        let decision = reqwest_error_decision(false, true, 1, 5);
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn reqwest_error_decision_retries_non_connect_errors_before_limit() {
        let decision = reqwest_error_decision(false, false, 1, 5);
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }

    #[test]
    fn reqwest_error_decision_switches_non_connect_errors_at_limit() {
        let decision = reqwest_error_decision(false, false, 5, 5);
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }
}
