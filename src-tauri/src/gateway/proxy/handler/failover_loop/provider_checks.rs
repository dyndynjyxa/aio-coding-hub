//! Usage: Provider pre-flight check helpers (gates, base URL, OAuth, Gemini OAuth).
//!
//! These helpers are used by `provider_iterator::prepare_provider` to validate
//! and resolve provider configuration before the retry loop begins.

use super::provider_iterator::{IterationCounters, SkipReason};
use super::*;
use crate::gateway::proxy::gemini_oauth::GeminiOAuthResponseMode;

pub(super) fn skip_with_reason(
    attempts: &mut Vec<FailoverAttempt>,
    provider_id: i64,
    provider_name: &str,
    base_url: &str,
    attempt_started_ms: u128,
    reason: SkipReason,
) {
    push_skipped_provider_attempt(
        attempts,
        SkippedProviderAttempt {
            provider_id,
            provider_name,
            base_url,
            error_category: reason.error_category,
            error_code: reason.error_code,
            reason: reason.reason,
            reason_code: None,
            attempt_started_ms,
        },
    );
}

/// Identity fields derived from a provider, used by gate/check functions.
pub(super) struct ProviderIdentity<'a> {
    pub(super) provider_id: i64,
    pub(super) provider_name_base: &'a String,
    pub(super) provider_base_url_display: &'a String,
}

pub(super) fn run_gates(
    ctx: CommonCtx<'_>,
    input: &RequestContext,
    provider: &crate::providers::ProviderForGateway,
    identity: &ProviderIdentity<'_>,
    counters: &mut IterationCounters,
    attempts: &mut Vec<FailoverAttempt>,
) -> Option<provider_gate::ProviderGateAllow> {
    let skipped_open_before = counters.skipped_open;
    let skipped_cooldown_before = counters.skipped_cooldown;
    let gate_allow = provider_gate::gate_provider(provider_gate::ProviderGateInput {
        ctx,
        provider_id: identity.provider_id,
        provider_name_base: identity.provider_name_base,
        provider_base_url_display: identity.provider_base_url_display,
        earliest_available_unix: &mut counters.earliest_available_unix,
        skipped_open: &mut counters.skipped_open,
        skipped_cooldown: &mut counters.skipped_cooldown,
    });
    if gate_allow.is_none() {
        let (reason_code, reason_label) = if counters.skipped_open > skipped_open_before {
            (Some(dc::REASON_CIRCUIT_OPEN), "open")
        } else if counters.skipped_cooldown > skipped_cooldown_before {
            (Some(dc::REASON_CIRCUIT_COOLDOWN), "cooldown")
        } else {
            (None, "unknown")
        };
        push_skipped_provider_attempt(
            attempts,
            SkippedProviderAttempt {
                provider_id: identity.provider_id,
                provider_name: identity.provider_name_base,
                base_url: identity.provider_base_url_display,
                error_category: "circuit_breaker",
                error_code: GatewayErrorCode::ProviderCircuitOpen.as_str(),
                reason: format!("provider skipped by circuit breaker ({reason_label})"),
                reason_code,
                attempt_started_ms: input.started.elapsed().as_millis(),
            },
        );
        return None;
    }

    if !provider_limits::gate_provider(provider_limits::ProviderLimitsInput {
        ctx,
        provider,
        earliest_available_unix: &mut counters.earliest_available_unix,
        skipped_limits: &mut counters.skipped_limits,
    }) {
        push_skipped_provider_attempt(
            attempts,
            SkippedProviderAttempt {
                provider_id: identity.provider_id,
                provider_name: identity.provider_name_base,
                base_url: identity.provider_base_url_display,
                error_category: "rate_limit",
                error_code: GatewayErrorCode::ProviderRateLimited.as_str(),
                reason: "provider skipped by rate limit".to_string(),
                reason_code: Some(dc::REASON_RATE_LIMITED),
                attempt_started_ms: input.started.elapsed().as_millis(),
            },
        );
        return None;
    }

    gate_allow
}

