//! Usage: CX2CC (Claude-to-Codex) request preparation.
//!
//! Translates an Anthropic-format request into OpenAI Responses API format
//! via a source provider, including credential resolution, protocol bridge
//! invocation, base URL override, and Codex session ID completion.

use super::provider_iterator::SkipReason;
use super::*;
use crate::app::gateway_runtime_access::app_gateway_status;
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

pub(super) struct Cx2ccPreparationInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) input: &'a RequestContext<R>,
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
pub(super) async fn prepare<R: tauri::Runtime>(args: Cx2ccPreparationInput<'_, R>) -> Cx2ccOutcome {
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
    response_fixer::push_special_setting(
        &args.input.special_settings,
        serde_json::json!({
            "type": "cx2cc_cost_basis",
            "scope": "request",
            "source_cli_key": source_cli_key,
            "source_provider_id": args.source_id,
            "source_provider_name": source_provider_name,
            "priced_model": openai_model,
        }),
    );
    // DEBUG: dump translated body for troubleshooting.
    {
        let debug_body: serde_json::Value =
            serde_json::from_slice(&upstream_body_bytes).unwrap_or_default();
        let has_instructions = debug_body.get("instructions").is_some();
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
            &args.input.state.app,
            "debug",
            "CX2CC_REQUEST_BODY",
            format!(
                "[CX2CC] keys={keys:?} has_instructions={has_instructions} instructions_len={} model={model_val}",
                instructions_val.len(),
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
