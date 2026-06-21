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

async fn proxy_cli_any<R>(
    State(state): State<GatewayAppState<R>>,
    Path((cli_key, path)): Path<(String, String)>,
    req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let forwarded_path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{path}")
    };
    proxy_impl(state, cli_key, forwarded_path, req).await
}

async fn proxy_cli_with_provider_any<R>(
    State(state): State<GatewayAppState<R>>,
    Path((cli_key, provider_id, path)): Path<(String, i64, String)>,
    mut req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
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

async fn proxy_openai_v1_any<R>(
    State(state): State<GatewayAppState<R>>,
    Path(path): Path<String>,
    req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let forwarded_path = if path.is_empty() {
        "/v1".to_string()
    } else {
        format!("/v1/{path}")
    };
    proxy_impl(state, "codex".to_string(), forwarded_path, req).await
}

async fn proxy_openai_v1_root<R>(
    State(state): State<GatewayAppState<R>>,
    req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    proxy_impl(state, "codex".to_string(), "/v1".to_string(), req).await
}

pub(super) fn build_router<R>(state: GatewayAppState<R>) -> Router
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route(
            "/:cli_key/_aio/provider/:provider_id/*path",
            any(proxy_cli_with_provider_any::<R>),
        )
        .route("/v1", any(proxy_openai_v1_root::<R>))
        .route("/v1/*path", any(proxy_openai_v1_any::<R>))
        .route("/:cli_key/*path", any(proxy_cli_any::<R>))
        .with_state(state)
}

#[cfg(test)]
#[allow(clippy::await_holding_lock, clippy::field_reassign_with_default)]
mod tests {
    use super::build_router;
    use crate::app::plugins::{official, runtime_executor::RuntimeGatewayPluginExecutor};
    use crate::domain::plugins::{
        PluginDetail, PluginHook, PluginHostCompatibility, PluginInstallSource, PluginManifest,
        PluginPermissionRisk, PluginRuntime, PluginStatus, PluginSummary,
    };
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use crate::gateway::plugins::context::{GatewayHookResult, GatewayPluginHookName};
    use crate::gateway::plugins::pipeline::{
        GatewayPluginPipeline, GatewayPluginPipelineConfig, InMemoryGatewayPluginExecutor,
    };
    use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
    use crate::gateway::runtime::GatewayAppState;
    use crate::infra::plugins::repository;
    use crate::{circuit_breaker, db, providers, request_logs, session_manager, settings};
    use axum::body::HttpBody;
    use axum::body::{to_bytes, Body};
    use axum::http::{header, Method, Request, StatusCode};
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tower::ServiceExt;

    #[derive(Default)]
    struct EnvRestore {
        saved: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvRestore {
        fn save_once(&mut self, key: &'static str) {
            if self.saved.iter().any(|(saved, _)| *saved == key) {
                return;
            }
            self.saved.push((key, std::env::var_os(key)));
        }

        fn set_var(&mut self, key: &'static str, value: impl Into<OsString>) {
            self.save_once(key);
            std::env::set_var(key, value.into());
        }

        fn remove_var(&mut self, key: &'static str) {
            self.save_once(key);
            std::env::remove_var(key);
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.saved.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            settings::clear_cache();
        }
    }

    fn isolate_app_env(home: &std::path::Path) -> EnvRestore {
        let mut env = EnvRestore::default();
        let home_os = home.as_os_str().to_os_string();
        env.set_var("HOME", home_os.clone());
        env.set_var("AIO_CODING_HUB_HOME_DIR", home_os.clone());
        env.set_var("USERPROFILE", home_os);
        env.set_var("AIO_CODING_HUB_DOTDIR_NAME", ".aio-coding-hub-route-test");
        env.remove_var("CODEX_HOME");
        settings::clear_cache();
        env
    }

    async fn spawn_hanging_upstream() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind upstream stub");
        let addr = listener.local_addr().expect("upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_json_upstream(body: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind json upstream stub");
        let addr = listener.local_addr().expect("json upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), task)
    }

    #[derive(Debug)]
    struct CapturedRawRequest {
        head: String,
        body: Vec<u8>,
    }

    impl CapturedRawRequest {
        fn text(&self) -> String {
            let mut out = self.head.clone();
            out.push_str("\r\n\r\n");
            out.push_str(&String::from_utf8_lossy(&self.body));
            out
        }

        fn has_header_line(&self, needle: &str) -> bool {
            self.head
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        }
    }

    fn find_http_head_split(bytes: &[u8]) -> Option<(usize, usize)> {
        let marker = b"\r\n\r\n";
        bytes
            .windows(marker.len())
            .position(|window| window == marker)
            .map(|idx| (idx, idx + marker.len()))
    }

    async fn read_complete_http_request_bytes(socket: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let Ok(size) = socket.read(&mut chunk).await else {
                break;
            };
            if size == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..size]);
            if buf.len() > 64 * 1024 {
                break;
            }

