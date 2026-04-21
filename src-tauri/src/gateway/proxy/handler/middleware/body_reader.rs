//! Middleware: reads the request body into memory, bounded by an OOM-safety
//! hard cap (`MAX_REQUEST_BODY_BYTES`). The cap is intentionally large — normal
//! AI CLI traffic (uploads, PDFs, long histories) must pass through transparently.
//!
//! A smaller diagnostic threshold (`LARGE_REQUEST_BODY_BYTES`) is applied later
//! in `ModelInferenceMiddleware` and only affects requests whose `model` field
//! cannot be inferred. See `model_inference.rs` for that heuristic.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::compute_observe_request;
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, respond_early_error_with_enqueue,
    EarlyErrorKind,
};
use crate::gateway::util::{body_for_introspection, MAX_REQUEST_BODY_BYTES};
use axum::body::to_bytes;

pub(in crate::gateway::proxy::handler) struct BodyReaderMiddleware;

impl BodyReaderMiddleware {
    /// Reads the request body into `ctx.body_bytes` and parses introspection JSON.
    ///
    /// Also strips the `x-aio-provider-id` header (already consumed as `forced_provider_id`).
    pub(in crate::gateway::proxy::handler) async fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        let body = ctx
            .request_body
            .take()
            .expect("request_body must be set before BodyReaderMiddleware");
        ctx.headers.remove("x-aio-provider-id");

        match to_bytes(body, MAX_REQUEST_BODY_BYTES).await {
            Ok(bytes) => {
                ctx.body_bytes = bytes;
            }
            Err(err) => {
                ctx.observe_request =
                    compute_observe_request(&ctx.cli_key, &ctx.forwarded_path, &ctx.headers, None);
                let contract = early_error_contract(EarlyErrorKind::BodyTooLarge);
                let log_ctx = build_early_error_log_ctx(&ctx);

                let resp = respond_early_error_with_enqueue(
                    &log_ctx,
                    contract,
                    body_too_large_message(&err.to_string()),
                    None,
                    None,
                    None,
                )
                .await;
                return MiddlewareAction::ShortCircuit(resp);
            }
        }

        let introspection_body = body_for_introspection(&ctx.headers, &ctx.body_bytes);
        ctx.introspection_json =
            serde_json::from_slice::<serde_json::Value>(introspection_body.as_ref()).ok();

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

pub(in crate::gateway::proxy::handler) fn body_too_large_message(err: &str) -> String {
    let limit_mb = MAX_REQUEST_BODY_BYTES / (1024 * 1024);
    format!(
        "failed to read request body: {err} (gateway hard cap: {limit_mb} MB; \
         this cap exists only to bound memory — contact the administrator if you \
         have a legitimate request above this size)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_too_large_message_includes_error() {
        let message = body_too_large_message("stream exceeded limit");
        assert!(message.contains("failed to read request body:"));
        assert!(message.contains("stream exceeded limit"));
    }
}
