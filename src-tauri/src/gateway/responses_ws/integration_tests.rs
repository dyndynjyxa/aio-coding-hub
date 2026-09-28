//! Exercise the real router and provider retry loop against local HTTP/WS servers.

#![allow(clippy::await_holding_lock)]

use super::protocol;
use super::state::Runtime;
use crate::gateway::codex_session_id::CodexSessionIdCache;
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
use crate::gateway::routes::build_router;
use crate::gateway::runtime::GatewayAppState;
use crate::{circuit_breaker, db, providers, request_logs, session_manager, settings};
use axum::extract::{ws::WebSocketUpgrade, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error, Message};

struct Fixture {
    app: tauri::App<tauri::test::MockRuntime>,
    db: db::Db,
    runtime: Arc<Runtime>,
    active: Arc<crate::gateway::active_requests::ActiveRequestRegistry>,
    circuit: Arc<circuit_breaker::CircuitBreaker>,
    previous_env: Vec<(&'static str, Option<OsString>)>,
    _home: tempfile::TempDir,
    _lock: MutexGuard<'static, ()>,
}

impl Fixture {
    async fn new(enabled: bool) -> Self {
        let lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let previous_env = ["AIO_CODING_HUB_HOME_DIR", "AIO_CODING_HUB_DOTDIR_NAME"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("AIO_CODING_HUB_HOME_DIR", home.path());
        std::env::set_var("AIO_CODING_HUB_DOTDIR_NAME", ".aio-ws-router-test");
        settings::clear_cache();
        let app = tauri::test::mock_app();
        let cfg = settings::AppSettings {
            codex_responses_websocket_enabled: enabled,
            codex_home_mode: settings::CodexHomeMode::UserHomeDefault,
            upstream_first_byte_timeout_seconds: 2,
            upstream_stream_idle_timeout_seconds: 60,
            failover_max_attempts_per_provider: 1,
            failover_max_providers_to_try: 2,
            circuit_breaker_failure_threshold: 1,
            provider_cooldown_seconds: 0,
            ..Default::default()
        };
        settings::write(app.handle(), &cfg).unwrap();
        crate::gateway::http_client::apply_proxy(None).unwrap();
        let proxy =
            crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123")
                .unwrap();
        assert!(proxy.ok, "{}", proxy.message);
        let cached =
            crate::gateway::proxy::cli_proxy_guard::cli_proxy_enabled_cached(app.handle(), "codex");
        if !cached.enabled {
            tokio::time::sleep(Duration::from_millis(cached.cache_ttl_ms as u64 + 10)).await;
        }
        let db = db::init_for_tests(&home.path().join("gateway.sqlite")).unwrap();
        Self {
            app,
            db,
            runtime: Arc::new(Runtime::new(enabled)),
            active: Arc::new(crate::gateway::active_requests::ActiveRequestRegistry::default()),
            circuit: Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: 1,
                    ..Default::default()
                },
                Default::default(),
                None,
            )),
            previous_env,
            _home: home,
            _lock: lock,
        }
    }

    fn provider(&self, name: &str, base_url: &str, supports_ws: bool) -> i64 {
        self.provider_with_headers(name, base_url, supports_ws, None, None)
    }

    fn provider_with_headers(
        &self,
        name: &str,
        base_url: &str,
        supports_ws: bool,
        provider_id: Option<i64>,
        tenant: Option<&str>,
    ) -> i64 {
        let priority = providers::default_route_list(&self.db, "codex")
            .unwrap()
            .len() as i64;
        let row = providers::upsert(
            &self.db,
            providers::ProviderUpsertParams {
                custom_headers: Some(
                    tenant
                        .into_iter()
                        .map(|value| providers::ProviderCustomHeader {
                            name: "x-tenant".into(),
                            value: value.into(),
                        })
                        .collect(),
                ),
                provider_id,
                cli_key: "codex".into(),
                name: name.into(),
                base_urls: vec![base_url.into()],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some(format!("sk-{name}")),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(priority),
                claude_models: None,
                model_policy: None,
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
                supports_websockets: Some(supports_ws),
                extension_values: None,
            },
        )
        .unwrap();
        let mut ids: Vec<_> = providers::default_route_list(&self.db, "codex")
            .unwrap()
            .into_iter()
            .map(|row| row.provider_id)
            .collect();
        if !ids.contains(&row.id) {
            ids.push(row.id);
        }
        providers::default_route_set_order(&self.db, "codex", ids).unwrap();
        row.id
    }

    async fn start(
        &self,
    ) -> (
        Server,
        tokio::sync::mpsc::Receiver<request_logs::RequestLogInsert>,
    ) {
        self.start_with_pipeline(GatewayPluginPipeline::empty_shared())
            .await
    }

    async fn start_with_pipeline(
        &self,
        plugin_pipeline: Arc<GatewayPluginPipeline>,
    ) -> (
        Server,
        tokio::sync::mpsc::Receiver<request_logs::RequestLogInsert>,
    ) {
        let (log_tx, log_rx) = tokio::sync::mpsc::channel(32);
        let state = GatewayAppState {
            app: self.app.handle().clone(),
            db: self.db.clone(),
            log_tx,
            circuit: self.circuit.clone(),
            session: Arc::new(session_manager::SessionManager::new()),
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: Arc::new(Mutex::new(RecentErrorCache::default())),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline,
            active_requests: self.active.clone(),
            responses_ws: self.runtime.clone(),
        };
        (Server::start(build_router(state)).await, log_rx)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (key, value) in self.previous_env.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        settings::clear_cache();
        let _ = crate::gateway::http_client::apply_proxy(None);
    }
}

