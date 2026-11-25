//! Request body sanitization before sending upstream.
//!
//! This module owns request-body cleaning logic that is independent of
//! authentication concerns.  For example, Claude OAuth upstreams reject
//! messages that contain empty text blocks, so this module strips them
//! before the request is forwarded.

use super::provider_iterator::PreparedProvider;
use crate::gateway::proxy::request_context::RequestContext;
use axum::body::Bytes;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CleanBodyOutcome {
    pub(super) body: Bytes,
    pub(super) removed_empty_text_blocks: usize,
}

impl CleanBodyOutcome {
    pub(super) fn changed(&self) -> bool {
        self.removed_empty_text_blocks > 0
    }
}

/// Clean request body (e.g. remove empty text blocks for Claude OAuth).
pub(super) fn clean_body<R: tauri::Runtime>(
    input: &RequestContext<R>,
    prepared: &PreparedProvider,
) -> CleanBodyOutcome {
    if input.cli_key == "claude" && prepared.oauth_adapter.is_some() {
        return clean_claude_oauth_body(&prepared.upstream_body_bytes);
    }
    if input.cli_key == "gemini" {
        return CleanBodyOutcome {
            body: strip_unsupported_gemini_thinking_level(
                &prepared.upstream_forwarded_path,
                &prepared.upstream_body_bytes,
            ),
            removed_empty_text_blocks: 0,
        };
    }
    CleanBodyOutcome {
        body: prepared.upstream_body_bytes.clone(),
        removed_empty_text_blocks: 0,
    }
}

fn clean_claude_oauth_body(upstream_body_bytes: &Bytes) -> CleanBodyOutcome {
    let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(upstream_body_bytes) else {
        return CleanBodyOutcome {
            body: upstream_body_bytes.clone(),
            removed_empty_text_blocks: 0,
        };
    };

    let mut removed_empty_text_blocks = 0usize;
    if let Some(messages) = json.get_mut("messages").and_then(|v| v.as_array_mut()) {
        for msg in messages {
            if let Some(content) = msg.get_mut("content").and_then(|v| v.as_array_mut()) {
                let before_len = content.len();
                content.retain(|block| {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        !text.trim().is_empty()
                    } else {
                        true
                    }
                });
                removed_empty_text_blocks =
                    removed_empty_text_blocks.saturating_add(before_len - content.len());
            }
        }
    }

    if removed_empty_text_blocks == 0 {
        return CleanBodyOutcome {
            body: upstream_body_bytes.clone(),
            removed_empty_text_blocks: 0,
        };
    }

    CleanBodyOutcome {
        body: serde_json::to_vec(&json)
            .unwrap_or_else(|_| upstream_body_bytes.to_vec())
            .into(),
        removed_empty_text_blocks,
    }
}

/// Antigravity CLI sends `thinkingLevel` (a Gemini 3+ field) even to its fixed
/// gemini-2.5 background models, which upstreams reject with 400. Drop it for
/// pre-3 targets so the model uses its default thinking.
fn strip_unsupported_gemini_thinking_level(forwarded_path: &str, body: &Bytes) -> Bytes {
    let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(body) else {
        return body.clone();
    };
    let model = crate::gateway::util::infer_requested_model_info(forwarded_path, None, Some(&json))
        .model
        .unwrap_or_default();
    let model = model.strip_prefix("models/").unwrap_or(&model);
    if !(model.starts_with("gemini-1") || model.starts_with("gemini-2")) {
        return body.clone();
    }
    let mut removed = false;
    // Code Assist (OAuth) wraps the GenAI request in `request`.
    for pointer in [
        "/generationConfig/thinkingConfig",
        "/request/generationConfig/thinkingConfig",
    ] {
        if let Some(config) = json.pointer_mut(pointer).and_then(|v| v.as_object_mut()) {
            removed |= config.remove("thinkingLevel").is_some();
        }
    }
    if !removed {
        return body.clone();
    }
    serde_json::to_vec(&json).map_or_else(|_| body.clone(), Bytes::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;

    #[test]
    fn clean_claude_oauth_body_removes_empty_text_blocks() {
        let body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "text", "text": "   "},
                    {"type": "tool_result", "content": "ok"}
                ]
            }]
        });
        let encoded = Bytes::from(serde_json::to_vec(&body).unwrap());

        let outcome = clean_claude_oauth_body(&encoded);

        assert_eq!(outcome.removed_empty_text_blocks, 1);
        assert!(outcome.changed());
        let cleaned: serde_json::Value = serde_json::from_slice(&outcome.body).unwrap();
        let content = cleaned["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
    }

    #[test]
    fn gemini_thinking_level_is_stripped_only_for_pre_3_models() {
        let body = Bytes::from(
            serde_json::to_vec(&serde_json::json!({
                "contents": [],
                "generationConfig": {"thinkingConfig": {"thinkingLevel": "LOW", "includeThoughts": true}}
            }))
            .unwrap(),
        );
        let stripped = strip_unsupported_gemini_thinking_level(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
            &body,
        );
        let json: serde_json::Value = serde_json::from_slice(&stripped).unwrap();
        assert_eq!(
            json["generationConfig"]["thinkingConfig"],
            serde_json::json!({"includeThoughts": true})
        );

        for path in [
            "/v1beta/models/gemini-3-flash-preview:generateContent",
            "/v1beta/models/custom-model:generateContent",
        ] {
            assert_eq!(
                strip_unsupported_gemini_thinking_level(path, &body),
                body,
                "{path}"
            );
        }
    }

    #[test]
    fn gemini_thinking_level_is_stripped_from_code_assist_wrapper() {
        let body = Bytes::from(
            serde_json::to_vec(&serde_json::json!({
                "model": "gemini-2.5-flash-lite",
                "request": {"generationConfig": {"thinkingConfig": {"thinkingLevel": "MINIMAL"}}}
            }))
            .unwrap(),
        );
        let stripped =
            strip_unsupported_gemini_thinking_level("/v1internal:generateContent", &body);
        let json: serde_json::Value = serde_json::from_slice(&stripped).unwrap();
        assert_eq!(
            json["request"]["generationConfig"]["thinkingConfig"],
            serde_json::json!({})
        );
    }

    #[test]
    fn gemini_thinking_level_strip_keeps_unparseable_or_unchanged_bodies() {
        let path = "/v1beta/models/gemini-2.5-flash:generateContent";
        let invalid = Bytes::from_static(b"{not json");
        assert_eq!(
            strip_unsupported_gemini_thinking_level(path, &invalid),
            invalid
        );
        let no_thinking = Bytes::from_static(br#"{ "contents": [] }"#);
        assert_eq!(
            strip_unsupported_gemini_thinking_level(path, &no_thinking),
            no_thinking
        );
    }

    #[test]
    fn clean_claude_oauth_body_is_noop_without_empty_text_blocks() {
        let body = serde_json::json!({
            "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}]
        });
        let encoded = Bytes::from(serde_json::to_vec(&body).unwrap());

        let outcome = clean_claude_oauth_body(&encoded);

        assert_eq!(outcome.removed_empty_text_blocks, 0);
        assert_eq!(outcome.body, encoded);
    }
}
