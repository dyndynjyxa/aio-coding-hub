//! Usage: Upstream request sending helpers (first-byte timeout aware).

use super::context::CommonCtx;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};

pub(super) enum SendResult {
    Ok(reqwest::Response),
    Err(reqwest::Error),
    Timeout,
}

pub(super) async fn send_upstream(
    ctx: CommonCtx<'_>,
    method: Method,
    url: reqwest::Url,
    headers: HeaderMap,
    body: Bytes,
) -> SendResult {
    let client = ctx.state.client();
    let send = client
        .request(method, url)
        .headers(headers)
        .body(body)
        .send();

    if let Some(timeout) = ctx.upstream_first_byte_timeout {
        match tokio::time::timeout(timeout, send).await {
            Ok(Ok(resp)) => SendResult::Ok(resp),
            Ok(Err(err)) => SendResult::Err(err),
            Err(_) => SendResult::Timeout,
        }
    } else {
        match send.await {
            Ok(resp) => SendResult::Ok(resp),
            Err(err) => SendResult::Err(err),
        }
    }
}
