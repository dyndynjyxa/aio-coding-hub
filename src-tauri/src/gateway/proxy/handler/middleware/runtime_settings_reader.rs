//! Middleware: reads application settings and populates `ctx.runtime_settings`.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::runtime_settings::handler_runtime_settings;
use crate::settings;

pub(in crate::gateway::proxy::handler) struct RuntimeSettingsMiddleware;

impl RuntimeSettingsMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let settings_cfg = match settings::read(&ctx.state.app) {
            Ok(cfg) => Some(cfg),
            Err(err) => {
                tracing::warn!(
                    "using default handler runtime settings because settings read failed: {err}"
                );
                None
            }
        };
        ctx.runtime_settings = Some(handler_runtime_settings(
            settings_cfg.as_ref(),
            ctx.is_claude_count_tokens,
        ));
        MiddlewareAction::Continue(Box::new(ctx))
    }
}