struct Server {
    addr: std::net::SocketAddr,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    async fn start(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { addr, task }
    }
    fn origin(&self) -> String {
        format!("http://{}", self.addr)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    Complete,
    KeepAlive,
    UnsupportedWs,
    UpgradeRejected(u16, &'static str),
    MalformedSse(&'static str),
    AllFail,
    DisconnectAfterContent,
    ErrorAfterContent,
    JsonComplete,
    JsonIncomplete,
    HoldAfterContent,
}
#[derive(Clone)]
struct Stub {
    name: &'static str,
    behavior: Behavior,
    calls: Arc<Mutex<Vec<(String, HeaderMap, Value)>>>,
    release: Arc<tokio::sync::Notify>,
}
impl Stub {
    async fn start(name: &'static str, behavior: Behavior) -> (Self, Server) {
        let stub = Self {
            name,
            behavior,
            calls: Default::default(),
            release: Default::default(),
        };
        let router = Router::new()
            .route("/v1/responses", get(stub_ws).post(stub_http))
            .with_state(stub.clone());
        (stub, Server::start(router).await)
    }
    fn transports(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(transport, _, _)| transport.clone())
            .collect()
    }
    fn assert_auth(&self) {
        for (_, headers, _) in self.calls.lock().unwrap().iter() {
            assert_eq!(
                headers[header::AUTHORIZATION],
                format!("Bearer sk-{}", self.name)
            );
        }
    }
}

fn events(name: &str) -> Vec<Value> {
    let id = format!("resp-{name}");
    let item = json!({"type":"message", "id":format!("msg-{name}"), "role":"assistant", "status":"completed", "content":[{"type":"output_text", "text":format!("answer-{name}"), "annotations":[]}]});
    vec![
        json!({"type":"response.created", "response":{"id":id, "status":"in_progress", "output":[]}}),
        json!({"type":"response.output_text.delta", "item_id":format!("msg-{name}"), "output_index":0, "content_index":0, "delta":format!("answer-{name}")}),
        json!({"type":"response.output_item.done", "output_index":0, "item":item}),
        json!({"type":"response.completed", "response":{"id":id, "status":"completed", "output":[item], "usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3}}}),
    ]
}

async fn stub_ws(
    State(stub): State<Stub>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    stub.calls
        .lock()
        .unwrap()
        .push(("ws".into(), headers, Value::Null));
    if let Behavior::UpgradeRejected(status, code) = stub.behavior {
        return (
            StatusCode::from_u16(status).unwrap(),
            Json(json!({"error":{"code":code,"message":code}})),
        )
            .into_response();
    }
    if matches!(stub.behavior, Behavior::UnsupportedWs | Behavior::AllFail) {
        return StatusCode::UPGRADE_REQUIRED.into_response();
    }
    upgrade
        .on_upgrade(move |mut socket| async move {
            while let Some(Ok(axum::extract::ws::Message::Text(body))) = socket.recv().await {
                let body: Value = serde_json::from_str(&body).unwrap();
                assert_eq!(body["type"], "response.create");
                assert!(body.get("stream").is_none());
                for event in events(stub.name).into_iter().take(
                    if matches!(
                        stub.behavior,
                        Behavior::DisconnectAfterContent | Behavior::ErrorAfterContent | Behavior::HoldAfterContent
                    ) {
                        2
                    } else {
                        4
                    },
                ) {
                    socket
                        .send(axum::extract::ws::Message::Text(event.to_string()))
                        .await
                        .unwrap();
                }
                if matches!(stub.behavior, Behavior::HoldAfterContent) {
                    stub.release.notified().await;
                    for event in events(stub.name).into_iter().skip(2) {
                        if socket.send(axum::extract::ws::Message::Text(event.to_string())).await.is_err() { return; }
                    }
                }
                if matches!(stub.behavior, Behavior::ErrorAfterContent) {
                    socket.send(axum::extract::ws::Message::Text(json!({
                        "type":"error", "error":{"type":"server_error", "code":"server_error", "message":"failed after output"}
                    }).to_string())).await.unwrap();
                }
                if !matches!(stub.behavior, Behavior::KeepAlive) { break; }
            }
            let _ = socket.close().await;
        })
        .into_response()
}

async fn stub_http(
    State(stub): State<Stub>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    stub.calls
        .lock()
        .unwrap()
        .push(("http".into(), headers, body));
    if let Behavior::MalformedSse(body) = stub.behavior {
        return ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response();
    }
    if matches!(stub.behavior, Behavior::AllFail) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":{"type":"api_error","message":"unavailable"}})),
        )
            .into_response();
    }
    if matches!(
        stub.behavior,
        Behavior::JsonComplete | Behavior::JsonIncomplete
    ) {
        let mut response = events(stub.name).pop().unwrap()["response"].take();
        if matches!(stub.behavior, Behavior::JsonIncomplete) {
            response["status"] = json!("incomplete");
            response["incomplete_details"] = json!({"reason":"max_output_tokens"});
        }
        return Json(response).into_response();
    }
    let body: Vec<u8> = events(stub.name)
        .iter()
        .flat_map(|event| protocol::sse_bytes(event).to_vec())
        .collect();
    ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response()
}

async fn connect(
    server: &Server,
    session: &str,
) -> Result<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Error> {
    connect_with_user_agent(server, session, Some("codex_exec/0.156.0 (router-test)")).await
}

async fn connect_with_user_agent(
    server: &Server,
    session: &str,
    user_agent: Option<&str>,
) -> Result<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Error> {
    let mut request = format!("ws://{}/v1/responses", server.addr)
        .into_client_request()
        .unwrap();
    if let Some(user_agent) = user_agent {
        request
            .headers_mut()
            .insert(header::USER_AGENT, user_agent.parse().unwrap());
    }
    request.headers_mut().insert(
        header::AUTHORIZATION,
        "Bearer downstream-must-be-replaced".parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("session-id", session.parse().unwrap());
    let stream = tokio::net::TcpStream::connect(server.addr).await.unwrap();
    tokio_tungstenite::client_async(request, stream)
        .await
        .map(|(socket, _)| socket)
}

fn create_message(session: Option<&str>) -> Message {
    let mut body = json!({"type":"response.create","model":"gpt-test","input":[{"role":"user","content":[{"type":"input_text","text":"hello"}]}]});
    if let Some(session) = session {
        let metadata = json!({"session_id":session,"thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"turn"}).to_string();
        body["client_metadata"] = json!({"x-codex-turn-metadata":metadata});
    }
    Message::Text(body.to_string())
}

async fn generate(server: &Server, session: &str) -> Vec<Value> {
    generate_with_metadata(server, session, true).await
}

async fn generate_with_metadata(server: &Server, session: &str, metadata: bool) -> Vec<Value> {
    let mut socket = connect(server, session)
        .await
        .expect("downstream WS upgrade");
    socket
        .send(create_message(metadata.then_some(session)))
        .await
        .unwrap();
    let mut observed = Vec::new();
    tokio::time::timeout(Duration::from_secs(8), async {
        while let Some(message) = socket.next().await {
            match message.expect("receive WS frame") {
                Message::Text(text) => {
                    let event: Value = serde_json::from_str(&text).unwrap();
                    let terminal = matches!(
                        event["type"].as_str(),
                        Some(
                            "response.completed"
                                | "response.incomplete"
                                | "response.failed"
                                | "error"
                        )
                    );
                    observed.push(event);
                    if terminal {
                        break;
                    }
                }
                Message::Ping(bytes) => {
                    socket.send(Message::Pong(bytes)).await.unwrap();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    })
    .await
    .expect("generation reaches terminal event");
    let _ = socket.close(None).await;
    observed
}

fn assert_completed(observed: &[Value], name: &str) {
    assert_eq!(
        observed.last().unwrap()["type"],
        "response.completed",
        "{observed:?}"
    );
    assert_eq!(
        observed.last().unwrap()["response"]["id"],
        format!("resp-{name}")
    );
    assert_eq!(
        observed
            .iter()
            .filter(|event| event["type"] == "response.output_text.delta")
            .count(),
        1
    );
}

async fn terminal_log(
    logs: &mut tokio::sync::mpsc::Receiver<request_logs::RequestLogInsert>,
) -> request_logs::RequestLogInsert {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let row = logs.recv().await.unwrap();
            if row.status.is_some() {
                return row;
            }
        }
    })
    .await
    .expect("terminal request log")
}

#[tokio::test(flavor = "current_thread")]
async fn ordinary_http_stream_remains_http_when_provider_supports_ws() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let response = reqwest::Client::new()
        .post(format!("{}/v1/responses", gateway.origin()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            json!({"model":"gpt-test", "stream":true, "input":[{"role":"user","content":"hello"}]})
                .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("response.completed"));
    assert_eq!(stub.transports(), ["http"]);
    stub.assert_auth();
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
}

#[tokio::test(flavor = "current_thread")]
async fn disabled_ws_upgrade_returns_426_without_upstream() {
    let fixture = Fixture::new(false).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    match connect(&gateway, "disabled").await {
        Err(Error::Http(response)) => assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED),
        other => panic!("expected 426, received {other:?}"),
    }
    assert!(stub.transports().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn ws_upgrade_does_not_depend_on_client_name_or_version() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    for user_agent in [
        Some("codex_exec/0.156.0 (router-test)"),
        Some("Codex Desktop/0.158.0-alpha.2.1"),
        Some("codex-tui/99.0.0"),
        Some("future-responses-client"),
        None,
    ] {
        let mut socket = connect_with_user_agent(&gateway, "version-independent", user_agent)
            .await
            .unwrap_or_else(|error| panic!("WS upgrade for {user_agent:?}: {error}"));
        socket.close(None).await.unwrap();
    }
    assert!(
        stub.transports().is_empty(),
        "handshake must not call a provider"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn ws_without_recovery_metadata_keeps_same_connection_context() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::KeepAlive).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "ordinary-ws", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    let first = recv_until(&mut socket, "response.completed").await;
    assert_eq!(first["response"]["id"], "resp-A");
    socket
        .send(Message::Text(
            json!({
                "type":"response.create", "model":"gpt-test", "previous_response_id":"resp-A",
                "input":[{"role":"user","content":"continue"}]
            })
            .to_string(),
        ))
        .await
        .unwrap();
    let second = recv_until(&mut socket, "response.completed").await;
    assert_eq!(second["response"]["id"], "resp-A");
    assert_eq!(
        stub.transports(),
        ["ws"],
        "both generations use one upstream connection"
    );
    socket.close(None).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn ws_without_recovery_metadata_cannot_replay_after_context_loss() {
    for new_connection in [false, true] {
        let fixture = Fixture::new(true).await;
        let (first, upstream) = Stub::start("A", Behavior::Complete).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &upstream.origin(), true);
        fixture.provider("B", &next_server.origin(), false);
        let (gateway, _logs) = fixture.start().await;
        let mut socket = connect(&gateway, "ordinary-context-loss").await.unwrap();
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.completed").await;
        if new_connection {
            socket.close(None).await.unwrap();
            socket = connect(&gateway, "ordinary-context-loss").await.unwrap();
        }
        socket
            .send(Message::Text(
                json!({
                    "type":"response.create", "model":"gpt-test", "previous_response_id":"resp-A",
                    "input":[{"role":"user","content":"continue"}]
                })
                .to_string(),
            ))
            .await
            .unwrap();
        let error = recv_until(&mut socket, "error").await;
        assert_eq!(error["type"], "error");
        assert_eq!(
            first.transports(),
            ["ws"],
            "no HTTP request may replay an incomplete history"
        );
        assert!(
            next.transports().is_empty(),
            "incomplete history cannot move to another provider"
        );
        let _ = socket.close(None).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ws_client_uses_http_for_provider_without_ws_capability() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    assert_completed(&generate(&gateway, "http-only").await, "A");
    assert_eq!(stub.transports(), ["http"]);
    stub.assert_auth();
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
}

#[tokio::test(flavor = "current_thread")]
async fn ws_capable_provider_uses_upstream_ws_and_selected_credentials() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    assert_completed(&generate(&gateway, "ws-success").await, "A");
    assert_eq!(stub.transports(), ["ws"]);
    stub.assert_auth();
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
}

#[tokio::test(flavor = "current_thread")]
async fn unsupported_ws_falls_back_within_provider_and_cools_future_ws_attempts() {
    for metadata in [true, false] {
        let fixture = Fixture::new(true).await;
        let (stub, upstream) = Stub::start("A", Behavior::UnsupportedWs).await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        assert_completed(
            &generate_with_metadata(&gateway, "fallback-1", metadata).await,
            "A",
        );
        assert_eq!(stub.transports(), ["ws", "http"]);
        let log = terminal_log(&mut logs).await;
        assert!(log
            .special_settings_json
            .unwrap_or_default()
            .contains("http_fallback"));
        assert_completed(
            &generate_with_metadata(&gateway, "fallback-2", metadata).await,
            "A",
        );
        assert_eq!(stub.transports(), ["ws", "http", "http"]);
        stub.assert_auth();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_upgrade_capability_errors_fall_back_to_same_provider_http() {
    for status in [400, 404] {
        let fixture = Fixture::new(true).await;
        let (first, upstream) = Stub::start(
            "A",
            Behavior::UpgradeRejected(status, "websocket_not_supported"),
        )
        .await;
        let a = fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        assert_completed(&generate(&gateway, "explicit-ws-unsupported").await, "A");
        assert_eq!(first.transports(), ["ws", "http"]);
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
        assert_eq!(
            fixture
                .circuit
                .snapshot(a, crate::shared::time::now_unix_seconds())
                .failure_count,
            0
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn upgrade_auth_and_model_errors_keep_existing_provider_failure_policy() {
    for (status, code, succeeds) in [
        (401, "invalid_api_key", true),
        (400, "invalid_model", false),
    ] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::UpgradeRejected(status, code)).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &first_server.origin(), true);
        fixture.provider("B", &next_server.origin(), false);
        let (gateway, _logs) = fixture.start().await;
        let output = generate(&gateway, "provider-upgrade-error").await;
        assert_eq!(
            first.transports(),
            ["ws"],
            "provider failure must not become HTTP fallback"
        );
        if succeeds {
            assert_completed(&output, "B");
            assert_eq!(next.transports(), ["http"]);
        } else {
            assert_eq!(output.last().unwrap()["type"], "error");
            assert!(next.transports().is_empty());
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn malformed_http_events_before_content_fail_over_to_next_provider() {
    for malformed in ["data: {bad-json}\n\n", "data: {\"response\":{}}\n\n"] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::MalformedSse(malformed)).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &first_server.origin(), false);
        fixture.provider("B", &next_server.origin(), false);
        let (gateway, mut logs) = fixture.start().await;
        assert_completed(&generate(&gateway, "bad-http-event").await, "B");
        assert_eq!(first.transports(), ["http"]);
        assert_eq!(next.transports(), ["http"]);
        let row = terminal_log(&mut logs).await;
        assert_eq!(row.status, Some(200));
        let attempts: Vec<Value> = serde_json::from_str(&row.attempts_json).unwrap();
        assert_ne!(attempts[0]["error_category"], "local");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn failed_ws_and_http_move_to_next_http_only_provider() {
    for metadata in [true, false] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::AllFail).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        let a = fixture.provider("A", &first_server.origin(), true);
        let b = fixture.provider("B", &next_server.origin(), false);
        let (gateway, mut logs) = fixture.start().await;
        assert_completed(
            &generate_with_metadata(&gateway, "failover", metadata).await,
            "B",
        );
        assert_eq!(first.transports(), ["ws", "http"]);
        assert_eq!(next.transports(), ["http"]);
        first.assert_auth();
        next.assert_auth();
        let log = terminal_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(
            attempts.as_array().unwrap().first().unwrap()["provider_id"],
            a
        );
        assert_eq!(
            attempts.as_array().unwrap().last().unwrap()["provider_id"],
            b
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn all_providers_failed_returns_one_error_without_a_completed_response() {
    let fixture = Fixture::new(true).await;
    let (first, first_server) = Stub::start("A", Behavior::AllFail).await;
    let (next, next_server) = Stub::start("B", Behavior::AllFail).await;
    fixture.provider("A", &first_server.origin(), true);
    fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let output = generate(&gateway, "all-failed").await;
    assert_eq!(
        output
            .iter()
            .filter(|event| event["type"] == "error")
            .count(),
        1
    );
    assert!(!output
        .iter()
        .any(|event| event["type"] == "response.completed"));
    assert_eq!(first.transports(), ["ws", "http"]);
    assert_eq!(next.transports(), ["http"]);
    let log = terminal_log(&mut logs).await;
    assert!(log.status.is_some_and(|status| status >= 400));
    assert!(log.error_code.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn content_then_disconnect_terminates_without_retry_or_next_provider() {
    for metadata in [true, false] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::DisconnectAfterContent).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &first_server.origin(), true);
        fixture.provider("B", &next_server.origin(), false);
        let (gateway, mut logs) = fixture.start().await;
        let observed = generate_with_metadata(&gateway, "partial-disconnect", metadata).await;
        assert!(observed
            .iter()
            .any(|event| event["type"] == "response.output_text.delta"));
        assert_eq!(observed.last().unwrap()["type"], "error", "{observed:?}");
        assert!(!observed
            .iter()
            .any(|event| event["type"] == "response.completed"));
        let log = terminal_log(&mut logs).await;
        assert!(log.error_code.is_some());
        assert_eq!(first.transports(), ["ws"]);
        assert!(next.transports().is_empty());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn json_completed_response_emits_output_items_before_terminal() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::JsonComplete).await;
    fixture.provider("A", &upstream.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let observed = generate(&gateway, "json-completed").await;
    assert_eq!(
        observed.last().unwrap()["type"],
        "response.completed",
        "{observed:?}"
    );
    let done: Vec<_> = observed
        .iter()
        .filter(|event| event["type"] == "response.output_item.done")
        .collect();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0]["item"]["content"][0]["text"], "answer-A");
    assert_eq!(stub.transports(), ["http"]);
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
}

#[tokio::test(flavor = "current_thread")]
async fn json_incomplete_is_terminal_without_retry_or_provider_switch() {
    let fixture = Fixture::new(true).await;
    let (first, first_server) = Stub::start("A", Behavior::JsonIncomplete).await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    fixture.provider("A", &first_server.origin(), false);
    fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let observed = generate(&gateway, "json-incomplete").await;
    assert_eq!(
        observed.last().unwrap()["type"],
        "response.incomplete",
        "{observed:?}"
    );
    assert_eq!(
        observed.last().unwrap()["response"]["incomplete_details"]["reason"],
        "max_output_tokens"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|event| event["type"] == "response.output_item.done")
            .count(),
        1
    );
    assert!(!observed
        .iter()
        .any(|event| event["type"] == "response.completed"));
    let _ = terminal_log(&mut logs).await;
    assert_eq!(first.transports(), ["http"]);
    assert!(next.transports().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn error_event_after_content_is_terminal_without_provider_switch() {
    let fixture = Fixture::new(true).await;
    let (first, first_server) = Stub::start("A", Behavior::ErrorAfterContent).await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    fixture.provider("A", &first_server.origin(), true);
    fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let observed = generate(&gateway, "post-content-error").await;
    assert!(observed
        .iter()
        .any(|event| event["type"] == "response.output_text.delta"));
    assert_eq!(observed.last().unwrap()["type"], "error", "{observed:?}");
    assert_eq!(observed.last().unwrap()["error"]["code"], "server_error");
    assert!(!observed
        .iter()
        .any(|event| event["type"] == "response.completed"));
    assert!(terminal_log(&mut logs).await.error_code.is_some());
    assert_eq!(first.transports(), ["ws"]);
    assert!(next.transports().is_empty());
}

#[derive(Clone, Default)]
struct CliRecoveryStub {
    context_lost: Arc<std::sync::atomic::AtomicBool>,
    calls: Arc<Mutex<Vec<(&'static str, Value)>>>,
    first_input: Arc<Mutex<Option<Vec<Value>>>>,
    tool_item: Arc<Mutex<Option<Value>>>,
}

fn shell_tool(tools: &[Value], namespace: Option<&str>) -> Option<(String, Option<String>)> {
    for tool in tools {
        let name = tool.get("name").and_then(Value::as_str)?;
        if tool["type"] == "namespace" {
            if let Some(found) = shell_tool(tool["tools"].as_array()?, Some(name)) {
                return Some(found);
            }
        } else if tool["type"] == "function"
            && matches!(name, "exec_command" | "shell_command" | "shell")
        {
            return Some((name.into(), namespace.map(str::to_string)));
        }
    }
    None
}

fn cli_tool_item(body: &Value) -> Value {
    let (name, namespace) =
        shell_tool(body["tools"].as_array().unwrap(), None).expect("CLI shell tool");
    let command = if cfg!(windows) {
        "Add-Content -LiteralPath probe-count.txt -Value executed"
    } else {
        "printf 'executed\\n' >> probe-count.txt"
    };
    let args = match name.as_str() {
        "exec_command" => json!({"cmd":command,"max_output_tokens":50}),
        "shell_command" => json!({"command":command,"timeout_ms":1000}),
        _ => json!({"command":["sh","-c",command],"timeout_ms":1000}),
    };
    let mut item = json!({"type":"function_call","id":"fc_router_tool","call_id":"router_tool","name":name,"arguments":args.to_string()});
    if let Some(namespace) = namespace {
        item["namespace"] = json!(namespace);
    }
    item
}

async fn cli_recovery_ws(
    State(stub): State<CliRecoveryStub>,
    upgrade: WebSocketUpgrade,
) -> Response {
    if stub.context_lost.load(std::sync::atomic::Ordering::Acquire) {
        return StatusCode::UPGRADE_REQUIRED.into_response();
    }
    upgrade.on_upgrade(move |mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            let axum::extract::ws::Message::Text(text) = message else { continue; };
            let body: Value = serde_json::from_str(&text).unwrap();
            stub.calls.lock().unwrap().push(("ws", body.clone()));
            let output = if body["generate"] == false {
                vec![json!({"type":"response.created","response":{"id":"warm"}}), json!({"type":"response.completed","response":{"id":"warm","status":"completed","output":[]}})]
            } else if body["input"].as_array().unwrap().iter().any(|item| item["type"] == "function_call_output") {
                stub.context_lost.store(true, std::sync::atomic::Ordering::Release);
                vec![protocol::error_event("previous_response_not_found", "Synthetic lost context")]
            } else {
                *stub.first_input.lock().unwrap() = Some(body["input"].as_array().unwrap().clone());
                let item = cli_tool_item(&body);
                *stub.tool_item.lock().unwrap() = Some(item.clone());
                vec![
                    json!({"type":"response.created","response":{"id":"resp-tool"}}),
                    json!({"type":"response.output_item.done","item":item}),
                    json!({"type":"response.completed","response":{"id":"resp-tool","status":"completed","output":[],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}),
                ]
            };
            for event in output {
                if socket.send(axum::extract::ws::Message::Text(event.to_string())).await.is_err() { return; }
            }
            if stub.context_lost.load(std::sync::atomic::Ordering::Acquire) { break; }
        }
        let _ = socket.close().await;
    }).into_response()
}

async fn cli_recovery_http(
    State(stub): State<CliRecoveryStub>,
    Json(body): Json<Value>,
) -> Response {
    stub.calls.lock().unwrap().push(("http", body));
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error":{"type":"api_error","message":"A unavailable after context loss"}})),
    )
        .into_response()
}

fn cli_test_config(origin: &str) -> String {
    format!(
        r#"model = "gpt-5.4"
model_provider = "probe"
approval_policy = "never"
sandbox_mode = "workspace-write"
web_search = "disabled"
[features]
shell_snapshot = false
background_shell = false
multi_agent = false
plugins = false
apps = false
[model_providers.probe]
name = "Local AIO router probe"
base_url = "{}/v1"
env_key = "AIO_PROBE_TOKEN"
wire_api = "responses"
supports_websockets = true
requires_openai_auth = false
request_max_retries = 0
stream_max_retries = 1
stream_idle_timeout_ms = 5000
"#,
        origin
    )
}

fn isolated_cli_command(path: &std::path::Path, root: &std::path::Path) -> tokio::process::Command {
    let mut command = std::process::Command::new(path);
    command.env_clear();
    for key in [
        "PATH",
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "VOLTA_HOME",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if std::env::var_os("VOLTA_HOME").is_none() {
        if let Some(home) = std::env::var_os("HOME") {
            command.env("VOLTA_HOME", std::path::PathBuf::from(home).join(".volta"));
        }
    }
    command
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("CODEX_HOME", root.join("codex"))
        .env("TMPDIR", root)
        .env("TMP", root)
        .env("TEMP", root)
        .env("AIO_PROBE_TOKEN", "local-test-token")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("RUST_LOG", "off")
        .current_dir(root.join("work"));
    #[cfg(unix)]
    crate::shared::process::configure_unix_process_group(&mut command);
    let mut command = tokio::process::Command::from(command);
    command.kill_on_drop(true);
    command
}

async fn run_cli(command: &mut tokio::process::Command) -> std::process::Output {
    use tokio::io::AsyncReadExt;
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("start selected Codex executable");
    let pid = child.id().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        bytes
    });
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        bytes
    });
    let result = tokio::time::timeout(Duration::from_secs(40), child.wait()).await;
    if result.is_err() {
        #[cfg(unix)]
        crate::shared::process::terminate_unix_process_group(pid);
        #[cfg(windows)]
        crate::shared::process::terminate_windows_process_tree(pid);
        let _ = child.kill().await;
    }
    let output = std::process::Output {
        status: result
            .expect("Codex CLI completed within 40 seconds")
            .expect("wait Codex CLI"),
        stdout: stdout_task.await.unwrap(),
        stderr: stderr_task.await.unwrap(),
    };
    assert!(
        output.stdout.len() < 1024 * 1024 && output.stderr.len() < 1024 * 1024,
        "bounded CLI output"
    );
    output
}

/// Explicit opt-in: AIO_CODEX_WS_TEST_CLI=/absolute/path/to/codex pnpm tauri:test -- real_codex_cli_rebuilds_context --ignored --nocapture
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_rebuilds_context_then_fails_over_without_repeating_tool() {
    let cli_path = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("set absolute AIO_CODEX_WS_TEST_CLI"),
    );
    assert!(
        cli_path.is_absolute() && cli_path.is_file(),
        "select an existing absolute CLI executable"
    );
    let fixture = Fixture::new(true).await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("work")).unwrap();
    std::fs::create_dir(root.path().join("codex")).unwrap();
    let version = run_cli(isolated_cli_command(&cli_path, root.path()).arg("--version")).await;
    assert!(
        version.status.success(),
        "selected CLI must report its version"
    );
    let version = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    println!("selected CLI: {version}");
    let recovery = CliRecoveryStub::default();
    let first = Server::start(
        Router::new()
            .route(
                "/v1/responses",
                get(cli_recovery_ws).post(cli_recovery_http),
            )
            .with_state(recovery.clone()),
    )
    .await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    let a = fixture.provider("A", &first.origin(), true);
    let b = fixture.provider("B", &next_server.origin(), false);
    let mut cfg = settings::read(fixture.app.handle()).unwrap();
    cfg.upstream_first_byte_timeout_seconds = 15;
    settings::write(fixture.app.handle(), &cfg).unwrap();
    let (gateway, mut logs) = fixture.start().await;
    let config = cli_test_config(&gateway.origin());
    std::fs::write(root.path().join("codex/config.toml"), config).unwrap();
    let output = run_cli(isolated_cli_command(&cli_path, root.path()).args([
        "exec", "--skip-git-repo-check", "--ephemeral", "--ignore-rules", "--json", "--color", "never", "--cd",
    ]).arg(root.path().join("work")).arg("Run this local protocol test. If instructed by the test server, append one line to probe-count.txt exactly once, then finish.")).await;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|event| event["type"] == "turn.completed"),
        "CLI did not complete the turn"
    );
    let count = std::fs::read_to_string(root.path().join("work/probe-count.txt")).unwrap();
    assert_eq!(
        count.lines().collect::<Vec<_>>(),
        ["executed"],
        "tool side effect must execute exactly once"
    );
    assert!(
        recovery
            .context_lost
            .load(std::sync::atomic::Ordering::Acquire),
        "must exercise context loss"
    );
    let calls = recovery.calls.lock().unwrap();
    assert!(
        calls.iter().any(|(transport, body)| *transport == "ws"
            && body.get("previous_response_id").is_some()
            && body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "function_call_output")),
        "must exercise an incremental tool round"
    );
    assert_eq!(
        next.transports(),
        ["http"],
        "next provider must receive one full HTTP request"
    );
    let next_calls = next.calls.lock().unwrap();
    let full = &next_calls[0].2;
    assert!(
        full.get("previous_response_id").is_none(),
        "no account-bound response reference may reach B"
    );
    let input = full["input"].as_array().unwrap();
    let original = recovery.first_input.lock().unwrap();
    let original = original.as_ref().unwrap();
    assert!(
        input.starts_with(original),
        "original input must survive rebuild"
    );
    let item = recovery.tool_item.lock().unwrap();
    let item = item.as_ref().unwrap();
    assert!(
        input.iter().any(|entry| entry["type"] == "function_call"
            && entry["call_id"] == item["call_id"]
            && entry["name"] == item["name"]
            && entry["arguments"] == item["arguments"]),
        "tool call must survive rebuild"
    );
    assert_eq!(
        input
            .iter()
            .filter(|entry| entry["type"] == "function_call_output"
                && entry["call_id"] == "router_tool")
            .count(),
        1
    );
    assert!(
        full.pointer("/client_metadata/x-codex-turn-state")
            .is_none(),
        "local recovery nonce must not leak upstream"
    );
    let mut terminal_rows = Vec::new();
    while let Ok(row) = logs.try_recv() {
        if row.status.is_some() {
            terminal_rows.push(row);
        }
    }
    assert!(!terminal_rows.is_empty(), "real generations must be logged");
    let attempts: Vec<Value> = terminal_rows
        .iter()
        .flat_map(|row| serde_json::from_str::<Vec<Value>>(&row.attempts_json).unwrap())
        .collect();
    assert!(
        attempts.len() <= 5,
        "recovery must not reset the original attempt budget: {}",
        attempts.len()
    );
    assert!(
        attempts
            .iter()
            .any(|attempt| attempt["provider_id"] == a && attempt["outcome"] != "success"),
        "failed provider A must remain in diagnostics"
    );
    assert_eq!(attempts.last().unwrap()["provider_id"], b);
    assert_eq!(attempts.last().unwrap()["outcome"], "success");
    println!(
        "{version} → AIO router → context rebuild → provider B HTTP: passed; tool executions=1"
    );
}

#[derive(Clone, Default)]
struct CliTwoTurnStub {
    connections: Arc<std::sync::atomic::AtomicUsize>,
    calls: Arc<Mutex<Vec<Value>>>,
}

async fn cli_two_turn_ws(
    State(stub): State<CliTwoTurnStub>,
    upgrade: WebSocketUpgrade,
) -> Response {
    stub.connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    upgrade.on_upgrade(move |mut socket| async move {
        while let Some(Ok(axum::extract::ws::Message::Text(text))) = socket.recv().await {
            let body: Value = serde_json::from_str(&text).unwrap();
            let generation = {
                let mut calls = stub.calls.lock().unwrap();
                calls.push(body.clone());
                calls.iter().filter(|body| body["generate"] != false).count()
            };
            let output = if body["generate"] == false {
                vec![json!({"type":"response.completed","response":{"id":"warm","status":"completed","output":[]}})]
            } else if generation == 2 {
                let item = cli_tool_item(&body);
                vec![
                    json!({"type":"response.output_item.done","item":item}),
                    json!({"type":"response.completed","response":{"id":"resp-tool","status":"completed","output":[item]}}),
                ]
            } else {
                events(if generation == 1 { "first" } else { "final" })
            };
            for event in output {
                if socket.send(axum::extract::ws::Message::Text(event.to_string())).await.is_err() { return; }
            }
        }
    }).into_response()
}

async fn cli_message_until(
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
    matches: impl Fn(&Value) -> bool,
) -> Value {
    while let Some(line) = lines.next_line().await.unwrap() {
        assert!(line.len() < 1024 * 1024, "bounded app-server response");
        let message: Value = serde_json::from_str(&line).expect("app-server JSON line");
        assert!(
            message.get("error").is_none(),
            "app-server request failed: {message}"
        );
        assert_ne!(
            message["method"], "error",
            "app-server turn failed: {message}"
        );
        if matches(&message) {
            return message;
        }
    }
    panic!("app-server exited before expected message")
}

/// stdio drives two real CLI turns; the model requests still use the production AIO WS router.
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_two_user_turns_and_tool_increment_keep_the_same_ws_context() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let cli_path = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("set absolute AIO_CODEX_WS_TEST_CLI"),
    );
    assert!(cli_path.is_absolute() && cli_path.is_file());
    let fixture = Fixture::new(true).await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("work")).unwrap();
    std::fs::create_dir(root.path().join("codex")).unwrap();
    let version = run_cli(isolated_cli_command(&cli_path, root.path()).arg("--version")).await;
    assert!(
        version.status.success(),
        "selected CLI must report its version"
    );
    let version = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    println!("selected CLI: {version}");
    let stub = CliTwoTurnStub::default();
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(cli_two_turn_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    std::fs::write(
        root.path().join("codex/config.toml"),
        cli_test_config(&gateway.origin()),
    )
    .unwrap();
    let mut child = isolated_cli_command(&cli_path, root.path())
        .arg("app-server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let result = tokio::time::timeout(Duration::from_secs(40), async {
        stdin.write_all(format!("{}\n", json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"codex_cli_rs","version":"1"},"capabilities":{"experimentalApi":true}}})).as_bytes()).await.unwrap();
        cli_message_until(&mut lines, |message| message["id"] == 1).await;
        stdin.write_all(format!("{}\n{}\n", json!({"method":"initialized","params":{}}), json!({"id":2,"method":"thread/start","params":{"cwd":root.path().join("work"),"model":"gpt-5.4","approvalPolicy":"never","sandbox":"workspace-write"}})).as_bytes()).await.unwrap();
        let thread = cli_message_until(&mut lines, |message| message["id"] == 2).await;
        let thread_id = thread["result"]["thread"]["id"].as_str().unwrap();
        for (id, text) in [(3, "Reply with a short answer."), (4, "Continue this local protocol test. If instructed, append one line to probe-count.txt exactly once, then finish.")] {
            stdin.write_all(format!("{}\n", json!({"id":id,"method":"turn/start","params":{"threadId":thread_id,"input":[{"type":"text","text":text,"text_elements":[]}]}})).as_bytes()).await.unwrap();
            let completed = cli_message_until(&mut lines, |message| message["method"] == "turn/completed").await;
            assert_eq!(completed["params"]["turn"]["status"], "completed", "{completed}");
        }
    }).await;
    #[cfg(unix)]
    crate::shared::process::terminate_unix_process_group(pid);
    #[cfg(windows)]
    crate::shared::process::terminate_windows_process_tree(pid);
    let _ = child.kill().await;
    let _ = child.wait().await;
    result.expect("two CLI turns completed within 40 seconds");
    let calls = stub.calls.lock().unwrap();
    let generations: Vec<_> = calls
        .iter()
        .filter(|body| body["generate"] != false)
        .collect();
    assert_eq!(
        generations.len(),
        3,
        "first turn, second turn, tool continuation"
    );
    assert_eq!(
        stub.connections.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    let owners: Vec<_> = generations
        .iter()
        .map(|body| {
            super::protocol::Owner::parse(
                body["client_metadata"]["x-codex-turn-metadata"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
        })
        .collect();
    assert_ne!(owners[0].turn, owners[1].turn);
    assert_eq!(owners[1], owners[2]);
    assert_eq!(owners[0].window, owners[1].window);
    assert_eq!(generations[1]["previous_response_id"], "resp-first");
    assert_eq!(generations[1]["input"].as_array().unwrap().len(), 1);
    assert_eq!(generations[2]["previous_response_id"], "resp-tool");
    assert_eq!(generations[2]["input"].as_array().unwrap().len(), 1);
    assert_eq!(generations[2]["input"][0]["type"], "function_call_output");
    assert!(generations.iter().all(|body| body
        .pointer("/client_metadata/x-codex-turn-state")
        .is_none()));
    assert_eq!(
        std::fs::read_to_string(root.path().join("work/probe-count.txt"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["executed"]
    );
}

async fn recv_until(
    socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    expected: &str,
) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(message) = socket.next().await {
            match message.unwrap() {
                Message::Text(text) => {
                    let event: Value = serde_json::from_str(&text).unwrap();
                    if event["type"] == expected {
                        return event;
                    }
                    assert_ne!(event["type"], "error", "unexpected error before {expected}");
                }
                Message::Close(_) => panic!("closed before {expected}"),
                _ => {}
            }
        }
        panic!("EOF before {expected}")
    })
    .await
    .expect("expected downstream event")
}

#[tokio::test(flavor = "current_thread")]
async fn full_input_recovery_after_attempt_deadline_uses_remaining_provider_budget() {
    let fixture = Fixture::new(true).await;
    let recovery = CliRecoveryStub::default();
    let first = Server::start(
        Router::new()
            .route(
                "/v1/responses",
                get(cli_recovery_ws).post(cli_recovery_http),
            )
            .with_state(recovery.clone()),
    )
    .await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    let a = fixture.provider("A", &first.origin(), true);
    let b = fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect(&gateway, "expired-attempt-recovery").await.unwrap();
    let Message::Text(create) = create_message(Some("expired-attempt-recovery")) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    body["tools"] =
        json!([{"type":"function","name":"shell_command","parameters":{"type":"object"}}]);
    let first_input = body["input"].as_array().unwrap().clone();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
        [protocol::TURN_STATE_HEADER]
        .as_str()
        .unwrap()
        .to_owned();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));

    let tool_output = json!({"type":"function_call_output","call_id":"router_tool","output":"synthetic tool result"});
    body["input"] = json!([tool_output]);
    body["previous_response_id"] = json!("resp-tool");
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let error = recv_until(&mut socket, "error").await;
    assert_eq!(error["error"]["code"], "previous_response_not_found");
    assert!(terminal_log(&mut logs)
        .await
        .special_settings_json
        .unwrap_or_default()
        .contains("full_input_retry"));
    let _ = socket.close(None).await;
    drop(socket);
    // The original attempt has expired, but the pending recovery TTL and B's slot have not.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let mut input = first_input;
    input.push(recovery.tool_item.lock().unwrap().clone().unwrap());
    input.push(tool_output);
    body["input"] = json!(input);
    body["stream"] = json!(true);
    body.as_object_mut().unwrap().remove("type");
    body.as_object_mut().unwrap().remove("previous_response_id");
    let response = reqwest::Client::new()
        .post(format!("{}/v1/responses", gateway.origin()))
        .header(header::CONTENT_TYPE, "application/json")
        .header(protocol::TURN_STATE_HEADER, nonce)
        .header(
            "x-codex-turn-metadata",
            body["client_metadata"]["x-codex-turn-metadata"]
                .as_str()
                .unwrap(),
        )
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("response.completed"));
    assert_eq!(
        recovery
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(|(transport, _)| *transport)
            .collect::<Vec<_>>(),
        ["ws", "ws"],
        "expired A must not receive another request"
    );
    assert_eq!(next.transports(), ["http"]);
    let log = terminal_log(&mut logs).await;
    let attempts: Vec<Value> = serde_json::from_str(&log.attempts_json).unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["provider_id"], a);
    assert_eq!(attempts[0]["error_code"], "GW_UPSTREAM_TIMEOUT");
    assert_eq!(attempts[0]["upstream_sent"], false);
    assert_eq!(attempts[1]["provider_id"], b);
    assert_eq!(attempts[1]["outcome"], "success");
}

