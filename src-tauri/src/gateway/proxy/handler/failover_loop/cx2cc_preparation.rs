//! Usage: CX2CC (Claude-to-Codex) request preparation.
//!
//! Translates an Anthropic-format request into OpenAI Responses API format
//! via a source provider, including credential resolution, protocol bridge
//! invocation, base URL override, and Codex session ID completion.

use super::provider_iterator::SkipReason;
use super::*;
use crate::app::gateway_runtime_access::app_gateway_status;
use crate::gateway::codex_session_id;
use crate::gateway::proxy::protocol_bridge::{self, BridgeContext};

/// All CX2CC-related state produced by preparation.
pub(super) struct Cx2ccResult {
    pub(super) cx2cc_active: bool,
    pub(super) cx2cc_source: Option<(crate::providers::ProviderForGateway, String)>,
    pub(super) cx2cc_codex_session_id: Option<String>,
    pub(super) effective_credential: String,
    pub(super) provider_base_url_base: String,
    pub(super) upstream_forwarded_path: String,
    pub(super) upstream_query: Option<String>,
    pub(super) upstream_body_bytes: Bytes,
    pub(super) strip_request_content_encoding: bool,
    pub(super) use_codex_chatgpt_backend: bool,
    pub(super) codex_chatgpt_account_id: Option<String>,
}

pub(super) struct Cx2ccPreparationInput<'a> {
    pub(super) ctx: CommonCtx<'a>,
    pub(super) input: &'a RequestContext,
    pub(super) provider_id: i64,
    pub(super) provider_name_base: &'a str,
    pub(super) source_id: Option<i64>,
    pub(super) anthropic_stream_requested: bool,
    pub(super) upstream_body_bytes: Bytes,
    pub(super) use_codex_chatgpt_backend: bool,
    pub(super) codex_chatgpt_account_id: Option<String>,
}

pub(super) enum Cx2ccOutcome {
    Ready(Box<Cx2ccResult>),
    Skipped(SkipReason),
}

