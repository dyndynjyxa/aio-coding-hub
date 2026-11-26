//! Real Codex steering through AIO, with isolated configuration and synthetic upstreams.

use super::*;
use axum::body::Body;
use axum::extract::ws::Message as ServerMessage;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone, Default)]
struct CliInterruptStub {
    requests: Arc<Mutex<Vec<Value>>>,
    connections: Arc<AtomicUsize>,
    client_interrupts: Arc<AtomicUsize>,
    upstream_interrupts: Arc<AtomicUsize>,
    failed_acks: Arc<AtomicUsize>,
    http_requests: Arc<AtomicUsize>,
    prewarm_done: Arc<tokio::sync::Notify>,
    release_http: Arc<tokio::sync::Notify>,
}

impl CliInterruptStub {
    fn output(&self, body: Value) -> (bool, Vec<Value>) {
        if body["generate"] == false {
            return (
                false,
                vec![
                    json!({"type":"response.completed","response":{"id":"resp-warm","status":"completed","output":[]}}),
                ],
            );
        }
        let first = {
            let mut requests = self.requests.lock().unwrap();
            requests.push(body);
            requests.len() == 1
        };
        let name = if first { "first" } else { "next" };
        let mut output = events(name);
        output.insert(1, json!({"type":"response.output_item.added","output_index":0,"item":{"id":format!("msg-{name}"),"type":"message","role":"assistant","status":"in_progress","content":[]}}));
        output.insert(2, json!({"type":"response.content_part.added","item_id":format!("msg-{name}"),"output_index":0,"content_index":0,"part":{"type":"output_text","text":"","annotations":[]}}));
        (first, output)
    }
}

async fn cli_interrupt_ws(
    State(stub): State<CliInterruptStub>,
    upgrade: WebSocketUpgrade,
) -> Response {
    stub.connections.fetch_add(1, Ordering::Relaxed);
    upgrade.on_upgrade(move |mut socket| async move {
        while let Some(Ok(ServerMessage::Text(text))) = socket.recv().await {
            let body: Value = serde_json::from_str(&text).unwrap();
            if body["type"] == "response.interrupt" {
                assert_eq!(body["response_id"], "resp-first");
                assert_eq!(body["mode"], "discard_partial_items");
                stub.upstream_interrupts.fetch_add(1, Ordering::Relaxed);
                socket.send(ServerMessage::Text(json!({"type":"response.interrupt.accepted","response_id":"resp-first"}).to_string())).await.unwrap();
                for event in events("first").into_iter().skip(2) {
                    socket.send(ServerMessage::Text(event.to_string())).await.unwrap();
                }
                continue;
            }
            assert_eq!(body["type"], "response.create");
            let (first, output) = stub.output(body.clone());
            let output = answer(output, &body, Behavior::Complete);
            let count = if first { 4 } else { output.len() };
            for event in output.into_iter().take(count) {
                socket.send(ServerMessage::Text(event.to_string())).await.unwrap();
            }
        }
    }).into_response()
}

async fn cli_interrupt_http(
    State(stub): State<CliInterruptStub>,
    Json(body): Json<Value>,
) -> Response {
    if body["generate"] != false {
        stub.http_requests.fetch_add(1, Ordering::Relaxed);
    }
    let (first, mut output) = stub.output(body);
    let tail = if first {
        output.split_off(4)
    } else {
        Vec::new()
    };
    let stream = futures_util::stream::iter(output).chain(
        futures_util::stream::once(async move {
            if first {
                stub.release_http.notified().await;
            }
            tail
        })
        .flat_map(futures_util::stream::iter),
    );
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(
            stream.map(|event| Ok::<_, std::convert::Infallible>(protocol::sse_bytes(&event))),
        ),
    )
        .into_response()
}

