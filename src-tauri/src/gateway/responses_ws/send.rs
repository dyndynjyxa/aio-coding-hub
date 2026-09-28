//! Per-attempt upstream WebSocket ownership; provider selection stays in the proxy loop.

use super::protocol::{self, TURN_STATE_HEADER};
use super::state::{Connection, RequestState, ReusableSocket};
use super::upstream::{self, ConnectError};
use crate::gateway::streams::{UpstreamResponse, UpstreamStreamError};
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

pub(in crate::gateway) enum SendOutcome {
    Response(UpstreamResponse),
    Rejected(UpstreamResponse),
    Transport(&'static str),
    ContextLost,
    Local(&'static str),
    Http(&'static str),
}

pub(in crate::gateway) fn socket_key(
    provider: i64,
    url: &reqwest::Url,
    headers: &HeaderMap,
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
    body: Bytes,
    deadline: Option<std::time::Instant>,
) -> SendOutcome {
    let mut payload: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(object)) => Value::Object(object),
        _ => return SendOutcome::Local("invalid Responses request body"),
    };
    let previous = payload.get("previous_response_id").and_then(Value::as_str);
    let key = socket_key(
        provider_id,
        &url,
        &headers,
        connection.runtime.epoch(),
        payload.get("model").and_then(Value::as_str),
        crate::gateway::http_client::generation(),
    );
    let mut reusable = connection
        .upstream
        .lock_or_recover()
        .take()
        .filter(|cached| cached.key == key);
    let allow_ws = supports_ws
        && request.is_none_or(|request| {
            request.client_ws && {
                let generation = request.generation.lock_or_recover();
                !generation.budget.http_only
                    && !generation.budget.http_provider_ids.contains(&provider_id)
            }
        })
        && !connection.runtime.cooling(&key);
    if previous.is_some() {
        let continuation = connection.continuation.lock_or_recover();
        let warm = connection.prewarm.lock_or_recover();
        let valid = continuation.as_ref().is_some_and(|current| {
            current.provider_id == provider_id
                && current.upstream_ws
                && Some(current.response_id.as_str()) == previous
        }) || warm.as_ref().is_some_and(|warm| {
            warm.provider_id == provider_id && Some(warm.response_id.as_str()) == previous
        });
        if !valid || !allow_ws || reusable.is_none() {
            return SendOutcome::ContextLost;
        }
    }
    if !allow_ws {
        let reason = if !supports_ws {
            "provider_http_only"
        } else if connection.runtime.cooling(&key) {
            "ws_cooldown_skip"
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
        let connected = match upstream::connect(url, headers, deadline).await {
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
    if serialized.len() > protocol::MAX_MESSAGE_BYTES {
        return SendOutcome::Local("Responses request exceeds WebSocket byte limit");
    }
    let send_cap = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let send_deadline = deadline
        .map(tokio::time::Instant::from_std)
        .map_or(send_cap, |deadline| deadline.min(send_cap));
    if !matches!(
        tokio::time::timeout_at(
            send_deadline,
            reusable.connection.socket.send(Message::Text(serialized))
        )
        .await,
        Ok(Ok(()))
    ) {
        connection.runtime.cool(key);
        return SendOutcome::Transport("ws_send_failed");
    }
    if let Some(request) = request {
        request.generation.lock_or_recover().upstream_ws = true;
    }
    let response_headers = reusable.connection.headers.clone();
    let stream = futures_util::stream::unfold(Some((reusable, probe)), move |state| {
        let connection = connection.clone();
        async move {
            let (mut reusable, probe) = state?;
            loop {
                match reusable.connection.socket.next().await {
                    Some(Ok(Message::Text(text))) => {
                        let mut event: Value = match serde_json::from_str(&text) {
                            Ok(event) => event,
                            Err(_) => {
                                connection.runtime.cool(reusable.key.clone());
                                return Some((
                                    Err(protocol_error("invalid upstream WebSocket JSON")),
                                    None,
                                ));
                            }
                        };
                        let kind = match protocol::event_kind(&event) {
                            Ok(kind) => kind,
                            Err(message) => {
                                connection.runtime.cool(reusable.key.clone());
                                return Some((Err(protocol_error(message)), None));
                            }
                        };
                        if let Some(headers) =
                            event.get_mut("headers").and_then(Value::as_object_mut)
                        {
                            if let Some(value) = headers
                                .remove(TURN_STATE_HEADER)
                                .and_then(|value| value.as_str().map(str::to_owned))
                            {
                                reusable.turn_state = Some(value);
                            }
                        }
                        let bytes = protocol::sse_bytes(&event);
                        if matches!(
                            kind,
                            protocol::EventKind::Completed | protocol::EventKind::Incomplete
                        ) {
                            if let Some(probe) = probe {
                                probe.succeeded();
                            }
                            *connection.upstream.lock_or_recover() = Some(reusable);
                            return Some((Ok(bytes), None));
                        }
                        if kind == protocol::EventKind::Failed {
                            return Some((Ok(bytes), None));
                        }
                        return Some((Ok(bytes), Some((reusable, probe))));
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        if reusable
                            .connection
                            .socket
                            .send(Message::Pong(bytes))
                            .await
                            .is_err()
                        {
                            connection.runtime.cool(reusable.key.clone());
                            return Some((Err(protocol_error("WebSocket pong failed")), None));
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    _ => {
                        connection.runtime.cool(reusable.key.clone());
                        return Some((
                            Err(protocol_error("WebSocket closed before terminal response")),
                            None,
                        ));
                    }
                }
            }
        }
    });
    let mut headers = response_headers;
    headers.remove(header::CONTENT_LENGTH);
    headers.remove(header::CONTENT_ENCODING);
    headers.remove(TURN_STATE_HEADER);
    headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    SendOutcome::Response(UpstreamResponse::new_ws(headers, stream))
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
    fn reusable_socket_key_separates_model_auth_route_and_http_client_reload() {
        let url = reqwest::Url::parse("https://provider.invalid/v1/responses").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer synthetic-a".parse().unwrap());
        let key = socket_key(1, &url, &headers, 0, Some("model-a"), 0);
        assert_ne!(key, socket_key(1, &url, &headers, 0, Some("model-b"), 0));
        assert_ne!(key, socket_key(1, &url, &headers, 0, Some("model-a"), 1));
        assert_ne!(key, socket_key(1, &url, &headers, 1, Some("model-a"), 0));
        headers.insert(header::AUTHORIZATION, "Bearer synthetic-b".parse().unwrap());
        assert_ne!(key, socket_key(1, &url, &headers, 0, Some("model-a"), 0));
    }
}