/// Prepare CX2CC translation for a source-provider-backed bridge provider.
pub(super) async fn prepare(args: Cx2ccPreparationInput<'_>) -> Cx2ccOutcome {
    let (
        source,
        source_cli_key,
        source_provider_name,
        source_cred,
        source_provider_base_url,
        mut use_codex_chatgpt_backend,
        mut codex_chatgpt_account_id,
    ) = if let Some(source_id) = args.source_id {
        let source_result =
            crate::providers::get_source_provider_for_gateway(&args.input.state.db, source_id);

        let (source, source_cli_key) = match source_result {
            Ok(pair) => pair,
            Err(err) => {
                let msg = format!(
                    "[CX2CC] source provider not found: {err} (provider={}, source_id={})",
                    args.provider_name_base, source_id
                );
                tracing::warn!(
                    trace_id = %args.input.trace_id,
                    provider_id = args.provider_id,
                    source_provider_id = source_id,
                    "cx2cc: source provider not found: {err}"
                );
                emit_gateway_log(&args.input.state.app, "warn", "CX2CC_SOURCE_NOT_FOUND", msg);
                return Cx2ccOutcome::Skipped(SkipReason {
                    error_category: "config",
                    error_code: GatewayErrorCode::InternalError.as_str(),
                    reason: format!("cx2cc source provider not found: {err}"),
                });
            }
        };

        let source_cred = match resolve_effective_credential(
            &args.input.state,
            &source_cli_key,
            &source,
        )
        .await
        {
            Ok(cred) => cred,
            Err(err) => {
                let msg = format!(
                        "[CX2CC] source credential resolution failed: {err} (provider={}, source_id={})",
                        args.provider_name_base, source_id
                    );
                tracing::warn!(
                    trace_id = %args.input.trace_id,
                    provider_id = args.provider_id,
                    source_provider_id = source_id,
                    "cx2cc: source provider credential resolution failed: {err}"
                );
                emit_gateway_log(
                    &args.input.state.app,
                    "warn",
                    "CX2CC_CREDENTIAL_FAILED",
                    msg,
                );
                return Cx2ccOutcome::Skipped(SkipReason {
                    error_category: "auth",
                    error_code: GatewayErrorCode::InternalError.as_str(),
                    reason: format!("cx2cc source provider credential failed: {err}"),
                });
            }
        };

        let provider_base_url_base = match select_provider_base_url_for_request(
            &args.input.state,
            &source,
            &source_cli_key,
            args.input.provider_base_url_ping_cache_ttl_seconds,
        )
        .await
        {
            Ok(url) => url,
            Err(err) => {
                let msg = format!(
                    "[CX2CC] source base_url resolution failed: {err} (provider={}, source_id={})",
                    args.provider_name_base, source_id
                );
                tracing::warn!(
                    trace_id = %args.input.trace_id,
                    provider_id = args.provider_id,
                    source_provider_id = source_id,
                    "cx2cc: source provider base_url resolution failed: {err}"
                );
                emit_gateway_log(&args.input.state.app, "warn", "CX2CC_BASE_URL_FAILED", msg);
                return Cx2ccOutcome::Skipped(SkipReason {
                    error_category: "translation",
                    error_code: GatewayErrorCode::InternalError.as_str(),
                    reason: format!("cx2cc source base_url failed: {err}"),
                });
            }
        };

        let source_provider_name = if source.name.trim().is_empty() {
            format!("Provider #{}", source.id)
        } else {
            source.name.clone()
        };

        (
            Some(source),
            source_cli_key,
            source_provider_name,
            source_cred,
            provider_base_url_base,
            args.use_codex_chatgpt_backend,
            args.codex_chatgpt_account_id.clone(),
        )
    } else {
        let gateway_base_url = app_gateway_status(&args.input.state.app).base_url;

        let Some(gateway_base_url) = gateway_base_url else {
            return Cx2ccOutcome::Skipped(SkipReason {
                error_category: "config",
                error_code: GatewayErrorCode::InternalError.as_str(),
                reason: "cx2cc local codex gateway base_url missing".to_string(),
            });
        };

        (
            None,
            "codex".to_string(),
            "Codex".to_string(),
            crate::infra::cli_proxy::PLACEHOLDER_KEY.to_string(),
            format!("{}/v1", gateway_base_url.trim_end_matches('/')),
            false,
            None,
        )
    };

    // Translate request via protocol bridge (IR path).
    let body_val: serde_json::Value =
        serde_json::from_slice(&args.upstream_body_bytes).unwrap_or_default();
    let requested_model = body_val.get("model").and_then(|m| m.as_str()).unwrap_or("");
    let bridge_ctx = BridgeContext {
        claude_models: args
            .input
            .providers
            .iter()
            .find(|p| p.id == args.provider_id)
            .map(|p| p.claude_models.clone())
            .unwrap_or_default(),
        cx2cc_settings: args.input.cx2cc_settings.clone(),
        requested_model: Some(requested_model.to_string()),
        mapped_model: None,
        stream_requested: args.anthropic_stream_requested,
        is_chatgpt_backend: false,
    };

    let original_body = body_val.clone();
    let translated = match protocol_bridge::get_bridge("cx2cc")
        .ok_or_else(|| "cx2cc bridge not registered".to_string())
        .and_then(|bridge| {
            bridge
                .translate_request(body_val, &bridge_ctx)
                .map_err(|e| e.to_string())
        }) {
        Ok(t) => t,
        Err(err) => {
            let msg = format!(
                "[CX2CC] request translation failed: {err} (provider={})",
                args.provider_name_base
            );
            tracing::warn!(
                trace_id = %args.input.trace_id,
                provider_id = args.provider_id,
                "cx2cc: request translation failed: {err}"
            );
            emit_gateway_log(&args.input.state.app, "warn", "CX2CC_TRANSLATE_FAILED", msg);
            return Cx2ccOutcome::Skipped(SkipReason {
                error_category: "translation",
                error_code: GatewayErrorCode::InternalError.as_str(),
                reason: format!("cx2cc translation failed: {err}"),
            });
        }
    };

    let mut responses_body = translated.body;
    apply_cx2cc_request_settings(&mut responses_body, &args.input.cx2cc_settings);
    let cache_key = ensure_cx2cc_prompt_cache_key(EnsureCx2ccPromptCacheKeyInput {
        ctx: args.ctx,
        source_credential: &source_cred,
        request_session_id: args.input.session_id.as_deref(),
        base_headers: &args.input.base_headers,
        original_body: &original_body,
        responses_body: &mut responses_body,
    });
    let openai_model = responses_body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    let mut upstream_body_bytes: Bytes = serde_json::to_vec(&responses_body)
        .unwrap_or_default()
        .into();
    let upstream_forwarded_path = translated.target_path;
    let upstream_query = None;
    let mut strip_request_content_encoding = true;

    let provider_base_url_base = source_provider_base_url;

    let cx2cc_codex_session_id = codex_session_id_completion::apply_if_needed(
        codex_session_id_completion::ApplyCodexSessionIdCompletionInput {
            ctx: args.ctx,
            enabled: args.input.enable_codex_session_id_completion,
            source_cli_key: &source_cli_key,
            session_id: args.input.session_id.as_deref(),
            base_headers: &args.input.base_headers,
            upstream_body_bytes: &mut upstream_body_bytes,
            strip_request_content_encoding: &mut strip_request_content_encoding,
        },
    );

    // Re-detect Codex ChatGPT backend using source provider.
    if let Some(source) = source.as_ref() {
        let cx2cc_is_chatgpt =
            is_codex_chatgpt_backend(&source_cli_key, source, &provider_base_url_base);
        if cx2cc_is_chatgpt {
            let details = crate::providers::get_oauth_details(&args.input.state.db, source.id).ok();
            codex_chatgpt_account_id = details.and_then(|d| {
                parse_codex_chatgpt_account_id(d.oauth_id_token.as_deref())
                    .or_else(|| parse_codex_chatgpt_account_id(Some(&d.oauth_access_token)))
            });
            use_codex_chatgpt_backend = true;
        }
    }

    tracing::info!(
        trace_id = %args.input.trace_id,
        provider_id = args.provider_id,
        openai_model = %openai_model,
        "cx2cc: request translated Anthropic -> OpenAI Responses API"
    );
    emit_gateway_log(
        &args.input.state.app,
        "info",
        "CX2CC_TRANSLATED",
        format!(
            "[CX2CC] translated -> model={openai_model}, bridge={}, source={source_provider_name}",
            args.provider_name_base
        ),
    );
    {
        let mut settings = args.input.special_settings.lock_or_recover();
        settings.push(serde_json::json!({
            "type": "cx2cc_cost_basis",
            "scope": "request",
            "source_cli_key": source_cli_key,
            "source_provider_id": args.source_id,
            "source_provider_name": source_provider_name,
            "priced_model": openai_model,
        }));
        settings.push(serde_json::json!({
            "type": "cx2cc_cache_diagnostics",
            "scope": "request",
            "prompt_cache_key_present": cache_key.key.is_some(),
            "prompt_cache_key_source": cache_key.source,
            "prompt_cache_retention": responses_body
                .get("prompt_cache_retention")
                .and_then(|v| v.as_str()),
            "source_cli_key": source_cli_key,
            "source_provider_id": args.source_id,
            "source_provider_name": source_provider_name,
            "model": openai_model,
            "instructions_len": responses_body
                .get("instructions")
                .and_then(|v| v.as_str())
                .map(str::len)
                .unwrap_or(0),
            "input_items": responses_body
                .get("input")
                .and_then(|v| v.as_array())
                .map(Vec::len)
                .unwrap_or(0),
            "anthropic_cache_control": protocol_bridge::inbound::anthropic::anthropic_cache_control_diagnostics(&original_body),
        }));
    }
    // DEBUG: dump translated body for troubleshooting.
    {
        let debug_body: serde_json::Value =
            serde_json::from_slice(&upstream_body_bytes).unwrap_or_default();
        let has_instructions = debug_body.get("instructions").is_some();
        let instructions_len = debug_body
            .get("instructions")
            .and_then(|v| v.as_str())
            .map(str::len)
            .unwrap_or(0);
        let has_prompt_cache_key = debug_body.get("prompt_cache_key").is_some();
        let prompt_cache_retention = debug_body
            .get("prompt_cache_retention")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let input_items = debug_body
            .get("input")
            .and_then(|v| v.as_array())
            .map(Vec::len)
            .unwrap_or(0);
        let model_val = debug_body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("<MISSING>");
        let keys: Vec<&str> = debug_body
            .as_object()
            .map(|m| m.keys().map(|k| k.as_str()).collect())
            .unwrap_or_default();
        emit_gateway_log(
            &args.input.state.app,
            "debug",
            "CX2CC_REQUEST_BODY",
            format!(
                "[CX2CC] keys={keys:?} model={model_val} has_instructions={has_instructions} instructions_len={instructions_len} input_items={input_items} has_prompt_cache_key={has_prompt_cache_key} prompt_cache_retention={prompt_cache_retention}",
            ),
        );
    }

    Cx2ccOutcome::Ready(Box::new(Cx2ccResult {
        cx2cc_active: true,
        cx2cc_source: source.map(|provider| (provider, source_cli_key.clone())),
        cx2cc_codex_session_id,
        effective_credential: source_cred,
        provider_base_url_base,
        upstream_forwarded_path,
        upstream_query,
        upstream_body_bytes,
        strip_request_content_encoding,
        use_codex_chatgpt_backend,
        codex_chatgpt_account_id,
    }))
}

