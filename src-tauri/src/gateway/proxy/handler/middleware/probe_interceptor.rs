//! Middleware: detects Claude probe requests and responds locally.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::{build_claude_probe_response_body, is_claude_probe_request};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

pub(in crate::gateway::proxy::handler) struct ProbeInterceptorMiddleware;

impl ProbeInterceptorMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(ctx: ProxyContext) -> MiddlewareAction {
        if ctx.cli_key != "claude" {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        if !is_claude_probe_request(&ctx.forwarded_path, ctx.introspection_json.as_ref()) {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let mut resp = (StatusCode::OK, Json(build_claude_probe_response_body())).into_response();
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        if let Ok(value) = HeaderValue::from_str(&ctx.trace_id) {
            resp.headers_mut().insert("x-trace-id", value);
        }

        MiddlewareAction::ShortCircuit(resp)
    }
}
