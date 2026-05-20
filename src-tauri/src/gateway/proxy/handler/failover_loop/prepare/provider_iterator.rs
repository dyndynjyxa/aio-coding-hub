//! Usage: Provider traversal + skip logic (gate, credential, OAuth, CX2CC, Gemini, Codex).
//!
//! Encapsulates all per-provider preparation that runs before the retry loop.

use super::provider_checks;
use super::*;
use crate::gateway::events::ClaudeModelMapping;
use crate::gateway::proxy::gemini_oauth::GeminiOAuthResponseMode;
use std::collections::HashSet;

/// All mutable state accumulated by the provider preparation phase that the
/// retry loop (and later finalization) needs.
pub(super) struct PreparedProvider {
    pub(super) provider_id: i64,
    pub(super) provider_name_base: String,
    pub(super) provider_base_url_base: String,
    pub(super) provider_base_url_display: String,
    pub(super) provider_index: u32,
    pub(super) session_reuse: Option<bool>,
    pub(super) effective_credential: String,
    pub(super) provider_max_attempts: u32,
    pub(super) oauth_adapter:
        Option<&'static dyn crate::gateway::oauth::provider_trait::OAuthProvider>,
    pub(super) upstream_forwarded_path: String,
    pub(super) upstream_query: Option<String>,
    pub(super) upstream_body_bytes: Bytes,
    pub(super) strip_request_content_encoding: bool,
    pub(super) gemini_oauth_response_mode: Option<GeminiOAuthResponseMode>,
    pub(super) use_codex_chatgpt_backend: bool,
    pub(super) codex_chatgpt_account_id: Option<String>,
    pub(super) cx2cc_active: bool,
    pub(super) cx2cc_source: Option<(crate::providers::ProviderForGateway, String)>,
    pub(super) cx2cc_codex_session_id: Option<String>,
    pub(super) circuit_snapshot: crate::circuit_breaker::CircuitSnapshot,
    pub(super) anthropic_stream_requested: bool,
    pub(super) stream_idle_timeout_seconds: Option<u32>,
    pub(super) claude_model_mapping: Option<ClaudeModelMapping>,
}

/// Counters accumulated across all providers in the iteration loop.
pub(super) struct IterationCounters {
    pub(super) providers_tried: usize,
    pub(super) earliest_available_unix: Option<i64>,
    pub(super) skipped_open: usize,
    pub(super) skipped_cooldown: usize,
    pub(super) skipped_limits: usize,
}

impl IterationCounters {
    pub(super) fn new() -> Self {
        Self {
            providers_tried: 0,
            earliest_available_unix: None,
            skipped_open: 0,
            skipped_cooldown: 0,
            skipped_limits: 0,
        }
    }
}

pub(super) enum PreparationOutcome {
    Ready(Box<PreparedProvider>),
    Skipped,
}

/// Structured skip reason used by CX2CC preparation and other skip paths.
pub(super) struct SkipReason {
    pub(super) error_category: &'static str,
    pub(super) error_code: &'static str,
    pub(super) reason: String,
}