#[tokio::test(flavor = "current_thread")]
async fn client_cancel_after_content_logs_499_without_provider_health_damage() {
    let fixture = Fixture::new(true).await;
    let (first, first_server) = Stub::start("A", Behavior::HoldAfterContent).await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    let a = fixture.provider("A", &first_server.origin(), true);
    fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect(&gateway, "cancel-after-content").await.unwrap();
    socket
        .send(create_message(Some("cancel-after-content")))
        .await
        .unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    assert_eq!(fixture.active.snapshot().len(), 1);
    socket.close(None).await.unwrap();
    drop(socket);
    let log = terminal_log(&mut logs).await;
    assert_eq!(log.status, Some(499));
    assert!(fixture.active.snapshot().is_empty());
    let health = fixture
        .circuit
        .snapshot(a, crate::shared::time::now_unix_seconds());
    assert_eq!(health.failure_count, 0);
    assert!(health.cooldown_until.is_none());
    assert!(next.transports().is_empty());
    first.release.notify_one();
}

#[tokio::test(flavor = "current_thread")]
async fn disabling_ws_finishes_accepted_generation_then_rejects_new_ws_and_keeps_http() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::HoldAfterContent).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect(&gateway, "disable-after-content").await.unwrap();
    socket
        .send(create_message(Some("disable-after-content")))
        .await
        .unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    fixture.runtime.set_enabled(false);
    stub.release.notify_one();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    let _ = socket
        .send(create_message(Some("next-disabled-generation")))
        .await;
    let closed = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap();
    assert!(
        matches!(closed, None | Some(Ok(Message::Close(_))) | Some(Err(_))),
        "socket must close before a new generation"
    );
    assert_eq!(stub.transports(), ["ws"]);
    match connect(&gateway, "disabled-new-socket").await {
        Err(Error::Http(response)) => assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED),
        other => panic!("expected 426 after disable: {other:?}"),
    }
    let response = reqwest::Client::new().post(format!("{}/v1/responses", gateway.origin()))
        .header(header::CONTENT_TYPE, "application/json")
        .body(json!({"model":"gpt-test", "stream":true, "input":[{"role":"user","content":"still works"}]}).to_string())
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("response.completed"));
    assert_eq!(stub.transports(), ["ws", "http"]);
}

