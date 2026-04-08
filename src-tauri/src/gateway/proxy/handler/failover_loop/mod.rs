//! Usage: Gateway proxy failover loop (provider iteration + retries + upstream response handling).

mod attempt_record;
mod claude_metadata_user_id_injection;
mod claude_model_mapping;
mod codex_chatgpt;
mod codex_session_id_completion;
mod context;
mod event_helpers;
mod finalize;
mod oauth;
mod provider_gate;
mod provider_limits;
mod request_end_helpers;
mod send;
mod send_timeout;
mod success_event_stream;
mod success_non_stream;
mod thinking_signature_rectifier_400;
mod upstream_error;

use super::super::request_context::RequestContext;
use attempt_record::{
    record_system_failure_and_decide, record_system_failure_and_decide_no_cooldown,
    RecordSystemFailureArgs,
};
use codex_chatgpt::{
    is_codex_chatgpt_backend, maybe_apply_codex_chatgpt_request_compat,
    maybe_inject_codex_chatgpt_headers, original_anthropic_stream_requested,
    parse_codex_chatgpt_account_id, should_apply_claude_model_mapping,
    strip_incompatible_protocol_headers,
};
use event_helpers::{
    emit_attempt_event_and_log, emit_attempt_event_and_log_with_circuit_before,
    AttemptCircuitFields,
};
use oauth::{
    refresh_oauth_credential_after_401, resolve_effective_credential,
    resolve_oauth_adapter_for_provider,
};
use request_end_helpers::{
    emit_request_event_and_enqueue_request_log, RequestEndArgs, RequestEndDeps,
};

use super::super::{
    errors::{classify_upstream_status, error_response},
    failover::{retry_backoff_delay, select_provider_base_url_for_request, FailoverDecision},
    gemini_oauth,
    http_util::{
        build_response, has_gzip_content_encoding, has_non_identity_content_encoding,
        is_event_stream, maybe_gunzip_response_body_bytes_with_limit,
    },
    mark_internal_forwarded_request, ErrorCategory, GatewayErrorCode,
};

use crate::usage;
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gateway::events::{
    decision_chain as dc, emit_attempt_event, emit_gateway_log, FailoverAttempt,
    GatewayAttemptEvent,
};
use crate::gateway::response_fixer;
use crate::gateway::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, TimingOnlyTeeStream,
    UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use crate::gateway::thinking_signature_rectifier;
use crate::gateway::util::{
    body_for_introspection, build_target_url, ensure_cli_required_headers, inject_provider_auth,
    now_unix_seconds, strip_hop_headers,
};
use crate::shared::mutex_ext::MutexExt;

use context::{
    build_stream_finalize_ctx, AttemptCtx, CommonCtx, CommonCtxArgs, CommonCtxOwned, LoopControl,
    LoopState, ProviderCtx, ProviderCtxOwned, MAX_NON_SSE_BODY_BYTES,
};

struct FinalizeOwnedCommon {
    cli_key: String,
    method_hint: String,
    forwarded_path: String,
    query: Option<String>,
    trace_id: String,
    session_id: Option<String>,
    requested_model: Option<String>,
    special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
}

fn finalize_owned_from_input(input: &RequestContext) -> FinalizeOwnedCommon {
    FinalizeOwnedCommon {
        cli_key: input.cli_key.clone(),
        method_hint: input.method_hint.clone(),
        forwarded_path: input.forwarded_path.clone(),
        query: input.query.clone(),
        trace_id: input.trace_id.clone(),
        session_id: input.session_id.clone(),
        requested_model: input.requested_model.clone(),
        special_settings: input.special_settings.clone(),
    }
}

struct SkippedProviderAttempt<'a> {
    provider_id: i64,
    provider_name: &'a str,
    base_url: &'a str,
    error_category: &'static str,
    error_code: &'static str,
    reason: String,
    reason_code: Option<&'static str>,
    attempt_started_ms: u128,
}

fn push_skipped_provider_attempt(
    attempts: &mut Vec<FailoverAttempt>,
    skipped: SkippedProviderAttempt<'_>,
) {
    attempts.push(FailoverAttempt {
        provider_id: skipped.provider_id,
        provider_name: skipped.provider_name.to_string(),
        base_url: skipped.base_url.to_string(),
        outcome: "skipped".to_string(),
        status: None,
        provider_index: None,
        retry_index: None,
        session_reuse: None,
        error_category: Some(skipped.error_category),
        error_code: Some(skipped.error_code),
        decision: Some("skip"),
        reason: Some(skipped.reason),
        selection_method: Some(dc::SELECTION_METHOD_FILTERED),
        reason_code: skipped.reason_code,
        attempt_started_ms: Some(skipped.attempt_started_ms),
        attempt_duration_ms: Some(0),
        circuit_state_before: None,
        circuit_state_after: None,
        circuit_failure_count: None,
        circuit_failure_threshold: None,
    });
}

fn is_gate_only_skipped_attempt(attempt: &FailoverAttempt) -> bool {
    if attempt.decision != Some("skip") {
        return false;
    }

    if attempt.provider_index.is_some() || attempt.retry_index.is_some() {
        return false;
    }

    matches!(
        attempt.reason_code,
        Some(dc::REASON_CIRCUIT_OPEN | dc::REASON_CIRCUIT_COOLDOWN | dc::REASON_RATE_LIMITED)
    )
}

fn should_finalize_as_all_providers_unavailable(attempts: &[FailoverAttempt]) -> bool {
    attempts.is_empty() || attempts.iter().all(is_gate_only_skipped_attempt)
}

fn apply_cx2cc_request_settings(
    responses_body: &mut serde_json::Value,
    cx2cc_settings: &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
) {
    if let Some(ref effort) = cx2cc_settings.model_reasoning_effort {
        responses_body["reasoning"] = serde_json::json!({ "effort": effort });
    }
    if let Some(ref tier) = cx2cc_settings.service_tier {
        responses_body["service_tier"] = serde_json::json!(tier);
    }
    if cx2cc_settings.disable_response_storage {
        responses_body["store"] = serde_json::json!(false);
    }
}

