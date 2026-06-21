//! Middleware: reads the request body into memory, bounded by an OOM-safety
//! hard cap. The cap is configurable via `AIO_GATEWAY_MAX_REQUEST_BODY_MB`.
//!
//! A smaller diagnostic threshold (`LARGE_REQUEST_BODY_BYTES`) is applied later
//! in `ModelInferenceMiddleware` and only affects requests whose `model` field
//! cannot be inferred. See `model_inference.rs` for that heuristic.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::plugins::context::{GatewayPluginHookName, GatewayRequestHookInput};
use crate::gateway::proxy::compute_observe_request;
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, respond_early_error_with_enqueue,
    EarlyErrorKind,
};
use crate::gateway::proxy::request_body::GatewayRequestBody;
use crate::gateway::proxy::{errors::error_response, GatewayErrorCode};
use crate::gateway::util::max_request_body_bytes;
use axum::body::to_bytes;
use axum::http::StatusCode;

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
        let mut request_body_state =
            GatewayRequestBody::from_wire(ctx.body_bytes.clone(), &ctx.headers, request_body_limit);
        ctx.body_bytes = request_body_state.decoded_clone();
        ctx.introspection_json =
            serde_json::from_slice::<serde_json::Value>(request_body_state.decoded().as_ref()).ok();

        let hook_input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: ctx.trace_id.clone(),
            cli_key: ctx.cli_key.clone(),
            method: ctx.req_method.clone(),
            path: ctx.forwarded_path.clone(),
            query: ctx.query.clone(),
            headers: request_body_state.semantic_headers(&ctx.headers),
            body: request_body_state.decoded_clone(),
            requested_model: ctx.requested_model.clone(),
        };
        match ctx.state.plugin_pipeline.run_request_hook(hook_input).await {
            Ok(output) => {
                crate::gateway::plugins::audit::persist_gateway_plugin_audit_events(
                    &ctx.state.db,
                    &ctx.trace_id,
                    output.audit_events.clone(),
                );
                if let Some(blocked) = output.blocked {
                    tracing::warn!(
                        trace_id = %ctx.trace_id,
                        status = blocked.status,
                        reason = %blocked.reason,
                        "plugin blocked gateway request after body read"
                    );
                    let mut resp = axum::response::IntoResponse::into_response((
                        axum::http::StatusCode::FORBIDDEN,
                        blocked.reason,
                    ));
                    resp.headers_mut().insert(
                        "x-trace-id",
                        axum::http::HeaderValue::from_str(&ctx.trace_id)
                            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("unknown")),
                    );
                    return MiddlewareAction::ShortCircuit(resp);
                }
                ctx.headers = output.headers;
                request_body_state.replace_decoded(output.body);
                ctx.body_bytes = request_body_state.decoded_clone();
                ctx.introspection_json = serde_json::from_slice::<serde_json::Value>(
                    request_body_state.decoded().as_ref(),
                )
                .ok();
            }
            Err(mut err) => {
                crate::gateway::plugins::audit::persist_gateway_plugin_error_audit_events(
                    &ctx.state.db,
                    &ctx.trace_id,
                    &mut err,
                );
                tracing::warn!(
                    trace_id = %ctx.trace_id,
                    "plugin afterBodyRead hook failed: {}",
                    err
                );
                return MiddlewareAction::ShortCircuit(error_response(
                    StatusCode::BAD_GATEWAY,
                    ctx.trace_id,
                    GatewayErrorCode::InternalError.as_str(),
                    format!("gateway plugin request hook failed: {err}"),
                    vec![],
                ));
            }
        }

        ctx.request_body_state = Some(request_body_state);
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
