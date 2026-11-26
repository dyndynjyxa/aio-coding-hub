//! Per-attempt upstream WebSocket ownership; provider selection stays in the proxy loop.

use super::protocol::{self, TURN_STATE_HEADER};
use super::state::{Connection, InterruptTarget, RequestState, ReusableSocket, WsProbe};
use super::upstream::{self, ConnectError, UpstreamWebSocket};
use crate::gateway::streams::{UpstreamResponse, UpstreamStreamError};
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

pub(in crate::gateway) enum SendOutcome {
    Response(UpstreamResponse),
    Rejected(UpstreamResponse),
    Transport(&'static str),
    ContextLost {
        require_http: bool,
        reason: &'static str,
    },
    Local(&'static str),
    Http(&'static str),
}

pub(in crate::gateway) fn socket_key(
    provider: i64,
    url: &reqwest::Url,
    headers: &HeaderMap,
    custom_headers: &HeaderMap,
    epoch: u64,
    model: Option<&str>,
    client_generation: u64,
) -> String {
    let mut hash = Sha256::new();
    hash.update(provider.to_be_bytes());
    hash.update(epoch.to_be_bytes());
    hash.update(client_generation.to_be_bytes());
    hash.update(url.as_str().as_bytes());
    hash.update(model.unwrap_or_default().as_bytes());
    for name in [
        header::AUTHORIZATION.as_str(),
        "chatgpt-account-id",
        "openai-organization",
        "openai-project",
        "openai-beta",
        "originator",
    ] {
        if let Some(value) = headers.get(name) {
            hash.update(name.as_bytes());
            hash.update(value.as_bytes());
        }
    }
    let mut names: Vec<_> = custom_headers.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for name in names {
        let name_bytes = name.as_str().as_bytes();
        hash.update((name_bytes.len() as u64).to_be_bytes());
        hash.update(name_bytes);
        let values: Vec<_> = headers.get_all(name).iter().collect();
        hash.update((values.len() as u64).to_be_bytes());
        for value in values {
            hash.update((value.as_bytes().len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gateway) async fn send(
    connection: Arc<Connection>,
    request: Option<&RequestState>,
    provider_id: i64,
    supports_ws: bool,
    url: reqwest::Url,
    mut headers: HeaderMap,
    custom_headers: &HeaderMap,
    body: Bytes,
    deadline: Option<std::time::Instant>,
) -> SendOutcome {
    let mut payload: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(object)) => Value::Object(object),
        _ => return SendOutcome::Local("invalid Responses request body"),
    };
    let previous = payload
        .get("previous_response_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let key = socket_key(
        provider_id,
        &url,
        &headers,
        custom_headers,
        connection.runtime.epoch(),
        payload.get("model").and_then(Value::as_str),
        crate::gateway::http_client::generation(),
    );
    let idle = connection
        .upstream
        .lock_or_recover()
        .take()
        .filter(|cached| cached.key == key);
    let kept = idle.is_some();
    let mut reusable = match idle {
        Some(idle) => idle.resume().await,
        None => None,
    };
    let reused = reusable.is_some();
    let allow_ws = supports_ws
        && request.is_none_or(|request| {
            request.client_ws && {
                let generation = request.generation.lock_or_recover();
                !generation.budget.http_only
                    && !generation.budget.http_provider_ids.contains(&provider_id)
            }
        });
    if previous.is_some() {
        let continuation = connection.continuation.lock_or_recover();
        let warm = connection.prewarm.lock_or_recover();
        let valid = continuation.as_ref().is_some_and(|current| {
            current.provider_id == provider_id
                && current.upstream_ws
                && Some(current.response_id.as_str()) == previous.as_deref()
        }) || warm.as_ref().is_some_and(|warm| {
            warm.provider_id == provider_id
                && Some(warm.response_id.as_str()) == previous.as_deref()
        });
        if !valid || !allow_ws || reusable.is_none() {
            return SendOutcome::ContextLost {
                // While the socket cools the full resend goes over HTTP; staying on WS
                // would fail every continuation until the cooldown ends.
                require_http: !supports_ws || connection.runtime.cooling(&key),
                reason: if kept && !reused {
                    "ws_reused_socket_lost"
                } else {
                    "previous_response_not_found"
                },
            };
        }
    }
    if !allow_ws {
        let reason = if !supports_ws {
            "provider_http_only"
        } else {
            "session_http_only"
        };
        return SendOutcome::Http(reason);
    }
    let mut probe = None;
    if reusable.is_none() {
        probe = match connection.runtime.try_ws_probe(&key) {
            Ok(probe) => probe,
            Err(()) => return SendOutcome::Http("ws_cooldown_skip"),
        };
        if let Some(request) = request {
            if !request
                .generation
                .lock_or_recover()
                .budget
                .tried_ws
                .insert(provider_id)
            {
                return SendOutcome::Http("ws_budget_exhausted");
            }
        }
        headers.remove(TURN_STATE_HEADER);
        let connect_cap = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let deadline = deadline
            .map(tokio::time::Instant::from_std)
            .map_or(connect_cap, |deadline| deadline.min(connect_cap));
        let connected = match upstream::connect(url.clone(), headers.clone(), deadline).await {
            Ok(connected) => connected,
            Err(ConnectError::Rejected(response)) => {
                let outcome = classify_rejection(UpstreamResponse::from(*response), deadline).await;
                if matches!(outcome, SendOutcome::Transport(_)) {
                    connection.runtime.cool(key);
                }
                return outcome;
            }
            Err(ConnectError::Timeout) => {
                connection.runtime.cool(key);
                return SendOutcome::Transport("ws_connect_timeout");
            }
            Err(ConnectError::Client(_)) => {
                return SendOutcome::Local("upstream proxy configuration is unavailable")
            }
            Err(_) => {
                connection.runtime.cool(key);
                return SendOutcome::Transport("ws_connect_failed");
            }
        };
        reusable = Some(ReusableSocket {
            key: key.clone(),
            provider_id,
            turn_state: connected
                .headers
                .get(TURN_STATE_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            connection: connected,
        });
    }
    let mut reusable = reusable.unwrap();
    payload.as_object_mut().unwrap().remove("stream");
    payload["type"] = Value::String("response.create".into());
    if let Some(metadata) = payload
        .get_mut("client_metadata")
        .and_then(Value::as_object_mut)
    {
        metadata.remove(TURN_STATE_HEADER);
        if let Some(turn_state) = &reusable.turn_state {
            metadata.insert(TURN_STATE_HEADER.into(), Value::String(turn_state.clone()));
        }
    }
    let serialized = payload.to_string();
    let send_cap = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let send_deadline = deadline
        .map(tokio::time::Instant::from_std)
        .map_or(send_cap, |deadline| deadline.min(send_cap));
    let sent = matches!(
        tokio::time::timeout_at(
            send_deadline,
            reusable.connection.socket.send(Message::Text(serialized))
        )
        .await,
        Ok(Ok(()))
    );
    if !sent && !reused {
        connection.runtime.cool(key);
        return SendOutcome::Transport("ws_send_failed");
    }
    let prelude = if !reused {
        Prelude::Ready(VecDeque::new())
    } else if sent {
        let socket = &mut reusable.connection.socket;
        let deadline = deadline.map(tokio::time::Instant::from_std);
        read_prelude(socket, deadline).await
    } else {
        Prelude::Lost
    };
    let prelude = match prelude {
        Prelude::Ready(frames) => frames,
        // A socket kept between turns can end without fault of the endpoint.
        Prelude::Lost if previous.is_none() => {
            drop(reusable);
            return Box::pin(send(
                connection,
                request,
                provider_id,
                supports_ws,
                url,
                headers,
                custom_headers,
                body,
                deadline,
            ))
            .await;
        }
        Prelude::Lost => {
            return SendOutcome::ContextLost {
                require_http: false,
                reason: "ws_reused_socket_lost",
            }
        }
    };
    if let Some(request) = request {
        request.generation.lock_or_recover().upstream_ws = true;
    }
    let response_headers = reusable.connection.headers.clone();
    let (sender, controls) = tokio::sync::mpsc::channel(1);
    *connection.interrupt.lock_or_recover() = Some(InterruptTarget {
        response_id: None,
        sender,
    });
    let stream = futures_util::stream::unfold(
        Some(ResponseStream {
            connection,
            reusable,
            prelude,
            probe,
            controls,
            interruption: None,
            terminal: None,
            reusable_after_interrupt: true,
        }),
        |state| async move { state?.next().await },
    );
    let mut headers = response_headers;
    headers.remove(header::CONTENT_LENGTH);
    headers.remove(header::CONTENT_ENCODING);
    headers.remove(TURN_STATE_HEADER);
    headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    SendOutcome::Response(UpstreamResponse::new_ws(headers, stream))
}

enum Prelude {
    /// Frames read ahead of the reply, handed on first.
    Ready(VecDeque<String>),
    /// The kept socket ended before it answered.
    Lost,
}

/// Frames that may come ahead of a reply and say nothing of it.
const NEUTRAL_EVENTS: [&str; 6] = [
    "codex.rate_limits",
    "codex.response.metadata",
    "response.metadata",
    "responsesapi.websocket_timing",
    "response.interrupt.accepted",
    "response.interrupt.failed",
];
const MAX_PRELUDE_FRAMES: usize = 8;

/// Read a kept socket's first frames before anything is handed on: it may have
/// ended between turns.
async fn read_prelude(
    socket: &mut UpstreamWebSocket,
    deadline: Option<tokio::time::Instant>,
) -> Prelude {
    let mut frames = VecDeque::new();
    while frames.len() < MAX_PRELUDE_FRAMES {
        let message = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, socket.next()).await {
                Ok(message) => message,
                // The gate reports the deadline.
                Err(_) => break,
            },
            None => socket.next().await,
        };
        let text = match message {
            Some(Ok(Message::Text(text))) => text,
            // tungstenite answers pings while reading.
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            _ => return Prelude::Lost,
        };
        let step = serde_json::from_str(&text).map_or(Step::Reply, |event| prelude_step(&event));
        frames.push_back(text);
        match step {
            Step::Neutral => {}
            Step::Reply => break,
            Step::Lost => return Prelude::Lost,
        }
    }
    Prelude::Ready(frames)
}

#[derive(Debug, PartialEq)]
enum Step {
    Neutral,
    Reply,
    Lost,
}

fn prelude_step(event: &Value) -> Step {
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if NEUTRAL_EVENTS.contains(&kind) {
        return Step::Neutral;
    }
    if kind == "error"
        && event.pointer("/error/code").and_then(Value::as_str)
            == Some("websocket_connection_limit_reached")
    {
        return Step::Lost;
    }
    Step::Reply
}

struct ResponseStream {
    connection: Arc<Connection>,
    reusable: ReusableSocket,
    prelude: VecDeque<String>,
    probe: Option<WsProbe>,
    controls: tokio::sync::mpsc::Receiver<String>,
    // The bool records whether the upstream acknowledgement is still pending.
    interruption: Option<(String, bool)>,
    terminal: Option<tokio::time::Instant>,
    reusable_after_interrupt: bool,
}

impl ResponseStream {
    async fn interrupt(&mut self, response_id: String) -> Result<(), UpstreamStreamError> {
        let event = serde_json::json!({
            "type": "response.interrupt", "response_id": response_id,
            "mode": "discard_partial_items",
        });
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.reusable
                .connection
                .socket
                .send(Message::Text(event.to_string())),
        )
        .await
        {
            Ok(Ok(())) => {
                self.interruption = Some((response_id, true));
                Ok(())
            }
            _ => Err(protocol_error("WebSocket interrupt send failed")),
        }
    }

    fn finish(mut self, bytes: Bytes) -> (Result<Bytes, UpstreamStreamError>, Option<Self>) {
        if self.reusable_after_interrupt {
            if let Some(probe) = self.probe.take() {
                probe.succeeded();
            }
            *self.connection.upstream.lock_or_recover() = Some(self.reusable.park());
        }
        (Ok(bytes), None)
    }

    async fn next(mut self) -> Option<(Result<Bytes, UpstreamStreamError>, Option<Self>)> {
        loop {
            let ack_deadline = self.terminal.unwrap_or_else(tokio::time::Instant::now);
            let message = if let Some(text) = self.prelude.pop_front() {
                Some(Ok(Message::Text(text)))
            } else {
                tokio::select! {
                    biased;
                    control = self.controls.recv(), if self.interruption.is_none() && (!self.controls.is_closed() || !self.controls.is_empty()) => {
                        if let Some(response_id) = control {
                            if let Err(error) = self.interrupt(response_id).await {
                                return Some((Err(error), None));
                            }
                        }
                        continue;
                    },
                    _ = tokio::time::sleep_until(ack_deadline), if self.terminal.is_some() => {
                        return Some((Err(protocol_error("WebSocket interrupt acknowledgement timed out")), None));
                    },
                    message = self.reusable.connection.socket.next() => message,
                }
            };
            match message {
                Some(Ok(Message::Text(text))) => {
                    let mut event: Value = match serde_json::from_str(&text) {
                        Ok(event) => event,
                        Err(_) => {
                            self.connection.runtime.cool(self.reusable.key.clone());
                            return Some((
                                Err(protocol_error("invalid upstream WebSocket JSON")),
                                None,
                            ));
                        }
                    };
                    let kind = match protocol::event_kind(&event) {
                        Ok(kind) => kind,
                        Err(message) => {
                            self.connection.runtime.cool(self.reusable.key.clone());
                            return Some((Err(protocol_error(message)), None));
                        }
                    };
                    match event.get("type").and_then(Value::as_str) {
                        Some("response.created") => {
                            if let Some(target) =
                                self.connection.interrupt.lock_or_recover().as_mut()
                            {
                                target.response_id = event
                                    .pointer("/response/id")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned);
                            }
                        }
                        Some(
                            ack @ ("response.interrupt.accepted" | "response.interrupt.failed"),
                        ) => match self.interruption.as_mut() {
                            Some((id, pending))
                                if *pending
                                    && event.get("response_id").and_then(Value::as_str)
                                        == Some(id.as_str()) =>
                            {
                                *pending = false;
                                self.reusable_after_interrupt =
                                    ack == "response.interrupt.accepted";
                            }
                            _ => {
                                return Some((
                                    Err(protocol_error(
                                        "Unexpected WebSocket interrupt acknowledgement",
                                    )),
                                    None,
                                ))
                            }
                        },
                        _ => {}
                    }
                    if let Some(headers) = event.get_mut("headers").and_then(Value::as_object_mut) {
                        if let Some(value) = headers
                            .remove(TURN_STATE_HEADER)
                            .and_then(|value| value.as_str().map(str::to_owned))
                        {
                            self.reusable.turn_state = Some(value);
                        }
                    }
                    let bytes = protocol::sse_bytes(&event);
                    if kind == protocol::EventKind::Failed {
                        return Some((Ok(bytes), None));
                    }
                    if matches!(
                        kind,
                        protocol::EventKind::Completed | protocol::EventKind::Incomplete
                    ) {
                        // Seal the queue before caching: an interrupt already accepted by
                        // ingress must reach upstream even when completion wins the read race.
                        self.controls.close();
                        if let Ok(response_id) = self.controls.try_recv() {
                            if let Err(error) = self.interrupt(response_id).await {
                                return Some((Err(error), None));
                            }
                        }
                        if self
                            .interruption
                            .as_ref()
                            .is_some_and(|(_, pending)| *pending)
                        {
                            self.terminal = Some(
                                tokio::time::Instant::now() + std::time::Duration::from_secs(5),
                            );
                            return Some((Ok(bytes), Some(self)));
                        }
                        return Some(self.finish(bytes));
                    }
                    if self.terminal.is_some()
                        && self
                            .interruption
                            .as_ref()
                            .is_some_and(|(_, pending)| !*pending)
                    {
                        return Some(self.finish(bytes));
                    }
                    return Some((Ok(bytes), Some(self)));
                }
                Some(Ok(Message::Ping(bytes))) => {
                    if self
                        .reusable
                        .connection
                        .socket
                        .send(Message::Pong(bytes))
                        .await
                        .is_err()
                    {
                        self.connection.runtime.cool(self.reusable.key.clone());
                        return Some((Err(protocol_error("WebSocket pong failed")), None));
                    }
                }
                Some(Ok(Message::Pong(_))) => {}
                _ => {
                    self.connection.runtime.cool(self.reusable.key.clone());
                    return Some((
                        Err(protocol_error("WebSocket closed before terminal response")),
                        None,
                    ));
                }
            }
        }
    }
}

/// Inspect only explicit capability errors. Retain every consumed byte for the
/// existing status/body classifier when the rejection has another cause.
async fn classify_rejection(
    mut response: UpstreamResponse,
    deadline: tokio::time::Instant,
) -> SendOutcome {
    let status = response.status();
    if matches!(status.as_u16(), 405 | 426 | 501) {
        return SendOutcome::Transport("ws_upgrade_unsupported");
    }
    if !matches!(status.as_u16(), 400 | 404) {
        return SendOutcome::Rejected(response);
    }
    let mut consumed = Vec::new();
    let mut sample = Vec::new();
    const ERROR_SAMPLE_LIMIT: usize = 16 * 1024;
    while sample.len() < ERROR_SAMPLE_LIMIT {
        match tokio::time::timeout_at(deadline, response.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                let remaining = ERROR_SAMPLE_LIMIT - sample.len();
                sample.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                consumed.push(Ok(chunk));
                if let Ok(error) = serde_json::from_slice::<Value>(&sample) {
                    if matches!(
                        error.pointer("/error/code").and_then(Value::as_str),
                        Some("websocket_not_supported" | "unsupported_websocket_protocol")
                    ) {
                        return SendOutcome::Transport("ws_upgrade_unsupported");
                    }
                    break;
                }
            }
            Ok(Err(error)) => {
                consumed.push(Err(error));
                break;
            }
            Ok(Ok(None)) | Err(_) => break,
        }
    }
    let headers = response.headers().clone();
    let stream = futures_util::stream::iter(consumed).chain(response.bytes_stream());
    SendOutcome::Rejected(UpstreamResponse::new(status, headers, stream))
}

fn protocol_error(message: &str) -> UpstreamStreamError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn upgrade_capability_errors_are_narrow_and_other_bodies_are_preserved() {
        for (status, code, downgrade) in [
            (400, "websocket_not_supported", true),
            (404, "unsupported_websocket_protocol", true),
            (400, "model_not_found", false),
            (404, "model_not_found", false),
            (401, "websocket_not_supported", false),
            (403, "websocket_not_supported", false),
            (429, "websocket_not_supported", false),
        ] {
            let body = serde_json::json!({"error":{"code":code}}).to_string();
            let chunks = vec![
                Ok(Bytes::copy_from_slice(&body.as_bytes()[..10])),
                Ok(Bytes::copy_from_slice(&body.as_bytes()[10..])),
            ];
            let response = UpstreamResponse::new(
                reqwest::StatusCode::from_u16(status).unwrap(),
                HeaderMap::new(),
                futures_util::stream::iter(chunks),
            );
            let outcome = classify_rejection(
                response,
                tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            )
            .await;
            if downgrade {
                assert!(matches!(
                    outcome,
                    SendOutcome::Transport("ws_upgrade_unsupported")
                ));
            } else {
                let SendOutcome::Rejected(response) = outcome else {
                    panic!("non-capability error must reach the original classifier");
                };
                assert_eq!(response.status().as_u16(), status);
                let mut stream = response.bytes_stream();
                let mut collected = Vec::new();
                while let Some(chunk) = stream.next().await {
                    collected.extend_from_slice(&chunk.unwrap());
                }
                assert_eq!(collected, body.as_bytes());
            }
        }
    }

