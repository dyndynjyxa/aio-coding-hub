use axum::{
    body::Body,
    extract::{Path, State},
    http::Request,
    response::Response,
    routing::{any, get},
    Json, Router,
};
use serde::Serialize;

use super::proxy::proxy_impl;
use super::runtime::GatewayAppState;
use super::util::now_unix_seconds;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    app: &'static str,
    version: &'static str,
    ts: u64,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "aio-coding-hub",
        version: env!("CARGO_PKG_VERSION"),
        ts: now_unix_seconds(),
    })
}

async fn root() -> &'static str {
    "AIO Coding Hub is running"
}

async fn proxy_cli_any(
    State(state): State<GatewayAppState>,
    Path((cli_key, path)): Path<(String, String)>,
    req: Request<Body>,
) -> Response {
    let forwarded_path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{path}")
    };
    proxy_impl(state, cli_key, forwarded_path, req).await
}

async fn proxy_cli_with_provider_any(
    State(state): State<GatewayAppState>,
    Path((cli_key, provider_id, path)): Path<(String, i64, String)>,
    mut req: Request<Body>,
) -> Response {
    if let Ok(value) = axum::http::HeaderValue::from_str(&provider_id.to_string()) {
        req.headers_mut().insert("x-aio-provider-id", value);
    }

    let forwarded_path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{path}")
    };

    proxy_impl(state, cli_key, forwarded_path, req).await
}

async fn proxy_openai_v1_any(
    State(state): State<GatewayAppState>,
    Path(path): Path<String>,
    req: Request<Body>,
) -> Response {
    let forwarded_path = if path.is_empty() {
        "/v1".to_string()
    } else {
        format!("/v1/{path}")
    };
    proxy_impl(state, "codex".to_string(), forwarded_path, req).await
}

async fn proxy_openai_v1_root(
    State(state): State<GatewayAppState>,
    req: Request<Body>,
) -> Response {
    proxy_impl(state, "codex".to_string(), "/v1".to_string(), req).await
}

pub(super) fn build_router(state: GatewayAppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route(
            "/:cli_key/_aio/provider/:provider_id/*path",
            any(proxy_cli_with_provider_any),
        )
        .route("/v1", any(proxy_openai_v1_root))
        .route("/v1/*path", any(proxy_openai_v1_any))
        .route("/:cli_key/*path", any(proxy_cli_any))
        .with_state(state)
}
