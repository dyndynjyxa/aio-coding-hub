//! CX2CC (Claude-to-ChatGPT-Compatible) protocol bridge helpers.
//!
//! - **Model mapping**: translates Claude model names to OpenAI-compatible names
//!   using per-provider `ClaudeModels` overrides (or sensible defaults).
//! - **ChatGPT backend compat**: filters and adjusts request bodies so they
//!   conform to the ChatGPT Responses API field whitelist.

use super::traits::{BridgeContext, ModelMapper};
use crate::domain::providers::ClaudeModels;
use crate::gateway::proxy::cx2cc::settings::Cx2ccSettings;
use serde_json::Value;

// ─── Model Mapper ──────────────────────────────────────────────────────────

/// Maps Claude model names to OpenAI-compatible model names.
///
/// The mapping logic mirrors `cx2cc::models::map_claude_to_openai` but is
/// expressed as a [`ModelMapper`] trait implementation so the protocol bridge
/// framework can use it generically.
pub(crate) struct CX2CCModelMapper;

impl ModelMapper for CX2CCModelMapper {
    fn map(&self, source_model: &str, ctx: &BridgeContext) -> String {
        map_claude_to_openai(source_model, &ctx.claude_models, &ctx.cx2cc_settings)
    }
}

fn map_claude_to_openai(source_model: &str, cm: &ClaudeModels, settings: &Cx2ccSettings) -> String {
    if source_model.contains("opus") {
        if let Some(ref m) = cm.opus_model {
            return m.clone();
        }
        return settings.fallback_model_opus.clone();
    }

    if source_model.contains("haiku") {
        if let Some(ref m) = cm.haiku_model {
            return m.clone();
        }
        return settings.fallback_model_haiku.clone();
    }

    if source_model.contains("sonnet") {
        if let Some(ref m) = cm.sonnet_model {
            return m.clone();
        }
        return settings.fallback_model_sonnet.clone();
    }

    if let Some(ref m) = cm.main_model {
        return m.clone();
    }
    settings.fallback_model_main.clone()
}

// ─── ChatGPT Backend Compat ────────────────────────────────────────────────

/// Field whitelist for the ChatGPT Responses API.
///
/// When routing a Codex CLI request through a ChatGPT-compatible backend, only
/// these top-level keys are forwarded; everything else is stripped to avoid
/// 400-level rejections from the upstream provider.
pub(crate) const CODEX_CHATGPT_RESPONSES_ALLOWED_KEYS: &[&str] = &[
    "model",
    "instructions",
    "input",
    "tools",
    "tool_choice",
    "parallel_tool_calls",
    "store",
    "stream",
    "include",
    "reasoning",
    "service_tier",
    "prompt_cache_key",
    "text",
    "previous_response_id",
];

/// Filter a request body to only the ChatGPT Responses API allowed keys, then
/// force `stream: true` and `store: false`.
///
/// If `root` is not a JSON object it is returned unchanged.
pub(crate) fn codex_chatgpt_request_compat_value(root: &Value) -> Value {
    let Some(obj) = root.as_object() else {
        return root.clone();
    };

    let mut next = serde_json::Map::new();
    for key in CODEX_CHATGPT_RESPONSES_ALLOWED_KEYS {
        if let Some(value) = obj.get(*key).cloned() {
            next.insert((*key).to_string(), value);
        }
    }
    next.insert("stream".to_string(), Value::Bool(true));
    next.insert("store".to_string(), Value::Bool(false));
    Value::Object(next)
}

