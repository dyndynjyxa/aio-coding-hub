//! Interrupts are control frames, including while a create is streaming.

use super::*;
use axum::extract::ws::Message as UpstreamMessage;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Default)]
struct InterruptStub {
    frames: Arc<Mutex<Vec<Value>>>,
    connections: Arc<AtomicUsize>,
    interrupted: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    terminal_first: bool,
    incomplete: bool,
    delay_ack: bool,
    reject: bool,
    wrong_ack_id: bool,
    hold_ack: bool,
}

async fn interrupt_ws(State(stub): State<InterruptStub>, upgrade: WebSocketUpgrade) -> Response {
    stub.connections.fetch_add(1, Ordering::Relaxed);
    upgrade.on_upgrade(move |mut socket| async move {
        while let Some(Ok(UpstreamMessage::Text(text))) = socket.recv().await {
            let body: Value = serde_json::from_str(&text).unwrap();
            let first = {
                let mut frames = stub.frames.lock().unwrap();
                frames.push(body.clone());
                frames.len() == 1
            };
            assert_eq!(body["type"], "response.create");
            let output = answer(events(if first { "first" } else { "next" }), &body, Behavior::Complete);
            let prefix = if first { if body["generate"] == false { 1 } else { 2 } } else { output.len() };
            for event in output.iter().take(prefix) {
                if socket.send(UpstreamMessage::Text(event.to_string())).await.is_err() {
                    return;
                }
            }
            if !first {
                continue;
            }
            let control = loop {
                match socket.recv().await {
                    Some(Ok(UpstreamMessage::Text(text))) => break serde_json::from_str::<Value>(&text).unwrap(),
                    Some(Ok(UpstreamMessage::Ping(bytes))) => { let _ = socket.send(UpstreamMessage::Pong(bytes)).await; }
                    Some(Ok(UpstreamMessage::Pong(_))) => {}
                    _ => return,
                }
            };
            stub.frames.lock().unwrap().push(control.clone());
            stub.interrupted.notify_one();
            let terminal = json!({
                "type":if stub.incomplete { "response.incomplete" } else { "response.completed" },
                "response":{"id":"resp-first","status":if stub.incomplete { "incomplete" } else { "completed" },"output":[],"usage":{"input_tokens":11,"output_tokens":7,"total_tokens":18}}
            });
            if stub.terminal_first && socket.send(UpstreamMessage::Text(terminal.to_string())).await.is_err() {
                return;
            }
            if stub.hold_ack {
                while socket.recv().await.is_some() {}
                return;
            }
            if stub.delay_ack {
                stub.release.notified().await;
            }
            let ack = json!({
                "type":if stub.reject { "response.interrupt.failed" } else { "response.interrupt.accepted" },
                "response_id":if stub.wrong_ack_id { json!("resp-unrelated") } else { control["response_id"].clone() }
            });
            if socket.send(UpstreamMessage::Text(ack.to_string())).await.is_err() {
                return;
            }
            if !stub.terminal_first && socket.send(UpstreamMessage::Text(terminal.to_string())).await.is_err() {
                return;
            }
        }
    }).into_response()
}