async fn cli_interrupt_relay(
    State((target, stub)): State<(String, CliInterruptStub)>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let mut request = target.into_client_request().unwrap();
    for (name, value) in &headers {
        if !matches!(name.as_str(), "host" | "connection" | "upgrade")
            && !name.as_str().starts_with("sec-websocket-")
        {
            request.headers_mut().insert(name.clone(), value.clone());
        }
    }
    let (mut upstream, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(connection) => connection,
        Err(Error::Http(response)) => return response.status().into_response(),
        Err(error) => panic!("connect to the local test gateway: {error}"),
    };
    upgrade.on_upgrade(move |mut downstream| async move {
        loop {
            tokio::select! {
                message = downstream.recv() => {
                    let Some(Ok(message)) = message else { break; };
                    let message = match message {
                        ServerMessage::Text(text) => {
                            if serde_json::from_str::<Value>(&text).unwrap()["type"] == "response.interrupt" {
                                stub.client_interrupts.fetch_add(1, Ordering::Relaxed);
                            }
                            Message::Text(text)
                        }
                        ServerMessage::Binary(bytes) => Message::Binary(bytes),
                        ServerMessage::Ping(bytes) => Message::Ping(bytes),
                        ServerMessage::Pong(bytes) => Message::Pong(bytes),
                        ServerMessage::Close(_) => break,
                    };
                    if upstream.send(message).await.is_err() { break; }
                }
                message = upstream.next() => {
                    let Some(Ok(message)) = message else { break; };
                    let message = match message {
                        Message::Text(text) => {
                            let event: Value = serde_json::from_str(&text).unwrap();
                            if event["type"] == "response.interrupt.failed" {
                                stub.failed_acks.fetch_add(1, Ordering::Relaxed);
                                stub.release_http.notify_one();
                            }
                            if event["type"] == "response.completed" && event["response"]["id"] == "resp-warm" {
                                stub.prewarm_done.notify_one();
                            }
                            ServerMessage::Text(text)
                        }
                        Message::Binary(bytes) => ServerMessage::Binary(bytes),
                        Message::Ping(bytes) => ServerMessage::Ping(bytes),
                        Message::Pong(bytes) => ServerMessage::Pong(bytes),
                        Message::Close(_) | Message::Frame(_) => break,
                    };
                    if downstream.send(message).await.is_err() { break; }
                }
            }
        }
    }).into_response()
}