    #[tokio::test]
    async fn capability_error_probe_does_not_wait_past_the_attempt_deadline() {
        let response = UpstreamResponse::new(
            reqwest::StatusCode::BAD_REQUEST,
            HeaderMap::new(),
            futures_util::stream::pending(),
        );
        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            classify_rejection(response, tokio::time::Instant::now()),
        )
        .await
        .expect("handshake body probing must retain the attempt deadline");
        assert!(matches!(outcome, SendOutcome::Rejected(_)));
    }

    #[test]
    fn prelude_skips_neutral_frames_and_treats_the_connection_limit_as_a_lost_socket() {
        use serde_json::json;
        for kind in NEUTRAL_EVENTS {
            assert_eq!(prelude_step(&json!({"type":kind})), Step::Neutral);
        }
        let limit = json!({"type":"error","error":{"code":"websocket_connection_limit_reached"}});
        assert_eq!(prelude_step(&limit), Step::Lost);
        for reply in [
            json!({"type":"response.created","response":{"id":"resp-b"}}),
            json!({"type":"response.output_text.delta","delta":"x"}),
            // The gate recovers previous_response_not_found itself.
            json!({"type":"error","error":{"code":"previous_response_not_found"}}),
        ] {
            assert_eq!(prelude_step(&reply), Step::Reply, "{reply}");
        }
    }

    #[test]
    fn custom_headers_socket_key_tracks_effective_values_but_ignores_trace() {
        let url = reqwest::Url::parse("https://provider.invalid/v1/responses").unwrap();
        let config = crate::providers::custom_headers_to_map(&[
            crate::providers::ProviderCustomHeader {
                name: "X-Tenant".into(),
                value: "tenant-a".into(),
            },
            crate::providers::ProviderCustomHeader {
                name: "X-Domain".into(),
                value: "corp".into(),
            },
        ])
        .unwrap();
        let mut headers = config.clone();
        let key = socket_key(1, &url, &headers, &config, 0, None, 0);
        headers.insert("x-trace-id", "another-request".parse().unwrap());
        assert_eq!(key, socket_key(1, &url, &headers, &config, 0, None, 0));
        let mut reordered = HeaderMap::new();
        reordered.insert("x-domain", "corp".parse().unwrap());
        reordered.insert("x-tenant", "tenant-a".parse().unwrap());
        assert_eq!(key, socket_key(1, &url, &headers, &reordered, 0, None, 0));
        headers.insert("x-tenant", "plugin-tenant-b".parse().unwrap());
        assert_ne!(key, socket_key(1, &url, &headers, &config, 0, None, 0));
        headers.remove("x-tenant");
        assert_ne!(key, socket_key(1, &url, &headers, &config, 0, None, 0));
    }

    #[test]
    fn reusable_socket_key_separates_model_auth_route_and_http_client_reload() {
        let url = reqwest::Url::parse("https://provider.invalid/v1/responses").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer synthetic-a".parse().unwrap());
        let key = socket_key(1, &url, &headers, &HeaderMap::new(), 0, Some("model-a"), 0);
        assert_ne!(
            key,
            socket_key(1, &url, &headers, &HeaderMap::new(), 0, Some("model-b"), 0)
        );
        assert_ne!(
            key,
            socket_key(1, &url, &headers, &HeaderMap::new(), 0, Some("model-a"), 1)
        );
        assert_ne!(
            key,
            socket_key(1, &url, &headers, &HeaderMap::new(), 1, Some("model-a"), 0)
        );
        headers.insert(header::AUTHORIZATION, "Bearer synthetic-b".parse().unwrap());
        assert_ne!(
            key,
            socket_key(1, &url, &headers, &HeaderMap::new(), 0, Some("model-a"), 0)
        );
    }
}
