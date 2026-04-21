//! Middleware: removes invalid billing headers from Claude requests.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::push_special_setting;
use axum::body::Bytes;

pub(in crate::gateway::proxy::handler) struct BillingHeaderRectifierMiddleware;

impl BillingHeaderRectifierMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let enabled = ctx
            .runtime_settings
            .as_ref()
            .map(|rs| rs.enable_billing_header_rectifier)
            .unwrap_or(true);

        if ctx.cli_key != "claude" || !enabled {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let Some(root) = ctx.introspection_json.as_mut() else {
            return MiddlewareAction::Continue(Box::new(ctx));
        };

        let result = crate::gateway::billing_header_rectifier::rectify(root);
        if result.applied {
            if let Ok(next) = serde_json::to_vec(root) {
                ctx.body_bytes = Bytes::from(next);
                ctx.strip_request_content_encoding_seed = true;
            }
            push_special_setting(
                &ctx.special_settings,
                serde_json::json!({
                    "type": "billing_header_rectifier",
                    "scope": "request",
                    "hit": true,
                    "removedCount": result.removed_count,
                }),
            );
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}
