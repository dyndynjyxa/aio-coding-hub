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
    recent_errors: Arc<Mutex<RecentErrorCache>>,
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
            recent_errors: Default::default(),
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
        self.provider_for_cli_with_headers(
            "codex",
            name,
            base_url,
            supports_ws,
            provider_id,
            tenant,
        )
    }

    fn provider_for_cli_with_headers(
        &self,
        cli: &str,
        name: &str,
        base_url: &str,
        supports_ws: bool,
        provider_id: Option<i64>,
        tenant: Option<&str>,
    ) -> i64 {
        let priority = providers::default_route_list(&self.db, cli).unwrap().len() as i64;
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
                cli_key: cli.into(),
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
                oauth_min_remaining_percent: None,
                oauth_use_credits: false,
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
        let mut ids: Vec<_> = providers::default_route_list(&self.db, cli)
            .unwrap()
            .into_iter()
            .map(|row| row.provider_id)
            .collect();
        if !ids.contains(&row.id) {
            ids.push(row.id);
        }
        providers::default_route_set_order(&self.db, cli, ids).unwrap();
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
        (
            Server::start(self.router(log_tx, plugin_pipeline)).await,
            log_rx,
        )
    }

    fn router(
        &self,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        plugin_pipeline: Arc<GatewayPluginPipeline>,
    ) -> Router {
        let state = GatewayAppState {
            app: self.app.handle().clone(),
            db: self.db.clone(),
            log_tx,
            circuit: self.circuit.clone(),
            session: Arc::new(session_manager::SessionManager::new()),
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: self.recent_errors.clone(),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline,
            active_requests: self.active.clone(),
            responses_ws: self.runtime.clone(),
        };
        build_router(state)
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
    DisconnectIncrementalBeforeOutput,
    ErrorAfterContent(&'static str),
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
            .layer(axum::extract::DefaultBodyLimit::max(
                crate::gateway::util::max_request_body_bytes(),
            ))
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
        .push(("ws".into(), headers.clone(), Value::Null));
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
                if matches!(stub.behavior, Behavior::DisconnectIncrementalBeforeOutput)
                    && body.get("previous_response_id").is_some()
                {
                    stub.calls
                        .lock()
                        .unwrap()
                        .push(("ws".into(), headers.clone(), body));
                    break;
                }
                for event in events(stub.name).into_iter().take(
                    if matches!(
                        stub.behavior,
                        Behavior::DisconnectAfterContent
                            | Behavior::ErrorAfterContent(_)
                            | Behavior::HoldAfterContent
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
                        if socket
                            .send(axum::extract::ws::Message::Text(event.to_string()))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                if let Behavior::ErrorAfterContent(code) = stub.behavior {
                    socket.send(axum::extract::ws::Message::Text(json!({
                        "type":"error", "error":{"code":code, "message":"failed after output"}
                    }).to_string())).await.unwrap();
                }
                if !matches!(
                    stub.behavior,
                    Behavior::KeepAlive | Behavior::DisconnectIncrementalBeforeOutput
                ) {
                    break;
                }
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

#[tokio::test(flavor = "current_thread")]
#[ignore = "waits for the configured 70-second connection idle timeout"]
async fn connection_idle_timeout_uses_global_setting_and_zero_disables_it() {
    let fixture = Fixture::new(true).await;
    let (gateway, _logs) = fixture.start().await;
    let mut sockets = Vec::new();
    for seconds in [0, 70] {
        let mut settings = settings::read(fixture.app.handle()).unwrap();
        settings.upstream_stream_idle_timeout_seconds = seconds;
        settings::write(fixture.app.handle(), &settings).unwrap();
        let mut socket = connect(&gateway, &format!("idle-{seconds}")).await.unwrap();
        socket.send(Message::Ping(vec![1])).await.unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            Message::Pong(vec![1])
        );
        sockets.push(socket);
    }

    tokio::time::sleep(Duration::from_secs(61)).await;
    for socket in &mut sockets {
        assert!(
            tokio::time::timeout(Duration::from_millis(100), socket.next())
                .await
                .is_err(),
            "must stay open beyond the former fixed 60-second timeout"
        );
    }
    let closed = tokio::time::timeout(Duration::from_secs(15), sockets[1].next())
        .await
        .expect("configured idle timeout closes the connection");
    assert!(matches!(closed, Some(Ok(Message::Close(_)))));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), sockets[0].next())
            .await
            .is_err(),
        "zero disables connection idle timeout"
    );
    let _ = sockets[0].close(None).await;
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
async fn ws_forged_nonce_rejection_logs_the_same_trace_without_calling_upstream() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    let provider = fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let session = "local-rejection";
    let Message::Text(create) = create_message(Some(session)) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    body["input"][0]["content"][0]["text"] = json!("private-prompt-must-not-be-logged");
    let owner = protocol::Owner::parse(
        body["client_metadata"]["x-codex-turn-metadata"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let nonce = fixture.runtime.issue_nonce(&owner).unwrap();
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!("aio-ws-forged");
    let reason = "context recovery ownership mismatch";
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let error = recv_until(&mut socket, "error").await;
    assert_eq!(error["status"], 400);
    assert_eq!(error["error"]["code"], "invalid_request");
    assert_eq!(error["error"]["message"], reason);
    let trace = error["trace_id"].as_str().expect("local rejection trace");
    let log = terminal_log(&mut logs).await;
    assert_eq!(log.trace_id, trace);
    assert_eq!(log.status, Some(400));
    assert_eq!(log.error_code.as_deref(), Some("GW_REQUEST_REJECTED"));
    assert_eq!(log.method, "POST");
    assert_eq!(log.path, "/v1/responses");
    assert_eq!(
        serde_json::from_str::<Value>(&log.attempts_json).unwrap(),
        json!([])
    );
    let details = log.error_details_json.as_deref().unwrap();
    let parsed: Value = serde_json::from_str(details).unwrap();
    assert_eq!(parsed["error_category"], "local");
    assert_eq!(parsed["reason_code"], "invalid_request");
    assert_eq!(parsed["reason"], reason);
    let settings = log.special_settings_json.as_deref().unwrap();
    let parsed: Vec<Value> = serde_json::from_str(settings).unwrap();
    assert!(parsed
        .iter()
        .any(|setting| setting["type"] == "codex_responses_transport"
            && setting["client_transport"] == "responses_ws"
            && setting["failure_class"] == "local"
            && setting["upstream_sent"] == false));
    for encoded in [details, settings] {
        assert!(!encoded.contains(&nonce));
        assert!(!encoded.contains("aio-ws-forged"));
        assert!(!encoded.contains("private-prompt-must-not-be-logged"));
    }
    assert!(stub.transports().is_empty());
    assert!(fixture.active.snapshot().is_empty());
    assert_eq!(
        fixture
            .circuit
            .snapshot(provider, crate::shared::time::now_unix_seconds())
            .failure_count,
        0
    );
    assert!(logs.try_recv().is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn codex_retry_of_a_completed_generation_runs_again_on_ws_and_http() {
    // Codex resends the same input when it never saw the stream finish, e.g. the
    // connection dropped after the gateway forwarded the terminal event.
    for http in [false, true] {
        let fixture = Fixture::new(true).await;
        let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let session = "completed-retry";
        let Message::Text(create) = create_message(Some(session)) else {
            unreachable!()
        };
        let mut body: Value = serde_json::from_str(&create).unwrap();
        body["input"][0]["type"] = json!("message");
        let mut socket = connect(&gateway, session).await.unwrap();
        socket.send(Message::Text(body.to_string())).await.unwrap();
        let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
            [protocol::TURN_STATE_HEADER]
            .as_str()
            .unwrap()
            .to_owned();
        recv_until(&mut socket, "response.completed").await;
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
        socket.close(None).await.unwrap();
        drop(socket);

        body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
        if http {
            body.as_object_mut().unwrap().remove("type");
            body["stream"] = json!(true);
            let response = reqwest::Client::new()
                .post(format!("{}/v1/responses", gateway.origin()))
                .header(protocol::TURN_STATE_HEADER, &nonce)
                .header("session-id", session)
                .json(&body)
                .send()
                .await
                .unwrap();
            let status = response.status();
            let text = response.text().await.unwrap();
            assert_eq!(status, StatusCode::OK, "{text}");
            assert!(text.contains("response.completed"));
        } else {
            let mut socket = connect(&gateway, session).await.unwrap();
            socket.send(Message::Text(body.to_string())).await.unwrap();
            let echoed = recv_until(&mut socket, "response.metadata").await;
            assert_eq!(echoed["headers"][protocol::TURN_STATE_HEADER], nonce);
            recv_until(&mut socket, "response.completed").await;
        }
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
        assert_eq!(
            stub.transports(),
            if http { ["ws", "http"] } else { ["ws", "ws"] }
        );
    }
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
async fn ws_incremental_continuation_logs_session_reuse() {
    let fixture = Fixture::new(true).await;
    let (_stub, upstream) = Stub::start("A", Behavior::KeepAlive).await;
    let provider_id = fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "ws-session-reuse", None)
        .await
        .unwrap();
    let cache_key = "019a0000-0000-7000-8000-00000000c0de";
    socket
        .send(Message::Text(
            json!({
                "type":"response.create", "model":"gpt-test", "prompt_cache_key":cache_key,
                "input":[{"role":"user","content":"hello"}]
            })
            .to_string(),
        ))
        .await
        .unwrap();
    recv_until(&mut socket, "response.completed").await;
    let first = terminal_log(&mut logs).await;
    let first_attempts: Vec<Value> = serde_json::from_str(&first.attempts_json).unwrap();
    assert_eq!(first_attempts[0]["session_reuse"], Value::Null);

    // Codex 续接只带增量 input（单条），会话绑定仍须生效。
    socket
        .send(Message::Text(
            json!({
                "type":"response.create", "model":"gpt-test", "prompt_cache_key":cache_key,
                "previous_response_id":"resp-A",
                "input":[{"type":"function_call_output","call_id":"call-1","output":"ok"}]
            })
            .to_string(),
        ))
        .await
        .unwrap();
    recv_until(&mut socket, "response.completed").await;
    let second = terminal_log(&mut logs).await;
    assert_eq!(second.session_id, first.session_id);
    assert!(second.session_id.is_some());
    let attempts: Vec<Value> = serde_json::from_str(&second.attempts_json).unwrap();
    assert_eq!(attempts[0]["provider_id"], provider_id);
    assert_eq!(attempts[0]["session_reuse"], true);
    assert_eq!(attempts[0]["selection_method"], "session_reuse");
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
async fn http_fallback_moves_only_the_affected_session_to_continuous_http() {
    for (supports_ws, behavior) in [
        (false, Behavior::Complete),
        (true, Behavior::UnsupportedWs),
        (true, Behavior::DisconnectIncrementalBeforeOutput),
    ] {
        for metadata in [false, true] {
            let fixture = Fixture::new(true).await;
            let (stub, upstream) = Stub::start("A", behavior).await;
            fixture.provider("A", &upstream.origin(), supports_ws);
            let (gateway, mut logs) = fixture.start().await;
            // The transport session need not equal the recovery owner's session.
            let session = "http-fallback-transport";
            let Message::Text(create) = create_message(metadata.then_some("http-fallback-owner"))
            else {
                unreachable!()
            };
            let mut body: Value = serde_json::from_str(&create).unwrap();
            body["input"][0]["type"] = json!("message");
            let mut history = body["input"].as_array().unwrap().clone();
            let mut socket = connect(&gateway, session).await.unwrap();
            socket.send(Message::Text(body.to_string())).await.unwrap();
            let nonce = if metadata {
                Some(
                    recv_until(&mut socket, "response.metadata").await["headers"]
                        [protocol::TURN_STATE_HEADER]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
            } else {
                None
            };
            let completed = recv_until(&mut socket, "response.completed").await;
            history.extend(
                completed["response"]["output"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .cloned(),
            );
            assert_eq!(terminal_log(&mut logs).await.status, Some(200));
            let upstream_before = stub.transports();

            let next = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"continue"}]});
            history.push(next.clone());
            body["input"] = json!([next]);
            body["previous_response_id"] = json!("resp-A");
            if let Some(nonce) = &nonce {
                body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
            }
            socket.send(Message::Text(body.to_string())).await.unwrap();
            let error = recv_until(&mut socket, "error").await;
            assert_eq!(error["error"]["code"], "previous_response_not_found");
            assert!(error.get("status").is_none());
            let failure = terminal_log(&mut logs).await;
            assert_eq!(failure.status, Some(400));
            let attempts: Vec<Value> = serde_json::from_str(&failure.attempts_json).unwrap();
            if matches!(behavior, Behavior::DisconnectIncrementalBeforeOutput) {
                assert!(attempts
                    .iter()
                    .any(|attempt| attempt["reason"] == "ws_stream_failed_before_output"));
                assert_eq!(stub.transports(), ["ws", "ws"]);
            } else {
                assert!(
                    attempts.is_empty(),
                    "local context rejection must not call upstream"
                );
                assert_eq!(stub.transports(), upstream_before);
            }
            drop(socket);
            match connect(&gateway, session).await {
                Err(Error::Http(response)) => {
                    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED)
                }
                Ok(_) => panic!("expected affected session to switch to HTTP; got 101"),
                Err(error) => panic!("expected affected session to switch to HTTP: {error}"),
            }
            let mut other = connect(&gateway, "unaffected-session").await.unwrap();
            other.close(None).await.unwrap();
            let mut expected = stub.transports();

            body.as_object_mut().unwrap().remove("type");
            body.as_object_mut().unwrap().remove("previous_response_id");
            body["stream"] = json!(true);
            for round in 0..2 {
                body["input"] = json!(history);
                let mut request = reqwest::Client::new()
                    .post(format!("{}/v1/responses", gateway.origin()))
                    .header("session-id", session)
                    .json(&body);
                if let Some(nonce) = &nonce {
                    request = request.header(protocol::TURN_STATE_HEADER, nonce);
                }
                let response = request.send().await.unwrap();
                let status = response.status();
                let text = response.text().await.unwrap();
                assert_eq!(status, StatusCode::OK, "{text}");
                assert!(text.contains("response.completed"));
                let row = terminal_log(&mut logs).await;
                assert_eq!(row.status, Some(200));
                if metadata && round == 0 {
                    assert!(row
                        .special_settings_json
                        .unwrap_or_default()
                        .contains("recovery_from_trace_id"));
                }
                expected.push("http".into());
                assert_eq!(stub.transports(), expected);
                history.extend(
                    events("A").last().unwrap()["response"]["output"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned(),
                );
                history.push(json!({"type":"message","role":"user","content":[{"type":"input_text","text":"next HTTP round"}]}));
            }
            for (transport, headers, body) in stub.calls.lock().unwrap().iter() {
                if transport == "http" {
                    assert!(body.get("previous_response_id").is_none());
                    assert!(!headers.contains_key(protocol::TURN_STATE_HEADER));
                    assert!(body
                        .pointer("/client_metadata/x-codex-turn-state")
                        .is_none());
                }
            }
            stub.assert_auth();
        }
    }
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
    for (code, status) in [("server_error", 502), ("previous_response_not_found", 400)] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::ErrorAfterContent(code)).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &first_server.origin(), true);
        fixture.provider("B", &next_server.origin(), false);
        let (gateway, mut logs) = fixture.start().await;
        let observed = generate(&gateway, "post-content-error").await;
        assert!(observed
            .iter()
            .any(|event| event["type"] == "response.output_text.delta"));
        assert_eq!(observed.last().unwrap()["type"], "error", "{observed:?}");
        assert_eq!(observed.last().unwrap()["error"]["code"], code);
        assert_eq!(observed.last().unwrap()["status"], status);
        assert!(!observed
            .iter()
            .any(|event| event["type"] == "response.completed"));
        assert!(terminal_log(&mut logs).await.error_code.is_some());
        assert_eq!(first.transports(), ["ws"]);
        assert!(next.transports().is_empty());
    }
}

