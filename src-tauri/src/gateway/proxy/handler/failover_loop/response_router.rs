//! Usage: Response routing after a successful HTTP response from upstream.
//!
//! Dispatches success (stream / non-stream), handles Claude API key auth
//! fallback (401/403), OAuth 401 reactive refresh, and non-success upstream
//! error delegation.

use super::attempt_executor::{AttemptTiming, RetryLoopState};
use super::provider_iterator::PreparedProvider;
use super::retry_engine::AttemptIndices;
use super::*;
use crate::gateway::proxy::request_context::RequestContext;

/// Route an HTTP response from upstream to the appropriate handler.
///
/// Returns a `LoopControl` indicating whether to continue retrying,
/// break out of the retry loop, or return a final response.
#[allow(clippy::too_many_arguments)]
pub(super) async fn route_response(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    prepared: &mut PreparedProvider,
    retry_state: &mut RetryLoopState,
    indices: AttemptIndices,
    resp: reqwest::Response,
    timing: AttemptTiming,
    loop_state: &mut LoopState<'_>,
) -> LoopControl {
    let status = resp.status();
    let response_headers = resp.headers().clone();
    let response_content_type = response_headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    tracing::info!(
        trace_id = %input.trace_id,
        provider_id = prepared.provider_id,
        status = status.as_u16(),
        content_type = response_content_type,
        event_stream = is_event_stream(&response_headers),
        cx2cc_active = prepared.cx2cc_active,
        anthropic_stream_requested = prepared.anthropic_stream_requested,
        "upstream response received"
    );

    if prepared.cx2cc_active {
        emit_cx2cc_upstream_log(
            input,
            prepared,
            status,
            response_content_type,
            &response_headers,
        );
    }

    let circuit_before = prepared.circuit_snapshot.clone();
    let attempt_ctx = AttemptCtx {
        attempt_index: indices.attempt_index,
        retry_index: indices.retry_index,
        attempt_started_ms: timing.attempt_started_ms,
        attempt_started: timing.attempt_started,
        circuit_before: &circuit_before,
        gemini_oauth_response_mode: prepared.gemini_oauth_response_mode,
        cx2cc_active: prepared.cx2cc_active,
        anthropic_stream_requested: prepared.anthropic_stream_requested,
    };
    let provider_ctx = ProviderCtx {
        provider_id: prepared.provider_id,
        provider_name_base: &prepared.provider_name_base,
        provider_base_url_base: &prepared.provider_base_url_base,
        provider_index: prepared.provider_index,
        session_reuse: prepared.session_reuse,
        stream_idle_timeout_seconds: prepared.stream_idle_timeout_seconds,
    };

    if status.is_success() {
        if (prepared.anthropic_stream_requested || !prepared.cx2cc_active)
            && is_event_stream(&response_headers)
        {
            return success_event_stream::handle_success_event_stream(
                ctx,
                provider_ctx,
                attempt_ctx,
                loop_state.reborrow(),
                resp,
                status,
                response_headers,
            )
            .await;
        }
        return success_non_stream::handle_success_non_stream(
            ctx,
            provider_ctx,
            attempt_ctx,
            loop_state.reborrow(),
            resp,
            status,
            response_headers,
        )
        .await;
    }

    // Release provider_ctx (immutable borrow of prepared) before mutable borrows.
    let _ = provider_ctx;
    let _ = attempt_ctx;

    // --- Claude API key auth scheme fallback (401/403) ---
    if should_try_claude_auth_fallback(input, prepared, retry_state, indices.retry_index, status) {
        retry_state.claude_api_key_bearer_fallback = true;
        let mut settings = ctx.special_settings.lock_or_recover();
        settings.push(serde_json::json!({
            "type": "claude_auth_injection",
            "scope": "attempt",
            "hit": true,
            "action": "fallback_to_authorization_bearer",
            "providerId": prepared.provider_id,
            "providerName": prepared.provider_name_base.clone(),
            "status": status.as_u16(),
            "retryAttemptNumber": indices.retry_index,
            "retryAttemptNumberNext": indices.retry_index + 1,
        }));
        return LoopControl::ContinueRetry;
    }

    // --- OAuth 401 reactive refresh ---
    if status.as_u16() == 401 && !retry_state.oauth_reactive_refreshed_once {
        if let Some(ctrl) = try_oauth_reactive_refresh(input, prepared, retry_state).await {
            return ctrl;
        }
    }

    // Rebuild contexts after mutable operations are done.
    let circuit_before = prepared.circuit_snapshot.clone();
    let attempt_ctx = AttemptCtx {
        attempt_index: indices.attempt_index,
        retry_index: indices.retry_index,
        attempt_started_ms: timing.attempt_started_ms,
        attempt_started: timing.attempt_started,
        circuit_before: &circuit_before,
        gemini_oauth_response_mode: prepared.gemini_oauth_response_mode,
        cx2cc_active: prepared.cx2cc_active,
        anthropic_stream_requested: prepared.anthropic_stream_requested,
    };
    let provider_ctx = ProviderCtx {
        provider_id: prepared.provider_id,
        provider_name_base: &prepared.provider_name_base,
        provider_base_url_base: &prepared.provider_base_url_base,
        provider_index: prepared.provider_index,
        session_reuse: prepared.session_reuse,
        stream_idle_timeout_seconds: prepared.stream_idle_timeout_seconds,
    };

    // --- Non-success upstream error handling ---
    upstream_error::handle_non_success_response(upstream_error::HandleNonSuccessResponseInput {
        ctx,
        provider_ctx,
        attempt_ctx,
        loop_state: loop_state.reborrow(),
        enable_thinking_signature_rectifier: input.enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier: input.enable_thinking_budget_rectifier,
        resp,
        upstream: upstream_error::UpstreamRequestState {
            upstream_body_bytes: &mut prepared.upstream_body_bytes,
            strip_request_content_encoding: &mut prepared.strip_request_content_encoding,
            thinking_signature_rectifier_retried: &mut retry_state
                .thinking_signature_rectifier_retried,
            thinking_budget_rectifier_retried: &mut retry_state.thinking_budget_rectifier_retried,
        },
    })
    .await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn should_try_claude_auth_fallback(
    input: &RequestContext,
    prepared: &PreparedProvider,
    retry_state: &RetryLoopState,
    retry_index: u32,
    status: reqwest::StatusCode,
) -> bool {
    input.cli_key == "claude"
        && prepared.oauth_adapter.is_none()
        && !prepared.cx2cc_active
        && !retry_state.claude_api_key_bearer_fallback
        && retry_index < prepared.provider_max_attempts
        && matches!(status.as_u16(), 401 | 403)
}

async fn try_oauth_reactive_refresh(
    input: &RequestContext,
    prepared: &mut PreparedProvider,
    retry_state: &mut RetryLoopState,
) -> Option<LoopControl> {
    let refresh_target: Option<(&crate::providers::ProviderForGateway, &str)> =
        if prepared.oauth_adapter.is_some() {
            input
                .providers
                .iter()
                .find(|p| p.id == prepared.provider_id)
                .map(|p| (p, input.cli_key.as_str()))
        } else if prepared.cx2cc_active {
            prepared.cx2cc_source.as_ref().and_then(|(src, src_key)| {
                if src.auth_mode == "oauth" {
                    Some((src, src_key.as_str()))
                } else {
                    None
                }
            })
        } else {
            None
        };

    let (target_provider, target_cli_key) = refresh_target?;
    retry_state.oauth_reactive_refreshed_once = true;
    tracing::info!(
        provider_id = prepared.provider_id,
        target_provider_id = target_provider.id,
        cx2cc_active = prepared.cx2cc_active,
        cli_key = %target_cli_key,
        "oauth 401 detected, attempting reactive token refresh"
    );
    match refresh_oauth_credential_after_401(&input.state, target_cli_key, target_provider).await {
        Ok(refreshed_credential) => {
            prepared.effective_credential = refreshed_credential;
            tracing::info!(
                provider_id = prepared.provider_id,
                target_provider_id = target_provider.id,
                cx2cc_active = prepared.cx2cc_active,
                cli_key = %target_cli_key,
                "oauth 401 reactive refresh succeeded, retrying"
            );
            Some(LoopControl::ContinueRetry)
        }
        Err(err) => {
            tracing::warn!(
                provider_id = prepared.provider_id,
                target_provider_id = target_provider.id,
                cx2cc_active = prepared.cx2cc_active,
                cli_key = %target_cli_key,
                "oauth reactive refresh failed: {}",
                err
            );
            // Fall through to upstream error handling.
            None
        }
    }
}

fn emit_cx2cc_upstream_log(
    input: &RequestContext,
    prepared: &PreparedProvider,
    status: reqwest::StatusCode,
    response_content_type: &str,
    response_headers: &HeaderMap,
) {
    let source_provider_id = prepared.cx2cc_source.as_ref().map(|(source, _)| source.id);
    let source_provider_name = prepared
        .cx2cc_source
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
            prepared.provider_id,
            source_provider_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            source_provider_name,
            status.as_u16(),
            response_content_type,
            is_event_stream(response_headers),
            prepared.anthropic_stream_requested
        ),
    );
}
