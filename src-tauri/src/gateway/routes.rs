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
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
    use crate::gateway::runtime::GatewayAppState;
    use crate::{circuit_breaker, db, providers, request_logs, session_manager, settings};
    use axum::body::HttpBody;
    use axum::body::{to_bytes, Body};
    use axum::http::{header, Method, Request, StatusCode};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::ffi::OsString;
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
        }
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