/// Prepare a single provider for the retry loop.
pub(super) async fn prepare_provider<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    provider: &crate::providers::ProviderForGateway,
    counters: &mut IterationCounters,
    attempts: &mut Vec<FailoverAttempt>,
    failed_provider_ids: &HashSet<i64>,
    anthropic_stream_requested: bool,
) -> PreparationOutcome {
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
        return PreparationOutcome::Skipped;
    }

    let identity = provider_checks::ProviderIdentity {
        provider_id,
        provider_name_base: &provider_name_base,
        provider_base_url_display: &provider_base_url_display,
    };
    let gate_allow =
        match provider_checks::run_gates(ctx, input, provider, &identity, counters, attempts) {
            Some(allow) => allow,
            None => return PreparationOutcome::Skipped,
        };

    let is_cx2cc_bridge = provider.is_cx2cc_bridge();

    let mut effective_credential = if is_cx2cc_bridge {
        String::new()
    } else {
        match resolve_effective_credential(&input.state, &input.cli_key, provider).await {
            Ok(value) => value,
            Err(err) => {
                provider_checks::skip_with_reason(
                    attempts,
                    provider_id,
                    &provider_name_base,
                    &provider_base_url_display,
                    input.started.elapsed().as_millis(),
                    SkipReason {
                        error_category: "auth",
                        error_code: GatewayErrorCode::InternalError.as_str(),
                        reason: format!("provider skipped by credential resolution: {err}"),
                    },
                );
                return PreparationOutcome::Skipped;
            }
        }
    };

    let provider_max_attempts = provider_max_attempts_for_request(
        input.max_attempts_per_provider,
        provider.auth_mode == "oauth",
        codex_request_has_previous_response_id(input),
    );

    let mut provider_base_url_base = match provider_checks::resolve_base_url(
        input,
        provider,
        provider_id,
        &provider_name_base,
        &provider_base_url_display,
        attempts,
    )
    .await
    {
        Some(url) => url,
        None => return PreparationOutcome::Skipped,
    };

    let mut use_codex_chatgpt_backend =
        is_codex_chatgpt_backend(&input.cli_key, provider, &provider_base_url_base);
    let mut codex_chatgpt_account_id = if use_codex_chatgpt_backend {
        provider_checks::extract_codex_chatgpt_account_id(&input.state.db, provider.id)
    } else {
        None
    };

    let oauth_adapter = match provider_checks::resolve_oauth(
        input,
        provider,
        provider_id,
        &provider_name_base,
        &provider_base_url_display,
        attempts,
    ) {
        Some(adapter) => adapter,
        None => return PreparationOutcome::Skipped,
    };

    let mut upstream_forwarded_path = input.forwarded_path.clone();
    let mut upstream_query = input.query.clone();
    let mut upstream_body_bytes = input.body_bytes.clone();
    let mut strip_request_content_encoding = input.strip_request_content_encoding_seed;
    let mut gemini_oauth_response_mode = None;

    if let Some(adapter) = &oauth_adapter {
        if adapter.provider_type() == "gemini_oauth" {
            match provider_checks::prepare_gemini_oauth(
                input,
                &effective_credential,
                &mut provider_base_url_base,
            )
            .await
            {
                Some(prepared) => {
                    upstream_forwarded_path = prepared.forwarded_path;
                    upstream_query = prepared.query;
                    upstream_body_bytes = prepared.body_bytes;
                    strip_request_content_encoding = prepared.strip_request_content_encoding;
                    gemini_oauth_response_mode = Some(prepared.response_mode);
                }
                None => {
                    provider_checks::skip_with_reason(
                        attempts,
                        provider_id,
                        &provider_name_base,
                        &provider_base_url_display,
                        input.started.elapsed().as_millis(),
                        SkipReason {
                            error_category: "auth",
                            error_code: GatewayErrorCode::InternalError.as_str(),
                            reason: "provider skipped by gemini oauth translation".into(),
                        },
                    );
                    return PreparationOutcome::Skipped;
                }
            }
        }
    }

    // --- CX2CC translation ---
    let mut cx2cc_active = false;
    let mut cx2cc_source: Option<(crate::providers::ProviderForGateway, String)> = None;
    let mut cx2cc_codex_session_id: Option<String> = None;
    if is_cx2cc_bridge {
        let outcome = cx2cc_preparation::prepare(cx2cc_preparation::Cx2ccPreparationInput {
            ctx,
            input,
            provider_id,
            provider_name_base: &provider_name_base,
            source_id: provider.source_provider_id,
            anthropic_stream_requested,
            upstream_body_bytes,
            use_codex_chatgpt_backend,
            codex_chatgpt_account_id,
        })
        .await;
        match outcome {
            cx2cc_preparation::Cx2ccOutcome::Ready(boxed) => {
                let result = *boxed;
                cx2cc_active = result.cx2cc_active;
                cx2cc_source = result.cx2cc_source;
                cx2cc_codex_session_id = result.cx2cc_codex_session_id;
                effective_credential = result.effective_credential;
                provider_base_url_base = result.provider_base_url_base;
                upstream_forwarded_path = result.upstream_forwarded_path;
                upstream_query = result.upstream_query;
                upstream_body_bytes = result.upstream_body_bytes;
                strip_request_content_encoding = result.strip_request_content_encoding;
                use_codex_chatgpt_backend = result.use_codex_chatgpt_backend;
                codex_chatgpt_account_id = result.codex_chatgpt_account_id;
            }
            cx2cc_preparation::Cx2ccOutcome::Skipped(reason) => {
                provider_checks::skip_with_reason(
                    attempts,
                    provider_id,
                    &provider_name_base,
                    &provider_base_url_display,
                    input.started.elapsed().as_millis(),
                    reason,
                );
                return PreparationOutcome::Skipped;
            }
        }
    }

    let circuit_snapshot = gate_allow.circuit_after;
    counters.providers_tried = counters.providers_tried.saturating_add(1);
    let provider_index = counters.providers_tried as u32;
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
        claude_model_mapping: None,
    };

    let mut claude_model_mapping = None;
    if should_apply_claude_model_mapping(cx2cc_active, &upstream_forwarded_path) {
        claude_model_mapping = claude_model_mapping::apply_if_needed(
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

    if use_codex_chatgpt_backend {
        maybe_apply_codex_chatgpt_request_compat(
            &mut upstream_forwarded_path,
            &mut upstream_body_bytes,
            &mut strip_request_content_encoding,
        );
    }

    PreparationOutcome::Ready(Box::new(PreparedProvider {
        provider_id,
        provider_name_base,
        provider_base_url_base,
        provider_base_url_display,
        provider_index,
        session_reuse,
        effective_credential,
        provider_max_attempts,
        oauth_adapter,
        upstream_forwarded_path,
        upstream_query,
        upstream_body_bytes,
        strip_request_content_encoding,
        gemini_oauth_response_mode,
        use_codex_chatgpt_backend,
        codex_chatgpt_account_id,
        cx2cc_active,
        cx2cc_source,
        cx2cc_codex_session_id,
        circuit_snapshot,
        anthropic_stream_requested,
        stream_idle_timeout_seconds: provider.stream_idle_timeout_seconds,
        claude_model_mapping,
    }))
}

fn codex_request_has_previous_response_id<R: tauri::Runtime>(input: &RequestContext<R>) -> bool {
    codex_body_has_previous_response_id(&input.cli_key, &input.body_bytes)
}

fn codex_body_has_previous_response_id(cli_key: &str, body: &[u8]) -> bool {
    if cli_key != "codex" {
        return false;
    }

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

fn provider_max_attempts_for_request(
    configured_max_attempts: u32,
    needs_oauth_reactive_refresh_retry: bool,
    needs_codex_previous_response_id_retry: bool,
) -> u32 {
    let required_internal_retries = u32::from(needs_oauth_reactive_refresh_retry)
        + u32::from(needs_codex_previous_response_id_retry);
    configured_max_attempts.max(1 + required_internal_retries)
}

#[cfg(test)]
mod tests {
    use super::{codex_body_has_previous_response_id, provider_max_attempts_for_request};

    fn body(value: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&value).expect("serialize body")
    }

    #[test]
    fn codex_request_has_previous_response_id_detects_codex_continuation() {
        let body = body(serde_json::json!({
            "previous_response_id": "resp_123"
        }));

        assert!(codex_body_has_previous_response_id("codex", &body));
    }

    #[test]
    fn codex_request_has_previous_response_id_ignores_other_cli_or_missing_value() {
        let with_previous = body(serde_json::json!({
            "previous_response_id": "resp_123"
        }));
        let without_previous = body(serde_json::json!({}));

        assert!(!codex_body_has_previous_response_id(
            "claude",
            &with_previous
        ));
        assert!(!codex_body_has_previous_response_id(
            "codex",
            &without_previous
        ));
    }

    #[test]
    fn provider_max_attempts_reserves_budget_for_internal_retries() {
        assert_eq!(provider_max_attempts_for_request(1, false, false), 1);
        assert_eq!(provider_max_attempts_for_request(1, true, false), 2);
        assert_eq!(provider_max_attempts_for_request(1, false, true), 2);
        assert_eq!(provider_max_attempts_for_request(1, true, true), 3);
        assert_eq!(provider_max_attempts_for_request(5, true, true), 5);
    }
}