pub(super) async fn run(mut input: RequestContext) -> Response {
    let method = input.req_method.clone();
    let started = input.started;
    let created_at_ms = input.created_at_ms;
    let created_at = input.created_at;

    let introspection_body = body_for_introspection(&input.base_headers, input.body_bytes.as_ref());
    let ctx = CommonCtx::from(CommonCtxArgs {
        state: &input.state,
        cli_key: &input.cli_key,
        forwarded_path: &input.forwarded_path,
        observe: input.observe_request,
        method_hint: &input.method_hint,
        query: &input.query,
        trace_id: &input.trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: &input.session_id,
        requested_model: &input.requested_model,
        cx2cc_settings: &input.cx2cc_settings,
        effective_sort_mode_id: input.effective_sort_mode_id,
        special_settings: &input.special_settings,
        provider_cooldown_secs: input.provider_cooldown_secs,
        upstream_first_byte_timeout_secs: input.upstream_first_byte_timeout_secs,
        upstream_first_byte_timeout: input.upstream_first_byte_timeout,
        upstream_stream_idle_timeout: input.upstream_stream_idle_timeout,
        upstream_request_timeout_non_streaming: input.upstream_request_timeout_non_streaming,
        verbose_provider_error: input.verbose_provider_error,
        max_attempts_per_provider: input.max_attempts_per_provider,
        enable_response_fixer: input.enable_response_fixer,
        response_fixer_stream_config: input.response_fixer_stream_config,
        response_fixer_non_stream_config: input.response_fixer_non_stream_config,
        introspection_body: introspection_body.as_ref(),
    });
    let mut attempts: Vec<FailoverAttempt> = Vec::new();
    let mut failed_provider_ids: HashSet<i64> = HashSet::new();
    let mut last_error_category: Option<&'static str> = None;
    let mut last_error_code: Option<&'static str> = None;

    let max_providers_to_try = (input.max_providers_to_try as usize).max(1);
    let mut providers_tried: usize = 0;
    let mut earliest_available_unix: Option<i64> = None;
    let mut skipped_open: usize = 0;
    let mut skipped_cooldown: usize = 0;
    let mut skipped_limits: usize = 0;
    let anthropic_stream_requested =
        original_anthropic_stream_requested(input.introspection_json.as_ref());

    for provider in input.providers.iter() {
        if providers_tried >= max_providers_to_try {
            break;
        }

        let provider_id = provider.id;
        let provider_name_base = if provider.name.trim().is_empty() {
            format!("Provider #{} (auto-fixed)", provider.id)
        } else {
            provider.name.clone()
        };
        let provider_base_url_display = provider
            .base_urls
            .first()
            .cloned()
            .unwrap_or_else(String::new);

        if failed_provider_ids.contains(&provider_id) {
            continue;
        }

        let skipped_open_before = skipped_open;
        let skipped_cooldown_before = skipped_cooldown;
        let Some(gate_allow) = provider_gate::gate_provider(provider_gate::ProviderGateInput {
            ctx,
            provider_id,
            provider_name_base: &provider_name_base,
            provider_base_url_display: &provider_base_url_display,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
        }) else {
            let (reason_code, reason_label) = if skipped_open > skipped_open_before {
                (Some(dc::REASON_CIRCUIT_OPEN), "open")
            } else if skipped_cooldown > skipped_cooldown_before {
                (Some(dc::REASON_CIRCUIT_COOLDOWN), "cooldown")
            } else {
                (None, "unknown")
            };

            // Record skipped provider (circuit breaker gate)
            push_skipped_provider_attempt(
                &mut attempts,
                SkippedProviderAttempt {
                    provider_id,
                    provider_name: &provider_name_base,
                    base_url: &provider_base_url_display,
                    error_category: "circuit_breaker",
                    error_code: GatewayErrorCode::ProviderCircuitOpen.as_str(),
                    reason: format!("provider skipped by circuit breaker ({reason_label})"),
                    reason_code,
                    attempt_started_ms: started.elapsed().as_millis(),
                },
            );
            continue;
        };

        if !provider_limits::gate_provider(provider_limits::ProviderLimitsInput {
            ctx,
            provider,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_limits: &mut skipped_limits,
        }) {
            // Record skipped provider (rate limit gate)
            push_skipped_provider_attempt(
                &mut attempts,
                SkippedProviderAttempt {
                    provider_id,
                    provider_name: &provider_name_base,
                    base_url: &provider_base_url_display,
                    error_category: "rate_limit",
                    error_code: GatewayErrorCode::ProviderRateLimited.as_str(),
                    reason: "provider skipped by rate limit".to_string(),
                    reason_code: Some(dc::REASON_RATE_LIMITED),
                    attempt_started_ms: started.elapsed().as_millis(),
                },
            );
            continue;
        }

        // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

        // Resolve effective credential (API key or OAuth token with inline refresh).
        // CX2CC providers have no own credential — it will be overridden by the
        // source provider's credential in the CX2CC branch below.
        let mut effective_credential = if provider.source_provider_id.is_some() {
            String::new()
        } else {
            match resolve_effective_credential(&input.state, &input.cli_key, provider).await {
                Ok(value) => value,
                Err(err) => {
                    let err_text = err.to_string();
                    tracing::warn!(
                        trace_id = %input.trace_id,
                        cli_key = %input.cli_key,
                        provider_id = provider_id,
                        provider_name = %provider_name_base,
                        "provider skipped by credential resolution: {}",
                        err_text
                    );
                    push_skipped_provider_attempt(
                        &mut attempts,
                        SkippedProviderAttempt {
                            provider_id,
                            provider_name: &provider_name_base,
                            base_url: &provider_base_url_display,
                            error_category: "auth",
                            error_code: GatewayErrorCode::InternalError.as_str(),
                            reason: format!(
                                "provider skipped by credential resolution: {err_text}"
                            ),
                            reason_code: None,
                            attempt_started_ms: started.elapsed().as_millis(),
                        },
                    );
                    continue;
                }
            }
        };

        // OAuth providers get at least 2 retry attempts (to handle 401 reactive refresh).
        let provider_max_attempts = if provider.auth_mode == "oauth" {
            input.max_attempts_per_provider.max(2)
        } else {
            input.max_attempts_per_provider
        };
        let mut oauth_reactive_refreshed_once = false;

        let mut provider_base_url_base = match select_provider_base_url_for_request(
            &input.state,
            provider,
            &input.cli_key,
            input.provider_base_url_ping_cache_ttl_seconds,
        )
        .await
        {
            Ok(base_url) => base_url,
            Err(err) => {
                tracing::warn!(
                    trace_id = %input.trace_id,
                    cli_key = %input.cli_key,
                    provider_id = provider_id,
                    provider_name = %provider_name_base,
                    "provider skipped by base_url resolution: {}",
                    err
                );
                push_skipped_provider_attempt(
                    &mut attempts,
                    SkippedProviderAttempt {
                        provider_id,
                        provider_name: &provider_name_base,
                        base_url: &provider_base_url_display,
                        error_category: "system",
                        error_code: GatewayErrorCode::InternalError.as_str(),
                        reason: format!("provider skipped by base_url resolution: {err}"),
                        reason_code: None,
                        attempt_started_ms: started.elapsed().as_millis(),
                    },
                );
                continue;
            }
        };

        tracing::debug!(
            trace_id = %input.trace_id,
            cli_key = %input.cli_key,
            provider_id = provider_id,
            provider_name = %provider_name_base,
            auth_mode = %provider.auth_mode,
            base_url_resolved = %provider_base_url_base,
            base_urls_count = provider.base_urls.len(),
            "resolved provider base_url for request"
        );

        // Detect Codex ChatGPT backend for special handling.
        let mut use_codex_chatgpt_backend =
            is_codex_chatgpt_backend(&input.cli_key, provider, &provider_base_url_base);
        let mut codex_chatgpt_account_id = if use_codex_chatgpt_backend {
            // Extract ChatGPT account ID from id_token JWT.
            // The official Codex CLI requires this header for API calls.
            let details = crate::providers::get_oauth_details(&input.state.db, provider.id).ok();
            let account_id = details.and_then(|d| {
                // Try oauth_id_token first, then fall back to oauth_access_token
                // (legacy providers might have id_token stored in oauth_access_token).
                let result = parse_codex_chatgpt_account_id(d.oauth_id_token.as_deref())
                    .or_else(|| parse_codex_chatgpt_account_id(Some(&d.oauth_access_token)));
                tracing::debug!(
                    provider_id = provider.id,
                    has_oauth_id_token = d.oauth_id_token.is_some(),
                    parsed_account_id = ?result,
                    "codex chatgpt account_id extraction"
                );
                result
            });
            account_id
        } else {
            None
        };
        let oauth_adapter = if provider.auth_mode == "oauth" {
            match resolve_oauth_adapter_for_provider(
                &input.cli_key,
                provider.id,
                provider.oauth_provider_type.as_deref(),
            ) {
                Ok(adapter) => Some(adapter),
                Err(err) => {
                    let err_text = err.to_string();
                    tracing::warn!(
                        trace_id = %input.trace_id,
                        cli_key = %input.cli_key,
                        provider_id = provider_id,
                        provider_name = %provider_name_base,
                        "provider skipped by oauth adapter mismatch: {}",
                        err_text
                    );
                    push_skipped_provider_attempt(
                        &mut attempts,
                        SkippedProviderAttempt {
                            provider_id,
                            provider_name: &provider_name_base,
                            base_url: &provider_base_url_display,
                            error_category: "auth",
                            error_code: GatewayErrorCode::InternalError.as_str(),
                            reason: format!(
                                "provider skipped by oauth adapter mismatch: {err_text}"
                            ),
                            reason_code: None,
                            attempt_started_ms: started.elapsed().as_millis(),
                        },
                    );
                    continue;
                }
            }
        } else {
            None
        };

        let mut upstream_forwarded_path = input.forwarded_path.clone();
        let mut upstream_query = input.query.clone();
        let mut upstream_body_bytes = input.body_bytes.clone();
        let mut strip_request_content_encoding = input.strip_request_content_encoding_seed;
        let mut gemini_oauth_response_mode = None;
        let mut thinking_signature_rectifier_retried = false;
        let mut thinking_budget_rectifier_retried = false;

        let is_gemini_oauth = provider.auth_mode == "oauth"
            && oauth_adapter
                .as_ref()
                .map(|adapter| adapter.provider_type() == "gemini_oauth")
                .unwrap_or(false);

        if is_gemini_oauth {
            match gemini_oauth::prepare_upstream_request(
                &input.state.client,
                effective_credential.trim(),
                input.forwarded_path.as_str(),
                input.query.as_deref(),
                input.introspection_json.as_ref(),
                &input.body_bytes,
                input.requested_model.as_deref(),
            )
            .await
            {
                Ok(prepared) => {
                    provider_base_url_base = prepared.base_url;
                    upstream_forwarded_path = prepared.forwarded_path;
                    upstream_query = prepared.query;
                    upstream_body_bytes = prepared.body_bytes;
                    strip_request_content_encoding = prepared.strip_request_content_encoding;
                    gemini_oauth_response_mode = Some(prepared.response_mode);
                }
                Err(err) => {
                    tracing::warn!(
                        trace_id = %input.trace_id,
                        cli_key = %input.cli_key,
                        provider_id = provider_id,
                        provider_name = %provider_name_base,
                        "provider skipped by gemini oauth request translation: {}",
                        err
                    );
                    push_skipped_provider_attempt(
                        &mut attempts,
                        SkippedProviderAttempt {
                            provider_id,
                            provider_name: &provider_name_base,
                            base_url: &provider_base_url_display,
                            error_category: "auth",
                            error_code: GatewayErrorCode::InternalError.as_str(),
                            reason: format!(
                                "provider skipped by gemini oauth request translation: {err}"
                            ),
                            reason_code: None,
                            attempt_started_ms: started.elapsed().as_millis(),
                        },
                    );
                    continue;
                }
            }
        }

        // CX2CC: translate Anthropic → OpenAI Responses API via source provider.
        let mut cx2cc_active = false;
        let mut cx2cc_source: Option<(crate::providers::ProviderForGateway, String)> = None;
        let mut cx2cc_codex_session_id: Option<String> = None;
        if let Some(source_id) = provider.source_provider_id {
            match crate::providers::get_source_provider_for_gateway(&input.state.db, source_id) {
                Ok((source, source_cli_key)) => {
                    // Resolve source provider credential.
                    match resolve_effective_credential(&input.state, &source_cli_key, &source).await
                    {
                        Ok(source_cred) => {
                            // Translate request via protocol bridge (IR path).
                            let body_val: serde_json::Value =
                                serde_json::from_slice(&upstream_body_bytes).unwrap_or_default();
                            let requested_model =
                                body_val.get("model").and_then(|m| m.as_str()).unwrap_or("");
                            let bridge_ctx = super::super::protocol_bridge::BridgeContext {
                                claude_models: provider.claude_models.clone(),
                                cx2cc_settings: input.cx2cc_settings.clone(),
                                requested_model: Some(requested_model.to_string()),
                                mapped_model: None,
                                stream_requested: anthropic_stream_requested,
                                is_chatgpt_backend: false,
                            };
                            match super::super::protocol_bridge::get_bridge("cx2cc")
                                .ok_or_else(|| "cx2cc bridge not registered".to_string())
                                .and_then(|bridge| {
                                    bridge
                                        .translate_request(body_val, &bridge_ctx)
                                        .map_err(|e| e.to_string())
                                }) {
                                Ok(translated) => {
                                    let mut responses_body = translated.body;
                                    apply_cx2cc_request_settings(
                                        &mut responses_body,
                                        &input.cx2cc_settings,
                                    );
                                    let openai_model = responses_body
                                        .get("model")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    upstream_body_bytes = serde_json::to_vec(&responses_body)
                                        .unwrap_or_default()
                                        .into();
                                    upstream_forwarded_path = translated.target_path;
                                    upstream_query = None;
                                    strip_request_content_encoding = true;

                                    // Override base URL and credential with source provider.
                                    match select_provider_base_url_for_request(
                                        &input.state,
                                        &source,
                                        &source_cli_key,
                                        input.provider_base_url_ping_cache_ttl_seconds,
                                    )
                                    .await
                                    {
                                        Ok(url) => provider_base_url_base = url,
                                        Err(err) => {
                                            let msg = format!(
                                                "[CX2CC] source base_url resolution failed: {err} (provider={provider_name_base}, source_id={source_id})"
                                            );
                                            tracing::warn!(
                                                trace_id = %input.trace_id,
                                                provider_id = provider_id,
                                                source_provider_id = source_id,
                                                "cx2cc: source provider base_url resolution failed: {err}"
                                            );
                                            emit_gateway_log(
                                                &input.state.app,
                                                "warn",
                                                "CX2CC_BASE_URL_FAILED",
                                                msg,
                                            );
                                            push_skipped_provider_attempt(
                                                &mut attempts,
                                                SkippedProviderAttempt {
                                                    provider_id,
                                                    provider_name: &provider_name_base,
                                                    base_url: &provider_base_url_display,
                                                    error_category: "translation",
                                                    error_code: GatewayErrorCode::InternalError
                                                        .as_str(),
                                                    reason: format!(
                                                        "cx2cc source base_url failed: {err}"
                                                    ),
                                                    reason_code: None,
                                                    attempt_started_ms: started
                                                        .elapsed()
                                                        .as_millis(),
                                                },
                                            );
                                            continue;
                                        }
                                    }
                                    effective_credential = source_cred;
                                    cx2cc_active = true;
                                    cx2cc_source = Some((source.clone(), source_cli_key.clone()));
                                    cx2cc_codex_session_id =
                                        codex_session_id_completion::apply_if_needed(
                                            codex_session_id_completion::ApplyCodexSessionIdCompletionInput {
                                                ctx,
                                                enabled: input.enable_codex_session_id_completion,
                                                source_cli_key: &source_cli_key,
                                                session_id: input.session_id.as_deref(),
                                                base_headers: &input.base_headers,
                                                upstream_body_bytes: &mut upstream_body_bytes,
                                                strip_request_content_encoding: &mut strip_request_content_encoding,
                                            },
                                        );

                                    // Re-detect Codex ChatGPT backend using source provider.
                                    let cx2cc_is_chatgpt = is_codex_chatgpt_backend(
                                        &source_cli_key,
                                        &source,
                                        &provider_base_url_base,
                                    );
                                    if cx2cc_is_chatgpt {
                                        let details = crate::providers::get_oauth_details(
                                            &input.state.db,
                                            source.id,
                                        )
                                        .ok();
                                        codex_chatgpt_account_id =
                                            details.and_then(|d| {
                                                parse_codex_chatgpt_account_id(
                                                    d.oauth_id_token.as_deref(),
                                                )
                                                .or_else(|| {
                                                    parse_codex_chatgpt_account_id(Some(
                                                        &d.oauth_access_token,
                                                    ))
                                                })
                                            });
                                        use_codex_chatgpt_backend = true;
                                    }

                                    let source_provider_name = if source.name.trim().is_empty() {
                                        format!("Provider #{}", source.id)
                                    } else {
                                        source.name.clone()
                                    };

                                    tracing::info!(
                                        trace_id = %input.trace_id,
                                        provider_id = provider_id,
                                        source_provider_id = source_id,
                                        openai_model = %openai_model,
                                        "cx2cc: request translated Anthropic → OpenAI Responses API"
                                    );
                                    emit_gateway_log(
                                        &input.state.app,
                                        "info",
                                        "CX2CC_TRANSLATED",
                                        format!(
                                            "[CX2CC] translated → model={openai_model}, bridge={provider_name_base}, source={source_provider_name}"
                                        ),
                                    );
                                    {
                                        let mut settings = input.special_settings.lock_or_recover();
                                        settings.push(serde_json::json!({
                                            "type": "cx2cc_cost_basis",
                                            "scope": "request",
                                            "source_cli_key": source_cli_key,
                                            "source_provider_id": source_id,
                                            "source_provider_name": source_provider_name,
                                            "priced_model": openai_model,
                                        }));
                                    }
                                    // DEBUG: dump translated body for troubleshooting.
                                    {
                                        let debug_body: serde_json::Value =
                                            serde_json::from_slice(&upstream_body_bytes)
                                                .unwrap_or_default();
                                        let has_instructions =
                                            debug_body.get("instructions").is_some();
                                        let instructions_val = debug_body
                                            .get("instructions")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("<MISSING>");
                                        let model_val = debug_body
                                            .get("model")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("<MISSING>");
                                        let keys: Vec<&str> = debug_body
                                            .as_object()
                                            .map(|m| m.keys().map(|k| k.as_str()).collect())
                                            .unwrap_or_default();
                                        emit_gateway_log(
                                            &input.state.app,
                                            "debug",
                                            "CX2CC_REQUEST_BODY",
                                            format!(
                                                "[CX2CC] keys={keys:?} has_instructions={has_instructions} instructions_len={} model={model_val}",
                                                instructions_val.len(),
                                            ),
                                        );
                                    }
                                }
                                Err(err) => {
                                    let msg = format!(
                                        "[CX2CC] request translation failed: {err} (provider={provider_name_base})"
                                    );
                                    tracing::warn!(
                                        trace_id = %input.trace_id,
                                        provider_id = provider_id,
                                        "cx2cc: request translation failed: {err}"
                                    );
                                    emit_gateway_log(
                                        &input.state.app,
                                        "warn",
                                        "CX2CC_TRANSLATE_FAILED",
                                        msg,
                                    );
                                    push_skipped_provider_attempt(
                                        &mut attempts,
                                        SkippedProviderAttempt {
                                            provider_id,
                                            provider_name: &provider_name_base,
                                            base_url: &provider_base_url_display,
                                            error_category: "translation",
                                            error_code: GatewayErrorCode::InternalError.as_str(),
                                            reason: format!("cx2cc translation failed: {err}"),
                                            reason_code: None,
                                            attempt_started_ms: started.elapsed().as_millis(),
                                        },
                                    );
                                    continue;
                                }
                            }
                        }
                        Err(err) => {
                            let msg = format!(
                                "[CX2CC] source credential resolution failed: {err} (provider={provider_name_base}, source_id={source_id})"
                            );
                            tracing::warn!(
                                trace_id = %input.trace_id,
                                provider_id = provider_id,
                                source_provider_id = source_id,
                                "cx2cc: source provider credential resolution failed: {err}"
                            );
                            emit_gateway_log(
                                &input.state.app,
                                "warn",
                                "CX2CC_CREDENTIAL_FAILED",
                                msg,
                            );
                            push_skipped_provider_attempt(
                                &mut attempts,
                                SkippedProviderAttempt {
                                    provider_id,
                                    provider_name: &provider_name_base,
                                    base_url: &provider_base_url_display,
                                    error_category: "auth",
                                    error_code: GatewayErrorCode::InternalError.as_str(),
                                    reason: format!(
                                        "cx2cc source provider credential failed: {err}"
                                    ),
                                    reason_code: None,
                                    attempt_started_ms: started.elapsed().as_millis(),
                                },
                            );
                            continue;
                        }
                    }
                }
                Err(err) => {
                    let msg = format!(
                        "[CX2CC] source provider not found: {err} (provider={provider_name_base}, source_id={source_id})"
                    );
                    tracing::warn!(
                        trace_id = %input.trace_id,
                        provider_id = provider_id,
                        source_provider_id = source_id,
                        "cx2cc: source provider not found: {err}"
                    );
                    emit_gateway_log(&input.state.app, "warn", "CX2CC_SOURCE_NOT_FOUND", msg);
                    push_skipped_provider_attempt(
                        &mut attempts,
                        SkippedProviderAttempt {
                            provider_id,
                            provider_name: &provider_name_base,
                            base_url: &provider_base_url_display,
                            error_category: "config",
                            error_code: GatewayErrorCode::InternalError.as_str(),
                            reason: format!("cx2cc source provider not found: {err}"),
                            reason_code: None,
                            attempt_started_ms: started.elapsed().as_millis(),
                        },
                    );
                    continue;
                }
            }
        }

        let mut circuit_snapshot = gate_allow.circuit_after;

        providers_tried = providers_tried.saturating_add(1);
        let provider_index = providers_tried as u32;
        let session_reuse = match input.session_bound_provider_id {
            Some(id) => (id == provider_id && provider_index == 1).then_some(true),
            None => None,
        };
        let provider_ctx = ProviderCtx {
            provider_id,
            provider_name_base: &provider_name_base,
            provider_base_url_base: &provider_base_url_base,
            provider_index,
            session_reuse,
            stream_idle_timeout_seconds: provider.stream_idle_timeout_seconds,
        };

        if should_apply_claude_model_mapping(cx2cc_active, &upstream_forwarded_path) {
            claude_model_mapping::apply_if_needed(
                ctx,
                provider,
                provider_ctx,
                input.requested_model_location,
                input.introspection_json.as_ref(),
                claude_model_mapping::UpstreamRequestMut {
                    forwarded_path: &mut upstream_forwarded_path,
                    query: &mut upstream_query,
                    body_bytes: &mut upstream_body_bytes,
                    strip_request_content_encoding: &mut strip_request_content_encoding,
                },
            );
        }

        claude_metadata_user_id_injection::apply_if_needed(
            claude_metadata_user_id_injection::ApplyClaudeMetadataUserIdInjectionInput {
                ctx,
                provider_id,
                enabled: input.enable_claude_metadata_user_id_injection,
                session_id: input.session_id.as_deref(),
                base_headers: &input.base_headers,
                forwarded_path: upstream_forwarded_path.as_str(),
                upstream_body_bytes: &mut upstream_body_bytes,
                strip_request_content_encoding: &mut strip_request_content_encoding,
            },
        );

        // Codex ChatGPT backend: normalize path and enforce store=false.
        if use_codex_chatgpt_backend {
            maybe_apply_codex_chatgpt_request_compat(
                &mut upstream_forwarded_path,
                &mut upstream_body_bytes,
                &mut strip_request_content_encoding,
            );
        }

        // Claude API key providers: some relays only accept one auth scheme.
        // We default to Anthropic-style `x-api-key`, and fallback to
        // `Authorization: Bearer` once on 401/403.
        let mut claude_api_key_bearer_fallback = false;

        for retry_index in 1..=provider_max_attempts {
            let attempt_index = attempts.len().saturating_add(1) as u32;
            let attempt_started_ms = started.elapsed().as_millis();
            let attempt_started = Instant::now();
            let circuit_before = circuit_snapshot.clone();
            let attempt_ctx = AttemptCtx {
                attempt_index,
                retry_index,
                attempt_started_ms,
                attempt_started,
                circuit_before: &circuit_before,
                gemini_oauth_response_mode,
                cx2cc_active,
                anthropic_stream_requested,
            };

            let url = match build_target_url(
                &provider_base_url_base,
                &upstream_forwarded_path,
                upstream_query.as_deref(),
            ) {
                Ok(u) => u,
                Err(err) => {
                    tracing::warn!(
                        trace_id = %input.trace_id,
                        cli_key = %input.cli_key,
                        provider_id = provider_id,
                        provider_name = %provider_name_base,
                        base_url = %provider_base_url_base,
                        forwarded_path = %upstream_forwarded_path,
                        "build_target_url failed: {err}"
                    );
                    let category = ErrorCategory::SystemError;
                    let error_code = GatewayErrorCode::InternalError.as_str();
                    let decision = FailoverDecision::SwitchProvider;

                    let outcome = format!(
                        "build_target_url_error: category={} code={} decision={} err={err}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
                    );
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match record_system_failure_and_decide_no_cooldown(RecordSystemFailureArgs {
                        ctx,
                        provider_ctx,
                        attempt_ctx,
                        loop_state,
                        status: None,
                        error_code,
                        decision,
                        outcome,
                        reason: format!("invalid base_url: {err}"),
                    })
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
            };

            // Realtime routing UX: emit an attempt event as soon as a provider is selected (before awaiting upstream).
            //
            // Note: do NOT enqueue attempt_logs for this "started" event (avoid DB noise/IO); completion events still get persisted.
            let started_attempt = FailoverAttempt {
                provider_id,
                provider_name: provider_name_base.clone(),
                base_url: provider_base_url_base.clone(),
                outcome: "started".to_string(),
                status: None,
                provider_index: Some(provider_index),
                retry_index: Some(retry_index),
                session_reuse,
                error_category: None,
                error_code: None,
                decision: None,
                reason: None,
                selection_method: dc::selection_method(provider_index, retry_index, session_reuse),
                reason_code: None,
                attempt_started_ms: Some(attempt_started_ms),
                attempt_duration_ms: Some(0),
                circuit_state_before: Some(circuit_before.state.as_str()),
                circuit_state_after: None,
                circuit_failure_count: Some(circuit_before.failure_count),
                circuit_failure_threshold: Some(circuit_before.failure_threshold),
            };
            input
                .abort_guard
                .capture_in_flight_attempt(&started_attempt);
            if input.observe_request {
                emit_attempt_event(
                    &input.state.app,
                    GatewayAttemptEvent {
                        trace_id: input.trace_id.clone(),
                        cli_key: input.cli_key.clone(),
                        session_id: input.session_id.clone(),
                        method: input.method_hint.clone(),
                        path: input.forwarded_path.clone(),
                        query: input.query.clone(),
                        requested_model: input.requested_model.clone(),
                        attempt_index,
                        provider_id,
                        session_reuse,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: "started".to_string(),
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: 0,
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    },
                );
            }

            let mut headers = input.base_headers.clone();
            ensure_cli_required_headers(&input.cli_key, &mut headers);
            if input.cli_key == "claude" {
                mark_internal_forwarded_request(&mut headers);
            }
            codex_session_id_completion::inject_session_headers_if_needed(
                &mut headers,
                cx2cc_codex_session_id.as_deref(),
            );

            // Always clear all auth headers from base_headers first to prevent
            // client-sent tokens leaking to upstream (fail-closed, not fail-open).
            headers.remove(header::AUTHORIZATION);
            headers.remove("x-api-key");
            headers.remove("x-goog-api-key");
            headers.remove("x-goog-api-client");

            let upstream_cli_key = if cx2cc_active {
                cx2cc_source
                    .as_ref()
                    .map(|(_, source_cli_key)| source_cli_key.as_str())
                    .unwrap_or("codex")
            } else {
                input.cli_key.as_str()
            };
            strip_incompatible_protocol_headers(
                input.cli_key.as_str(),
                upstream_cli_key,
                &mut headers,
            );

            // For OAuth providers, use the adapter's inject_upstream_headers which adds
            // provider-specific headers (e.g., originator for Codex, anthropic-beta for Claude).
            // For api_key providers, use the legacy inject_provider_auth.
            if provider.auth_mode == "oauth" {
                let cred_trimmed = effective_credential.trim();
                tracing::debug!(
                    provider_id = provider.id,
                    cli_key = %input.cli_key,
                    oauth_provider_type = ?provider.oauth_provider_type,
                    credential_len = cred_trimmed.len(),
                    "injecting OAuth upstream headers"
                );
                match oauth_adapter {
                    Some(adapter) => {
                        if let Err(e) = adapter.inject_upstream_headers(&mut headers, cred_trimmed)
                        {
                            // Adapter injection failure = skip this provider, not silently proceed
                            // with no auth header (which would give upstream a wrong 401 anyway).
                            tracing::warn!(
                                provider_id = provider.id,
                                cli_key = %input.cli_key,
                                "OAuth inject_upstream_headers failed, skipping provider: {e}"
                            );
                            attempts.push(FailoverAttempt {
                                provider_id,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_display.clone(),
                                outcome: format!("oauth_inject_failed: {e}"),
                                status: Some(500),
                                provider_index: Some(attempt_index),
                                retry_index: Some(retry_index),
                                session_reuse: None,
                                error_category: Some("auth"),
                                error_code: Some(GatewayErrorCode::InternalError.as_str()),
                                decision: Some("switch"),
                                reason: Some(format!("OAuth header injection failed: {e}")),
                                selection_method: None,
                                reason_code: None,
                                attempt_started_ms: Some(attempt_started_ms),
                                attempt_duration_ms: Some(0),
                                circuit_state_before: Some(circuit_before.state.as_str()),
                                circuit_state_after: None,
                                circuit_failure_count: Some(circuit_before.failure_count),
                                circuit_failure_threshold: Some(circuit_before.failure_threshold),
                            });
                            break; // break retry loop, switch provider
                        }
                    }
                    None => {
                        // oauth_adapter=None means adapter mismatch was already caught above;
                        // we should not reach here. Treat as skip.
                        tracing::warn!(
                            provider_id = provider.id,
                            "oauth_adapter is None at injection point (should have been skipped earlier)"
                        );
                        break;
                    }
                }
            } else {
                let auth_cli_key = if cx2cc_active {
                    "codex"
                } else {
                    &input.cli_key
                };
                inject_provider_auth(auth_cli_key, effective_credential.trim(), &mut headers);

                if !cx2cc_active && auth_cli_key == "claude" {
                    if claude_api_key_bearer_fallback {
                        // Bearer-only fallback for strict relays.
                        let value = format!("Bearer {}", effective_credential.trim());
                        if let Ok(header_value) = HeaderValue::from_str(&value) {
                            headers.remove("x-api-key");
                            headers.insert(header::AUTHORIZATION, header_value);
                        }
                    }

                    if retry_index == 1 || claude_api_key_bearer_fallback {
                        let mut settings = ctx.special_settings.lock_or_recover();
                        settings.push(serde_json::json!({
                            "type": "claude_auth_injection",
                            "scope": "attempt",
                            "providerId": provider_id,
                            "providerName": provider_name_base.clone(),
                            "retryAttemptNumber": retry_index,
                            "mode": if claude_api_key_bearer_fallback { "authorization_bearer" } else { "x_api_key" },
                        }));
                    }
                }
            }
            if use_codex_chatgpt_backend {
                maybe_inject_codex_chatgpt_headers(
                    &mut headers,
                    codex_chatgpt_account_id.as_deref(),
                );
            }
            if strip_request_content_encoding {
                headers.remove(header::CONTENT_ENCODING);
            }

            // Fix empty text blocks for Claude OAuth
            let cleaned_body = if input.cli_key == "claude" && provider.auth_mode == "oauth" {
                if let Ok(mut json) =
                    serde_json::from_slice::<serde_json::Value>(&upstream_body_bytes)
                {
                    if let Some(messages) = json.get_mut("messages").and_then(|v| v.as_array_mut())
                    {
                        for msg in messages {
                            if let Some(content) =
                                msg.get_mut("content").and_then(|v| v.as_array_mut())
                            {
                                content.retain(|block| {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        !text.trim().is_empty()
                                    } else {
                                        true
                                    }
                                });
                            }
                        }
                    }
                    serde_json::to_vec(&json)
                        .unwrap_or_else(|_| upstream_body_bytes.to_vec())
                        .into()
                } else {
                    upstream_body_bytes.clone()
                }
            } else {
                upstream_body_bytes.clone()
            };

            let send_result =
                send::send_upstream(ctx, method.clone(), url, headers, cleaned_body).await;

            match send_result {
                send::SendResult::Ok(resp) => {
                    let status = resp.status();
                    let response_headers = resp.headers().clone();
                    let response_content_type = response_headers
                        .get(header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("");
                    tracing::info!(
                        trace_id = %input.trace_id,
                        provider_id = provider_id,
                        status = status.as_u16(),
                        content_type = response_content_type,
                        event_stream = is_event_stream(&response_headers),
                        cx2cc_active,
                        anthropic_stream_requested,
                        "upstream response received"
                    );
                    if cx2cc_active {
                        let source_provider_id = cx2cc_source.as_ref().map(|(source, _)| source.id);
                        let source_provider_name = cx2cc_source
                            .as_ref()
                            .map(|(source, _)| {
                                if source.name.trim().is_empty() {
                                    format!("Provider #{}", source.id)
                                } else {
                                    source.name.clone()
                                }
                            })
                            .unwrap_or_else(|| "<unknown>".to_string());
                        emit_gateway_log(
                            &input.state.app,
                            "info",
                            "CX2CC_UPSTREAM_RESPONSE",
                            format!(
                                "[CX2CC] upstream response received trace_id={} bridge_provider_id={} source_provider_id={} source_provider={} status={} content_type={:?} event_stream={} anthropic_stream_requested={}",
                                input.trace_id,
                                provider_id,
                                source_provider_id
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "-".to_string()),
                                source_provider_name,
                                status.as_u16(),
                                response_content_type,
                                is_event_stream(&response_headers),
                                anthropic_stream_requested
                            ),
                        );
                    }

                    if status.is_success() {
                        if (anthropic_stream_requested || !cx2cc_active)
                            && is_event_stream(&response_headers)
                        {
                            let loop_state = LoopState::new(
                                &mut attempts,
                                &mut failed_provider_ids,
                                &mut last_error_category,
                                &mut last_error_code,
                                &mut circuit_snapshot,
                                &mut input.abort_guard,
                            );
                            match success_event_stream::handle_success_event_stream(
                                ctx,
                                provider_ctx,
                                attempt_ctx,
                                loop_state,
                                resp,
                                status,
                                response_headers,
                            )
                            .await
                            {
                                LoopControl::ContinueRetry => continue,
                                LoopControl::BreakRetry => break,
                                LoopControl::Return(resp) => return resp,
                            }
                        }

                        let loop_state = LoopState::new(
                            &mut attempts,
                            &mut failed_provider_ids,
                            &mut last_error_category,
                            &mut last_error_code,
                            &mut circuit_snapshot,
                            &mut input.abort_guard,
                        );
                        match success_non_stream::handle_success_non_stream(
                            ctx,
                            provider_ctx,
                            attempt_ctx,
                            loop_state,
                            resp,
                            status,
                            response_headers,
                        )
                        .await
                        {
                            LoopControl::ContinueRetry => continue,
                            LoopControl::BreakRetry => break,
                            LoopControl::Return(resp) => return resp,
                        }
                    }

                    // Claude API key auth scheme fallback:
                    // Some relay services only accept `Authorization: Bearer`, not `x-api-key`.
                    if input.cli_key == "claude"
                        && provider.auth_mode != "oauth"
                        && !cx2cc_active
                        && !claude_api_key_bearer_fallback
                        && retry_index < provider_max_attempts
                        && matches!(status.as_u16(), 401 | 403)
                    {
                        claude_api_key_bearer_fallback = true;
                        let mut settings = ctx.special_settings.lock_or_recover();
                        settings.push(serde_json::json!({
                            "type": "claude_auth_injection",
                            "scope": "attempt",
                            "hit": true,
                            "action": "fallback_to_authorization_bearer",
                            "providerId": provider_id,
                            "providerName": provider_name_base.clone(),
                            "status": status.as_u16(),
                            "retryAttemptNumber": retry_index,
                            "retryAttemptNumberNext": retry_index + 1,
                        }));
                        continue;
                    }

                    // OAuth 401 reactive refresh: if we get a 401 on an OAuth provider,
                    // try refreshing the token once and retry.
                    // For CX2CC providers, refresh the source provider's OAuth token instead.
                    if status.as_u16() == 401 && !oauth_reactive_refreshed_once {
                        let refresh_target: Option<(&crate::providers::ProviderForGateway, &str)> =
                            if provider.auth_mode == "oauth" {
                                Some((provider, &input.cli_key))
                            } else if cx2cc_active {
                                cx2cc_source.as_ref().and_then(|(src, src_key)| {
                                    if src.auth_mode == "oauth" {
                                        Some((src, src_key.as_str()))
                                    } else {
                                        None
                                    }
                                })
                            } else {
                                None
                            };

                        if let Some((target_provider, target_cli_key)) = refresh_target {
                            oauth_reactive_refreshed_once = true;
                            tracing::info!(
                                provider_id = provider.id,
                                target_provider_id = target_provider.id,
                                cx2cc_active,
                                cli_key = %target_cli_key,
                                "oauth 401 detected, attempting reactive token refresh"
                            );
                            match refresh_oauth_credential_after_401(
                                &input.state,
                                target_cli_key,
                                target_provider,
                            )
                            .await
                            {
                                Ok(refreshed_credential) => {
                                    effective_credential = refreshed_credential;
                                    tracing::info!(
                                        provider_id = provider.id,
                                        target_provider_id = target_provider.id,
                                        cx2cc_active,
                                        cli_key = %target_cli_key,
                                        "oauth 401 reactive refresh succeeded, retrying"
                                    );
                                    continue;
                                }
                                Err(err) => {
                                    tracing::warn!(
                                        provider_id = provider.id,
                                        target_provider_id = target_provider.id,
                                        cx2cc_active,
                                        cli_key = %target_cli_key,
                                        "oauth reactive refresh failed: {}",
                                        err
                                    );
                                }
                            }
                        }
                    }

                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match upstream_error::handle_non_success_response(
                        upstream_error::HandleNonSuccessResponseInput {
                            ctx,
                            provider_ctx,
                            attempt_ctx,
                            loop_state,
                            enable_thinking_signature_rectifier: input
                                .enable_thinking_signature_rectifier,
                            enable_thinking_budget_rectifier: input
                                .enable_thinking_budget_rectifier,
                            resp,
                            upstream: upstream_error::UpstreamRequestState {
                                upstream_body_bytes: &mut upstream_body_bytes,
                                strip_request_content_encoding: &mut strip_request_content_encoding,
                                thinking_signature_rectifier_retried:
                                    &mut thinking_signature_rectifier_retried,
                                thinking_budget_rectifier_retried:
                                    &mut thinking_budget_rectifier_retried,
                            },
                        },
                    )
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
                send::SendResult::Timeout => {
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match send_timeout::handle_timeout(ctx, provider_ctx, attempt_ctx, loop_state)
                        .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
                send::SendResult::Err(err) => {
                    let loop_state = LoopState::new(
                        &mut attempts,
                        &mut failed_provider_ids,
                        &mut last_error_category,
                        &mut last_error_code,
                        &mut circuit_snapshot,
                        &mut input.abort_guard,
                    );
                    match upstream_error::handle_reqwest_error(
                        ctx,
                        provider_ctx,
                        attempt_ctx,
                        loop_state,
                        err,
                    )
                    .await
                    {
                        LoopControl::ContinueRetry => continue,
                        LoopControl::BreakRetry => break,
                        LoopControl::Return(resp) => return resp,
                    }
                }
            }
        }
    }

    if should_finalize_as_all_providers_unavailable(&attempts) && !input.providers.is_empty() {
        let owned = finalize_owned_from_input(&input);
        return finalize::all_providers_unavailable(finalize::AllUnavailableInput {
            state: &input.state,
            abort_guard: &mut input.abort_guard,
            observe: input.observe_request,
            attempts,
            cli_key: owned.cli_key,
            method_hint: owned.method_hint,
            forwarded_path: owned.forwarded_path,
            query: owned.query,
            trace_id: owned.trace_id,
            started,
            created_at_ms,
            created_at,
            session_id: owned.session_id,
            requested_model: owned.requested_model,
            special_settings: owned.special_settings,
            verbose_provider_error: input.verbose_provider_error,
            earliest_available_unix,
            skipped_open,
            skipped_cooldown,
            skipped_limits,
            fingerprint_key: input.fingerprint_key,
            fingerprint_debug: input.fingerprint_debug.clone(),
            unavailable_fingerprint_key: input.unavailable_fingerprint_key,
            unavailable_fingerprint_debug: input.unavailable_fingerprint_debug.clone(),
        })
        .await;
    }

    let owned = finalize_owned_from_input(&input);
    finalize::all_providers_failed(finalize::AllFailedInput {
        state: &input.state,
        abort_guard: &mut input.abort_guard,
        observe: input.observe_request,
        attempts,
        last_error_category,
        last_error_code,
        cli_key: owned.cli_key,
        method_hint: owned.method_hint,
        forwarded_path: owned.forwarded_path,
        query: owned.query,
        trace_id: owned.trace_id,
        started,
        created_at_ms,
        created_at,
        session_id: owned.session_id,
        requested_model: owned.requested_model,
        special_settings: owned.special_settings,
        verbose_provider_error: input.verbose_provider_error,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::should_finalize_as_all_providers_unavailable;
    use crate::gateway::events::{decision_chain as dc, FailoverAttempt};
    use crate::gateway::proxy::GatewayErrorCode;

    fn skipped_attempt(reason_code: Option<&'static str>) -> FailoverAttempt {
        FailoverAttempt {
            provider_id: 1,
            provider_name: "provider".to_string(),
            base_url: "https://example.com".to_string(),
            outcome: "skipped".to_string(),
            status: None,
            provider_index: None,
            retry_index: None,
            session_reuse: None,
            error_category: Some("circuit_breaker"),
            error_code: Some(GatewayErrorCode::ProviderCircuitOpen.as_str()),
            decision: Some("skip"),
            reason: Some("provider skipped by circuit breaker (cooldown)".to_string()),
            selection_method: Some(dc::SELECTION_METHOD_FILTERED),
            reason_code,
            attempt_started_ms: Some(1),
            attempt_duration_ms: Some(0),
            circuit_state_before: None,
            circuit_state_after: None,
            circuit_failure_count: None,
            circuit_failure_threshold: None,
        }
    }

    fn real_attempt() -> FailoverAttempt {
        FailoverAttempt {
            provider_id: 1,
            provider_name: "provider".to_string(),
            base_url: "https://example.com".to_string(),
            outcome: "request_error".to_string(),
            status: Some(502),
            provider_index: Some(1),
            retry_index: Some(1),
            session_reuse: Some(false),
            error_category: Some("SYSTEM_ERROR"),
            error_code: Some(GatewayErrorCode::UpstreamConnectFailed.as_str()),
            decision: Some("switch"),
            reason: Some("reqwest connect error".to_string()),
            selection_method: Some("ordered"),
            reason_code: Some(dc::REASON_SYSTEM_ERROR),
            attempt_started_ms: Some(1),
            attempt_duration_ms: Some(10),
            circuit_state_before: Some("CLOSED"),
            circuit_state_after: None,
            circuit_failure_count: Some(0),
            circuit_failure_threshold: Some(5),
        }
    }

    #[test]
    fn skip_only_gate_attempts_finalize_as_unavailable() {
        let attempts = vec![
            skipped_attempt(Some(dc::REASON_CIRCUIT_COOLDOWN)),
            skipped_attempt(Some(dc::REASON_RATE_LIMITED)),
        ];

        assert!(should_finalize_as_all_providers_unavailable(&attempts));
    }

    #[test]
    fn empty_attempts_still_finalize_as_unavailable() {
        assert!(should_finalize_as_all_providers_unavailable(&[]));
    }

    #[test]
    fn real_attempts_do_not_finalize_as_unavailable() {
        let attempts = vec![
            skipped_attempt(Some(dc::REASON_CIRCUIT_OPEN)),
            real_attempt(),
        ];

        assert!(!should_finalize_as_all_providers_unavailable(&attempts));
    }

    #[test]
    fn non_gate_skip_attempts_do_not_finalize_as_unavailable() {
        let attempts = vec![skipped_attempt(None)];

        assert!(!should_finalize_as_all_providers_unavailable(&attempts));
    }
}