struct EnsureCx2ccPromptCacheKeyInput<'a> {
    ctx: CommonCtx<'a>,
    source_credential: &'a str,
    request_session_id: Option<&'a str>,
    base_headers: &'a HeaderMap,
    original_body: &'a serde_json::Value,
    responses_body: &'a mut serde_json::Value,
}

struct Cx2ccPromptCacheKey {
    key: Option<String>,
    source: &'static str,
}

fn ensure_cx2cc_prompt_cache_key(input: EnsureCx2ccPromptCacheKeyInput<'_>) -> Cx2ccPromptCacheKey {
    if let Some((key, source)) = existing_prompt_cache_key_candidate(
        input.responses_body,
        input.original_body,
        input.base_headers,
        input.request_session_id,
    ) {
        input.responses_body["prompt_cache_key"] = serde_json::json!(key);
        return Cx2ccPromptCacheKey {
            key: input
                .responses_body
                .get("prompt_cache_key")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            source,
        };
    }

    let mut headers = input.base_headers.clone();
    if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", input.source_credential)) {
        headers.insert(header::AUTHORIZATION, value);
    }

    let mut cache = input.ctx.state.codex_session_cache.lock_or_recover();
    let result = codex_session_id::complete_codex_session_identifiers(
        &mut cache,
        input.ctx.created_at,
        input.ctx.created_at_ms,
        &mut headers,
        Some(input.responses_body),
    );

    Cx2ccPromptCacheKey {
        key: Some(result.session_id),
        source: match result.source {
            "fingerprint_cache" => "fingerprint_cache",
            _ => "generated_fingerprint_uuid",
        },
    }
}