async fn interrupt_http(State(stub): State<InterruptStub>) -> Response {
    let output = events("first");
    let prefix: Vec<u8> = output
        .iter()
        .take(2)
        .flat_map(|event| protocol::sse_bytes(event).to_vec())
        .collect();
    let terminal: Vec<u8> = output
        .iter()
        .skip(2)
        .flat_map(|event| protocol::sse_bytes(event).to_vec())
        .collect();
    let stream = futures_util::stream::once(async move { Ok::<_, std::io::Error>(prefix) }).chain(
        futures_util::stream::once(async move {
            stub.release.notified().await;
            Ok(terminal)
        }),
    );
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

fn interrupt(response_id: &str) -> Message {
    Message::Text(json!({"type":"response.interrupt","response_id":response_id,"mode":"discard_partial_items"}).to_string())
}

async fn next_event(
    socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Text(text) => return serde_json::from_str(&text).unwrap(),
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.unwrap(),
                Message::Pong(_) => {}
                other => panic!("expected event, received {other:?}"),
            }
        }
    })
    .await
    .expect("interrupt event must arrive without waiting for generation completion")
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_native_preserves_event_order_and_allows_socket_continuation() {
    for (terminal_first, incomplete) in [(false, false), (true, false), (false, true), (true, true)]
    {
        let fixture = Fixture::new(true).await;
        let stub = InterruptStub {
            terminal_first,
            incomplete,
            delay_ack: terminal_first,
            ..Default::default()
        };
        let upstream = Server::start(
            Router::new()
                .route("/v1/responses", get(interrupt_ws))
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let mut socket = connect_with_user_agent(&gateway, "interrupt-order", None)
            .await
            .unwrap();
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.output_text.delta").await;
        socket.send(interrupt("resp-first")).await.unwrap();
        let terminal_kind = if incomplete {
            "response.incomplete"
        } else {
            "response.completed"
        };
        if terminal_first {
            let terminal = next_event(&mut socket).await;
            assert_eq!(terminal["type"], terminal_kind);
            assert_eq!(terminal["response"]["usage"]["output_tokens"], 7);
            stub.release.notify_one();
        }
        let ack = next_event(&mut socket).await;
        assert_eq!(ack["type"], "response.interrupt.accepted");
        assert_eq!(ack["response_id"], "resp-first");
        if !terminal_first {
            assert_eq!(next_event(&mut socket).await["type"], terminal_kind);
        }
        socket.send(Message::Text(json!({"type":"response.create","model":"gpt-test","previous_response_id":"resp-first","input":[{"role":"user","content":"continue"}]}).to_string())).await.unwrap();
        assert_eq!(
            recv_until(&mut socket, "response.completed").await["response"]["id"],
            "resp-next"
        );
        assert_eq!(stub.connections.load(Ordering::Relaxed), 1);
        let frames = stub.frames.lock().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(
            frames[1],
            json!({"type":"response.interrupt","response_id":"resp-first","mode":"discard_partial_items"})
        );
        drop(frames);
        let log = terminal_log(&mut logs).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.output_tokens, Some(7));
        assert_eq!(terminal_log(&mut logs).await.status, Some(200));
        socket.close(None).await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_idle_and_stale_ids_fail_without_dispatching_or_consuming_create_slot() {
    let fixture = Fixture::new(true).await;
    let stub = InterruptStub::default();
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(interrupt_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "interrupt-invalid", None)
        .await
        .unwrap();
    socket.send(interrupt("resp-stale")).await.unwrap();
    assert_eq!(
        next_event(&mut socket).await["type"],
        "response.interrupt.failed"
    );
    assert_eq!(stub.connections.load(Ordering::Relaxed), 0);
    socket
        .send(Message::Text(
            json!({"type":"response.create","model":"gpt-test"}).to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(
        next_event(&mut socket).await["error"]["code"],
        "invalid_request"
    );
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.send(interrupt("resp-stale")).await.unwrap();
    let failure = next_event(&mut socket).await;
    assert_eq!(failure["type"], "response.interrupt.failed");
    assert_eq!(failure["response_id"], "resp-stale");
    assert_eq!(stub.frames.lock().unwrap().len(), 1);
    socket.send(interrupt("resp-first")).await.unwrap();
    assert_eq!(
        next_event(&mut socket).await["type"],
        "response.interrupt.accepted"
    );
    recv_until(&mut socket, "response.completed").await;
    socket.close(None).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_http_reports_failed_ack_and_keeps_the_original_stream() {
    let fixture = Fixture::new(true).await;
    let stub = InterruptStub::default();
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", axum::routing::post(interrupt_http))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), false);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "interrupt-http", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.send(interrupt("resp-first")).await.unwrap();
    let ack = next_event(&mut socket).await;
    assert_eq!(ack["type"], "response.interrupt.failed");
    assert_eq!(ack["response_id"], "resp-first");
    stub.release.notify_one();
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-first"
    );
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    socket.close(None).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_rejected_by_upstream_does_not_reuse_the_socket() {
    for terminal_first in [false, true] {
        let fixture = Fixture::new(true).await;
        let stub = InterruptStub {
            reject: true,
            terminal_first,
            delay_ack: terminal_first,
            ..Default::default()
        };
        let upstream = Server::start(
            Router::new()
                .route("/v1/responses", get(interrupt_ws))
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let mut socket = connect_with_user_agent(&gateway, "interrupt-rejected", None)
            .await
            .unwrap();
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.output_text.delta").await;
        socket.send(interrupt("resp-first")).await.unwrap();
        if terminal_first {
            let terminal = next_event(&mut socket).await;
            assert_eq!(terminal["type"], "response.completed");
            assert_eq!(terminal["response"]["usage"]["output_tokens"], 7);
            stub.release.notify_one();
        }
        assert_eq!(
            next_event(&mut socket).await["type"],
            "response.interrupt.failed"
        );
        if !terminal_first {
            recv_until(&mut socket, "response.completed").await;
        }
        let log = terminal_log(&mut logs).await;
        assert_eq!(log.status, Some(200));
        assert_eq!(log.output_tokens, Some(7));
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.completed").await;
        assert_eq!(stub.connections.load(Ordering::Relaxed), 2);
        socket.close(None).await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_prewarm_continuation_waits_for_the_terminal_ack() {
    let fixture = Fixture::new(true).await;
    let stub = InterruptStub {
        terminal_first: true,
        delay_ack: true,
        ..Default::default()
    };
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(interrupt_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, _logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "interrupt-prewarm", None)
        .await
        .unwrap();
    let Message::Text(first) = create_message(None) else {
        unreachable!()
    };
    let mut first: Value = serde_json::from_str(&first).unwrap();
    first["generate"] = json!(false);
    socket.send(Message::Text(first.to_string())).await.unwrap();
    recv_until(&mut socket, "response.created").await;
    socket.send(interrupt("resp-first")).await.unwrap();
    assert_eq!(next_event(&mut socket).await["type"], "response.completed");
    stub.release.notify_one();
    assert_eq!(
        next_event(&mut socket).await["type"],
        "response.interrupt.accepted"
    );
    socket.send(Message::Text(json!({"type":"response.create","model":"gpt-test","previous_response_id":"resp-first","input":[{"role":"user","content":"continue"}]}).to_string())).await.unwrap();
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-next"
    );
    assert_eq!(stub.connections.load(Ordering::Relaxed), 1);
    assert_eq!(stub.frames.lock().unwrap().len(), 3);
    socket.close(None).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_wrong_ack_id_closes_without_reusing_the_upstream_socket() {
    for terminal_first in [false, true] {
        let fixture = Fixture::new(true).await;
        let stub = InterruptStub {
            terminal_first,
            delay_ack: terminal_first,
            wrong_ack_id: true,
            ..Default::default()
        };
        let upstream = Server::start(
            Router::new()
                .route("/v1/responses", get(interrupt_ws))
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, _logs) = fixture.start().await;
        let mut socket = connect_with_user_agent(&gateway, "interrupt-wrong-ack", None)
            .await
            .unwrap();
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.output_text.delta").await;
        socket.send(interrupt("resp-first")).await.unwrap();
        if terminal_first {
            assert_eq!(next_event(&mut socket).await["type"], "response.completed");
            stub.release.notify_one();
        } else {
            assert_eq!(next_event(&mut socket).await["type"], "error");
        }
        let closed = tokio::time::timeout(Duration::from_secs(3), socket.next())
            .await
            .unwrap();
        assert!(
            matches!(closed, None | Some(Ok(Message::Close(_))) | Some(Err(_))),
            "a wrong ack must retire the connection: {closed:?}"
        );
        let mut next = connect_with_user_agent(&gateway, "interrupt-wrong-ack", None)
            .await
            .unwrap();
        next.send(create_message(None)).await.unwrap();
        assert_eq!(
            recv_until(&mut next, "response.completed").await["response"]["id"],
            "resp-next"
        );
        assert_eq!(stub.connections.load(Ordering::Relaxed), 2);
        next.close(None).await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_missing_ack_closes_after_delivering_terminal_and_usage() {
    let fixture = Fixture::new(true).await;
    let stub = InterruptStub {
        terminal_first: true,
        hold_ack: true,
        ..Default::default()
    };
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(interrupt_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "interrupt-missing-ack", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.send(interrupt("resp-first")).await.unwrap();
    let terminal = next_event(&mut socket).await;
    assert_eq!(terminal["type"], "response.completed");
    assert_eq!(terminal["response"]["usage"]["output_tokens"], 7);
    let closed = tokio::time::timeout(Duration::from_secs(7), socket.next())
        .await
        .expect("a missing interrupt ack must not leave the connection hanging");
    assert!(
        matches!(closed, None | Some(Ok(Message::Close(_))) | Some(Err(_))),
        "unexpected event after completed response: {closed:?}"
    );
    let log = terminal_log(&mut logs).await;
    assert_eq!(log.status, Some(200));
    assert_eq!(log.output_tokens, Some(7));
    assert!(fixture.active.snapshot().is_empty());
    assert_eq!(stub.frames.lock().unwrap().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn interrupt_waiting_for_ack_releases_request_on_client_disconnect() {
    let fixture = Fixture::new(true).await;
    let stub = InterruptStub {
        hold_ack: true,
        ..Default::default()
    };
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(interrupt_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "interrupt-disconnect", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.send(interrupt("resp-first")).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), stub.interrupted.notified())
        .await
        .expect("interrupt is forwarded while generation is active");
    socket.close(None).await.unwrap();
    drop(socket);
    assert_eq!(terminal_log(&mut logs).await.status, Some(499));
    assert!(fixture.active.snapshot().is_empty());
}