async fn cli_interrupt_http_relay(
    State((target, _)): State<(String, CliInterruptStub)>,
    mut headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    headers.remove(header::HOST);
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(target.replacen("ws://", "http://", 1))
        .headers(headers)
        .body(body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let mut forwarded = Response::new(Body::from_stream(response.bytes_stream()));
    *forwarded.status_mut() = status;
    *forwarded.headers_mut() = headers;
    forwarded
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires an explicitly selected real Codex CLI executable"]
async fn real_codex_cli_lite_steering_continues_through_ws_and_http() {
    let cli = std::path::PathBuf::from(
        std::env::var_os("AIO_CODEX_WS_TEST_CLI").expect("set absolute AIO_CODEX_WS_TEST_CLI"),
    );
    assert!(cli.is_absolute() && cli.is_file());
    for native_ws in [true, false] {
        let fixture = Fixture::new(true).await;
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("work")).unwrap();
        std::fs::create_dir(root.path().join("codex")).unwrap();
        let version = run_cli(isolated_cli_command(&cli, root.path()).arg("--version")).await;
        assert!(version.status.success());
        let catalog =
            run_cli(isolated_cli_command(&cli, root.path()).args(["debug", "models", "--bundled"]))
                .await;
        assert!(catalog.status.success());
        let catalog: Value = serde_json::from_slice(&catalog.stdout).unwrap();
        let mut model = catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|model| model["slug"] == "gpt-5.4" || model["slug"] == "gpt-5.5")
            .expect("bundled model for the isolated protocol probe")
            .clone();
        model["slug"] = json!("gpt-5.4");
        model["use_responses_lite"] = json!(true);
        let mut http_model = model.clone();
        http_model["slug"] = json!("gpt-probe-http");
        let catalog_path = root.path().join("codex/probe-models.json");
        std::fs::write(
            &catalog_path,
            json!({"models":[model,http_model]}).to_string(),
        )
        .unwrap();

        let stub = CliInterruptStub::default();
        let upstream = Server::start(
            Router::new()
                .route(
                    "/v1/responses",
                    get(cli_interrupt_ws).post(cli_interrupt_http),
                )
                .with_state(stub.clone()),
        )
        .await;
        let provider_id = fixture.provider("A", &upstream.origin(), true);
        let (gateway, _logs) = fixture.start().await;
        let relay = Server::start(
            Router::new()
                .route(
                    "/v1/responses",
                    get(cli_interrupt_relay).post(cli_interrupt_http_relay),
                )
                .with_state((format!("ws://{}/v1/responses", gateway.addr), stub.clone())),
        )
        .await;
        std::fs::write(
            root.path().join("codex/config.toml"),
            format!(
                "model_catalog_json = {}\n{}",
                toml::Value::String(catalog_path.to_string_lossy().into_owned()),
                cli_test_config(&relay.origin())
            ),
        )
        .unwrap();
        let mut child = isolated_cli_command(&cli, root.path())
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
            let model = if native_ws {
                "gpt-5.4"
            } else {
                // HTTP prewarm forces client HTTP; changing model after WS prewarm clears its reference.
                stub.prewarm_done.notified().await;
                fixture.provider_with_headers("A", &upstream.origin(), false, Some(provider_id), None);
                "gpt-probe-http"
            };
            stdin.write_all(format!("{}\n", json!({"id":3,"method":"turn/start","params":{"threadId":thread_id,"model":model,"input":[{"type":"text","text":"Reply with a short synthetic answer.","text_elements":[]}]}})).as_bytes()).await.unwrap();
            let turn = cli_message_until(&mut lines, |message| message["id"] == 3).await;
            let turn_id = turn["result"]["turn"]["id"].as_str().unwrap();
            cli_message_until(&mut lines, |message| message["method"] == "item/agentMessage/delta").await;
            stdin.write_all(format!("{}\n", json!({"id":4,"method":"turn/steer","params":{"threadId":thread_id,"expectedTurnId":turn_id,"input":[{"type":"text","text":"Use the second synthetic instruction instead.","text_elements":[]}]}})).as_bytes()).await.unwrap();
            cli_message_until(&mut lines, |message| message["id"] == 4).await;
            let completed = cli_message_until(&mut lines, |message| message["method"] == "turn/completed").await;
            assert_eq!(completed["params"]["turn"]["status"], "completed");
        }).await;
        #[cfg(unix)]
        crate::shared::process::terminate_unix_process_group(pid);
        #[cfg(windows)]
        crate::shared::process::terminate_windows_process_tree(pid);
        let _ = child.kill().await;
        let _ = child.wait().await;
        result.expect("steering completed through the real AIO router");
        assert_eq!(stub.client_interrupts.load(Ordering::Relaxed), 1);
        assert_eq!(
            stub.upstream_interrupts.load(Ordering::Relaxed),
            usize::from(native_ws)
        );
        assert_eq!(
            stub.failed_acks.load(Ordering::Relaxed),
            usize::from(!native_ws)
        );
        assert_eq!(stub.connections.load(Ordering::Relaxed), 1);
        assert_eq!(
            stub.http_requests.load(Ordering::Relaxed),
            if native_ws { 0 } else { 2 }
        );
        let requests = stub.requests.lock().unwrap();
        assert_eq!(requests.len(), 2, "steering runs each generation once");
        if native_ws {
            assert_eq!(requests[1]["previous_response_id"], "resp-first");
        }
        println!(
            "{}: lite steering through AIO {} completed, interrupt=1, generations=2",
            String::from_utf8_lossy(&version.stdout).trim(),
            if native_ws { "WS" } else { "HTTP" }
        );
    }
}