fn existing_prompt_cache_key_candidate(
    responses_body: &serde_json::Value,
    original_body: &serde_json::Value,
    headers: &HeaderMap,
    request_session_id: Option<&str>,
) -> Option<(String, &'static str)> {
    prompt_cache_key_from_value(responses_body, "prompt_cache_key")
        .map(|key| (key, "body_prompt_cache_key"))
        .or_else(|| {
            prompt_cache_key_from_value(original_body, "prompt_cache_key")
                .map(|key| (key, "body_prompt_cache_key"))
        })
        .or_else(|| {
            prompt_cache_key_from_value(original_body, "session_id")
                .map(|key| (key, "request_session_id"))
        })
        .or_else(|| header_session_id(headers).map(|key| (key, "header_session_id")))
        .or_else(|| {
            request_session_id
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .map(|key| (key, "request_session_id"))
        })
}

fn prompt_cache_key_from_value(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn header_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("session_id")
        .or_else(|| headers.get("x-session-id"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::existing_prompt_cache_key_candidate;
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn cache_key_candidate_keeps_existing_body_key() {
        let headers = HeaderMap::new();
        let (key, source) = existing_prompt_cache_key_candidate(
            &json!({"prompt_cache_key": "existing-key"}),
            &json!({"session_id": "request-session"}),
            &headers,
            Some("context-session"),
        )
        .expect("candidate");

        assert_eq!(key, "existing-key");
        assert_eq!(source, "body_prompt_cache_key");
    }

    #[test]
    fn cache_key_candidate_keeps_original_body_key() {
        let headers = HeaderMap::new();
        let (key, source) = existing_prompt_cache_key_candidate(
            &json!({}),
            &json!({"prompt_cache_key": "original-key", "session_id": "request-session"}),
            &headers,
            None,
        )
        .expect("candidate");

        assert_eq!(key, "original-key");
        assert_eq!(source, "body_prompt_cache_key");
    }

    #[test]
    fn cache_key_candidate_uses_request_session_before_header() {
        let mut headers = HeaderMap::new();
        headers.insert("session_id", HeaderValue::from_static("header-session"));
        let (key, source) = existing_prompt_cache_key_candidate(
            &json!({}),
            &json!({"session_id": "request-session"}),
            &headers,
            Some("context-session"),
        )
        .expect("candidate");

        assert_eq!(key, "request-session");
        assert_eq!(source, "request_session_id");
    }

    #[test]
    fn cache_key_candidate_uses_header_before_context_session() {
        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", HeaderValue::from_static("header-session"));
        let (key, source) =
            existing_prompt_cache_key_candidate(&json!({}), &json!({}), &headers, Some("ctx"))
                .expect("candidate");

        assert_eq!(key, "header-session");
        assert_eq!(source, "header_session_id");
    }
}