pub(super) async fn resolve_base_url(
    input: &RequestContext,
    provider: &crate::providers::ProviderForGateway,
    provider_id: i64,
    provider_name_base: &str,
    provider_base_url_display: &str,
    attempts: &mut Vec<FailoverAttempt>,
) -> Option<String> {
    match select_provider_base_url_for_request(
        &input.state,
        provider,
        &input.cli_key,
        input.provider_base_url_ping_cache_ttl_seconds,
    )
    .await
    {
        Ok(base_url) => {
            tracing::debug!(
                trace_id = %input.trace_id,
                cli_key = %input.cli_key,
                provider_id = provider_id,
                provider_name = %provider_name_base,
                auth_mode = %provider.auth_mode,
                base_url_resolved = %base_url,
                base_urls_count = provider.base_urls.len(),
                "resolved provider base_url for request"
            );
            Some(base_url)
        }
        Err(err) => {
            tracing::warn!(
                trace_id = %input.trace_id,
                cli_key = %input.cli_key,
                provider_id = provider_id,
                provider_name = %provider_name_base,
                "provider skipped by base_url resolution: {}",
                err
            );
            skip_with_reason(
                attempts,
                provider_id,
                provider_name_base,
                provider_base_url_display,
                input.started.elapsed().as_millis(),
                SkipReason {
                    error_category: "system",
                    error_code: GatewayErrorCode::InternalError.as_str(),
                    reason: format!("provider skipped by base_url resolution: {err}"),
                },
            );
            None
        }
    }
}

pub(super) fn extract_codex_chatgpt_account_id(
    db: &crate::db::Db,
    provider_id: i64,
) -> Option<String> {
    let details = crate::providers::get_oauth_details(db, provider_id).ok();
    details.and_then(|d| {
        let result = parse_codex_chatgpt_account_id(d.oauth_id_token.as_deref())
            .or_else(|| parse_codex_chatgpt_account_id(Some(&d.oauth_access_token)));
        tracing::debug!(
            provider_id = provider_id,
            has_oauth_id_token = d.oauth_id_token.is_some(),
            parsed_account_id = ?result,
            "codex chatgpt account_id extraction"
        );
        result
    })
}

pub(super) fn resolve_oauth(
    input: &RequestContext,
    provider: &crate::providers::ProviderForGateway,
    provider_id: i64,
    provider_name_base: &str,
    provider_base_url_display: &str,
    attempts: &mut Vec<FailoverAttempt>,
) -> Option<Option<&'static dyn crate::gateway::oauth::provider_trait::OAuthProvider>> {
    if provider.auth_mode != "oauth" {
        return Some(None);
    }
    match resolve_oauth_adapter_for_provider(
        &input.cli_key,
        provider.id,
        provider.oauth_provider_type.as_deref(),
    ) {
        Ok(adapter) => Some(Some(adapter)),
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
            skip_with_reason(
                attempts,
                provider_id,
                provider_name_base,
                provider_base_url_display,
                input.started.elapsed().as_millis(),
                SkipReason {
                    error_category: "auth",
                    error_code: GatewayErrorCode::InternalError.as_str(),
                    reason: format!("provider skipped by oauth adapter mismatch: {err_text}"),
                },
            );
            None
        }
    }
}

pub(super) struct GeminiOAuthPrepared {
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) body_bytes: Bytes,
    pub(super) strip_request_content_encoding: bool,
    pub(super) response_mode: GeminiOAuthResponseMode,
}

pub(super) async fn prepare_gemini_oauth(
    input: &RequestContext,
    effective_credential: &str,
    provider_base_url_base: &mut String,
) -> Option<GeminiOAuthPrepared> {
    let client = input.state.client();
    match gemini_oauth::prepare_upstream_request(
        &client,
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
            *provider_base_url_base = prepared.base_url;
            Some(GeminiOAuthPrepared {
                forwarded_path: prepared.forwarded_path,
                query: prepared.query,
                body_bytes: prepared.body_bytes,
                strip_request_content_encoding: prepared.strip_request_content_encoding,
                response_mode: prepared.response_mode,
            })
        }
        Err(err) => {
            tracing::warn!(
                trace_id = %input.trace_id,
                "provider skipped by gemini oauth request translation: {}",
                err
            );
            None
        }
    }
}
