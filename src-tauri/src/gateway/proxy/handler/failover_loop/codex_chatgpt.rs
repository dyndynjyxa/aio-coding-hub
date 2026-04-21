//! Usage: Codex ChatGPT backend compatibility helpers for `failover_loop`.

use crate::gateway::proxy::protocol_bridge::cx2cc as bridge_cx2cc;
use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

const CODEX_ORIGINATOR_HEADER_VALUE: &str = "codex_cli_rs";

pub(super) fn is_codex_chatgpt_backend(
    cli_key: &str,
    provider: &crate::providers::ProviderForGateway,
    provider_base_url: &str,
) -> bool {
    if cli_key != "codex" || provider.auth_mode != "oauth" {
        return false;
    }
    let Ok(url) = reqwest::Url::parse(provider_base_url) else {
        return false;
    };
    let path = url.path().trim_end_matches('/');
    path.ends_with("/backend-api/codex")
}

fn normalize_codex_chatgpt_forwarded_path(forwarded_path: &str) -> String {
    match forwarded_path.trim_end_matches('/') {
        "/v1" | "/v1/codex" | "/codex" => return "/".to_string(),
        _ => {}
    }

    if let Some(stripped) = forwarded_path.strip_prefix("/v1/codex/") {
        return format!("/{stripped}");
    }
    if let Some(stripped) = forwarded_path.strip_prefix("/v1/") {
        return format!("/{stripped}");
    }
    if let Some(stripped) = forwarded_path.strip_prefix("/codex/") {
        return format!("/{stripped}");
    }
    forwarded_path.to_string()
}

