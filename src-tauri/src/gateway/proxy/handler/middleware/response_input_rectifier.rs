//! Middleware: normalizes non-array Responses API input for Codex and Grok.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::push_special_setting;
use crate::gateway::response_input_rectifier;
use axum::body::Bytes;

pub(in crate::gateway::proxy::handler) struct ResponseInputRectifierMiddleware;

impl ResponseInputRectifierMiddleware {
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        let enabled = ctx
            .runtime_settings
            .as_ref()
            .map(|settings| settings.enable_response_input_rectifier)
            .unwrap_or(true);
        if !enabled
            || !matches!(ctx.cli_key.as_str(), "codex" | "grok")
            || !is_responses_path(&ctx.forwarded_path)
        {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let Some(root) = ctx.introspection_json.as_mut() else {
            return MiddlewareAction::Continue(Box::new(ctx));
        };
        let Some(result) = response_input_rectifier::rectify_response_input(root) else {
            return MiddlewareAction::Continue(Box::new(ctx));
        };
        let Ok(next) = serde_json::to_vec(root) else {
            return MiddlewareAction::Continue(Box::new(ctx));
        };

        ctx.body_bytes = Bytes::from(next);
        ctx.strip_request_content_encoding_seed = true;
        push_special_setting(
            &ctx.special_settings,
            serde_json::json!({
                "type": "response_input_rectifier",
                "scope": "request",
                "hit": true,
                "action": result.action.as_str(),
                "originalType": result.original_type.as_str(),
            }),
        );

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

fn is_responses_path(path: &str) -> bool {
    matches!(path.trim_end_matches('/'), "/responses" | "/v1/responses")
}

#[cfg(test)]
mod tests {
    use super::is_responses_path;

    #[test]
    fn accepts_supported_responses_paths_only() {
        for path in [
            "/responses",
            "/responses/",
            "/v1/responses",
            "/v1/responses/",
        ] {
            assert!(is_responses_path(path));
        }
        for path in ["/v1/chat/completions", "/v1/responses/extra", "responses"] {
            assert!(!is_responses_path(path));
        }
    }
}
