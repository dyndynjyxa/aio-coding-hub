//! Middleware: computes request fingerprints and applies recent-error-cache gate.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::request_fingerprint as fp;

pub(in crate::gateway::proxy::handler) struct RequestFingerprintMiddleware;

impl RequestFingerprintMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let fingerprints = fp::build_request_fingerprints(
            &ctx.cli_key,
            ctx.effective_sort_mode_id,
            &ctx.method_hint,
            &ctx.forwarded_path,
            ctx.query.as_deref(),
            ctx.session_id.as_deref(),
            ctx.requested_model.as_deref(),
            &ctx.headers,
            &ctx.body_bytes,
        );

        match fp::apply_recent_error_cache_gate(
            &ctx.state.recent_errors,
            &fingerprints,
            ctx.trace_id,
        ) {
            Ok(next_trace_id) => {
                ctx.trace_id = next_trace_id;
            }
            Err(resp) => {
                return MiddlewareAction::ShortCircuit(*resp);
            }
        }

        ctx.fingerprint_key = fingerprints.fingerprint_key;
        ctx.fingerprint_debug = fingerprints.fingerprint_debug;
        ctx.unavailable_fingerprint_key = fingerprints.unavailable_fingerprint_key;
        ctx.unavailable_fingerprint_debug = fingerprints.unavailable_fingerprint_debug;

        MiddlewareAction::Continue(Box::new(ctx))
    }
}