            let Some((head_start, body_start)) = find_http_head_split(&buf) else {
                continue;
            };
            let headers = String::from_utf8_lossy(&buf[..head_start]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if buf.len().saturating_sub(body_start) >= content_length {
                break;
            }
        }
        buf
    }

    fn split_raw_http_request(bytes: Vec<u8>) -> CapturedRawRequest {
        let Some((head_start, body_start)) = find_http_head_split(&bytes) else {
            return CapturedRawRequest {
                head: String::from_utf8_lossy(&bytes).to_string(),
                body: Vec::new(),
            };
        };
        CapturedRawRequest {
            head: String::from_utf8_lossy(&bytes[..head_start]).to_string(),
            body: bytes[body_start..].to_vec(),
        }
    }

    async fn read_complete_http_request(socket: &mut tokio::net::TcpStream) -> String {
        let buf = read_complete_http_request_bytes(socket).await;
        String::from_utf8_lossy(&buf).to_string()
    }

    async fn spawn_capturing_json_upstream(
        body: &'static str,
    ) -> (
        String,
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind capturing json upstream stub");
        let addr = listener.local_addr().expect("capturing upstream addr");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let request = read_complete_http_request(&mut socket).await;
                let captured_body = request
                    .split_once("\r\n\r\n")
                    .map(|(_, body)| body.to_string())
                    .unwrap_or_default();
                let _ = tx.send(captured_body);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), rx, task)
    }

    async fn spawn_capturing_raw_upstream(
        body: &'static str,
    ) -> (
        String,
        tokio::sync::oneshot::Receiver<CapturedRawRequest>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind capturing raw upstream stub");
        let addr = listener.local_addr().expect("capturing raw upstream addr");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let request =
                    split_raw_http_request(read_complete_http_request_bytes(&mut socket).await);
                let _ = tx.send(request);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), rx, task)
    }

    async fn spawn_codex_previous_response_retry_upstream() -> (
        String,
        tokio::sync::mpsc::Receiver<CapturedRawRequest>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind retry upstream stub");
        let addr = listener.local_addr().expect("retry upstream addr");
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        let task = tokio::spawn(async move {
            for index in 0..2 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let request =
                    split_raw_http_request(read_complete_http_request_bytes(&mut socket).await);
                let _ = tx.send(request).await;
                let (status_line, body) = if index == 0 {
                    (
                        "400 Bad Request",
                        r#"{"error":{"message":"No response found for previous_response_id resp_old","param":"previous_response_id"}}"#,
                    )
                } else {
                    (
                        "200 OK",
                        r#"{"id":"stub-ok","object":"response","output":[]}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), rx, task)
    }

    fn gzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).expect("gzip write");
        encoder.finish().expect("gzip finish")
    }

    fn gunzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut decoder = flate2::read::GzDecoder::new(input);
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut out).expect("gzip read");
        out
    }

    async fn spawn_status_json_upstream(
        status_line: &'static str,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind status json upstream stub");
        let addr = listener.local_addr().expect("status json upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_large_known_length_error_upstream(
        status_line: &'static str,
        declared_content_length: usize,
        sent_body: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind large error upstream stub");
        let addr = listener.local_addr().expect("large error upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let headers = format!(
                    "HTTP/1.1 {status_line}\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {declared_content_length}\r\nconnection: keep-alive\r\n\r\n"
                );
                let _ = socket.write_all(headers.as_bytes()).await;
                let _ = socket.write_all(&sent_body).await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_unknown_length_json_upstream(
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind unknown-length json upstream stub");
        let addr = listener
            .local_addr()
            .expect("unknown-length json upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{}",
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_sse_upstream(body: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind sse upstream stub");
        let addr = listener.local_addr().expect("sse upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_stalling_sse_upstream(
        first_chunk: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stalling sse upstream stub");
        let addr = listener.local_addr().expect("stalling sse upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let headers = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "content-type: text/event-stream; charset=utf-8\r\n",
                    "transfer-encoding: chunked\r\n",
                    "connection: keep-alive\r\n",
                    "\r\n"
                );
                let _ = socket.write_all(headers.as_bytes()).await;
                let chunk = format!("{:X}\r\n{}\r\n", first_chunk.len(), first_chunk);
                let _ = socket.write_all(chunk.as_bytes()).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });

        (format!("http://{addr}"), task)
    }

    async fn spawn_delayed_chunked_sse_upstream(
        first_chunk: &'static str,
        second_chunk: &'static str,
        delay: Duration,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind delayed sse upstream stub");
        let addr = listener.local_addr().expect("delayed sse upstream addr");
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0_u8; 1024];
                let _ = socket.read(&mut buf).await;
                let headers = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "content-type: text/event-stream; charset=utf-8\r\n",
                    "transfer-encoding: chunked\r\n",
                    "connection: close\r\n",
                    "\r\n"
                );
                let _ = socket.write_all(headers.as_bytes()).await;
                let first = format!("{:X}\r\n{}\r\n", first_chunk.len(), first_chunk);
                let _ = socket.write_all(first.as_bytes()).await;
                tokio::time::sleep(delay).await;
                let second = format!("{:X}\r\n{}\r\n0\r\n\r\n", second_chunk.len(), second_chunk);
                let _ = socket.write_all(second.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        (format!("http://{addr}"), task)
    }

    fn insert_provider_with_priority(
        db: &db::Db,
        cli_key: &str,
        name: &str,
        base_url: String,
        priority: i64,
    ) -> i64 {
        providers::upsert(
            db,
            providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: cli_key.to_string(),
                name: name.to_string(),
                base_urls: vec![base_url],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some("sk-test".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(priority),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert provider")
        .id
    }

    fn insert_codex_provider_with_priority(
        db: &db::Db,
        name: &str,
        base_url: String,
        priority: i64,
    ) -> i64 {
        insert_provider_with_priority(db, "codex", name, base_url, priority)
    }

    fn insert_codex_provider(db: &db::Db, base_url: String) -> i64 {
        insert_codex_provider_with_priority(db, "Timeout Stub", base_url, 0)
    }

    fn insert_codex_oauth_provider_with_priority(db: &db::Db, name: &str, priority: i64) -> i64 {
        providers::upsert(
            db,
            providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: name.to_string(),
                base_urls: vec![],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(providers::ProviderAuthMode::Oauth),
                api_key: None,
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(priority),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert oauth provider")
        .id
    }

    fn insert_cx2cc_bridge_provider(db: &db::Db, source_provider_id: i64, priority: i64) -> i64 {
        providers::upsert(
            db,
            providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "claude".to_string(),
                name: "CX2CC Bridge Stub".to_string(),
                base_urls: vec![],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: None,
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(priority),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: Some(source_provider_id),
                bridge_type: Some("cx2cc".to_string()),
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert cx2cc bridge provider")
        .id
    }

    async fn recv_terminal_request_log(
        log_rx: &mut tokio::sync::mpsc::Receiver<request_logs::RequestLogInsert>,
    ) -> request_logs::RequestLogInsert {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let log = log_rx.recv().await.expect("request log item");
                if log.status.is_some() {
                    break log;
                }
            }
        })
        .await
        .expect("terminal request log enqueue")
    }

    fn gateway_state(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        gateway_state_with_parts(
            app,
            db,
            log_tx,
            Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            Arc::new(session_manager::SessionManager::new()),
        )
    }

    fn gateway_state_with_parts(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        circuit: Arc<circuit_breaker::CircuitBreaker>,
        session: Arc<session_manager::SessionManager>,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        GatewayAppState {
            app,
            db,
            log_tx,
            circuit,
            session,
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: Arc::new(Mutex::new(RecentErrorCache::default())),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline: GatewayPluginPipeline::empty_shared(),
        }
    }

    fn gateway_state_with_plugin_pipeline(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        plugin_pipeline: Arc<GatewayPluginPipeline>,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        let mut state = gateway_state(app, db, log_tx);
        state.plugin_pipeline = plugin_pipeline;
        state
    }

    fn request_rewrite_plugin() -> PluginDetail {
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: "test.request-rewrite".to_string(),
                name: "Request Rewrite".to_string(),
                current_version: Some("1.0.0".to_string()),
                status: PluginStatus::Enabled,
                runtime: "declarativeRules".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: PluginManifest {
                id: "test.request-rewrite".to_string(),
                name: "Request Rewrite".to_string(),
                version: "1.0.0".to_string(),
                api_version: "1.0.0".to_string(),
                runtime: PluginRuntime::DeclarativeRules {
                    rules: vec!["rules/main.json".to_string()],
                },
                hooks: vec![PluginHook {
                    name: GatewayPluginHookName::RequestAfterBodyRead
                        .as_str()
                        .to_string(),
                    priority: 10,
                    failure_policy: Some("fail-open".to_string()),
                }],
                permissions: vec![
                    "request.body.read".to_string(),
                    "request.body.write".to_string(),
                ],
                host_compatibility: PluginHostCompatibility {
                    app: ">=0.56.0 <1.0.0".to_string(),
                    plugin_api: "^1.0.0".to_string(),
                    platforms: vec![],
                },
                entry: None,
                config_schema: None,
                config_version: None,
                description: None,
                author: None,
                homepage: None,
                repository: None,
                license: None,
                checksum: None,
                signature: None,
                category: None,
            },
            install_source: PluginInstallSource::Official,
            installed_dir: None,
            config: serde_json::json!({}),
            granted_permissions: vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn fail_closed(mut plugin: PluginDetail) -> PluginDetail {
        plugin.manifest.hooks[0].failure_policy = Some("fail-closed".to_string());
        plugin
    }

    fn before_send_header_plugin() -> PluginDetail {
        let mut plugin = request_rewrite_plugin();
        plugin.summary.plugin_id = "test.before-send".to_string();
        plugin.summary.name = "Before Send".to_string();
        plugin.manifest.id = "test.before-send".to_string();
        plugin.manifest.name = "Before Send".to_string();
        plugin.manifest.hooks[0].name = GatewayPluginHookName::RequestBeforeSend
            .as_str()
            .to_string();
        plugin.manifest.permissions = vec![
            "request.meta.read".to_string(),
            "request.header.write".to_string(),
        ];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        plugin
    }

    fn response_after_plugin() -> PluginDetail {
        let mut plugin = request_rewrite_plugin();
        plugin.summary.plugin_id = "test.response-after".to_string();
        plugin.summary.name = "Response After".to_string();
        plugin.manifest.id = "test.response-after".to_string();
        plugin.manifest.name = "Response After".to_string();
        plugin.manifest.hooks[0].name = GatewayPluginHookName::ResponseAfter.as_str().to_string();
        plugin.manifest.permissions = vec![
            "response.body.read".to_string(),
            "response.body.write".to_string(),
        ];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        plugin
    }

    fn stream_chunk_plugin() -> PluginDetail {
        let mut plugin = request_rewrite_plugin();
        plugin.summary.plugin_id = "test.stream-chunk".to_string();
        plugin.summary.name = "Stream Chunk".to_string();
        plugin.manifest.id = "test.stream-chunk".to_string();
        plugin.manifest.name = "Stream Chunk".to_string();
        plugin.manifest.hooks[0].name = GatewayPluginHookName::ResponseChunk.as_str().to_string();
        plugin.manifest.permissions =
            vec!["stream.inspect".to_string(), "stream.modify".to_string()];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        plugin
    }

    fn log_redaction_plugin() -> PluginDetail {
        let mut plugin = request_rewrite_plugin();
        plugin.summary.plugin_id = "test.log-redaction".to_string();
        plugin.summary.name = "Log Redaction".to_string();
        plugin.manifest.id = "test.log-redaction".to_string();
        plugin.manifest.name = "Log Redaction".to_string();
        plugin.manifest.hooks[0].name =
            GatewayPluginHookName::LogBeforePersist.as_str().to_string();
        plugin.manifest.permissions = vec!["log.redact".to_string()];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        plugin
    }

    fn official_privacy_filter_for_tests() -> PluginDetail {
        let fixture = official::official_plugin("official.privacy-filter")
            .expect("official privacy filter fixture");
        let permissions = fixture.manifest.permissions.clone();
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: fixture.manifest.id.clone(),
                name: fixture.manifest.name.clone(),
                current_version: Some(fixture.manifest.version.clone()),
                status: PluginStatus::Enabled,
                runtime: "native:privacyFilter".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: fixture.manifest,
            install_source: PluginInstallSource::Official,
            installed_dir: Some(fixture.root_dir.to_string_lossy().to_string()),
            config: fixture.default_config,
            granted_permissions: permissions,
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn gateway_error_plugin() -> PluginDetail {
        let mut plugin = request_rewrite_plugin();
        plugin.summary.plugin_id = "test.gateway-error".to_string();
        plugin.summary.name = "Gateway Error".to_string();
        plugin.manifest.id = "test.gateway-error".to_string();
        plugin.manifest.name = "Gateway Error".to_string();
        plugin.manifest.hooks[0].name = GatewayPluginHookName::Error.as_str().to_string();
        plugin.manifest.permissions = vec![
            "response.body.read".to_string(),
            "response.body.write".to_string(),
            "response.header.write".to_string(),
        ];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        plugin
    }

    fn persist_test_plugin(db: &db::Db, plugin: &PluginDetail) {
        repository::insert_plugin(
            db,
            repository::InsertPluginInput {
                manifest: plugin.manifest.clone(),
                install_source: PluginInstallSource::Official,
                status: PluginStatus::Enabled,
                installed_dir: None,
            },
        )
        .expect("insert test plugin");
        repository::save_plugin_permissions(
            db,
            &plugin.summary.plugin_id,
            &plugin.granted_permissions,
            &[],
        )
        .expect("grant test plugin permissions");
    }

    fn persist_plugin_detail(db: &db::Db, plugin: &PluginDetail) {
        repository::insert_plugin(
            db,
            repository::InsertPluginInput {
                manifest: plugin.manifest.clone(),
                install_source: plugin.install_source,
                status: plugin.summary.status,
                installed_dir: plugin.installed_dir.clone(),
            },
        )
        .expect("insert plugin detail");
        repository::save_plugin_permissions(
            db,
            &plugin.summary.plugin_id,
            &plugin.granted_permissions,
            &plugin.pending_permissions,
        )
        .expect("save plugin detail permissions");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_timeout_stub_returns_bad_gateway_and_emits_request_log() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.upstream_first_byte_timeout_seconds = 1;
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, upstream_task) = spawn_hanging_upstream().await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-timeout","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamTimeout.as_str())
        );

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log enqueue")
            .expect("request log item");
        assert_eq!(log.cli_key, "codex");
        assert_eq!(log.path, "/v1/chat/completions");
        assert_eq!(log.status, Some(524));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamTimeout.as_str())
        );

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamTimeout.as_str())
        );
        assert_eq!(
            attempts[0].get("decision").and_then(Value::as_str),
            Some("switch")
        );

        let provider_chain: Value =
            serde_json::from_str(log.provider_chain_json.as_deref().expect("provider chain"))
                .expect("provider chain json");
        assert_eq!(
            provider_chain
                .as_array()
                .and_then(|items| items.first())
                .and_then(|item| item.get("provider_id"))
                .and_then(Value::as_i64),
            Some(provider_id)
        );

        let error_details: Value =
            serde_json::from_str(log.error_details_json.as_deref().expect("error details"))
                .expect("error details json");
        assert_eq!(
            error_details
                .get("gateway_error_code")
                .and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamTimeout.as_str())
        );

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_request_after_body_read_rewrites_upstream_body() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-plugin-request-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, captured_rx, upstream_task) = spawn_capturing_json_upstream(
            r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#,
        )
        .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor = InMemoryGatewayPluginExecutor::new().with_request_handler(
            "test.request-rewrite",
            |_ctx| GatewayHookResult {
                request_body: Some(
                    r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"rewritten"}]}"#
                        .to_string(),
                ),
                ..GatewayHookResult::continue_unchanged()
            },
        );
        let plugin = request_rewrite_plugin();
        persist_test_plugin(&db, &plugin);
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin.clone()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db.clone(),
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"original"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured body");
        assert!(captured.contains(r#""content":"rewritten""#));
        assert!(!captured.contains(r#""content":"original""#));

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));
        let plugin_detail = repository::get_plugin(&db, &plugin.summary.plugin_id)
            .expect("read persisted plugin detail");
        assert!(plugin_detail.audit_logs.iter().any(|audit| {
            audit.trace_id.as_deref() == Some(request_log.trace_id.as_str())
                && audit.event_type == "plugin.hook.completed"
        }));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn official_privacy_filter_redacts_gzipped_codex_responses_before_upstream() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.enable_codex_session_id_completion = false;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("privacy-filter-gzip-test.sqlite"))
            .expect("init test db");
        let fixture = official::official_plugin("official.privacy-filter")
            .expect("official privacy filter fixture");
        let permissions = fixture.manifest.permissions.clone();
        let plugin = PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: fixture.manifest.id.clone(),
                name: fixture.manifest.name.clone(),
                current_version: Some(fixture.manifest.version.clone()),
                status: PluginStatus::Enabled,
                runtime: "native:privacyFilter".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: fixture.manifest,
            install_source: PluginInstallSource::Official,
            installed_dir: Some(fixture.root_dir.to_string_lossy().to_string()),
            config: fixture.default_config,
            granted_permissions: permissions.clone(),
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        };
        repository::insert_plugin(
            &db,
            repository::InsertPluginInput {
                manifest: plugin.manifest.clone(),
                install_source: PluginInstallSource::Official,
                status: PluginStatus::Enabled,
                installed_dir: plugin.installed_dir.clone(),
            },
        )
        .expect("insert official privacy filter");
        repository::save_plugin_permissions(&db, &plugin.summary.plugin_id, &permissions, &[])
            .expect("grant official privacy filter permissions");

        let (upstream_base_url, captured_rx, upstream_task) =
            spawn_capturing_raw_upstream(r#"{"id":"stub-ok","object":"response","output":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let plain_body = serde_json::json!({
            "model": "gpt-plugin",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "你知道 13344441520 是哪里的手机号嘛"
                }]
            }]
        })
        .to_string();
        let compressed_body = gzip_bytes(plain_body.as_bytes());
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_ENCODING, "gzip")
            .body(Body::from(compressed_body))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured request");

        assert!(captured.has_header_line("content-encoding: gzip"));
        let decoded_body = gunzip_bytes(&captured.body);
        let decoded_body_text = String::from_utf8_lossy(&decoded_body);
        assert!(decoded_body_text.contains("[电话]"));
        assert!(!decoded_body_text.contains("13344441520"));

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));
        assert!(!request_log.attempts_json.contains("13344441520"));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn official_privacy_filter_redacts_full_codex_responses_payload_before_upstream_and_logs()
    {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.enable_codex_session_id_completion = false;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("privacy-filter-full-codex-payload-test.sqlite"),
        )
        .expect("init test db");
        let fixture = official::official_plugin("official.privacy-filter")
            .expect("official privacy filter fixture");
        let permissions = fixture.manifest.permissions.clone();
        let plugin = PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: fixture.manifest.id.clone(),
                name: fixture.manifest.name.clone(),
                current_version: Some(fixture.manifest.version.clone()),
                status: PluginStatus::Enabled,
                runtime: "native:privacyFilter".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: fixture.manifest,
            install_source: PluginInstallSource::Official,
            installed_dir: Some(fixture.root_dir.to_string_lossy().to_string()),
            config: fixture.default_config,
            granted_permissions: permissions.clone(),
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        };
        repository::insert_plugin(
            &db,
            repository::InsertPluginInput {
                manifest: plugin.manifest.clone(),
                install_source: PluginInstallSource::Official,
                status: PluginStatus::Enabled,
                installed_dir: plugin.installed_dir.clone(),
            },
        )
        .expect("insert official privacy filter");
        repository::save_plugin_permissions(&db, &plugin.summary.plugin_id, &permissions, &[])
            .expect("grant official privacy filter permissions");

        let (upstream_base_url, captured_rx, upstream_task) =
            spawn_capturing_raw_upstream(r#"{"id":"stub-ok","object":"response","output":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let plain_body = serde_json::json!({
            "model": "gpt-plugin",
            "instructions": "developer prompt with sys@example.com",
            "input": [
                {
                    "type": "message",
                    "role": "developer",
                    "content": [{
                        "type": "input_text",
                        "text": "developer-visible phone 13344441521"
                    }]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "你知道 13344441520 是哪里的手机号嘛"
                    }]
                },
                {
                    "type": "function_call",
                    "call_id": "call_123",
                    "name": "lookup_phone",
                    "arguments": "{\"phone\":\"13344441522\"}"
                }
            ],
            "tools": [{
                "type": "function",
                "name": "lookup_phone",
                "description": "Lookup 13344441523",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "phone": {
                            "type": "string",
                            "description": "Phone like 13344441524"
                        }
                    }
                }
            }],
            "tool_choice": "auto",
            "reasoning": { "effort": "xhigh" },
            "client_metadata": {
                "x-codex-window-id": "13344441525"
            }
        })
        .to_string();
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(plain_body))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured request");

        let body_text = String::from_utf8_lossy(&captured.body);
        assert!(body_text.contains("[电话]"));
        assert!(body_text.contains("[邮箱]"));
        assert!(!body_text.contains("13344441520"));
        assert!(!body_text.contains("13344441521"));
        assert!(
            body_text.contains("13344441522"),
            "function_call.arguments should remain unchanged: {body_text}"
        );
        assert!(
            body_text.contains("13344441523"),
            "tool description should remain unchanged: {body_text}"
        );
        assert!(
            body_text.contains("13344441524"),
            "tool parameters should remain unchanged: {body_text}"
        );
        assert!(
            body_text.contains("13344441525"),
            "client_metadata should remain unchanged: {body_text}"
        );

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));
        assert!(!request_log.attempts_json.contains("13344441520"));
        assert!(!request_log.attempts_json.contains("13344441521"));
        assert!(!request_log
            .provider_chain_json
            .as_deref()
            .unwrap_or_default()
            .contains("13344441520"));
        assert!(!request_log
            .error_details_json
            .as_deref()
            .unwrap_or_default()
            .contains("13344441520"));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn official_privacy_filter_before_send_redacts_final_upstream_body() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.enable_codex_session_id_completion = false;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("privacy-filter-before-send.sqlite"))
            .expect("init test db");
        let mut plugin = official_privacy_filter_for_tests();
        plugin
            .manifest
            .hooks
            .retain(|hook| hook.name != "gateway.request.afterBodyRead");
        persist_plugin_detail(&db, &plugin);

        let (upstream_base_url, captured_rx, upstream_task) =
            spawn_capturing_raw_upstream(r#"{"id":"stub-ok","object":"response","output":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({
                    "model": "gpt-plugin",
                    "input": [{
                        "type": "message",
                        "role": "user",
                        "content": [{
                            "type": "input_text",
                            "text": "你知道 13344441520 是哪里的手机号嘛"
                        }]
                    }]
                })
                .to_string(),
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured request");

        let body_text = String::from_utf8_lossy(&captured.body);
        assert!(body_text.contains("[电话]"));
        assert!(!body_text.contains("13344441520"));

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));
        assert!(!request_log.attempts_json.contains("13344441520"));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn request_before_send_mutation_survives_codex_internal_retry() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.enable_codex_session_id_completion = false;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("privacy-filter-retry.sqlite"))
            .expect("init test db");
        let mut plugin = before_send_header_plugin();
        plugin.manifest.permissions = vec![
            "request.body.read".to_string(),
            "request.body.write".to_string(),
        ];
        plugin.granted_permissions = plugin.manifest.permissions.clone();
        persist_plugin_detail(&db, &plugin);

        let (upstream_base_url, mut captured_rx, upstream_task) =
            spawn_codex_previous_response_retry_upstream().await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let executor =
            InMemoryGatewayPluginExecutor::new().with_request_handler("test.before-send", {
                let call_count = Arc::clone(&call_count);
                move |ctx| {
                    let call = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let mut result = GatewayHookResult::continue_unchanged();
                    if call == 0 {
                        let body = ctx.request.body.expect("request body visible");
                        result.request_body = Some(body.replace("13344441520", "[电话]"));
                    }
                    result
                }
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({
                    "model": "gpt-plugin",
                    "previous_response_id": "resp_old",
                    "input": [{
                        "type": "message",
                        "role": "user",
                        "content": [{
                            "type": "input_text",
                            "text": "你知道 13344441520 是哪里的手机号嘛"
                        }]
                    }]
                })
                .to_string(),
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let first = tokio::time::timeout(Duration::from_secs(2), captured_rx.recv())
            .await
            .expect("first captured request")
            .expect("first request");
        let second = tokio::time::timeout(Duration::from_secs(2), captured_rx.recv())
            .await
            .expect("second captured request")
            .expect("second request");
        assert!(!String::from_utf8_lossy(&first.body).contains("13344441520"));
        assert!(String::from_utf8_lossy(&first.body).contains("[电话]"));

        let second_body = String::from_utf8_lossy(&second.body);
        assert!(
            second_body.contains("[电话]"),
            "retry request should keep the beforeSend redaction: {second_body}"
        );
        assert!(
            !second_body.contains("13344441520"),
            "retry request leaked the original phone number: {second_body}"
        );
        assert!(!second_body.contains("previous_response_id"));

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));
        assert!(!request_log.attempts_json.contains("13344441520"));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_preserves_gzipped_codex_request_when_plugins_do_not_mutate_body() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.enable_codex_session_id_completion = false;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-gzip-passthrough-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, captured_rx, upstream_task) =
            spawn_capturing_raw_upstream(r#"{"id":"stub-ok","object":"response","output":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let plain_body = serde_json::json!({
            "model": "gpt-plugin",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "你知道 13344441520 是哪里的手机号嘛"
                }]
            }]
        })
        .to_string();
        let compressed_body = gzip_bytes(plain_body.as_bytes());
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/codex/_aio/provider/{provider_id}/v1/responses"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_ENCODING, "gzip")
            .body(Body::from(compressed_body.clone()))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured request");

        assert!(captured.has_header_line("content-encoding: gzip"));
        assert_eq!(captured.body, compressed_body);
        assert!(!captured.text().contains("13344441520"));
        assert!(!captured.text().contains("[电话]"));

        let request_log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(request_log.status, Some(200));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_request_after_body_read_fail_closed_error_stops_request() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-plugin-after-body-fail-closed-test.sqlite"),
        )
        .expect("init test db");
        let (upstream_base_url, captured_rx, upstream_task) = spawn_capturing_json_upstream(
            r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#,
        )
        .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor = InMemoryGatewayPluginExecutor::new().with_request_handler(
            "test.request-rewrite",
            |_ctx| {
                let mut result = GatewayHookResult::continue_unchanged();
                result
                    .headers
                    .insert("x-aio-forbidden".to_string(), "1".to_string());
                result
            },
        );
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![fail_closed(request_rewrite_plugin())],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"original"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::InternalError.as_str())
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), captured_rx)
                .await
                .is_err(),
            "fail-closed afterBodyRead should not send the request upstream"
        );
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_request_before_send_adds_upstream_header() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-plugin-before-send-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, captured_rx, upstream_task) = spawn_capturing_raw_upstream(
            r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#,
        )
        .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor =
            InMemoryGatewayPluginExecutor::new().with_request_handler("test.before-send", |_ctx| {
                let mut result = GatewayHookResult::continue_unchanged();
                result
                    .headers
                    .insert("x-plugin-before-send".to_string(), "applied".to_string());
                result
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![before_send_header_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let captured = tokio::time::timeout(Duration::from_secs(2), captured_rx)
            .await
            .expect("captured upstream request")
            .expect("captured raw request");
        assert!(
            captured
                .text()
                .to_ascii_lowercase()
                .contains("x-plugin-before-send: applied"),
            "captured upstream request did not include plugin header:\n{}",
            captured.text()
        );

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_request_before_send_fail_closed_error_stops_request() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-plugin-before-send-fail-closed-test.sqlite"),
        )
        .expect("init test db");
        let (upstream_base_url, captured_rx, upstream_task) = spawn_capturing_raw_upstream(
            r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#,
        )
        .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor =
            InMemoryGatewayPluginExecutor::new().with_request_handler("test.before-send", |_ctx| {
                let mut result = GatewayHookResult::continue_unchanged();
                result
                    .headers
                    .insert("x-aio-forbidden".to_string(), "1".to_string());
                result
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![fail_closed(before_send_header_plugin())],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::InternalError.as_str())
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), captured_rx)
                .await
                .is_err(),
            "fail-closed beforeSend should not send the request upstream"
        );
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_response_after_rewrites_non_stream_body() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-plugin-response-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, upstream_task) =
            spawn_json_upstream(r#"{"id":"original","object":"chat.completion","choices":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor = InMemoryGatewayPluginExecutor::new().with_response_handler(
            "test.response-after",
            |_ctx| GatewayHookResult {
                response_body: Some(
                    r#"{"id":"rewritten","object":"chat.completion","choices":[]}"#.to_string(),
                ),
                ..GatewayHookResult::continue_unchanged()
            },
        );
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![response_after_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload.get("id").and_then(Value::as_str), Some("rewritten"));

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_response_after_fail_closed_error_replaces_upstream_success() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-plugin-response-fail-closed-test.sqlite"),
        )
        .expect("init test db");
        let (upstream_base_url, upstream_task) =
            spawn_json_upstream(r#"{"id":"original","object":"chat.completion","choices":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor = InMemoryGatewayPluginExecutor::new().with_response_handler(
            "test.response-after",
            |_ctx| {
                let mut result = GatewayHookResult::continue_unchanged();
                result
                    .headers
                    .insert("x-aio-forbidden".to_string(), "1".to_string());
                result
            },
        );
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![fail_closed(response_after_plugin())],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::InternalError.as_str())
        );
        assert_ne!(payload.get("id").and_then(Value::as_str), Some("original"));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_response_chunk_rewrites_stream_body() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-plugin-stream-test.sqlite"))
            .expect("init test db");
        let (upstream_base_url, upstream_task) =
            spawn_sse_upstream("data: secret-stream\n\n").await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor =
            InMemoryGatewayPluginExecutor::new().with_stream_handler("test.stream-chunk", |ctx| {
                let chunk = ctx.stream.chunk.expect("visible stream chunk");
                assert!(chunk.contains("secret-stream"));
                GatewayHookResult {
                    stream_chunk: Some(chunk.replace("secret-stream", "redacted-stream")),
                    ..GatewayHookResult::continue_unchanged()
                }
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![stream_chunk_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","stream":true,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")));
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let body = String::from_utf8_lossy(&body);
        assert!(
            body.contains("redacted-stream"),
            "stream body was not rewritten: {body}"
        );
        assert!(
            !body.contains("secret-stream"),
            "stream body leaked secret: {body}"
        );

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_response_chunk_block_emits_stream_error_event() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-plugin-stream-block-test.sqlite"),
        )
        .expect("init test db");
        let (upstream_base_url, upstream_task) =
            spawn_sse_upstream("data: dangerous-command\n\n").await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor =
            InMemoryGatewayPluginExecutor::new().with_stream_handler("test.stream-chunk", |ctx| {
                assert!(ctx
                    .stream
                    .chunk
                    .as_deref()
                    .is_some_and(|chunk| chunk.contains("dangerous-command")));
                let mut result = GatewayHookResult::continue_unchanged();
                result.action = crate::gateway::plugins::context::GatewayHookAction::Block;
                result.reason = Some("dangerous command detected".to_string());
                result
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![stream_chunk_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","stream":true,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let body = String::from_utf8_lossy(&body);
        assert!(
            body.contains("event: error"),
            "stream block did not emit error event: {body}"
        );
        assert!(
            body.contains("plugin_blocked"),
            "stream block reason missing: {body}"
        );
        assert!(
            !body.contains("dangerous-command"),
            "blocked stream leaked chunk: {body}"
        );

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(502));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::Fake200.as_str())
        );
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn plugin_log_redaction_rewrites_request_log_before_enqueue() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-plugin-log-redaction-test.sqlite"),
        )
        .expect("init test db");
        let (upstream_base_url, upstream_task) =
            spawn_json_upstream(r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#)
                .await;
        let provider_id = insert_codex_provider(&db, upstream_base_url);

        let executor =
            InMemoryGatewayPluginExecutor::new().with_log_handler("test.log-redaction", |ctx| {
                let message = ctx.log.message.expect("visible log message");
                assert!(message.contains("secret-query"));
                GatewayHookResult {
                    log_message: Some(message.replace("secret-query", "[REDACTED]")),
                    ..GatewayHookResult::continue_unchanged()
                }
            });
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![log_redaction_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions?token=secret-query"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.query.as_deref(), Some("token=[REDACTED]"));
        assert_ne!(log.query.as_deref(), Some("token=secret-query"));
        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_plugin_error_hook_rewrites_gateway_error_response() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-plugin-error-test.sqlite"))
            .expect("init test db");

        let executor = InMemoryGatewayPluginExecutor::new().with_response_handler(
            "test.gateway-error",
            |ctx| {
                assert_eq!(ctx.hook_name, "gateway.error");
                assert_eq!(ctx.response.status, Some(503));
                assert!(ctx
                    .response
                    .body
                    .as_deref()
                    .is_some_and(|body| body.contains("GW_NO_ENABLED_PROVIDER")));
                let mut result = GatewayHookResult {
                    response_body: Some(
                        r#"{"error_code":"GW_NO_ENABLED_PROVIDER","message":"plugin-friendly error","attempts":[]}"#
                            .to_string(),
                    ),
                    ..GatewayHookResult::continue_unchanged()
                };
                result
                    .headers
                    .insert("x-plugin-error".to_string(), "rewritten".to_string());
                result
            },
        );
        let plugin_pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![gateway_error_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_plugin_pipeline(
            app_handle,
            db,
            log_tx,
            plugin_pipeline,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-plugin","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response
                .headers()
                .get("x-plugin-error")
                .and_then(|value| value.to_str().ok()),
            Some("rewritten")
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("message").and_then(Value::as_str),
            Some("plugin-friendly error")
        );

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(503));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::NoEnabledProvider.as_str())
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_fails_over_from_timeout_to_second_provider_success() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.upstream_first_byte_timeout_seconds = 1;
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 2;
        app_settings.provider_cooldown_seconds = 0;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-failover-test.sqlite"))
            .expect("init test db");
        let (timeout_base_url, timeout_task) = spawn_hanging_upstream().await;
        let success_body = r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        let timeout_provider_id =
            insert_codex_provider_with_priority(&db, "Timeout Stub", timeout_base_url, 0);
        let success_provider_id =
            insert_codex_provider_with_priority(&db, "Success Stub", success_base_url, 1);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-failover","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload.get("id").and_then(Value::as_str), Some("stub-ok"));

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log enqueue")
            .expect("request log item");
        assert_eq!(log.cli_key, "codex");
        assert_eq!(log.path, "/v1/chat/completions");
        assert_eq!(log.status, Some(200));
        assert_eq!(log.error_code, None);
        assert_eq!(log.requested_model.as_deref(), Some("gpt-route-failover"));

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(timeout_provider_id)
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamTimeout.as_str())
        );
        assert_eq!(
            attempts[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );
        assert_eq!(
            attempts[1].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        let provider_chain: Value =
            serde_json::from_str(log.provider_chain_json.as_deref().expect("provider chain"))
                .expect("provider chain json");
        let chain = provider_chain.as_array().expect("provider chain array");
        assert_eq!(chain.len(), 2);
        assert_eq!(
            chain[0].get("provider_id").and_then(Value::as_i64),
            Some(timeout_provider_id)
        );
        assert_eq!(
            chain[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );

        timeout_task.abort();
        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_429_quota_fails_over_without_same_provider_retry() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 5;
        app_settings.failover_max_providers_to_try = 2;
        app_settings.provider_cooldown_seconds = 30;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-429-quota-test.sqlite"))
            .expect("init test db");
        let quota_body = r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota"}}"#;
        let success_body = r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#;
        let (quota_base_url, quota_task) =
            spawn_status_json_upstream("429 Too Many Requests", quota_body).await;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        let quota_provider_id =
            insert_codex_provider_with_priority(&db, "429 Quota Stub", quota_base_url, 0);
        let success_provider_id =
            insert_codex_provider_with_priority(&db, "Success Stub", success_base_url, 1);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db,
            log_tx,
            circuit.clone(),
            session,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-429-quota","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.error_code, None);

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(quota_provider_id)
        );
        assert_eq!(
            attempts[0].get("retry_index").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            attempts[0].get("decision").and_then(Value::as_str),
            Some("switch")
        );
        assert!(attempts[0]
            .get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("rule=quota_exhausted")));
        assert_eq!(
            attempts[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );
        assert_eq!(
            attempts[1].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        let circuit_snapshot = circuit.snapshot(quota_provider_id, 0);
        assert!(circuit_snapshot.cooldown_until.is_some());

        quota_task.abort();
        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_skips_exhausted_oauth_snapshot_without_opening_circuit() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 2;
        app_settings.provider_cooldown_seconds = 30;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-oauth-quota-test.sqlite"))
            .expect("init test db");
        let now = crate::gateway::util::now_unix_seconds() as i64;
        let oauth_provider_id =
            insert_codex_oauth_provider_with_priority(&db, "OAuth Quota Stub", 0);
        crate::domain::provider_oauth_limits::save_exhausted_snapshot(
            &db,
            oauth_provider_id,
            Some(now + 3_600),
        )
        .expect("save oauth exhausted snapshot");

        let success_body = r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        let success_provider_id =
            insert_codex_provider_with_priority(&db, "Success Stub", success_base_url, 1);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db,
            log_tx,
            circuit.clone(),
            session,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-oauth-quota","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.error_code, None);

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(oauth_provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("skipped")
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::ProviderRateLimited.as_str())
        );
        assert_eq!(
            attempts[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );
        assert_eq!(
            attempts[1].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        let oauth_circuit_snapshot = circuit.snapshot(oauth_provider_id, 0);
        assert_eq!(
            oauth_circuit_snapshot.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(oauth_circuit_snapshot.failure_count, 0);
        assert!(oauth_circuit_snapshot.cooldown_until.is_none());

        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_large_known_length_5xx_uses_bounded_error_preview() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.provider_cooldown_seconds = 0;
        settings::write(&app_handle, &app_settings).expect("write settings");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-large-5xx-test.sqlite"))
            .expect("init test db");
        let diagnostic = "route-large-5xx-diagnostic-prefix";
        let tail_marker = "route-large-5xx-tail-should-not-appear";
        let mut sent_body = diagnostic.as_bytes().to_vec();
        sent_body.resize(96 * 1024, b'x');
        sent_body.extend_from_slice(tail_marker.as_bytes());
        let declared_content_length = sent_body.len() + 10 * 1024 * 1024;
        let (upstream_base_url, upstream_task) = spawn_large_known_length_error_upstream(
            "500 Internal Server Error",
            declared_content_length,
            sent_body,
        )
        .await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "Large Error Stub", upstream_base_url, 0);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/codex/_aio/provider/{provider_id}/v1/chat/completions"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-large-5xx","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = tokio::time::timeout(Duration::from_secs(2), router.oneshot(request))
            .await
            .expect("route should not wait for the full declared error body")
            .expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::Upstream5xx.as_str())
        );

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log enqueue")
            .expect("request log item");
        assert_eq!(log.cli_key, "codex");
        assert_eq!(log.path, "/v1/chat/completions");
        assert_eq!(log.status, Some(502));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::Upstream5xx.as_str())
        );

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::Upstream5xx.as_str())
        );
        let reason = attempts[0]
            .get("reason")
            .and_then(Value::as_str)
            .expect("attempt reason");
        assert!(reason.contains(diagnostic));
        assert!(!reason.contains(tail_marker));

        let error_details: Value =
            serde_json::from_str(log.error_details_json.as_deref().expect("error details"))
                .expect("error details json");
        assert_eq!(
            error_details
                .get("upstream_body_preview")
                .and_then(Value::as_str)
                .map(|value| value.contains(diagnostic)),
            Some(true)
        );
        assert_eq!(
            error_details
                .get("upstream_body_preview")
                .and_then(Value::as_str)
                .map(|value| value.contains(tail_marker)),
            Some(false)
        );

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_large_known_length_400_rectifier_path_is_bounded() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.enable_thinking_signature_rectifier = true;
        app_settings.enable_thinking_budget_rectifier = true;
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.provider_cooldown_seconds = 0;
        settings::write(&app_handle, &app_settings).expect("write settings");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-large-400-rectifier-test.sqlite"),
        )
        .expect("init test db");
        let diagnostic = "route-large-400-rectifier-prefix";
        let tail_marker = "route-large-400-rectifier-tail-should-not-appear";
        let mut sent_body = diagnostic.as_bytes().to_vec();
        sent_body.resize(96 * 1024, b'y');
        sent_body.extend_from_slice(tail_marker.as_bytes());
        let declared_content_length = sent_body.len() + 10 * 1024 * 1024;
        let (upstream_base_url, upstream_task) = spawn_large_known_length_error_upstream(
            "400 Bad Request",
            declared_content_length,
            sent_body,
        )
        .await;
        let provider_id =
            insert_provider_with_priority(&db, "claude", "Large 400 Stub", upstream_base_url, 0);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/claude/_aio/provider/{provider_id}/v1/messages"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"claude-3-5-sonnet","max_tokens":128,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = tokio::time::timeout(Duration::from_secs(2), router.oneshot(request))
            .await
            .expect("rectifier path should not wait for the full declared error body")
            .expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let body_text = String::from_utf8_lossy(&body);
        assert!(body_text.contains(diagnostic));
        assert!(!body_text.contains(tail_marker));
        assert!(body.len() < declared_content_length);

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.cli_key, "claude");
        assert_eq!(log.path, "/v1/messages");
        assert_eq!(log.status, Some(400));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::Upstream4xx.as_str())
        );

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("error_category").and_then(Value::as_str),
            Some("NON_RETRYABLE_CLIENT_ERROR")
        );

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_large_known_length_cx2cc_success_transform_is_bounded() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 1;
        app_settings.provider_cooldown_seconds = 0;
        settings::write(&app_handle, &app_settings).expect("write settings");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-large-cx2cc-success-test.sqlite"),
        )
        .expect("init test db");
        let diagnostic = "route-large-cx2cc-success-prefix";
        let mut sent_body = diagnostic.as_bytes().to_vec();
        sent_body.resize(96 * 1024, b'z');
        let declared_content_length = sent_body.len() + 32 * 1024 * 1024;
        let (upstream_base_url, upstream_task) =
            spawn_large_known_length_error_upstream("200 OK", declared_content_length, sent_body)
                .await;
        let source_provider_id =
            insert_provider_with_priority(&db, "codex", "CX2CC Source Stub", upstream_base_url, 0);
        let provider_id = insert_cx2cc_bridge_provider(&db, source_provider_id, 0);

        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state(app_handle, db, log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/claude/_aio/provider/{provider_id}/v1/messages"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"claude-3-5-sonnet","max_tokens":128,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = tokio::time::timeout(Duration::from_secs(2), router.oneshot(request))
            .await
            .expect("cx2cc transform path should reject the oversized body from headers")
            .expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamBodyReadError.as_str())
        );

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.cli_key, "claude");
        assert_eq!(log.path, "/v1/messages");
        assert_eq!(log.status, Some(502));
        assert_eq!(
            log.error_code.as_deref(),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamBodyReadError.as_str())
        );

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some(crate::gateway::proxy::GatewayErrorCode::UpstreamBodyReadError.as_str())
        );
        let reason = attempts[0]
            .get("reason")
            .and_then(Value::as_str)
            .expect("attempt reason");
        assert!(reason.contains("non-stream transform buffer limit"));

        upstream_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_success_log_persists_after_buffered_writer_drain() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-writer-test.sqlite"))
            .expect("init test db");
        let success_body = r#"{"id":"persisted-ok","object":"chat.completion","choices":[]}"#;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "Persisted Stub", success_base_url, 0);

        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state(app_handle, db.clone(), log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-persisted","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            payload.get("id").and_then(Value::as_str),
            Some("persisted-ok")
        );

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        assert_eq!(detail.status, Some(200));
        assert_eq!(detail.error_code, None);
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-persisted")
        );
        assert_eq!(detail.final_provider_id, provider_id);

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_internal_forwarded_codex_response_is_not_logged() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-internal-codex-not-logged-test.sqlite"),
        )
        .expect("init test db");
        let success_body = r#"{"id":"internal-ok","object":"response","model":"gpt-internal"}"#;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        insert_codex_provider_with_priority(&db, "Internal Forward Stub", success_base_url, 0);

        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state(app_handle, db.clone(), log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/responses")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-aio-gateway-forwarded", "aio-coding-hub")
            .body(Body::from(r#"{"model":"gpt-internal","input":"hello"}"#))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        assert!(request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .is_none());

        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_codex_models_response_is_not_logged() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-codex-models-test.sqlite"))
            .expect("init test db");
        let success_body = r#"{"object":"list","data":[{"id":"gpt-5.5","object":"model"}]}"#;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        insert_codex_provider_with_priority(&db, "Models Stub", success_base_url, 0);

        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state(app_handle, db.clone(), log_tx));
        let request = Request::builder()
            .method(Method::GET)
            .uri("/v1/models")
            .body(Body::empty())
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        assert!(request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .is_none());

        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_sse_stream_persists_success_after_body_consumed() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-sse-test.sqlite"))
            .expect("init test db");
        let sse_body = concat!(
            "data: {\"id\":\"chatcmpl-sse\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (sse_base_url, sse_task) = spawn_sse_upstream(sse_body).await;
        let provider_id = insert_codex_provider_with_priority(&db, "SSE Stub", sse_base_url, 0);

        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state(app_handle, db.clone(), log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-sse","stream":true,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")));
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let body_text = String::from_utf8(body.to_vec()).expect("utf8 body");
        assert!(body_text.contains("data: [DONE]"));

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        assert_eq!(detail.status, Some(200));
        assert_eq!(detail.error_code, None);
        assert_eq!(detail.requested_model.as_deref(), Some("gpt-route-sse"));
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_some());

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        sse_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_sse_stream_client_abort_persists_499_log() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-sse-abort-test.sqlite"))
            .expect("init test db");
        let first_chunk = "data: {\"id\":\"chatcmpl-abort\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"}}]}\n\n";
        let (sse_base_url, sse_task) = spawn_stalling_sse_upstream(first_chunk).await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "SSE Abort Stub", sse_base_url, 0);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db.clone(),
            log_tx,
            circuit.clone(),
            session.clone(),
        ));
        let session_id = "sess-route-sse-abort";
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-session-id", session_id)
            .body(Body::from(
                r#"{"model":"gpt-route-sse-abort","stream":true,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")));
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();

        let mut body = Box::pin(response.into_body());
        let first_frame = tokio::time::timeout(
            Duration::from_secs(2),
            std::future::poll_fn(|cx| body.as_mut().poll_frame(cx)),
        )
        .await
        .expect("first stream frame timeout")
        .expect("first stream frame")
        .expect("first stream frame ok");
        let first_chunk = first_frame.into_data().expect("data frame");
        assert!(String::from_utf8_lossy(&first_chunk).contains("hello"));
        drop(body);

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        let logged_session_id = detail
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .expect("logged session id");
        assert_eq!(detail.status, Some(499));
        assert_eq!(detail.error_code.as_deref(), Some("GW_STREAM_ABORTED"));
        assert!(detail.excluded_from_stats);
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-sse-abort")
        );
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_some());

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("stream_error: code=GW_STREAM_ABORTED")
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some("GW_STREAM_ABORTED")
        );
        assert_eq!(
            attempts[0].get("error_category").and_then(Value::as_str),
            Some("CLIENT_ABORT")
        );

        let special_settings: Value = serde_json::from_str(
            detail
                .special_settings_json
                .as_deref()
                .expect("special settings json"),
        )
        .expect("special settings json parses");
        let special_settings = special_settings.as_array().expect("special settings array");
        assert!(special_settings.iter().any(|entry| {
            entry.get("type").and_then(Value::as_str) == Some("client_abort")
                && entry.get("scope").and_then(Value::as_str) == Some("stream")
        }));

        let error_details: Value = serde_json::from_str(
            detail
                .error_details_json
                .as_deref()
                .expect("error details json"),
        )
        .expect("error details json parses");
        assert_eq!(
            error_details
                .get("gateway_error_code")
                .and_then(Value::as_str),
            Some("GW_STREAM_ABORTED")
        );
        assert_eq!(
            error_details.get("error_category").and_then(Value::as_str),
            Some("CLIENT_ABORT")
        );
        let circuit_snapshot = circuit.snapshot(provider_id, 0);
        assert_eq!(
            circuit_snapshot.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(circuit_snapshot.failure_count, 0);
        assert_eq!(
            session.get_bound_provider("codex", logged_session_id, 0),
            None
        );

        sse_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_codex_responses_abort_drains_completion_as_success() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-responses-relay-abort-test.sqlite"),
        )
        .expect("init test db");
        let first_chunk = concat!(
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n"
        );
        let completion_chunk = concat!(
            "event: response.completed\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-relay-abort\",\"status\":\"completed\",\"model\":\"gpt-route-responses-relay\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n"
        );
        let (sse_base_url, sse_task) = spawn_delayed_chunked_sse_upstream(
            first_chunk,
            completion_chunk,
            Duration::from_millis(50),
        )
        .await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "Responses Relay Stub", sse_base_url, 0);

        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state(app_handle, db.clone(), log_tx));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/responses")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-responses-relay","stream":true,"input":"hello"}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")));
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();

        let mut body = Box::pin(response.into_body());
        let first_frame = tokio::time::timeout(
            Duration::from_secs(2),
            std::future::poll_fn(|cx| body.as_mut().poll_frame(cx)),
        )
        .await
        .expect("first relay frame timeout")
        .expect("first relay frame")
        .expect("first relay frame ok");
        let first_chunk = first_frame.into_data().expect("data frame");
        assert!(String::from_utf8_lossy(&first_chunk).contains("hello"));
        drop(body);

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/responses");
        assert_eq!(detail.status, Some(200));
        assert_eq!(detail.error_code, None);
        assert!(!detail.excluded_from_stats);
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-responses-relay")
        );
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_some());
        assert_eq!(detail.input_tokens, Some(1));
        assert_eq!(detail.output_tokens, Some(2));
        assert_eq!(detail.total_tokens, Some(3));

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        let special_settings: Value = serde_json::from_str(
            detail
                .special_settings_json
                .as_deref()
                .expect("special settings json"),
        )
        .expect("special settings json parses");
        let special_settings = special_settings.as_array().expect("special settings array");
        let abort_entry = special_settings
            .iter()
            .find(|entry| {
                entry.get("type").and_then(Value::as_str) == Some("client_abort")
                    && entry.get("scope").and_then(Value::as_str) == Some("stream")
            })
            .expect("client abort diagnostics");
        assert_eq!(
            abort_entry.get("completion_seen").and_then(Value::as_bool),
            Some(true)
        );
        assert!(abort_entry
            .get("drained_chunks")
            .and_then(Value::as_i64)
            .is_some_and(|count| count >= 1));

        sse_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_sse_fake_200_persists_error_without_session_binding() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("gateway-route-sse-fake-200-test.sqlite"))
            .expect("init test db");
        let fake_200_body = concat!(
            "event: error\n",
            "data: {\"type\":\"error\",\"error\":{\"message\":\"quota exhausted\",\"type\":\"insufficient_quota\"}}\n\n"
        );
        let (sse_base_url, sse_task) = spawn_sse_upstream(fake_200_body).await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "SSE Fake 200 Stub", sse_base_url, 0);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db.clone(),
            log_tx,
            circuit.clone(),
            session.clone(),
        ));
        let session_id = "sess-route-fake-200";
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-session-id", session_id)
            .body(Body::from(
                r#"{"model":"gpt-route-fake-200","stream":true,"messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        assert!(body.is_empty());

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        let logged_session_id = detail
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .expect("logged session id");
        assert_eq!(detail.status, Some(502));
        assert_eq!(detail.error_code.as_deref(), Some("GW_FAKE_200"));
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-fake-200")
        );
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_some());

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("stream_error: code=GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_category").and_then(Value::as_str),
            Some("PROVIDER_ERROR")
        );

        let error_details: Value = serde_json::from_str(
            detail
                .error_details_json
                .as_deref()
                .expect("error details json"),
        )
        .expect("error details json parses");
        assert_eq!(
            error_details
                .get("gateway_error_code")
                .and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            error_details.get("error_code").and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            error_details.get("error_category").and_then(Value::as_str),
            Some("PROVIDER_ERROR")
        );

        let circuit_snapshot = circuit.snapshot(provider_id, 0);
        assert_eq!(
            circuit_snapshot.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(circuit_snapshot.failure_count, 1);
        assert_eq!(
            session.get_bound_provider("codex", logged_session_id, 0),
            None
        );

        sse_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_json_fake_200_returns_bad_gateway_without_session_binding() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-json-fake-200-test.sqlite"),
        )
        .expect("init test db");
        let fake_200_body =
            r#"{"error":{"message":"quota exhausted","type":"insufficient_quota"}}"#;
        let (json_base_url, json_task) = spawn_json_upstream(fake_200_body).await;
        let provider_id =
            insert_codex_provider_with_priority(&db, "JSON Fake 200 Stub", json_base_url, 0);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db.clone(),
            log_tx,
            circuit.clone(),
            session.clone(),
        ));
        let session_id = "sess-route-json-fake-200";
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-session-id", session_id)
            .body(Body::from(
                r#"{"model":"gpt-route-json-fake-200","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        assert!(String::from_utf8_lossy(&body).contains("GW_FAKE_200"));

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        let logged_session_id = detail
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .expect("logged session id");
        assert_eq!(detail.status, Some(502));
        assert_eq!(detail.error_code.as_deref(), Some("GW_FAKE_200"));
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-json-fake-200")
        );
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_none());

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("body_error: code=GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_category").and_then(Value::as_str),
            Some("PROVIDER_ERROR")
        );
        assert_eq!(
            attempts[0].get("decision").and_then(Value::as_str),
            Some("switch")
        );

        let error_details: Value = serde_json::from_str(
            detail
                .error_details_json
                .as_deref()
                .expect("error details json"),
        )
        .expect("error details json parses");
        assert_eq!(
            error_details
                .get("gateway_error_code")
                .and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            error_details.get("error_code").and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            error_details.get("error_category").and_then(Value::as_str),
            Some("PROVIDER_ERROR")
        );

        let circuit_snapshot = circuit.snapshot(provider_id, 0);
        assert_eq!(
            circuit_snapshot.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(circuit_snapshot.failure_count, 1);
        assert!(circuit_snapshot.cooldown_until.is_some());
        assert_eq!(
            session.get_bound_provider("codex", logged_session_id, 0),
            None
        );

        json_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_json_fake_200_quota_fails_over_to_next_provider() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let mut app_settings = settings::AppSettings::default();
        app_settings.failover_max_attempts_per_provider = 1;
        app_settings.failover_max_providers_to_try = 2;
        app_settings.provider_cooldown_seconds = 30;
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-json-fake-200-quota-failover-test.sqlite"),
        )
        .expect("init test db");
        let fake_200_body =
            r#"{"error":{"message":"quota exhausted","type":"insufficient_quota"}}"#;
        let success_body = r#"{"id":"stub-ok","object":"chat.completion","choices":[]}"#;
        let (quota_base_url, quota_task) = spawn_json_upstream(fake_200_body).await;
        let (success_base_url, success_task) = spawn_json_upstream(success_body).await;
        let quota_provider_id =
            insert_codex_provider_with_priority(&db, "Quota Stub", quota_base_url, 0);
        let success_provider_id =
            insert_codex_provider_with_priority(&db, "Success Stub", success_base_url, 1);

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db,
            log_tx,
            circuit.clone(),
            session,
        ));
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"model":"gpt-route-json-fake-200-quota","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload.get("id").and_then(Value::as_str), Some("stub-ok"));

        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.error_code, None);

        let attempts: Value = serde_json::from_str(&log.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(quota_provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("body_error: code=GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("decision").and_then(Value::as_str),
            Some("switch")
        );
        assert_eq!(
            attempts[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );
        assert_eq!(
            attempts[1].get("outcome").and_then(Value::as_str),
            Some("success")
        );

        let provider_chain: Value =
            serde_json::from_str(log.provider_chain_json.as_deref().expect("provider chain"))
                .expect("provider chain json");
        let chain = provider_chain.as_array().expect("provider chain array");
        assert_eq!(
            chain[0].get("provider_id").and_then(Value::as_i64),
            Some(quota_provider_id)
        );
        assert_eq!(
            chain[1].get("provider_id").and_then(Value::as_i64),
            Some(success_provider_id)
        );

        let circuit_snapshot = circuit.snapshot(quota_provider_id, 0);
        assert!(circuit_snapshot.cooldown_until.is_some());

        quota_task.abort();
        success_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_runtime_router_unknown_length_json_fake_200_logs_error_without_session_binding() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("home dir");
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let app_settings = settings::AppSettings::default();
        settings::write(&app_handle, &app_settings).expect("write settings");
        crate::cli_proxy::set_enabled(&app_handle, "codex", true, "http://127.0.0.1:37123")
            .expect("enable codex cli proxy");

        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(
            &db_dir
                .path()
                .join("gateway-route-unknown-length-json-fake-200-test.sqlite"),
        )
        .expect("init test db");
        let fake_200_body =
            r#"{"error":{"message":"quota exhausted","type":"insufficient_quota"}}"#;
        let (json_base_url, json_task) = spawn_unknown_length_json_upstream(fake_200_body).await;
        let provider_id = insert_codex_provider_with_priority(
            &db,
            "Unknown Length JSON Fake 200 Stub",
            json_base_url,
            0,
        );

        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));
        let session = Arc::new(session_manager::SessionManager::new());
        let (log_tx, writer_task) =
            request_logs::start_buffered_writer(app_handle.clone(), db.clone());
        let router = build_router(gateway_state_with_parts(
            app_handle,
            db.clone(),
            log_tx,
            circuit.clone(),
            session.clone(),
        ));
        let session_id = "sess-route-unknown-length-json-fake-200";
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/chat/completions")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-session-id", session_id)
            .body(Body::from(
                r#"{"model":"gpt-route-unknown-length-json-fake-200","messages":[{"role":"user","content":"hello"}]}"#,
            ))
            .expect("request");

        let response = router.oneshot(request).await.expect("route response");
        assert_eq!(response.status(), StatusCode::OK);
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .and_then(|value| value.to_str().ok())
            .expect("trace header")
            .to_string();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        assert!(String::from_utf8_lossy(&body).contains("quota exhausted"));

        tokio::time::timeout(Duration::from_secs(2), writer_task)
            .await
            .expect("writer drain timeout")
            .expect("writer task joins");

        let detail = request_logs::get_by_trace_id(&db, &trace_id)
            .expect("query request log")
            .expect("persisted request log");
        assert_eq!(detail.cli_key, "codex");
        assert_eq!(detail.path, "/v1/chat/completions");
        let logged_session_id = detail
            .session_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .expect("logged session id");
        assert_eq!(detail.status, Some(502));
        assert_eq!(detail.error_code.as_deref(), Some("GW_FAKE_200"));
        assert_eq!(
            detail.requested_model.as_deref(),
            Some("gpt-route-unknown-length-json-fake-200")
        );
        assert_eq!(detail.final_provider_id, provider_id);
        assert!(detail.ttfb_ms.is_some());

        let attempts: Value = serde_json::from_str(&detail.attempts_json).expect("attempts json");
        let attempts = attempts.as_array().expect("attempt array");
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].get("provider_id").and_then(Value::as_i64),
            Some(provider_id)
        );
        assert_eq!(
            attempts[0].get("outcome").and_then(Value::as_str),
            Some("stream_error: code=GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_code").and_then(Value::as_str),
            Some("GW_FAKE_200")
        );
        assert_eq!(
            attempts[0].get("error_category").and_then(Value::as_str),
            Some("PROVIDER_ERROR")
        );

        let circuit_snapshot = circuit.snapshot(provider_id, 0);
        assert_eq!(
            circuit_snapshot.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(circuit_snapshot.failure_count, 1);
        assert_eq!(
            session.get_bound_provider("codex", logged_session_id, 0),
            None
        );

        json_task.abort();
    }
}