#[path = "integration_plugin_tests.rs"]
mod plugin_tests;

#[tokio::test(flavor = "current_thread")]
async fn custom_headers_ws_fallback_and_failover_do_not_leak_identity() {
    for failover in [false, true] {
        let fixture = Fixture::new(true).await;
        let (first, a) = Stub::start(
            "A",
            if failover {
                Behavior::AllFail
            } else {
                Behavior::UnsupportedWs
            },
        )
        .await;
        let (second, b) = Stub::start("B", Behavior::Complete).await;
        fixture.provider_with_headers("A", &a.origin(), true, None, Some("tenant-a"));
        fixture.provider("B", &b.origin(), false);
        let (gateway, mut logs) = fixture.start().await;
        let observed = generate(&gateway, "custom-fallback").await;
        assert_eq!(observed.last().unwrap()["type"], "response.completed");
        assert_eq!(first.transports(), ["ws", "http"]);
        for (_, headers, _) in first.calls.lock().unwrap().iter() {
            assert_eq!(headers["x-tenant"], "tenant-a");
        }
        for (_, headers, _) in second.calls.lock().unwrap().iter() {
            assert!(!headers.contains_key("x-tenant"));
        }
        assert_eq!(second.transports().len(), usize::from(failover));
        first.assert_auth();
        second.assert_auth();
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn custom_headers_ws_reuse_and_changed_identity_reject_old_continuation() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::KeepAlive).await;
    let id = fixture.provider_with_headers("A", &upstream.origin(), true, None, Some("tenant-a"));
    let (gateway, _logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "custom-reuse", None)
        .await
        .unwrap();
    for _ in 0..2 {
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.completed").await;
    }
    assert_eq!(stub.transports(), ["ws"]);
    fixture.provider_with_headers("A", &upstream.origin(), true, Some(id), Some("tenant-b"));
    // Do not invalidate here: the effective-header key must protect the preparation/send race too.
    socket.send(Message::Text(json!({"type":"response.create","model":"gpt-test","previous_response_id":"resp-A","input":[{"role":"user","content":"continue"}]}).to_string())).await.unwrap();
    recv_until(&mut socket, "error").await;
    assert_eq!(
        stub.transports(),
        ["ws"],
        "old context must never reach the new identity"
    );
    let observed = generate(&gateway, "custom-new-identity").await;
    assert_eq!(observed.last().unwrap()["type"], "response.completed");
    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1["x-tenant"], "tenant-a");
    assert_eq!(calls[1].1["x-tenant"], "tenant-b");
}