#[derive(Clone, Copy, Default)]
enum CliRecoveryFailure {
    #[default]
    ContextLost,
    Disconnect,
    ContextLostAfterOutput,
}

#[derive(Clone, Default)]
struct CliRecoveryStub {
    failure: CliRecoveryFailure,
    context_lost: Arc<std::sync::atomic::AtomicBool>,
    calls: Arc<Mutex<Vec<(&'static str, Value)>>>,
    tool_item: Arc<Mutex<Option<Value>>>,
}

fn cli_first_generation_input(calls: &[(&str, Value)]) -> Vec<Value> {
    let mut input = Vec::new();
    for (_, body) in calls {
        // A formal increment can reuse input sent by a preceding prewarm.
        if body.get("previous_response_id").is_none() {
            input.clear();
        }
        input.extend(body["input"].as_array().unwrap().iter().cloned());
        if body["generate"] != false {
            break;
        }
    }
    input
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

fn cli_tool_item(body: &Value, suffix: &str) -> Value {
    let (name, namespace) =
        shell_tool(body["tools"].as_array().unwrap(), None).expect("CLI shell tool");
    let command = if cfg!(windows) {
        format!("Add-Content -LiteralPath probe-count.txt -Value executed{suffix}")
    } else {
        format!("printf 'executed{suffix}\\n' >> probe-count.txt")
    };
    let args = match name.as_str() {
        "exec_command" => json!({"cmd":command,"max_output_tokens":50}),
        "shell_command" => json!({"command":command,"timeout_ms":1000}),
        _ => json!({"command":["sh","-c",command],"timeout_ms":1000}),
    };
    let mut item = json!({"type":"function_call","id":format!("fc_router_tool{suffix}"),"call_id":format!("router_tool{suffix}"),"name":name,"arguments":args.to_string()});
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
                let error = protocol::error_event("previous_response_not_found", "Synthetic lost context");
                match stub.failure {
                    CliRecoveryFailure::Disconnect => break,
                    CliRecoveryFailure::ContextLost => vec![error],
                    CliRecoveryFailure::ContextLostAfterOutput => events("partial").into_iter().take(3).chain(std::iter::once(error)).collect(),
                }
            } else {
                let item = cli_tool_item(&body, "");
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
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    assert!(!headers.contains_key(protocol::TURN_STATE_HEADER));
    assert!(body
        .pointer("/client_metadata/x-codex-turn-state")
        .is_none());
    stub.calls.lock().unwrap().push(("http", body.clone()));
    if matches!(stub.failure, CliRecoveryFailure::Disconnect) {
        let output = if body["input"].as_array().unwrap().iter().any(|item| {
            item["type"] == "function_call_output" && item["call_id"] == "router_tool-http"
        }) {
            events("final")
        } else {
            let item = cli_tool_item(&body, "-http");
            vec![
                json!({"type":"response.output_item.done","item":item}),
                json!({"type":"response.completed","response":{"id":"resp-http-tool","status":"completed","output":[item]}}),
            ]
        };
        return (
            [(header::CONTENT_TYPE, "text/event-stream")],
            output
                .iter()
                .flat_map(|event| protocol::sse_bytes(event).to_vec())
                .collect::<Vec<_>>(),
        )
            .into_response();
    }
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
    run_cli_with_timeout(command, Duration::from_secs(40)).await
}

async fn run_cli_with_timeout(
    command: &mut tokio::process::Command,
    timeout: Duration,
) -> std::process::Output {
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
    let result = tokio::time::timeout(timeout, child.wait()).await;
    if result.is_err() {
        #[cfg(unix)]
        crate::shared::process::terminate_unix_process_group(pid);
        #[cfg(windows)]
        crate::shared::process::terminate_windows_process_tree(pid);
        let _ = child.kill().await;
    }
    let output = std::process::Output {
        status: result
            .expect("selected CLI completed within the test deadline")
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

/// Explicit opt-in: AIO_CODEX_WS_TEST_CLI=/absolute/path/to/codex pnpm tauri:test -- real_codex_cli_rebuilds_context --lib -- --ignored --nocapture
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
    let original = cli_first_generation_input(&calls);
    let original_digest = protocol::HistoryDigest::from_items(&original);
    assert!(original_digest.is_recoverable());
    assert!(input.len() >= original.len());
    assert_eq!(
        protocol::HistoryDigest::from_items(&input[..original.len()]),
        original_digest,
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

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_switches_to_http_after_ws_loss_without_repeating_tools() {
    run_real_codex_cli_ws_failure(CliRecoveryFailure::Disconnect).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a selected Codex CLI whose status-bearing 400 errors are terminal"]
async fn real_codex_cli_keeps_post_output_context_error_terminal() {
    run_real_codex_cli_ws_failure(CliRecoveryFailure::ContextLostAfterOutput).await;
}

async fn run_real_codex_cli_ws_failure(failure: CliRecoveryFailure) {
    let cli = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("selected Codex executable"),
    );
    assert!(cli.is_absolute() && cli.is_file());
    let fixture = Fixture::new(true).await;
    let stub = CliRecoveryStub {
        failure,
        ..Default::default()
    };
    let upstream = Server::start(
        Router::new()
            .route(
                "/v1/responses",
                get(cli_recovery_ws).post(cli_recovery_http),
            )
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    let (log_tx, mut logs) = tokio::sync::mpsc::channel(32);
    let router = fixture
        .router(log_tx, GatewayPluginPipeline::empty_shared())
        .layer(axum::middleware::from_fn(
            move |request: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let recorded = recorded.clone();
                async move {
                    let method = request.method().to_string();
                    let response = next.run(request).await;
                    recorded
                        .lock()
                        .unwrap()
                        .push((method, response.status().as_u16()));
                    response
                }
            },
        ));
    let gateway = Server::start(router).await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("work")).unwrap();
    std::fs::create_dir(root.path().join("codex")).unwrap();
    std::fs::write(
        root.path().join("codex/config.toml"),
        cli_test_config(&gateway.origin()),
    )
    .unwrap();
    let version = run_cli(isolated_cli_command(&cli, root.path()).arg("--version")).await;
    assert!(version.status.success());
    let output = run_cli(isolated_cli_command(&cli, root.path()).args([
        "exec", "--skip-git-repo-check", "--ephemeral", "--ignore-rules", "--json", "--color", "never", "--cd",
    ]).arg(root.path().join("work")).arg("Run this local protocol test. Execute each tool requested by the test server exactly once, then finish.")).await;
    if matches!(failure, CliRecoveryFailure::ContextLostAfterOutput) {
        assert!(!output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("answer-partial"),
            "CLI must receive the partial output: {stdout}"
        );
        assert!(stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|event| event["type"] == "turn.failed"));
        assert_eq!(*requests.lock().unwrap(), [("GET".into(), 101)]);
        let calls = stub.calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|(_, body)| body["generate"] != false)
                .map(|(transport, _)| *transport)
                .collect::<Vec<_>>(),
            ["ws", "ws"]
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("work/probe-count.txt"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["executed"]
        );
        println!(
            "{} → partial output → terminal context error: passed; no extra generation",
            String::from_utf8_lossy(&version.stdout).trim()
        );
        return;
    }
    assert!(
        output.status.success(),
        "CLI failed: status={}, stderr={}, stdout={}, AIO requests={:?}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
        requests.lock().unwrap()
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .any(|event| event["type"] == "turn.completed"));
    let executions = std::fs::read_to_string(root.path().join("work/probe-count.txt")).unwrap();
    assert_eq!(
        executions.lines().collect::<Vec<_>>(),
        ["executed", "executed-http"]
    );
    assert_eq!(
        *requests.lock().unwrap(),
        [
            ("GET".into(), 101),
            ("GET".into(), 426),
            ("POST".into(), 200),
            ("POST".into(), 200),
        ],
        "AIO must move the real CLI to HTTP before the second tool"
    );
    let calls = stub.calls.lock().unwrap();
    let generations: Vec<_> = calls
        .iter()
        .filter(|(_, body)| body["generate"] != false)
        .collect();
    assert_eq!(
        generations
            .iter()
            .map(|(transport, _)| *transport)
            .collect::<Vec<_>>(),
        ["ws", "ws", "http", "http"]
    );
    let original = cli_first_generation_input(&calls);
    let original_digest = protocol::HistoryDigest::from_items(&original);
    assert!(original_digest.is_recoverable());
    let tool = stub.tool_item.lock().unwrap();
    let tool = tool.as_ref().unwrap();
    let failed_output = calls
        .iter()
        .filter(|(transport, _)| *transport == "ws")
        .flat_map(|(_, body)| body["input"].as_array().unwrap())
        .find(|item| item["type"] == "function_call_output" && item["call_id"] == tool["call_id"])
        .unwrap();
    let failed_output_digest =
        protocol::HistoryDigest::from_items(std::slice::from_ref(failed_output));
    assert!(failed_output_digest.is_recoverable());
    for (round, (_, body)) in calls
        .iter()
        .filter(|(transport, _)| *transport == "http")
        .enumerate()
    {
        assert!(body.get("previous_response_id").is_none());
        let input = body["input"].as_array().unwrap();
        assert!(input.len() >= original.len());
        assert_eq!(
            protocol::HistoryDigest::from_items(&input[..original.len()]),
            original_digest
        );
        assert!(input.iter().any(|item| item["type"] == "function_call"
            && item["call_id"] == tool["call_id"]
            && item["name"] == tool["name"]
            && item["arguments"] == tool["arguments"]));
        let outputs: Vec<_> = input
            .iter()
            .filter(|item| item["type"] == "function_call_output")
            .collect();
        assert_eq!(
            outputs
                .iter()
                .map(|item| item["call_id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["router_tool", "router_tool-http"][..round + 1]
        );
        assert!(outputs.iter().all(|output| input
            .iter()
            .any(|item| item["type"] == "function_call" && item["call_id"] == output["call_id"])));
        assert_eq!(
            protocol::HistoryDigest::from_items(std::slice::from_ref(outputs[0])),
            failed_output_digest,
            "the original tool result must survive both HTTP rounds"
        );
    }
    let mut rows = Vec::new();
    while let Ok(row) = logs.try_recv() {
        if row.status.is_some() {
            rows.push(row);
        }
    }
    assert_eq!(
        rows.iter()
            .map(|row| row.status.unwrap())
            .collect::<Vec<_>>(),
        [200, 400, 200, 200]
    );
    let owner = generations[0]
        .1
        .pointer("/client_metadata/x-codex-turn-metadata")
        .and_then(Value::as_str)
        .and_then(protocol::Owner::parse);
    if owner.is_some() {
        assert!(
            rows[2]
                .special_settings_json
                .as_deref()
                .unwrap_or_default()
                .contains("recovery_from_trace_id"),
            "identified CLI must claim the original recovery budget"
        );
    }
    println!(
        "{} → AIO 426 → two HTTP tool rounds: passed; each tool executed once; recovery owner={}",
        String::from_utf8_lossy(&version.stdout).trim(),
        owner.is_some()
    );
}

#[derive(Clone, Default)]
struct CliTwoTurnStub {
    connections: Arc<std::sync::atomic::AtomicUsize>,
    calls: Arc<Mutex<Vec<Value>>>,
    auto_compact: bool,
}

impl CliTwoTurnStub {
    fn response(&self, body: Value) -> Vec<Value> {
        let generation = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(body.clone());
            calls
                .iter()
                .filter(|body| body["generate"] != false)
                .count()
        };
        if body["generate"] == false {
            vec![
                json!({"type":"response.completed","response":{"id":"warm","status":"completed","output":[]}}),
            ]
        } else if self.auto_compact
            && body
                .pointer("/client_metadata/x-codex-turn-metadata")
                .and_then(Value::as_str)
                .and_then(|metadata| serde_json::from_str::<Value>(metadata).ok())
                .is_some_and(|metadata| metadata["request_kind"] == "compaction")
        {
            if body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "compaction_trigger")
            {
                let item = json!({"type":"compaction","id":"cmp_probe","encrypted_content":"synthetic-checkpoint"});
                vec![
                    json!({"type":"response.output_item.done","output_index":0,"item":item}),
                    json!({"type":"response.completed","response":{"id":"resp-compaction","status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3}}}),
                ]
            } else {
                events("summary")
            }
        } else if generation == if self.auto_compact { 1 } else { 2 } {
            let item = cli_tool_item(&body, "");
            let mut output = vec![
                json!({"type":"response.output_item.done","item":item}),
                json!({"type":"response.completed","response":{"id":"resp-tool","status":"completed","output":[item]}}),
            ];
            if self.auto_compact {
                output[1]["response"]["usage"] =
                    json!({"input_tokens":200000,"output_tokens":10,"total_tokens":200010});
            }
            output
        } else {
            events(if generation == 1 { "first" } else { "final" })
        }
    }
}

async fn cli_two_turn_ws(
    State(stub): State<CliTwoTurnStub>,
    upgrade: WebSocketUpgrade,
) -> Response {
    stub.connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    upgrade
        .on_upgrade(move |mut socket| async move {
            while let Some(Ok(axum::extract::ws::Message::Text(text))) = socket.recv().await {
                let body: Value = serde_json::from_str(&text).unwrap();
                for event in stub.response(body) {
                    if socket
                        .send(axum::extract::ws::Message::Text(event.to_string()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        })
        .into_response()
}

async fn cli_two_turn_http(
    State(stub): State<CliTwoTurnStub>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    assert!(!headers.contains_key(protocol::TURN_STATE_HEADER));
    let body: Vec<u8> = stub
        .response(body)
        .iter()
        .flat_map(|event| protocol::sse_bytes(event).to_vec())
        .collect();
    ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response()
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
    recv_until_with_timeout(socket, expected, Duration::from_secs(3)).await
}

async fn recv_until_with_timeout(
    socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    expected: &str,
    timeout: Duration,
) -> Value {
    tokio::time::timeout(timeout, async {
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
    .unwrap_or_else(|_| panic!("expected downstream event {expected} within {timeout:?}"))
}

#[tokio::test(flavor = "current_thread")]
async fn completed_turn_compaction_starts_fresh_generation_on_ws_and_http() {
    for (http, remote) in [(false, false), (true, false), (false, true), (true, true)] {
        let fixture = Fixture::new(true).await;
        let stub = CliTwoTurnStub::default();
        let upstream = Server::start(
            Router::new()
                .route(
                    "/v1/responses",
                    get(cli_two_turn_ws).post(cli_two_turn_http),
                )
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let session = "completed-compaction";
        let mut socket = connect(&gateway, session).await.unwrap();
        let Message::Text(create) = create_message(Some(session)) else {
            unreachable!()
        };
        let mut body: Value = serde_json::from_str(&create).unwrap();
        body["input"] = json!([
            {"type":"message","role":"user","content":[{"type":"input_text","text":"first request"}]},
            {"type":"message","role":"assistant","content":[{"type":"output_text","text":"first answer"}]},
            {"type":"message","role":"user","content":[{"type":"input_text","text":"more context"}]}
        ]);
        body["tools"] =
            json!([{"type":"function","name":"shell_command","parameters":{"type":"object"}}]);
        socket.send(Message::Text(body.to_string())).await.unwrap();
        let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
            [protocol::TURN_STATE_HEADER]
            .as_str()
            .unwrap()
            .to_owned();
        recv_until(&mut socket, "response.completed").await;
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
        if http {
            socket.close(None).await.unwrap();
            body.as_object_mut().unwrap().remove("type");
            body["stream"] = json!(true);
        }
        body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
        let tool_output =
            json!({"type":"function_call_output","call_id":"router_tool","output":"done"});
        for generation in 0..3 {
            if generation == 1 {
                body["input"] = if http {
                    let mut history = body["input"].as_array().unwrap().clone();
                    history.extend([cli_tool_item(&body, ""), tool_output.clone()]);
                    json!(history)
                } else {
                    body["previous_response_id"] = json!("resp-tool");
                    json!([tool_output])
                };
            } else {
                body.as_object_mut().unwrap().remove("previous_response_id");
                body["input"] = json!([{"type":"message","role":"user","content":[{"type":"input_text","text":"compacted summary"}]}]);
                if remote {
                    body["input"].as_array_mut().unwrap().push(json!({
                        "type":"compaction", "id":format!("cmp_{generation}"),
                        "encrypted_content":format!("synthetic-checkpoint-{generation}")
                    }));
                }
                let mut metadata: Value = serde_json::from_str(
                    body["client_metadata"]["x-codex-turn-metadata"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
                metadata["context_window_id"] = json!(format!("compacted-{generation}"));
                body["client_metadata"]["x-codex-turn-metadata"] = json!(metadata.to_string());
            }
            if http {
                let response = reqwest::Client::new()
                    .post(format!("{}/v1/responses", gateway.origin()))
                    .header(protocol::TURN_STATE_HEADER, &nonce)
                    .header("session-id", session)
                    .json(&body)
                    .send()
                    .await
                    .unwrap();
                let status = response.status();
                let response_body = response.text().await.unwrap();
                assert_eq!(
                    status,
                    StatusCode::OK,
                    "generation {generation}: {response_body}"
                );
                assert!(response_body.contains("response.completed"));
            } else {
                socket.send(Message::Text(body.to_string())).await.unwrap();
                let response = recv_until(&mut socket, "response.completed").await;
                assert_eq!(
                    response["response"]["id"],
                    if generation == 0 {
                        "resp-tool"
                    } else {
                        "resp-final"
                    }
                );
            }
            let log = terminal_log(&mut logs).await;
            assert_eq!(log.status, Some(200));
            let attempts: Vec<Value> = serde_json::from_str(&log.attempts_json).unwrap();
            assert_eq!(
                attempts.len(),
                1,
                "each generation needs a fresh attempt budget"
            );
            let settings: Vec<Value> =
                serde_json::from_str(log.special_settings_json.as_deref().unwrap()).unwrap();
            assert!(settings
                .iter()
                .filter(|setting| setting["type"] == "codex_responses_transport")
                .all(|setting| setting["recovery_from_trace_id"].is_null()));
        }
        let calls = stub.calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        let compacted_items = if remote { 2 } else { 1 };
        assert_eq!(calls[1]["input"].as_array().unwrap().len(), compacted_items);
        assert_eq!(calls[3]["input"].as_array().unwrap().len(), compacted_items);
        if remote && http {
            assert_eq!(calls[2]["input"][1], calls[1]["input"][1]);
            assert_eq!(calls[2]["input"].as_array().unwrap().len(), 4);
        }
        assert!(calls.iter().all(|body| body
            .pointer("/client_metadata/x-codex-turn-state")
            .is_none()));
        assert_eq!(
            stub.connections.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        if !http {
            socket.close(None).await.unwrap();
        }
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_auto_compaction_preserves_the_turn_nonce_without_repeating_tool() {
    let cli_path = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("set absolute AIO_CODEX_WS_TEST_CLI"),
    );
    assert!(cli_path.is_absolute() && cli_path.is_file());
    for remote in [false, true] {
        let fixture = Fixture::new(true).await;
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("work")).unwrap();
        std::fs::create_dir(root.path().join("codex")).unwrap();
        let stub = CliTwoTurnStub {
            auto_compact: true,
            ..Default::default()
        };
        let upstream = Server::start(
            Router::new()
                .route(
                    "/v1/responses",
                    get(cli_two_turn_ws).post(cli_two_turn_http),
                )
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let mut config = cli_test_config(&gateway.origin());
        if remote {
            config = config.replace("name = \"Local AIO router probe\"", "name = \"OpenAI\"");
        }
        std::fs::write(
        root.path().join("codex/config.toml"),
        format!("model_auto_compact_token_limit = 20000\nmodel_auto_compact_token_limit_scope = \"total\"\nmodel_post_turn_compact_threshold_percent = 0\n{config}"),
    ).unwrap();
        let output = run_cli(isolated_cli_command(&cli_path, root.path()).args([
        "exec", "--skip-git-repo-check", "--ephemeral", "--ignore-rules", "--json", "--color", "never", "--cd",
    ]).arg(root.path().join("work")).arg("Run the local protocol probe, execute the requested shell tool once, then finish after compacting context.")).await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "CLI failed: {}\n{stdout}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .any(|event| event["type"] == "turn.completed"),
            "CLI turn did not complete: {stdout}"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("work/probe-count.txt"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        let calls = stub.calls.lock().unwrap();
        let formal_calls: Vec<&Value> = calls
            .iter()
            .filter(|body| body["generate"] != false)
            .collect();
        let metadata: Vec<Value> = formal_calls
            .iter()
            .map(|body| {
                serde_json::from_str(
                    body["client_metadata"]["x-codex-turn-metadata"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap()
            })
            .collect();
        assert_eq!(
            metadata.len(),
            3,
            "one tool request, one compaction, one continuation: {metadata:?}"
        );
        assert_eq!(metadata[1]["request_kind"], "compaction");
        for field in ["session_id", "thread_id", "turn_id"] {
            assert_eq!(metadata[0][field], metadata[1][field]);
            assert_eq!(metadata[0][field], metadata[2][field]);
        }
        for field in ["window_id", "context_window_id"] {
            assert_eq!(metadata[0][field], metadata[1][field]);
            assert_ne!(metadata[0][field], metadata[2][field]);
        }
        let contains_compaction = |body: &Value, kind: &str| {
            body["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == kind)
        };
        assert_eq!(
            contains_compaction(formal_calls[1], "compaction_trigger"),
            remote
        );
        assert_eq!(contains_compaction(formal_calls[2], "compaction"), remote);
        if remote {
            assert!(formal_calls[2]["input"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["type"] == "compaction"
                    && item["encrypted_content"] == "synthetic-checkpoint"));
        }
        assert!(calls.iter().all(|body| body
            .pointer("/client_metadata/x-codex-turn-state")
            .is_none()));
        drop(calls);
        for generation in 0..3 {
            let log = terminal_log(&mut logs).await;
            assert_eq!(log.status, Some(200));
            let settings: Vec<Value> =
                serde_json::from_str(log.special_settings_json.as_deref().unwrap()).unwrap();
            assert!(
                settings
                    .iter()
                    .any(|setting| setting["type"] == "codex_responses_transport"
                        && setting["scope"] == "request"
                        && setting["client_transport"] == "responses_ws"),
                "formal generation {generation} must enter over WS: {settings:?}"
            );
            let selected: Vec<_> = settings
                .iter()
                .filter(|setting| {
                    setting["type"] == "codex_responses_transport"
                        && setting["scope"] == "attempt"
                        && setting["transport_action"] == "selected"
                })
                .collect();
            assert_eq!(
                selected.len(),
                1,
                "formal generation {generation}: {settings:?}"
            );
            assert_eq!(selected[0]["upstream_transport"], "responses_ws");
            assert!(selected[0]["failure_class"].is_null());
        }
        assert!(logs.try_recv().is_err());
        println!("real CLI → AIO WS → {} automatic compaction → completed; formal WS requests=3, tool executions=1",
        if remote { "remote checkpoint" } else { "local summary" });
    }
}

#[tokio::test(flavor = "current_thread")]
async fn lost_context_with_unverifiable_history_still_asks_codex_for_a_full_resend() {
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
    fixture.provider("A", &first.origin(), true);
    fixture.provider("B", &next_server.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let session = "unverifiable-context-loss";
    let Message::Text(create) = create_message(Some(session)) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    // An item type the gateway cannot verify, e.g. one added by a newer Codex.
    body["input"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"future_item","payload":"opaque"}));
    body["tools"] =
        json!([{"type":"function","name":"shell_command","parameters":{"type":"object"}}]);
    let first_input = body["input"].as_array().unwrap().clone();
    let mut socket = connect(&gateway, session).await.unwrap();
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
    terminal_log(&mut logs).await;
    drop(socket);

    // Without a recovery grant the full resend still runs, as a fresh generation.
    let mut input = first_input;
    input.push(recovery.tool_item.lock().unwrap().clone().unwrap());
    input.push(tool_output);
    body["input"] = json!(input);
    body.as_object_mut().unwrap().remove("previous_response_id");
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    recv_until(&mut socket, "response.completed").await;
    let log = terminal_log(&mut logs).await;
    assert_eq!(log.status, Some(200));
    assert!(!log
        .special_settings_json
        .unwrap_or_default()
        .contains("\"recovery_from_trace_id\":\""));
    assert_eq!(next.transports(), ["http"]);
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
    let mut retry = connect(&gateway, "expired-attempt-recovery").await.unwrap();
    retry.close(None).await.unwrap();
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
async fn turn_continues_after_the_gateway_forgets_its_nonce() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let session = "forgotten-nonce";
    let Message::Text(create) = create_message(Some(session)) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
        [protocol::TURN_STATE_HEADER]
        .as_str()
        .unwrap()
        .to_owned();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    drop(socket);

    // Restarting AIO or toggling WS settings forgets every owner; Codex keeps the nonce.
    fixture.runtime.invalidate();
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
    body["input"] = json!([body["input"][0], events("A")[2]["item"], body["input"][0]]);
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let echoed = recv_until(&mut socket, "response.metadata").await;
    assert_eq!(echoed["headers"][protocol::TURN_STATE_HEADER], nonce);
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(stub.transports(), ["ws", "ws"]);
}

#[tokio::test(flavor = "current_thread")]
async fn codex_preempted_generation_continues_the_same_turn_on_a_new_socket() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::HoldAfterContent).await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let session = "codex-preempt";
    let Message::Text(create) = create_message(Some(session)) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    let user = |text: &str| json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]});
    let answer = events("A")[2]["item"].clone();

    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
        [protocol::TURN_STATE_HEADER]
        .as_str()
        .unwrap()
        .to_owned();
    recv_until(&mut socket, "response.output_text.delta").await;
    stub.release.notify_waiters();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    socket.close(None).await.unwrap();
    drop(socket);

    // Steering or sub-agent input makes Codex abandon the in-flight stream...
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
    body["input"] = json!([body["input"][0], answer, user("tool result")]);
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.close(None).await.unwrap();
    drop(socket);
    assert_eq!(terminal_log(&mut logs).await.status, Some(499));

    // ...and resend the same turn, with the sub-agent message, on a fresh socket.
    body["input"].as_array_mut().unwrap().push(json!({
        "type":"agent_message", "id":"amsg_worker", "author":"/root/worker", "recipient":"/root",
        "content":[{"type":"input_text","text":"Message Type: FINAL_ANSWER"},{"type":"encrypted_content","encrypted_content":"opaque"}],
        "internal_chat_message_metadata_passthrough":{"turn_id":"turn","create_time":1.5}
    }));
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    stub.release.notify_waiters();
    let completed = recv_until(&mut socket, "response.completed").await;
    assert_eq!(completed["response"]["id"], "resp-A");
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(stub.transports(), ["ws", "ws", "ws"]);
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

#[tokio::test(flavor = "current_thread")]
async fn unavailable_cache_is_scoped_to_final_candidates() {
    let fixture = Fixture::new(true).await;
    let (first, first_server) = Stub::start("A", Behavior::Complete).await;
    let (next, next_server) = Stub::start("B", Behavior::Complete).await;
    let a = fixture.provider("A", &first_server.origin(), false);
    fixture.provider("B", &next_server.origin(), false);
    fixture
        .circuit
        .record_failure(a, crate::gateway::util::now_unix_seconds() as i64, None);
    let (gateway, _logs) = fixture.start().await;
    let client = reqwest::Client::new();
    let body =
        json!({"model":"gpt-test", "stream":true, "input":[{"role":"user","content":"hello"}]});
    let forced = client
        .post(format!("{}/v1/responses", gateway.origin()))
        .header("session-id", "review-cache")
        .header("x-aio-provider-id", a.to_string())
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(forced.status().as_u16(), 503);
    let forced_body: Value = forced.json().await.unwrap();
    let ordinary = client
        .post(format!("{}/v1/responses", gateway.origin()))
        .header("session-id", "review-cache")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(ordinary.status().as_u16(), 200);
    assert!(ordinary.text().await.unwrap().contains("answer-B"));
    assert!(first.transports().is_empty());
    assert_eq!(next.transports(), ["http"]);
    assert_eq!(forced_body["error_code"], "GW_ALL_PROVIDERS_UNAVAILABLE");
}

#[tokio::test(flavor = "current_thread")]
async fn recovered_budget_does_not_cache_global_unavailability() {
    use super::protocol::{HistoryDigest, Owner};
    use super::state::{Budget, Generation, RecoveryIdentity, RequestState};
    for all_budget_failed in [false, true] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start("A", Behavior::Complete).await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        let a = fixture.provider("A", &first_server.origin(), false);
        let b = fixture.provider("B", &next_server.origin(), false);
        if !all_budget_failed {
            fixture.circuit.record_failure(
                b,
                crate::gateway::util::now_unix_seconds() as i64,
                None,
            );
        }
        let metadata = json!({"session_id":"review-recovery", "thread_id":"thread", "window_id":"window", "context_window_id":"context", "turn_id":"turn"}).to_string();
        let owner = Owner::parse(&metadata).unwrap();
        let nonce = fixture.runtime.issue_nonce(&owner).unwrap();
        let body = json!({"model":"gpt-test", "stream":true, "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}]});
        let mut budget = Budget {
            providers: vec![a, b],
            failed_providers: std::collections::HashSet::from([a]),
            ..Budget::default()
        };
        if all_budget_failed {
            budget.failed_providers.insert(b);
        }
        let original = RequestState {
            connection: fixture.runtime.connection().unwrap(),
            client_ws: true,
            generation: Arc::new(Mutex::new(Generation {
                identity: Some(RecoveryIdentity {
                    owner,
                    nonce: nonce.clone(),
                }),
                expected: HistoryDigest::from_items(body["input"].as_array().unwrap()),
                input: HistoryDigest::from_items(body["input"].as_array().unwrap()),
                properties: super::state::request_properties(&body, None),
                previous: None,
                committed: false,
                terminal: false,
                incomplete: false,
                failed: false,
                recovered: false,
                from_trace: None,
                trace_id: "review-original-upstream-failure".into(),
                budget,
                upstream_ws: false,
            })),
        };
        fixture.runtime.begin_generation(&original).unwrap();
        fixture.runtime.suspend(&original).unwrap();
        let (gateway, _logs) = fixture.start().await;
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/responses", gateway.origin()))
            .header(protocol::TURN_STATE_HEADER, nonce)
            .header("x-codex-turn-metadata", metadata)
            .header("session-id", "review-recovery")
            .json(&body)
            .send()
            .await
            .unwrap();
        let recovery_status = response.status().as_u16();
        let recovery_body: Value = response.json().await.unwrap();
        let ordinary = client
            .post(format!("{}/v1/responses", gateway.origin()))
            .header("session-id", "new-request")
            .json(&body)
            .send()
            .await
            .unwrap();
        let ordinary_status = ordinary.status().as_u16();
        let ordinary_text = ordinary.text().await.unwrap();
        assert_eq!(recovery_status, if all_budget_failed { 502 } else { 503 });
        assert_eq!(
            recovery_body["error_code"],
            if all_budget_failed {
                "GW_UPSTREAM_ALL_FAILED"
            } else {
                "GW_ALL_PROVIDERS_UNAVAILABLE"
            }
        );
        assert_eq!(ordinary_status, 200);
        assert!(ordinary_text.contains("answer-A"));
        assert_eq!(first.transports(), ["http"]);
        assert!(next.transports().is_empty());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn circuit_failure_uses_inbound_protocol_for_every_cli_and_auxiliary_route() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    let (gateway, _logs) = fixture.start().await;
    let client = reqwest::Client::new();
    for (cli, paths, protocol) in [
        (
            "codex",
            vec!["/v1/responses", "/responses", "/v1/chat/completions"],
            "openai",
        ),
        ("grok", vec!["/v1/responses", "/chat/completions"], "openai"),
        (
            "claude",
            vec!["/v1/messages", "/v1/messages/count_tokens"],
            "anthropic",
        ),
        (
            "gemini",
            vec![
                "/v1beta/models/gemini-test:generateContent",
                "/v1beta/models/gemini-test:streamGenerateContent",
                "/v1beta/models/gemini-test:countTokens",
            ],
            "gemini",
        ),
    ] {
        let provider = fixture.provider_for_cli_with_headers(
            cli,
            cli,
            &upstream.origin(),
            cli == "codex",
            None,
            None,
        );
        fixture.circuit.record_failure(
            provider,
            crate::gateway::util::now_unix_seconds() as i64,
            Some("GW_UPSTREAM_TIMEOUT"),
        );
        for path in paths {
            for stream in [false, true] {
                let url = format!("{}/{cli}/_aio/provider/{provider}{path}", gateway.origin());
                let body = json!({"model":"test", "stream":stream, "input":[], "messages":[{"role":"user","content":"hello"}], "contents":[{"parts":[{"text":"hello"}]}]});
                let response = client.post(&url).json(&body).send().await.unwrap();
                assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{url}");
                assert!(response.headers()[header::CONTENT_TYPE]
                    .to_str()
                    .unwrap()
                    .starts_with("application/json"));
                let trace = response.headers()["x-trace-id"]
                    .to_str()
                    .unwrap()
                    .to_owned();
                assert!(response.headers().contains_key(header::RETRY_AFTER));
                let error: Value = response.json().await.unwrap();
                assert_eq!(error["trace_id"], trace);
                assert_eq!(error["error_code"], "GW_ALL_PROVIDERS_UNAVAILABLE");
                assert_eq!(error["error"]["message"], error["message"]);
                assert!(error["message"]
                    .as_str()
                    .unwrap()
                    .contains("circuit breakers"));
                match protocol {
                    "openai" => {
                        assert_eq!(error["error"]["code"], "GW_ALL_PROVIDERS_UNAVAILABLE");
                        assert_eq!(error["error"]["type"], "server_error");
                    }
                    "anthropic" => {
                        assert_eq!(error["type"], "error");
                        assert_eq!(error["error"]["type"], "api_error");
                    }
                    _ => {
                        assert_eq!(error["error"]["code"], 503);
                        assert_eq!(error["error"]["status"], "UNAVAILABLE");
                    }
                }
            }
        }
    }
    assert!(stub.transports().is_empty());
    assert!(fixture.active.snapshot().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn prewarm_unavailability_does_not_hide_formal_log_or_override_ws_http_retries() {
    for enabled_provider in [false, true] {
        let fixture = Fixture::new(true).await;
        let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
        if enabled_provider {
            let a = fixture.provider("A", &upstream.origin(), true);
            fixture.circuit.record_failure(
                a,
                crate::gateway::util::now_unix_seconds() as i64,
                None,
            );
        }
        let (gateway, mut logs) = fixture.start().await;
        let mut body = json!({"type":"response.create","model":"gpt-test","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}],"generate":false});
        let mut warm = connect(&gateway, "unavailable").await.unwrap();
        warm.send(Message::Text(body.to_string())).await.unwrap();
        let error = recv_until(&mut warm, "error").await;
        assert_eq!(error["status"], 503);
        assert!(logs.try_recv().is_err());
        assert!(!fixture.runtime.prefers_http("unavailable"));
        drop(warm);

        body.as_object_mut().unwrap().remove("generate");
        let metadata = json!({"session_id":"unavailable","thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"turn"}).to_string();
        body["client_metadata"] = json!({"x-codex-turn-metadata":metadata});
        let mut socket = connect(&gateway, "unavailable").await.unwrap();
        socket.send(Message::Text(body.to_string())).await.unwrap();
        let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
            [protocol::TURN_STATE_HEADER]
            .as_str()
            .unwrap()
            .to_owned();
        let original = recv_until(&mut socket, "error").await;
        assert_eq!(original["status"], 503);
        let code = if enabled_provider {
            "GW_ALL_PROVIDERS_UNAVAILABLE"
        } else {
            "GW_NO_ENABLED_PROVIDER"
        };
        assert_eq!(original["error"]["code"], code);
        let log = terminal_log(&mut logs).await;
        assert_eq!(log.status, Some(503));
        assert_eq!(log.trace_id, original["trace_id"].as_str().unwrap());
        drop(socket);

        body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
        let mut socket = connect(&gateway, "unavailable").await.unwrap();
        socket.send(Message::Text(body.to_string())).await.unwrap();
        let replay = recv_until(&mut socket, "error").await;
        assert_eq!(replay["error"], original["error"]);
        // A failure with a recovery time is replayed until then; without one the retry
        // is routed again, still with no upstream call.
        let routed_again = |trace: &Value| {
            if enabled_provider {
                assert_eq!(trace, &original["trace_id"]);
            } else {
                assert_ne!(trace, &original["trace_id"]);
            }
        };
        routed_again(&replay["trace_id"]);
        if enabled_provider {
            assert_eq!(
                replay["headers"]["retry-after"],
                replay["retry_after_seconds"].as_u64().unwrap().to_string()
            );
        } else {
            assert_eq!(terminal_log(&mut logs).await.status, Some(503));
        }
        drop(socket);

        let response = reqwest::Client::new()
            .post(format!("{}/v1/responses", gateway.origin()))
            .header("session-id", "unavailable")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let replay: Value = response.json().await.unwrap();
        assert_eq!(replay["error"], original["error"]);
        routed_again(&replay["trace_id"]);
        if !enabled_provider {
            assert_eq!(terminal_log(&mut logs).await.status, Some(503));
        }
        assert!(logs.try_recv().is_err());
        assert!(stub.transports().is_empty());
        assert!(fixture.active.snapshot().is_empty());

        // A different request in the turn (e.g. merged steering) is routed again, not replayed.
        body["input"][0]["content"][0]["text"] = json!("different request");
        let response = reqwest::Client::new()
            .post(format!("{}/v1/responses", gateway.origin()))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let routed: Value = response.json().await.unwrap();
        assert_eq!(routed["error"]["code"], code);
        assert!(stub.transports().is_empty());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn unavailable_tool_delta_on_a_new_socket_returns_the_original_failure() {
    let fixture = Fixture::new(true).await;
    let upstream = CliRecoveryStub::default();
    let server = Server::start(
        Router::new()
            .route("/v1/responses", get(cli_recovery_ws))
            .with_state(upstream.clone()),
    )
    .await;
    let a = fixture.provider("A", &server.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let Message::Text(create) = create_message(Some("unavailable-tool-delta")) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    body["tools"] =
        json!([{"type":"function","name":"shell_command","parameters":{"type":"object"}}]);
    let mut socket = connect(&gateway, "unavailable-tool-delta").await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
        [protocol::TURN_STATE_HEADER]
        .as_str()
        .unwrap()
        .to_owned();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));

    fixture
        .circuit
        .record_failure(a, crate::gateway::util::now_unix_seconds() as i64, None);
    body["input"] = json!([{"type":"function_call_output","call_id":"router_tool","output":"synthetic result"}]);
    body["previous_response_id"] = json!("resp-tool");
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let original = recv_until(&mut socket, "error").await;
    assert_eq!(original["status"], 503);
    assert_eq!(original["error"]["code"], "GW_ALL_PROVIDERS_UNAVAILABLE");
    let log = terminal_log(&mut logs).await;
    assert_eq!(log.status, Some(503));
    assert_eq!(log.trace_id, original["trace_id"].as_str().unwrap());
    drop(socket);

    let mut socket = connect(&gateway, "unavailable-tool-delta").await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let replay = recv_until(&mut socket, "error").await;
    assert_eq!(replay["status"], 503);
    assert_eq!(replay["error"], original["error"]);
    assert_eq!(replay["trace_id"], original["trace_id"]);
    assert!(replay["headers"]["retry-after"].is_string());
    assert_eq!(upstream.calls.lock().unwrap().len(), 1);
    assert!(logs.try_recv().is_err());
    assert!(fixture.active.snapshot().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn retry_after_circuit_reset_routes_again_instead_of_replaying_the_failure() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
    let a = fixture.provider("A", &upstream.origin(), true);
    fixture
        .circuit
        .record_failure(a, crate::gateway::util::now_unix_seconds() as i64, None);
    let (gateway, mut logs) = fixture.start().await;
    let session = "reset-after-unavailable";
    let Message::Text(create) = create_message(Some(session)) else {
        unreachable!()
    };
    let mut body: Value = serde_json::from_str(&create).unwrap();
    body["input"][0]["type"] = json!("message");
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    let nonce = recv_until(&mut socket, "response.metadata").await["headers"]
        [protocol::TURN_STATE_HEADER]
        .as_str()
        .unwrap()
        .to_owned();
    let original = recv_until(&mut socket, "error").await;
    assert_eq!(original["error"]["code"], "GW_ALL_PROVIDERS_UNAVAILABLE");
    assert!(original["retry_after_seconds"].as_u64().unwrap() > 0);
    assert_eq!(terminal_log(&mut logs).await.status, Some(503));
    drop(socket);

    // What GatewayRuntime::circuit_reset_provider does when the user resets the circuit.
    let now = crate::gateway::util::now_unix_seconds() as i64;
    fixture.circuit.reset(a, now);
    fixture.recent_errors.lock().unwrap().clear();
    fixture.runtime.clear_failures();

    // Codex retries the same full request on a new socket within the old Retry-After window.
    body["client_metadata"][protocol::TURN_STATE_HEADER] = json!(nonce);
    let mut socket = connect(&gateway, session).await.unwrap();
    socket.send(Message::Text(body.to_string())).await.unwrap();
    recv_until(&mut socket, "response.completed").await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(stub.transports(), ["ws"]);
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_reports_circuit_failure_with_finite_retries() {
    use std::sync::atomic::AtomicUsize;
    let cli = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("selected Codex executable"),
    );
    let mut reports = Vec::new();
    for (stream, request, retry_after) in [
        (Some(0), Some(0), None),
        (Some(1), Some(0), Some(1)),
        (None, Some(0), Some(1)),
        (Some(0), None, Some(1)),
        (Some(1), None, Some(1)),
        (None, None, Some(1)),
        (Some(0), Some(1), Some(31)),
    ] {
        let fixture = Fixture::new(true).await;
        let (stub, upstream) = Stub::start("A", Behavior::Complete).await;
        let a = fixture.provider("A", &upstream.origin(), true);
        fixture
            .circuit
            .record_failure(a, crate::gateway::util::now_unix_seconds() as i64, None);
        let pipeline = retry_after.map_or_else(GatewayPluginPipeline::empty_shared, |seconds| {
            plugin_tests::unavailable_pipeline(Arc::new(AtomicUsize::new(0)), Some(seconds), None)
        });
        let requests = Arc::new(Mutex::new(Vec::new()));
        let counts = requests.clone();
        let (log_tx, mut logs) = tokio::sync::mpsc::channel(32);
        let router = fixture.router(log_tx, pipeline).layer(axum::middleware::from_fn(move |request: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let counts = counts.clone();
            async move {
                let method = request.method().to_string();
                let path = request.uri().path().to_owned();
                let response = next.run(request).await;
                counts.lock().unwrap().push(json!({"method":method,"path":path,"status":response.status().as_u16(),"trace_id":response.headers().get("x-trace-id").and_then(|value| value.to_str().ok())}));
                response
            }
        }));
        let gateway = Server::start(router).await;
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("work")).unwrap();
        std::fs::create_dir(root.path().join("codex")).unwrap();
        let mut config = cli_test_config(&gateway.origin());
        config = config.replace(
            "stream_max_retries = 1\n",
            &stream.map_or_else(String::new, |count| {
                format!("stream_max_retries = {count}\n")
            }),
        );
        config = config.replace(
            "request_max_retries = 0\n",
            &request.map_or_else(String::new, |count| {
                format!("request_max_retries = {count}\n")
            }),
        );
        std::fs::write(root.path().join("codex/config.toml"), config).unwrap();
        let version = run_cli(isolated_cli_command(&cli, root.path()).arg("--version")).await;
        let started = std::time::Instant::now();
        let output = run_cli_with_timeout(
            isolated_cli_command(&cli, root.path())
                .args([
                    "exec",
                    "--skip-git-repo-check",
                    "--ephemeral",
                    "--ignore-rules",
                    "--json",
                    "--color",
                    "never",
                    "--cd",
                ])
                .arg(root.path().join("work"))
                .arg("Reply OK."),
            Duration::from_secs(180),
        )
        .await;
        let elapsed = started.elapsed().as_secs_f64();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let events: Vec<Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|event| matches!(event["type"].as_str(), Some("error" | "turn.failed")))
            .collect();
        let final_error = events
            .iter()
            .find(|event| event["type"] == "turn.failed")
            .expect("CLI returns a failed turn")
            .to_string();
        assert!(!output.status.success());
        assert!(
            final_error.contains("No available providers")
                && final_error.contains("circuit breakers"),
            "stream={stream:?}, request={request:?}: {final_error}"
        );
        assert!(!stdout.contains("ownership mismatch"));
        assert!(stub.transports().is_empty());
        assert!(fixture.active.snapshot().is_empty());
        let mut rows = Vec::new();
        while let Ok(row) = logs.try_recv() {
            rows.push(row);
        }
        // Reads within Retry-After reuse the first failure; once it passes, each retry is
        // routed again and logged as its own 503, still without an upstream call.
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.status == Some(503)), "{rows:?}");
        if retry_after.is_none() || (stream, request) == (Some(0), Some(0)) {
            assert_eq!(rows.len(), 1, "prewarm and result reads are excluded");
        }
        if retry_after == Some(31) {
            assert!(elapsed >= 30.0, "must cross the pending recovery TTL");
        }
        let requests = requests.lock().unwrap();
        let report = json!({"client":String::from_utf8_lossy(&version.stdout).trim(),"stream_max_retries":stream,"request_max_retries":request,"controlled_retry_after":retry_after,"elapsed_seconds":elapsed,"ws_handshakes":requests.iter().filter(|request| request["status"] == 101).count(),"http_posts":requests.iter().filter(|request| request["method"] == "POST").count(),"upstream_calls":0,"request_log_rows":rows.len(),"requests":*requests,"events":events,"exit":output.status.code()});
        println!("Codex circuit failure: stream={stream:?}, request={request:?}, wait={retry_after:?}, elapsed={elapsed:.2}s, WS={}, HTTP={}, upstream=0, logs={}", report["ws_handshakes"], report["http_posts"], rows.len());
        reports.push(report);
        std::fs::write(
            "/tmp/aio-codex-unavailable-validation.json",
            serde_json::to_string_pretty(&reports).unwrap(),
        )
        .unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Claude CLI executable"]
async fn real_claude_cli_reports_circuit_failure_and_new_request_recovers() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let cli = std::path::PathBuf::from(
        std::env::var_os("AIO_CLAUDE_TEST_CLI").expect("selected Claude executable"),
    );
    let fixture = Fixture::new(true).await;
    let upstream_calls = Arc::new(AtomicUsize::new(0));
    let count = upstream_calls.clone();
    let upstream = Server::start(Router::new().route("/v1/messages", axum::routing::post(move || {
        let count = count.clone();
        async move {
            count.fetch_add(1, Ordering::Relaxed);
            let events = [
                json!({"type":"message_start","message":{"id":"msg_local","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}),
                json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}}),
                json!({"type":"content_block_stop","index":0}),
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}),
                json!({"type":"message_stop"}),
            ];
            let body: String = events.iter().map(|event| format!("event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap())).collect();
            ([(header::CONTENT_TYPE, "text/event-stream")], body)
        }
    }))).await;
    let a =
        fixture.provider_for_cli_with_headers("claude", "A", &upstream.origin(), false, None, None);
    fixture
        .circuit
        .record_failure(a, crate::gateway::util::now_unix_seconds() as i64, None);
    let hooks = Arc::new(AtomicUsize::new(0));
    let (gateway, mut logs) = fixture
        .start_with_pipeline(plugin_tests::unavailable_pipeline(
            hooks.clone(),
            Some(1),
            None,
        ))
        .await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("work")).unwrap();
    std::fs::create_dir(root.path().join("claude")).unwrap();
    let version = run_cli(isolated_cli_command(&cli, root.path()).arg("--version")).await;
    assert!(version.status.success());
    let mut command = isolated_cli_command(&cli, root.path());
    command
        .env("CLAUDE_CONFIG_DIR", root.path().join("claude"))
        .env("ANTHROPIC_API_KEY", "local-test-key")
        .env(
            "ANTHROPIC_BASE_URL",
            format!("{}/claude/_aio/provider/{a}", gateway.origin()),
        )
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .args([
            "--bare",
            "--print",
            "--no-session-persistence",
            "--setting-sources",
            "",
            "--tools",
            "",
            "--model",
            "claude-sonnet-4-5",
            "--output-format",
            "json",
            "Reply OK.",
        ]);
    let started = std::time::Instant::now();
    let output = run_cli_with_timeout(&mut command, Duration::from_secs(240)).await;
    let elapsed = started.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stdout} {stderr}");
    assert!(
        stdout.contains("No available providers") && stdout.contains("circuit breakers"),
        "{stdout} {stderr}"
    );
    assert_eq!(upstream_calls.load(Ordering::Relaxed), 0);
    assert!(fixture.active.snapshot().is_empty());
    let mut rows = 0;
    while let Ok(log) = logs.try_recv() {
        if let Some(status) = log.status {
            assert_eq!(status, 503);
            rows += 1;
        }
    }
    assert!(rows > 0);
    fixture
        .circuit
        .reset(a, crate::gateway::util::now_unix_seconds() as i64);
    let recovered = run_cli_with_timeout(&mut command, Duration::from_secs(40)).await;
    assert!(
        recovered.status.success(),
        "{} {}",
        String::from_utf8_lossy(&recovered.stdout),
        String::from_utf8_lossy(&recovered.stderr)
    );
    assert!(String::from_utf8_lossy(&recovered.stdout).contains("OK"));
    assert!(upstream_calls.load(Ordering::Relaxed) > 0);
    let report = json!({"client":String::from_utf8_lossy(&version.stdout).trim(),"controlled_retry_after":1,"elapsed_seconds":elapsed,"exit":output.status.code(),"error":stdout,"error_hooks":hooks.load(Ordering::Relaxed),"unavailable_upstream_calls":0,"failure_log_rows":rows,"new_request_exit":recovered.status.code(),"new_request_upstream_calls":upstream_calls.load(Ordering::Relaxed)});
    std::fs::write(
        "/tmp/aio-claude-unavailable-validation.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("Claude circuit failure and recovery: {rows} failure logs, rejected upstream=0, recovered upstream={}", upstream_calls.load(Ordering::Relaxed));
}
