//! Usage: Gateway proxy forwarding layer.

use super::request_context::RequestContext;
use axum::response::Response;

#[path = "../handler/failover_loop/mod.rs"]
mod failover_loop;

pub(super) async fn forward<R>(ctx: RequestContext<R>) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    failover_loop::run(ctx).await
}
