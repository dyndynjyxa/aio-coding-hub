//! Middleware: infers the requested model from path/query/JSON body, computes
//! observe_request flag, and classifies the request kind (Claude `/compact`).
//!
//! Also applies the "large body + missing model" diagnostic heuristic aligned
//! with claude-code-hub's `LARGE_REQUEST_BODY_BYTES`: if the body exceeds
//! `LARGE_REQUEST_BODY_BYTES` and no model can be inferred from any source, we
//! return a 400 with a diagnostic message, because this combination is almost
//! always an upstream-client bug (truncation / non-JSON body / dropped model
//! field) rather than a legitimate request.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::compute_observe_request;
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, push_special_setting,
    respond_early_error_with_spawn, EarlyErrorKind,
};
use crate::gateway::proxy::model_rewrite::replace_model_in_body_json;
use crate::gateway::proxy::CLAUDE_LOGGED_MESSAGES_PATH;
use crate::gateway::util::{
    infer_requested_model_info, RequestedModelLocation, LARGE_REQUEST_BODY_BYTES,
};
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};

/// Claude Code `/compact` replaces the whole system prompt with this marker
/// (verified verbatim against claude-cli 2.1.198). Detection is best-effort:
/// if a future CLI version changes the wording, requests silently fall back to
/// the normal timeout policy.
const COMPACT_SYSTEM_PROMPT_PREFIX: &str =
    "You are a helpful AI assistant tasked with summarizing conversations.";

pub(in crate::gateway::proxy::handler) struct ModelInferenceMiddleware;

impl ModelInferenceMiddleware {
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        let model_info = infer_requested_model_info(
            &ctx.forwarded_path,
            ctx.query.as_deref(),
            ctx.introspection_json.as_ref(),
        );
        ctx.requested_model = model_info.model;
        ctx.requested_model_location = model_info.location;
        strip_claude_desktop_1m_marker(&mut ctx);
        if ctx.cli_key == "claude_desktop" && has_1m_context_beta(&ctx.headers) {
            ctx.requests_1m_context = true;
        }

        ctx.observe_request = compute_observe_request(
            &ctx.cli_key,
            &ctx.req_method,
            &ctx.forwarded_path,
            &ctx.headers,
            ctx.introspection_json.as_ref(),
        );

        ctx.is_compact_request = is_compact_request(
            &ctx.cli_key,
            &ctx.req_method,
            &ctx.forwarded_path,
            ctx.introspection_json.as_ref(),
        );
        if ctx.is_compact_request {
            push_special_setting(
                &ctx.special_settings,
                serde_json::json!({ "type": "request_kind", "kind": "compact" }),
            );
        }

