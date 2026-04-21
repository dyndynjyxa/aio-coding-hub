//! Middleware: blocks recursive proxy loops by checking `x-aio-gateway-forwarded`.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::events::emit_gateway_log;
use crate::gateway::proxy::errors::error_response;
use crate::gateway::proxy::{is_internal_forwarded_request, GatewayErrorCode};
use axum::http::StatusCode;

pub(in crate::gateway::proxy::handler) struct RecursionGuardMiddleware;

impl RecursionGuardMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(ctx: ProxyContext) -> MiddlewareAction {
        if ctx.cli_key != "claude" || !is_internal_forwarded_request(&ctx.headers) {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        emit_gateway_log(
            &ctx.state.app,
            "warn",
            GatewayErrorCode::InternalError.as_str(),
            format!(
                "detected recursive claude proxy hop, blocking request \
                 trace_id={} path={}",
                ctx.trace_id, ctx.forwarded_path
            ),
        );

        MiddlewareAction::ShortCircuit(error_response(
            StatusCode::LOOP_DETECTED,
            ctx.trace_id,
            GatewayErrorCode::InternalError.as_str(),
            proxy_loop_detected_message(&ctx.cli_key),
            vec![],
        ))
    }
}

fn proxy_loop_detected_message(cli_key: &str) -> String {
    format!(
        "recursive proxy request blocked for cli_key={cli_key}; \
         upstream preserved aio internal forward marker"
    )
}
