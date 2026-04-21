//! Middleware: auto-generates session IDs for Codex requests.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::codex_session_id;
use crate::gateway::proxy::handler::early_error::push_special_setting;
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;

pub(in crate::gateway::proxy::handler) struct CodexSessionCompletionMiddleware;

impl CodexSessionCompletionMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let enabled = ctx
            .runtime_settings
            .as_ref()
            .map(|rs| rs.enable_codex_session_id_completion)
            .unwrap_or(true);

        if ctx.cli_key != "codex" || !enabled {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let result = {
            let mut cache = ctx.state.codex_session_cache.lock_or_recover();
            codex_session_id::complete_codex_session_identifiers(
                &mut cache,
                ctx.created_at,
                ctx.created_at_ms,
                &mut ctx.headers,
                ctx.introspection_json.as_mut(),
            )
        };

        if result.changed_body {
            if let Some(root) = ctx.introspection_json.as_ref() {
                if let Ok(next) = serde_json::to_vec(root) {
                    ctx.body_bytes = Bytes::from(next);
                    ctx.strip_request_content_encoding_seed = true;
                }
            }
        }

        push_special_setting(
            &ctx.special_settings,
            serde_json::json!({
                "type": "codex_session_id_completion",
                "scope": "request",
                "hit": result.applied,
                "sessionId": result.session_id,
                "action": result.action,
                "source": result.source,
                "changedHeader": result.changed_headers,
                "changedBody": result.changed_body,
            }),
        );

        MiddlewareAction::Continue(Box::new(ctx))
    }
}