pub(super) fn parse_codex_chatgpt_account_id(id_token: Option<&str>) -> Option<String> {
    let token = id_token.map(str::trim).filter(|value| !value.is_empty())?;
    let payload_part = token.split('.').nth(1)?;
    let payload = URL_SAFE_NO_PAD.decode(payload_part).ok().or_else(|| {
        let mut padded = payload_part.to_string();
        while padded.len() % 4 != 0 {
            padded.push('=');
        }
        URL_SAFE_NO_PAD.decode(padded).ok()
    })?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    json.get("https://api.openai.com/auth")
        .and_then(|value| value.get("chatgpt_account_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn maybe_inject_codex_chatgpt_headers(
    headers: &mut HeaderMap,
    account_id: Option<&str>,
) {
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static(crate::gateway::oauth::DEFAULT_OAUTH_USER_AGENT),
    );
    if !headers.contains_key("originator") {
        headers.insert(
            "originator",
            HeaderValue::from_static(CODEX_ORIGINATOR_HEADER_VALUE),
        );
    }
    if headers.contains_key("chatgpt-account-id") {
        return;
    }
    let Some(value) = account_id.map(str::trim).filter(|value| !value.is_empty()) else {
        tracing::warn!("codex chatgpt: missing chatgpt-account-id, request may fail with 401");
        return;
    };
    if let Ok(header_value) = HeaderValue::from_str(value) {
        headers.insert("chatgpt-account-id", header_value);
    }
}

fn strip_headers_where(headers: &mut HeaderMap, should_strip: impl Fn(&str) -> bool) {
    let keys_to_remove: Vec<HeaderName> = headers
        .keys()
        .filter(|name| should_strip(name.as_str()))
        .cloned()
        .collect();

    for key in keys_to_remove {
        headers.remove(key);
    }
}

pub(super) fn strip_incompatible_protocol_headers(
    source_cli_key: &str,
    target_cli_key: &str,
    headers: &mut HeaderMap,
) {
    if source_cli_key == target_cli_key {
        return;
    }

    if let ("claude", "codex") = (source_cli_key, target_cli_key) {
        strip_headers_where(headers, |name| {
            name.starts_with("anthropic-")
                || name.starts_with("x-stainless-")
                || name.starts_with("x-claude-")
        });
    }
}

pub(super) fn codex_chatgpt_request_compat_value(root: &serde_json::Value) -> serde_json::Value {
    bridge_cx2cc::codex_chatgpt_request_compat_value(root)
}

pub(super) fn original_anthropic_stream_requested(
    introspection_json: Option<&serde_json::Value>,
) -> bool {
    bridge_cx2cc::original_anthropic_stream_requested(introspection_json)
}

pub(super) fn maybe_apply_codex_chatgpt_request_compat(
    forwarded_path: &mut String,
    upstream_body_bytes: &mut Bytes,
    strip_request_content_encoding: &mut bool,
) {
    *forwarded_path = normalize_codex_chatgpt_forwarded_path(forwarded_path);
    if forwarded_path.as_str() != "/responses" {
        return;
    }
    let Ok(root) = serde_json::from_slice::<serde_json::Value>(upstream_body_bytes.as_ref()) else {
        return;
    };
    let next = codex_chatgpt_request_compat_value(&root);
    if next == root {
        return;
    }
    if let Ok(encoded) = serde_json::to_vec(&next) {
        *upstream_body_bytes = Bytes::from(encoded);
        *strip_request_content_encoding = true;
    }
}

pub(super) fn should_apply_claude_model_mapping(cx2cc_active: bool, forwarded_path: &str) -> bool {
    if !cx2cc_active {
        return true;
    }

    !matches!(
        forwarded_path.trim_end_matches('/'),
        "/v1/responses" | "/responses"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        codex_chatgpt_request_compat_value, maybe_apply_codex_chatgpt_request_compat,
        maybe_inject_codex_chatgpt_headers, normalize_codex_chatgpt_forwarded_path,
        should_apply_claude_model_mapping, strip_incompatible_protocol_headers,
    };
    use axum::body::Bytes;
    use axum::http::{header, HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn skips_claude_model_mapping_for_cx2cc_responses_requests() {
        assert!(!should_apply_claude_model_mapping(true, "/v1/responses"));
        assert!(!should_apply_claude_model_mapping(true, "/responses"));
    }

    #[test]
    fn keeps_claude_model_mapping_for_non_cx2cc_requests() {
        assert!(should_apply_claude_model_mapping(false, "/v1/messages"));
        assert!(should_apply_claude_model_mapping(true, "/v1/messages"));
    }

    #[test]
    fn codex_chatgpt_request_compat_filters_unsupported_responses_fields() {
        let root = json!({
            "model": "gpt-5",
            "instructions": "system prompt",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "hi"}]}],
            "stream": true,
            "max_output_tokens": 1024,
            "temperature": 0.2,
            "top_p": 0.9,
            "store": true,
        });

        let next = codex_chatgpt_request_compat_value(&root);

        assert_eq!(next["model"], "gpt-5");
        assert_eq!(next["instructions"], "system prompt");
        assert_eq!(next["stream"], true);
        assert_eq!(next["store"], false);
        assert!(next.get("max_output_tokens").is_none());
        assert!(next.get("temperature").is_none());
        assert!(next.get("top_p").is_none());
    }

    #[test]
    fn codex_chatgpt_request_compat_rewrites_path_and_body_for_responses() {
        let mut forwarded_path = "/v1/responses".to_string();
        let mut upstream_body_bytes = Bytes::from(
            serde_json::to_vec(&json!({
                "model": "gpt-5",
                "instructions": "system prompt",
                "input": [],
                "stream": false,
                "max_output_tokens": 2048,
                "temperature": 0.3,
                "store": true,
            }))
            .unwrap(),
        );
        let mut strip_request_content_encoding = false;

        maybe_apply_codex_chatgpt_request_compat(
            &mut forwarded_path,
            &mut upstream_body_bytes,
            &mut strip_request_content_encoding,
        );

        let next: serde_json::Value = serde_json::from_slice(&upstream_body_bytes).unwrap();
        assert_eq!(forwarded_path, "/responses");
        assert_eq!(next["stream"], true);
        assert_eq!(next["store"], false);
        assert!(next.get("max_output_tokens").is_none());
        assert!(next.get("temperature").is_none());
        assert!(strip_request_content_encoding);
    }

    #[test]
    fn codex_chatgpt_normalizes_pi_local_v1_endpoint_paths() {
        assert_eq!(normalize_codex_chatgpt_forwarded_path("/v1/codex"), "/");
        assert_eq!(
            normalize_codex_chatgpt_forwarded_path("/v1/codex/responses"),
            "/responses"
        );
    }

    #[test]
    fn codex_chatgpt_request_compat_handles_pi_local_v1_codex_responses_path() {
        let mut forwarded_path = "/v1/codex/responses".to_string();
        let mut upstream_body_bytes = Bytes::from(
            serde_json::to_vec(&json!({
                "model": "gpt-5",
                "instructions": "system prompt",
                "input": [],
                "stream": false,
                "temperature": 0.3,
                "store": true,
            }))
            .unwrap(),
        );
        let mut strip_request_content_encoding = false;

        maybe_apply_codex_chatgpt_request_compat(
            &mut forwarded_path,
            &mut upstream_body_bytes,
            &mut strip_request_content_encoding,
        );

        let next: serde_json::Value = serde_json::from_slice(&upstream_body_bytes).unwrap();
        assert_eq!(forwarded_path, "/responses");
        assert_eq!(next["stream"], true);
        assert_eq!(next["store"], false);
        assert!(next.get("temperature").is_none());
        assert!(strip_request_content_encoding);
    }

    #[test]
    fn strip_incompatible_protocol_headers_removes_claude_specific_headers_for_codex() {
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_static("claude-code-20250219"),
        );
        headers.insert(
            "x-stainless-helper-method",
            HeaderValue::from_static("stream"),
        );
        headers.insert("x-claude-trace", HeaderValue::from_static("trace-1"));
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("text/event-stream"),
        );

        strip_incompatible_protocol_headers("claude", "codex", &mut headers);

        assert!(!headers.contains_key("anthropic-version"));
        assert!(!headers.contains_key("anthropic-beta"));
        assert!(!headers.contains_key("x-stainless-helper-method"));
        assert!(!headers.contains_key("x-claude-trace"));
        assert_eq!(
            headers
                .get(header::ACCEPT)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
    }

    #[test]
    fn codex_chatgpt_identity_headers_override_user_agent_and_originator() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("Claude-Code/1.0"),
        );

        maybe_inject_codex_chatgpt_headers(&mut headers, Some("acct_123"));

        assert_eq!(
            headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok()),
            Some(crate::gateway::oauth::DEFAULT_OAUTH_USER_AGENT)
        );
        assert_eq!(
            headers
                .get("originator")
                .and_then(|value| value.to_str().ok()),
            Some("codex_cli_rs")
        );
        assert_eq!(
            headers
                .get("chatgpt-account-id")
                .and_then(|value| value.to_str().ok()),
            Some("acct_123")
        );
    }
}