        if is_large_body_missing_model(ctx.body_bytes.len(), ctx.requested_model.as_deref()) {
            let contract = early_error_contract(EarlyErrorKind::LargeBodyMissingModel);
            let message = large_body_missing_model_message(ctx.body_bytes.len());
            let special_settings_json =
                crate::gateway::response_fixer::special_settings_json(&ctx.special_settings);
            let log_ctx = build_early_error_log_ctx(&ctx);
            let resp = respond_early_error_with_spawn(
                &log_ctx,
                contract,
                message,
                special_settings_json,
                None,
                None,
            );
            return MiddlewareAction::ShortCircuit(resp);
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

/// Claude Desktop names the picker's 1M variant `<route>[1m]`. Its embedded
/// Claude Code strips the marker and sends a beta header instead, but Desktop's
/// direct one-shot requests (title generation etc.) send it verbatim. Strip it
/// before provider resolution so model policy eligibility and mappings see the
/// route ID, and no upstream receives the local marker.
fn strip_claude_desktop_1m_marker<R: tauri::Runtime>(ctx: &mut ProxyContext<R>) {
    if ctx.cli_key != "claude_desktop"
        || ctx.requested_model_location != Some(RequestedModelLocation::BodyJson)
    {
        return;
    }
    let Some(original) = ctx.requested_model.clone() else {
        return;
    };
    let Some(route) = strip_1m_marker(&original) else {
        return;
    };
    ctx.requests_1m_context = true;
    let Some(root) = ctx.introspection_json.as_mut() else {
        return;
    };
    if !replace_model_in_body_json(root, route) {
        return;
    }
    let Ok(next) = serde_json::to_vec(root) else {
        return;
    };
    ctx.body_bytes = Bytes::from(next);
    ctx.strip_request_content_encoding_seed = true;
    ctx.requested_model = Some(route.to_string());
    push_special_setting(
        &ctx.special_settings,
        serde_json::json!({
            "type": "claude_desktop_1m_marker",
            "scope": "request",
            "hit": true,
            "sourceModel": original,
            "targetModel": route,
        }),
    );
}

/// The embedded Claude Code asks for 1M through the `context-1m-*` beta.
fn has_1m_context_beta(headers: &HeaderMap) -> bool {
    headers
        .get_all("anthropic-beta")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|beta| beta.trim().starts_with("context-1m-"))
}

fn strip_1m_marker(model: &str) -> Option<&str> {
    const MARKER: &str = "[1m]";
    let split = model.len().checked_sub(MARKER.len())?;
    let (route, marker) = (model.get(..split)?, model.get(split..)?);
    let route = route.trim_end();
    (marker.eq_ignore_ascii_case(MARKER) && !route.is_empty()).then_some(route)
}

/// Detects a Claude Code `/compact` request.
///
/// Only inspects the parsed `system` field (array form, first block's `text`).
/// Never searches the raw body: conversation content may legitimately contain
/// the marker text and must not cause a false positive.
pub(in crate::gateway::proxy::handler) fn is_compact_request(
    cli_key: &str,
    method: &Method,
    forwarded_path: &str,
    introspection_json: Option<&serde_json::Value>,
) -> bool {
    if !matches!(cli_key, "claude" | "claude_desktop")
        || *method != Method::POST
        || forwarded_path != CLAUDE_LOGGED_MESSAGES_PATH
    {
        return false;
    }

    introspection_json
        .and_then(|root| root.get("system"))
        .and_then(|system| system.as_array())
        .and_then(|blocks| blocks.first())
        .and_then(|block| block.get("text"))
        .and_then(|text| text.as_str())
        .is_some_and(|text| text.starts_with(COMPACT_SYSTEM_PROMPT_PREFIX))
}

pub(in crate::gateway::proxy::handler) fn is_large_body_missing_model(
    body_len: usize,
    requested_model: Option<&str>,
) -> bool {
    body_len >= LARGE_REQUEST_BODY_BYTES && requested_model.map(str::is_empty).unwrap_or(true)
}

pub(in crate::gateway::proxy::handler) fn large_body_missing_model_message(
    body_len: usize,
) -> String {
    let body_mb = body_len as f64 / (1024.0 * 1024.0);
    let threshold_mb = LARGE_REQUEST_BODY_BYTES / (1024 * 1024);
    format!(
        "Missing required field 'model'. Request body ({body_mb:.1} MB) exceeded the \
         gateway's diagnostic threshold ({threshold_mb} MB). If you did send 'model', \
         the body may have been truncated or malformed by an upstream client/proxy. \
         Please verify the request body integrity and JSON format."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_triggers_when_body_large_and_model_missing() {
        assert!(is_large_body_missing_model(LARGE_REQUEST_BODY_BYTES, None));
        assert!(is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES + 1,
            None,
        ));
        assert!(is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES,
            Some(""),
        ));
    }

