//! Usage: Auth header injection for a single attempt.
//!
//! Centralizes the logic for building request headers and injecting
//! provider-specific authentication before sending upstream.
//! Body cleaning lives in `request_sanitizer`.

use super::attempt_executor::RetryLoopState;
use super::provider_iterator::PreparedProvider;
use super::*;
use crate::gateway::proxy::request_context::RequestContext;

/// Context needed for building a failed-attempt record when OAuth injection fails.
pub(super) struct AuthErrorCtx<'a> {
    pub(super) attempt_index: u32,
    pub(super) retry_index: u32,
    pub(super) attempt_started_ms: u128,
    pub(super) circuit_before: &'a crate::circuit_breaker::CircuitSnapshot,
}

/// Inject authentication headers based on provider type and auth mode.
///
/// Returns `Err(Box<FailoverAttempt>)` when OAuth header injection fails
/// (the attempt should be pushed to the attempts list and the retry
/// loop should break).
pub(super) fn inject_auth<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &PreparedProvider,
    retry_state: &RetryLoopState,
    error_ctx: &AuthErrorCtx<'_>,
    headers: &mut HeaderMap,
) -> Result<(), Box<FailoverAttempt>> {
    // Always clear all auth headers (fail-closed).
    clear_all_auth_headers(headers);

    let upstream_cli_key = if prepared.cx2cc_active {
        prepared
            .cx2cc_source
            .as_ref()
            .map(|(_, source_cli_key)| source_cli_key.as_str())
            .unwrap_or("codex")
    } else {
        input.cli_key.as_str()
    };
    strip_incompatible_protocol_headers(input.cli_key.as_str(), upstream_cli_key, headers);

    if prepared.oauth_adapter.is_some() {
        inject_oauth_auth(prepared, input, error_ctx, headers)?;
    } else {
        inject_standard_auth(
            ctx,
            input,
            prepared,
            retry_state,
            error_ctx.retry_index,
            headers,
        );
    }

    if prepared.use_codex_chatgpt_backend {
        maybe_inject_codex_chatgpt_headers(headers, prepared.codex_chatgpt_account_id.as_deref());
    }

    // Inject provider-configured custom headers last so they are not removed by
    // the auth-header clearing above. These carry non-standard identity/auth
    // headers some upstream gateways require (e.g. tenant/user identity).
    inject_custom_headers(&prepared.custom_headers, headers);

    Ok(())
}

/// Header names the gateway manages itself; provider custom headers must not
/// clobber transport framing or routing.
fn is_protected_custom_header(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "host" | "content-length" | "content-encoding" | "transfer-encoding" | "connection"
    )
}