#[tokio::test(flavor = "current_thread")]
async fn custom_headers_hot_update_drains_active_generation_before_new_identity() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::HoldAfterContent).await;
    let id = fixture.provider_with_headers("A", &upstream.origin(), true, None, Some("tenant-a"));
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect(&gateway, "custom-active").await.unwrap();
    socket
        .send(create_message(Some("custom-active")))
        .await
        .unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    fixture.provider_with_headers("A", &upstream.origin(), true, Some(id), Some("tenant-b"));
    fixture.runtime.invalidate();
    stub.release.notify_one();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    let closed = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap();
    assert!(matches!(
        closed,
        None | Some(Ok(Message::Close(_))) | Some(Err(_))
    ));
    let mut next = connect(&gateway, "custom-active-next").await.unwrap();
    next.send(create_message(Some("custom-active-next")))
        .await
        .unwrap();
    recv_until(&mut next, "response.output_text.delta").await;
    stub.release.notify_one();
    recv_until(&mut next, "response.completed").await;
    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1["x-tenant"], "tenant-a");
    assert_eq!(calls[1].1["x-tenant"], "tenant-b");
}

#[tokio::test(flavor = "current_thread")]
async fn custom_headers_local_cx2cc_gateway_uses_final_codex_provider() {
    use tauri::Manager;
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::JsonComplete).await;
    fixture.provider_with_headers("A", &upstream.origin(), false, None, Some("final-tenant"));
    let conn = fixture.db.open_connection().unwrap();
    conn.execute("INSERT INTO providers(cli_key,name,base_url,api_key_plaintext,bridge_type,created_at,updated_at) VALUES ('claude','local-bridge','','','cx2cc',1,1)", []).unwrap();
    let bridge = conn.last_insert_rowid();
    drop(conn);
    providers::default_route_set_order(&fixture.db, "claude", vec![bridge]).unwrap();
    fixture
        .app
        .manage(crate::app::gateway_state::GatewayState::default());
    let cfg = settings::read(fixture.app.handle()).unwrap();
    let started = crate::app::gateway_control::app_start_gateway_with_config(
        fixture.app.handle(),
        fixture.db.clone(),
        &cfg,
        None,
    )
    .unwrap();
    let response = reqwest::Client::new()
        .post(format!("{}/claude/_aio/provider/{bridge}/v1/messages", started.status.base_url.unwrap()))
        .json(&json!({"model":"claude-sonnet-4","max_tokens":128,"messages":[{"role":"user","content":"hello"}]}))
        .send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    let (shutdown, task, log_task, circuit_task, oauth_shutdown, oauth_task) =
        crate::app::gateway_control::app_take_running_gateway(fixture.app.handle()).unwrap();
    let _ = shutdown.send(());
    let _ = oauth_shutdown.send(true);
    for task in [task, log_task, circuit_task, oauth_task] {
        task.abort();
    }
    assert_eq!(status, StatusCode::OK, "{body}");
    stub.assert_auth();
    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "http");
    assert_eq!(calls[0].1["x-tenant"], "final-tenant");
}