/// Returns `true` when the original Anthropic request body (captured before
/// CX2CC translation) had `"stream": true`.
///
/// `introspection_json` is the pre-translation snapshot of the request body
/// that the failover loop keeps for diagnostics / compat decisions.
pub(crate) fn original_anthropic_stream_requested(introspection_json: Option<&Value>) -> bool {
    introspection_json
        .and_then(|body| body.get("stream"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::providers::ClaudeModels;
    use serde_json::json;

    fn default_ctx() -> BridgeContext {
        BridgeContext {
            claude_models: ClaudeModels::default(),
            cx2cc_settings: Cx2ccSettings::default(),
            requested_model: None,
            mapped_model: None,
            stream_requested: false,
            is_chatgpt_backend: false,
        }
    }

    fn ctx_with_models(cm: ClaudeModels) -> BridgeContext {
        BridgeContext {
            claude_models: cm,
            ..default_ctx()
        }
    }

    // ── Model mapping: default values ──────────────────────────────────────

    #[test]
    fn maps_opus_to_default_o3() {
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-opus-20240229", &default_ctx()),
            "gpt-5.4"
        );
    }

    #[test]
    fn maps_haiku_to_default_gpt41_mini() {
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-haiku-20240307", &default_ctx()),
            "gpt-5.4"
        );
    }

    #[test]
    fn maps_sonnet_to_default_gpt41() {
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-5-sonnet-20241022", &default_ctx()),
            "gpt-5.4"
        );
    }

    #[test]
    fn maps_unknown_model_to_default_gpt41() {
        let mapper = CX2CCModelMapper;
        assert_eq!(mapper.map("some-unknown-model", &default_ctx()), "gpt-5.4");
    }

    // ── Model mapping: custom overrides ────────────────────────────────────

    #[test]
    fn opus_override() {
        let cm = ClaudeModels {
            opus_model: Some("my-opus".into()),
            ..ClaudeModels::default()
        };
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-opus-20240229", &ctx_with_models(cm)),
            "my-opus"
        );
    }

    #[test]
    fn haiku_override() {
        let cm = ClaudeModels {
            haiku_model: Some("my-haiku".into()),
            ..ClaudeModels::default()
        };
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-haiku-20240307", &ctx_with_models(cm)),
            "my-haiku"
        );
    }

    #[test]
    fn sonnet_override() {
        let cm = ClaudeModels {
            sonnet_model: Some("my-sonnet".into()),
            ..ClaudeModels::default()
        };
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("claude-3-5-sonnet-20241022", &ctx_with_models(cm)),
            "my-sonnet"
        );
    }

    #[test]
    fn main_model_override_for_unknown() {
        let cm = ClaudeModels {
            main_model: Some("my-main".into()),
            ..ClaudeModels::default()
        };
        let mapper = CX2CCModelMapper;
        assert_eq!(
            mapper.map("some-unknown-model", &ctx_with_models(cm)),
            "my-main"
        );
    }

    #[test]
    fn runtime_settings_override_fallbacks() {
        let mut ctx = default_ctx();
        ctx.cx2cc_settings = Cx2ccSettings {
            fallback_model_opus: "custom-opus".into(),
            fallback_model_sonnet: "custom-sonnet".into(),
            fallback_model_haiku: "custom-haiku".into(),
            fallback_model_main: "custom-main".into(),
            ..Cx2ccSettings::default()
        };

        let mapper = CX2CCModelMapper;
        assert_eq!(mapper.map("claude-3-opus-20240229", &ctx), "custom-opus");
        assert_eq!(mapper.map("claude-3-haiku-20240307", &ctx), "custom-haiku");
        assert_eq!(
            mapper.map("claude-3-5-sonnet-20241022", &ctx),
            "custom-sonnet"
        );
        assert_eq!(mapper.map("some-unknown-model", &ctx), "custom-main");
    }

    // ── ChatGPT compat filter ──────────────────────────────────────────────

    #[test]
    fn compat_keeps_allowed_keys_only() {
        let root = json!({
            "model": "gpt-5",
            "instructions": "system prompt",
            "input": "hello",
            "temperature": 0.7,
            "extra_field": true,
        });
        let next = codex_chatgpt_request_compat_value(&root);

        assert_eq!(next["model"], "gpt-5");
        assert_eq!(next["instructions"], "system prompt");
        assert_eq!(next["input"], "hello");
        // Stripped fields
        assert!(next.get("temperature").is_none());
        assert!(next.get("extra_field").is_none());
    }

    #[test]
    fn compat_forces_stream_true_and_store_false() {
        let root = json!({
            "model": "gpt-5",
            "stream": false,
            "store": true,
        });
        let next = codex_chatgpt_request_compat_value(&root);

        assert_eq!(next["stream"], true);
        assert_eq!(next["store"], false);
    }

    #[test]
    fn compat_returns_non_object_unchanged() {
        let root = json!("just a string");
        let next = codex_chatgpt_request_compat_value(&root);
        assert_eq!(next, root);
    }

    // ── original_anthropic_stream_requested ────────────────────────────────

    #[test]
    fn detects_stream_true() {
        let body = json!({"stream": true, "model": "claude-3"});
        assert!(original_anthropic_stream_requested(Some(&body)));
    }

    #[test]
    fn detects_stream_false() {
        let body = json!({"stream": false, "model": "claude-3"});
        assert!(!original_anthropic_stream_requested(Some(&body)));
    }

    #[test]
    fn returns_false_when_no_stream_field() {
        let body = json!({"model": "claude-3"});
        assert!(!original_anthropic_stream_requested(Some(&body)));
    }

    #[test]
    fn returns_false_when_none() {
        assert!(!original_anthropic_stream_requested(None));
    }
}