    #[test]
    fn heuristic_silent_when_model_present() {
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES,
            Some("claude-sonnet-4"),
        ));
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES * 2,
            Some("gpt-5"),
        ));
    }

    #[test]
    fn heuristic_silent_when_body_below_threshold() {
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES - 1,
            None,
        ));
        assert!(!is_large_body_missing_model(0, None));
    }

    #[test]
    fn strips_only_a_trailing_1m_marker() {
        assert_eq!(strip_1m_marker("claude-opus-5[1m]"), Some("claude-opus-5"));
        assert_eq!(strip_1m_marker("claude-opus-5 [1M]"), Some("claude-opus-5"));
        assert_eq!(strip_1m_marker("claude-opus-5"), None);
        assert_eq!(strip_1m_marker("[1m]"), None);
        assert_eq!(strip_1m_marker("deepseek[1m]-flash"), None);
    }

    #[test]
    fn detects_1m_context_beta_among_other_betas() {
        let mut headers = HeaderMap::new();
        assert!(!has_1m_context_beta(&headers));
        headers.append("anthropic-beta", "oauth-2025-04-20".parse().unwrap());
        assert!(!has_1m_context_beta(&headers));
        headers.append(
            "anthropic-beta",
            "fine-grained-tool-streaming-2025-05-14, context-1m-2025-08-07"
                .parse()
                .unwrap(),
        );
        assert!(has_1m_context_beta(&headers));
    }

    #[test]
    fn diagnostic_message_mentions_actual_size_and_threshold() {
        let message = large_body_missing_model_message(LARGE_REQUEST_BODY_BYTES + 1);
        assert!(message.contains("model"));
        assert!(message.contains(&format!("{} MB", LARGE_REQUEST_BODY_BYTES / (1024 * 1024))));
        assert!(message.contains("truncated"));
    }

    fn compact_body() -> serde_json::Value {
        serde_json::json!({
            "model": "claude-3-5-sonnet",
            "system": [
                {
                    "type": "text",
                    "text": format!("{COMPACT_SYSTEM_PROMPT_PREFIX} Follow the instructions."),
                }
            ],
            "messages": [{"role": "user", "content": "summarize"}]
        })
    }

    #[test]
    fn compact_request_detected_for_claude_messages_post() {
        for cli_key in ["claude", "claude_desktop"] {
            assert!(is_compact_request(
                cli_key,
                &Method::POST,
                "/v1/messages",
                Some(&compact_body()),
            ));
        }
    }

    #[test]
    fn compact_request_rejects_string_form_system() {
        let body = serde_json::json!({
            "system": COMPACT_SYSTEM_PROMPT_PREFIX,
            "messages": [{"role": "user", "content": "summarize"}]
        });
        assert!(!is_compact_request(
            "claude",
            &Method::POST,
            "/v1/messages",
            Some(&body),
        ));
    }

    #[test]
    fn compact_request_ignores_marker_text_inside_messages() {
        let body = serde_json::json!({
            "system": [{"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude."}],
            "messages": [
                {"role": "user", "content": COMPACT_SYSTEM_PROMPT_PREFIX}
            ]
        });
        assert!(!is_compact_request(
            "claude",
            &Method::POST,
            "/v1/messages",
            Some(&body),
        ));
    }

    #[test]
    fn compact_request_rejects_other_cli_key_path_or_method() {
        let body = compact_body();
        assert!(!is_compact_request(
            "codex",
            &Method::POST,
            "/v1/messages",
            Some(&body),
        ));
        assert!(!is_compact_request(
            "claude",
            &Method::POST,
            "/v1/messages/count_tokens",
            Some(&body),
        ));
        assert!(!is_compact_request(
            "claude",
            &Method::GET,
            "/v1/messages",
            Some(&body),
        ));
    }

    #[test]
    fn compact_request_falls_back_false_without_json_or_system() {
        assert!(!is_compact_request(
            "claude",
            &Method::POST,
            "/v1/messages",
            None,
        ));
        assert!(!is_compact_request(
            "claude",
            &Method::POST,
            "/v1/messages",
            Some(&serde_json::json!({"messages": []})),
        ));
    }
}