/// Apply provider-configured custom headers to the outgoing request. Invalid
/// header names/values and protected transport headers are skipped rather than
/// failing the request.
fn inject_custom_headers(
    custom_headers: &[crate::providers::ProviderCustomHeader],
    headers: &mut HeaderMap,
) {
    use axum::http::header::{HeaderName, HeaderValue};

    for header in custom_headers {
        let name = header.name.trim();
        if name.is_empty() || is_protected_custom_header(name) {
            continue;
        }
        let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) else {
            tracing::warn!(header = %name, "skipping provider custom header with invalid name");
            continue;
        };
        let Ok(header_value) = HeaderValue::from_str(&header.value) else {
            tracing::warn!(header = %name, "skipping provider custom header with invalid value");
            continue;
        };
        headers.insert(header_name, header_value);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn inject_oauth_auth<R: tauri::Runtime>(
    prepared: &PreparedProvider,
    input: &RequestContext<R>,
    error_ctx: &AuthErrorCtx<'_>,
    headers: &mut HeaderMap,
) -> Result<(), Box<FailoverAttempt>> {
    let cred_trimmed = prepared.effective_credential.trim();
    tracing::debug!(
        provider_id = prepared.provider_id,
        cli_key = %input.cli_key,
        credential_len = cred_trimmed.len(),
        "injecting OAuth upstream headers"
    );
    match prepared.oauth_adapter {
        Some(adapter) => {
            if let Err(e) = adapter.inject_upstream_headers(headers, cred_trimmed) {
                tracing::warn!(
                    provider_id = prepared.provider_id,
                    cli_key = %input.cli_key,
                    "OAuth inject_upstream_headers failed, skipping provider: {e}"
                );
                return Err(Box::new(FailoverAttempt {
                    provider_id: prepared.provider_id,
                    provider_name: prepared.provider_name_base.clone(),
                    base_url: prepared.provider_base_url_display.clone(),
                    outcome: format!("oauth_inject_failed: {e}"),
                    status: Some(500),
                    provider_index: Some(error_ctx.attempt_index),
                    retry_index: Some(error_ctx.retry_index),
                    session_reuse: None,
                    error_category: Some("auth"),
                    error_code: Some(GatewayErrorCode::InternalError.as_str()),
                    decision: Some("switch"),
                    reason: Some(format!("OAuth header injection failed: {e}")),
                    selection_method: None,
                    reason_code: None,
                    attempt_started_ms: Some(error_ctx.attempt_started_ms),
                    attempt_duration_ms: Some(0),
                    circuit_state_before: Some(error_ctx.circuit_before.state.as_str()),
                    circuit_state_after: None,
                    circuit_failure_count: Some(error_ctx.circuit_before.failure_count),
                    circuit_failure_threshold: Some(error_ctx.circuit_before.failure_threshold),
                    circuit_recover_at_unix: None,
                    circuit_trigger_error_code: None,
                    provider_bridged: Some(prepared.provider_bridged),
                    timeout_secs: None,
                }));
            }
            Ok(())
        }
        None => {
            tracing::warn!(
                provider_id = prepared.provider_id,
                "oauth_adapter is None at injection point (should have been skipped earlier)"
            );
            Err(Box::new(FailoverAttempt {
                provider_id: prepared.provider_id,
                provider_name: prepared.provider_name_base.clone(),
                base_url: prepared.provider_base_url_display.clone(),
                outcome: "oauth_adapter_missing".to_string(),
                status: Some(500),
                provider_index: Some(error_ctx.attempt_index),
                retry_index: Some(error_ctx.retry_index),
                session_reuse: None,
                error_category: Some("auth"),
                error_code: Some(GatewayErrorCode::InternalError.as_str()),
                decision: Some("switch"),
                reason: Some("OAuth adapter unexpectedly None".to_string()),
                selection_method: None,
                reason_code: None,
                attempt_started_ms: Some(error_ctx.attempt_started_ms),
                attempt_duration_ms: Some(0),
                circuit_state_before: Some(error_ctx.circuit_before.state.as_str()),
                circuit_state_after: None,
                circuit_failure_count: Some(error_ctx.circuit_before.failure_count),
                circuit_failure_threshold: Some(error_ctx.circuit_before.failure_threshold),
                circuit_recover_at_unix: None,
                circuit_trigger_error_code: None,
                provider_bridged: Some(prepared.provider_bridged),
                timeout_secs: None,
            }))
        }
    }
}

fn inject_standard_auth<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    prepared: &PreparedProvider,
    retry_state: &RetryLoopState,
    retry_index: u32,
    headers: &mut HeaderMap,
) {
    let auth_cli_key = if prepared.cx2cc_active {
        "codex"
    } else {
        &input.cli_key
    };
    inject_provider_auth(auth_cli_key, prepared.effective_credential.trim(), headers);

    if !prepared.cx2cc_active && auth_cli_key == "claude" {
        if retry_state.claude_api_key_bearer_fallback {
            let value = format!("Bearer {}", prepared.effective_credential.trim());
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.remove("x-api-key");
                headers.insert(header::AUTHORIZATION, header_value);
            }
        }

        if retry_index == 1 || retry_state.claude_api_key_bearer_fallback {
            response_fixer::push_special_setting(
                ctx.special_settings,
                serde_json::json!({
                    "type": "claude_auth_injection",
                    "scope": "attempt",
                    "providerId": prepared.provider_id,
                    "providerName": prepared.provider_name_base.clone(),
                    "retryAttemptNumber": retry_index,
                    "mode": if retry_state.claude_api_key_bearer_fallback { "authorization_bearer" } else { "x_api_key" },
                }),
            );
        }
    }
}

#[cfg(test)]
mod custom_header_tests {
    use super::{inject_custom_headers, is_protected_custom_header};
    use crate::providers::ProviderCustomHeader;
    use axum::http::HeaderMap;

    fn header(name: &str, value: &str) -> ProviderCustomHeader {
        ProviderCustomHeader {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn injects_valid_headers() {
        let mut headers = HeaderMap::new();
        inject_custom_headers(
            &[header("X-User-Id", "42"), header("X-Domain", "corp")],
            &mut headers,
        );
        assert_eq!(headers.get("x-user-id").unwrap(), "42");
        assert_eq!(headers.get("x-domain").unwrap(), "corp");
    }

    #[test]
    fn skips_protected_transport_headers() {
        assert!(is_protected_custom_header("Host"));
        assert!(is_protected_custom_header("content-length"));
        let mut headers = HeaderMap::new();
        inject_custom_headers(&[header("Host", "evil.example.com")], &mut headers);
        assert!(headers.get("host").is_none());
    }

    #[test]
    fn skips_invalid_names_and_values() {
        let mut headers = HeaderMap::new();
        inject_custom_headers(
            &[header("bad name", "v"), header("X-Valid", "ok\ninjected")],
            &mut headers,
        );
        assert!(headers.is_empty());
    }
}
