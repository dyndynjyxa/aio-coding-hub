//! A create arriving before the current terminal event waits on the same connection.

use super::*;
use axum::extract::ws::Message as UpstreamMessage;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Default)]
struct PendingStub {
    calls: Arc<Mutex<Vec<Value>>>,
    connections: Arc<AtomicUsize>,
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    fail: bool,
}

async fn pending_ws(State(stub): State<PendingStub>, upgrade: WebSocketUpgrade) -> Response {
    stub.connections.fetch_add(1, Ordering::Relaxed);
    upgrade.on_upgrade(move |mut socket| async move {
        while let Some(Ok(UpstreamMessage::Text(text))) = socket.recv().await {
            let body: Value = serde_json::from_str(&text).unwrap();
            let first = {
                let mut calls = stub.calls.lock().unwrap();
                calls.push(body.clone());
                calls.len() == 1
            };
            let output = if body["generate"] == false {
                vec![json!({"type":"response.completed","response":{"id":"resp-first","status":"completed","output":[]}})]
            } else {
                answer(events(if first { "first" } else { "next" }), &body, Behavior::Complete)
            };
            let prefix = if first && body["generate"] != false { 2 } else { 0 };
            for event in output.iter().take(prefix) {
                socket.send(UpstreamMessage::Text(event.to_string())).await.unwrap();
            }
            if first {
                stub.started.notify_one();
                loop {
                    tokio::select! {
                        biased;
                        message = socket.recv() => match message {
                            Some(Ok(UpstreamMessage::Ping(bytes))) => {
                                socket.send(UpstreamMessage::Pong(bytes)).await.unwrap();
                            }
                            Some(Ok(UpstreamMessage::Pong(_))) => {}
                            Some(Ok(UpstreamMessage::Text(_))) => panic!("next create reached upstream before the first terminal"),
                            _ => return,
                        },
                        _ = stub.release.notified() => break,
                    }
                }
                if stub.fail {
                    socket.send(UpstreamMessage::Text(json!({"type":"error","error":{"code":"server_error","message":"failed after output"}}).to_string())).await.unwrap();
                    return;
                }
            }
            for event in output.iter().skip(prefix) {
                if socket.send(UpstreamMessage::Text(event.to_string())).await.is_err() {
                    return;
                }
            }
        }
    }).into_response()
}

fn continuation() -> Message {
    Message::Text(
        json!({
            "type":"response.create", "model":"gpt-test", "previous_response_id":"resp-first",
            "input":[{"role":"user","content":"continue"}]
        })
        .to_string(),
    )
}

async fn ping_barrier(socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) {
    let payload = b"pending-create-read".to_vec();
    socket.send(Message::Ping(payload.clone())).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Pong(bytes) if bytes == payload => break,
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.unwrap(),
                other => panic!("unexpected frame before pending-create barrier: {other:?}"),
            }
        }
    })
    .await
    .expect("pending create is read before releasing the first response");
}

