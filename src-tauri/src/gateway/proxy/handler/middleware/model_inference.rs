//! Middleware: infers the requested model from path/query/JSON body and computes
//! observe_request flag.
//!
//! Also applies the "large body + missing model" diagnostic heuristic aligned
//! with claude-code-hub's `LARGE_REQUEST_BODY_BYTES`: if the body exceeds
//! `LARGE_REQUEST_BODY_BYTES` and no model can be inferred from any source, we
//! return a 400 with a diagnostic message, because this combination is almost
//! always an upstream-client bug (truncation / non-JSON body / dropped model
//! field) rather than a legitimate request.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::compute_observe_request;
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, respond_early_error_with_spawn, EarlyErrorKind,
};
use crate::gateway::util::{infer_requested_model_info, LARGE_REQUEST_BODY_BYTES};

pub(in crate::gateway::proxy::handler) struct ModelInferenceMiddleware;

impl ModelInferenceMiddleware {
    pub(in crate::gateway::proxy::handler) fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let model_info = infer_requested_model_info(
            &ctx.forwarded_path,
            ctx.query.as_deref(),
            ctx.introspection_json.as_ref(),
        );
        ctx.requested_model = model_info.model;
        ctx.requested_model_location = model_info.location;

        ctx.observe_request = compute_observe_request(
            &ctx.cli_key,
            &ctx.forwarded_path,
            &ctx.headers,
            ctx.introspection_json.as_ref(),
        );

        if is_large_body_missing_model(ctx.body_bytes.len(), ctx.requested_model.as_deref()) {
            let contract = early_error_contract(EarlyErrorKind::LargeBodyMissingModel);
            let message = large_body_missing_model_message(ctx.body_bytes.len());
            let log_ctx = build_early_error_log_ctx(&ctx);
            let resp =
                respond_early_error_with_spawn(&log_ctx, contract, message, None, None, None);
            return MiddlewareAction::ShortCircuit(resp);
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

pub(in crate::gateway::proxy::handler) fn is_large_body_missing_model(
    body_len: usize,
    requested_model: Option<&str>,
) -> bool {
    body_len >= LARGE_REQUEST_BODY_BYTES && requested_model.map(str::is_empty).unwrap_or(true)
}

pub(in crate::gateway::proxy::handler) fn large_body_missing_model_message(
    body_len: usize,
) -> String {
    let body_mb = body_len as f64 / (1024.0 * 1024.0);
    let threshold_mb = LARGE_REQUEST_BODY_BYTES / (1024 * 1024);
    format!(
        "Missing required field 'model'. Request body ({body_mb:.1} MB) exceeded the \
         gateway's diagnostic threshold ({threshold_mb} MB). If you did send 'model', \
         the body may have been truncated or malformed by an upstream client/proxy. \
         Please verify the request body integrity and JSON format."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_triggers_when_body_large_and_model_missing() {
        assert!(is_large_body_missing_model(LARGE_REQUEST_BODY_BYTES, None));
        assert!(is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES + 1,
            None,
        ));
        assert!(is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES,
            Some(""),
        ));
    }

    #[test]
    fn heuristic_silent_when_model_present() {
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES,
            Some("claude-sonnet-4"),
        ));
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES * 2,
            Some("gpt-5"),
        ));
    }

    #[test]
    fn heuristic_silent_when_body_below_threshold() {
        assert!(!is_large_body_missing_model(
            LARGE_REQUEST_BODY_BYTES - 1,
            None,
        ));
        assert!(!is_large_body_missing_model(0, None));
    }

    #[test]
    fn diagnostic_message_mentions_actual_size_and_threshold() {
        let message = large_body_missing_model_message(LARGE_REQUEST_BODY_BYTES + 1);
        assert!(message.contains("model"));
        assert!(message.contains(&format!("{} MB", LARGE_REQUEST_BODY_BYTES / (1024 * 1024))));
        assert!(message.contains("truncated"));
    }
}
