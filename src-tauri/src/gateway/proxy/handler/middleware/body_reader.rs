//! Middleware: reads the request body into memory, bounded by an OOM-safety
//! hard cap. The cap is configurable via `AIO_GATEWAY_MAX_REQUEST_BODY_MB`.
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
use crate::gateway::util::{body_for_introspection, max_request_body_bytes};
use axum::body::to_bytes;

pub(in crate::gateway::proxy::handler) struct BodyReaderMiddleware;

impl BodyReaderMiddleware {
    /// Reads the request body into `ctx.body_bytes` and parses introspection JSON.
    ///
    /// Also strips the `x-aio-provider-id` header (already consumed as `forced_provider_id`).
    pub(in crate::gateway::proxy::handler) async fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        let body = ctx
            .request_body
            .take()
            .expect("request_body must be set before BodyReaderMiddleware");
        ctx.headers.remove("x-aio-provider-id");

        let request_body_limit = max_request_body_bytes();
        match to_bytes(body, request_body_limit).await {
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
                    body_too_large_message(&err.to_string(), request_body_limit),
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

pub(in crate::gateway::proxy::handler) fn body_too_large_message(
    err: &str,
    limit_bytes: usize,
) -> String {
    let limit_mb = limit_bytes / (1024 * 1024);
    format!(
        "failed to read request body: {err} (gateway hard cap: {limit_mb} MB; \
         set AIO_GATEWAY_MAX_REQUEST_BODY_MB if this request is legitimate)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_too_large_message_includes_error() {
        let message = body_too_large_message("stream exceeded limit", 64 * 1024 * 1024);
        assert!(message.contains("failed to read request body:"));
        assert!(message.contains("stream exceeded limit"));
        assert!(message.contains("64 MB"));
    }
}