#[tokio::test(flavor = "current_thread")]
async fn pending_create_waits_for_prewarm_and_streaming_generation_completion() {
    for prewarm in [true, false] {
        let fixture = Fixture::new(true).await;
        let stub = PendingStub::default();
        let upstream = Server::start(
            Router::new()
                .route("/v1/responses", get(pending_ws))
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, _logs) = fixture.start().await;
        let mut socket = connect_with_user_agent(&gateway, "pending-create", None)
            .await
            .unwrap();
        let Message::Text(first) = create_message(None) else {
            unreachable!()
        };
        let mut first: Value = serde_json::from_str(&first).unwrap();
        if prewarm {
            first["generate"] = json!(false);
        }
        socket.send(Message::Text(first.to_string())).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), stub.started.notified())
            .await
            .unwrap();
        if !prewarm {
            recv_until(&mut socket, "response.output_text.delta").await;
        }
        socket.send(continuation()).await.unwrap();
        ping_barrier(&mut socket).await;
        assert_eq!(stub.calls.lock().unwrap().len(), 1);
        stub.release.notify_one();
        assert_eq!(
            recv_until(&mut socket, "response.completed").await["response"]["id"],
            "resp-first"
        );
        assert_eq!(
            recv_until(&mut socket, "response.completed").await["response"]["id"],
            "resp-next"
        );
        ping_barrier(&mut socket).await;
        let calls = stub.calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "each create is sent exactly once");
        assert_eq!(calls[1]["previous_response_id"], "resp-first");
        assert_eq!(stub.connections.load(Ordering::Relaxed), 1);
        drop(calls);
        socket.close(None).await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn pending_create_slot_limit_preserves_the_active_and_queued_generations() {
    let fixture = Fixture::new(true).await;
    let stub = PendingStub::default();
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(pending_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "pending-limit", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    socket.send(continuation()).await.unwrap();
    ping_barrier(&mut socket).await;
    socket.send(create_message(None)).await.unwrap();
    let error = recv_until(&mut socket, "error").await;
    assert_eq!(error["error"]["code"], "response_in_progress");
    assert_eq!(stub.calls.lock().unwrap().len(), 1);
    stub.release.notify_one();
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-first"
    );
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-next"
    );
    ping_barrier(&mut socket).await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(stub.calls.lock().unwrap().len(), 2);
    assert!(fixture.active.snapshot().is_empty());
    socket.close(None).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn pending_create_is_discarded_after_failure_disconnect_or_disable() {
    for stop in ["failure", "disconnect", "disable"] {
        let fixture = Fixture::new(true).await;
        let stub = PendingStub {
            fail: stop == "failure",
            ..Default::default()
        };
        let upstream = Server::start(
            Router::new()
                .route("/v1/responses", get(pending_ws))
                .with_state(stub.clone()),
        )
        .await;
        fixture.provider("A", &upstream.origin(), true);
        let (gateway, mut logs) = fixture.start().await;
        let mut socket = connect_with_user_agent(&gateway, "pending-stop", None)
            .await
            .unwrap();
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.output_text.delta").await;
        socket.send(continuation()).await.unwrap();
        ping_barrier(&mut socket).await;
        if stop == "disconnect" {
            socket.close(None).await.unwrap();
            drop(socket);
            assert_eq!(terminal_log(&mut logs).await.status, Some(499));
        } else {
            if stop == "disable" {
                fixture.runtime.set_enabled(false);
            }
            stub.release.notify_one();
            recv_until(
                &mut socket,
                if stop == "failure" {
                    "error"
                } else {
                    "response.completed"
                },
            )
            .await;
            let closed = tokio::time::timeout(Duration::from_secs(3), socket.next())
                .await
                .unwrap();
            assert!(
                matches!(closed, None | Some(Ok(Message::Close(_))) | Some(Err(_))),
                "pending create must be dropped on {stop}: {closed:?}"
            );
            let log = terminal_log(&mut logs).await;
            assert_eq!(log.status, Some(if stop == "failure" { 502 } else { 200 }));
        }
        assert_eq!(
            stub.calls.lock().unwrap().len(),
            1,
            "pending create dispatched on {stop}"
        );
        assert_eq!(stub.connections.load(Ordering::Relaxed), 1);
        assert!(fixture.active.snapshot().is_empty());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn pending_create_respects_and_releases_the_shared_buffer_budget() {
    let fixture = Fixture::new(true).await;
    let stub = PendingStub::default();
    let upstream = Server::start(
        Router::new()
            .route("/v1/responses", get(pending_ws))
            .with_state(stub.clone()),
    )
    .await;
    fixture.provider("A", &upstream.origin(), true);
    let (gateway, mut logs) = fixture.start().await;
    let mut socket = connect_with_user_agent(&gateway, "pending-buffer-budget", None)
        .await
        .unwrap();
    socket.send(create_message(None)).await.unwrap();
    recv_until(&mut socket, "response.output_text.delta").await;
    let mut occupied = Vec::new();
    while let Ok(permit) = fixture.runtime.reserve_raw_buffers() {
        occupied.push(permit);
    }
    assert!(!occupied.is_empty());
    socket.send(continuation()).await.unwrap();
    let error = recv_until(&mut socket, "error").await;
    assert_eq!(error["error"]["code"], "resource_exhausted");
    assert_eq!(stub.calls.lock().unwrap().len(), 1);

    drop(occupied.pop().unwrap());
    socket.send(continuation()).await.unwrap();
    ping_barrier(&mut socket).await;
    assert!(fixture.runtime.reserve_raw_buffers().is_err());
    stub.release.notify_one();
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-first"
    );
    assert_eq!(
        recv_until(&mut socket, "response.completed").await["response"]["id"],
        "resp-next"
    );
    ping_barrier(&mut socket).await;
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(terminal_log(&mut logs).await.status, Some(200));
    assert_eq!(stub.calls.lock().unwrap().len(), 2);
    assert!(fixture.runtime.reserve_raw_buffers().is_ok());
    socket.close(None).await.unwrap();
}
