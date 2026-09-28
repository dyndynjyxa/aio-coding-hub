//! Bound neutral prefixes and commit only the final, plugin-visible Responses events.

use super::protocol::{self, EventDecoder, EventKind, HistoryDigest};
use super::state::{Continuation, RequestState};
use crate::gateway::streams::{
    is_plugin_stream_error_chunk, UpstreamByteStream, UpstreamResponse, UpstreamStreamError,
};
use crate::shared::mutex_ext::MutexExt;
use bytes::Bytes;
use futures_util::StreamExt;
use serde_json::Value;
use std::collections::VecDeque;
use std::time::Instant;

pub(in crate::gateway) enum Failure {
    Event(Value),
    Stream,
    Protocol,
    Timeout,
    Local(&'static str),
}

#[derive(Debug, thiserror::Error)]
#[error("Local Responses stream stopped: {0}")]
pub(in crate::gateway) struct LocalStreamFailure(&'static str);

fn failure_error(failure: Failure) -> UpstreamStreamError {
    match failure {
        Failure::Local(reason) => Box::new(LocalStreamFailure(reason)),
        _ => stream_error(),
    }
}

pub(in crate::gateway) struct Gate {
    upstream: UpstreamByteStream,
    decoder: EventDecoder,
    ready: VecDeque<Bytes>,
    request: RequestState,
    terminal: bool,
    pending_terminal: Option<(EventKind, Option<String>)>,
    output: HistoryDigest,
}

impl Gate {
    pub(in crate::gateway) async fn prepare(
        upstream: UpstreamByteStream,
        request: RequestState,
    ) -> Result<Self, Failure> {
        let deadline = request.generation.lock_or_recover().budget.deadline;
        let mut gate = Self {
            upstream,
            decoder: EventDecoder::default(),
            ready: VecDeque::new(),
            request,
            terminal: false,
            pending_terminal: None,
            output: HistoryDigest::default(),
        };
        let mut prefix_bytes = 0usize;
        loop {
            let event = match deadline {
                Some(deadline) if deadline <= Instant::now() => return Err(Failure::Timeout),
                Some(deadline) => tokio::time::timeout_at(
                    tokio::time::Instant::from_std(deadline),
                    gate.next_event(),
                )
                .await
                .map_err(|_| Failure::Timeout)??,
                None => gate.next_event().await?,
            };
            let kind = protocol::event_kind(&event).map_err(|_| Failure::Protocol)?;
            if kind == EventKind::Failed {
                return Err(Failure::Event(event));
            }
            let bytes = protocol::sse_bytes(&event);
            if kind == EventKind::Metadata {
                prefix_bytes = prefix_bytes.saturating_add(bytes.len());
                if prefix_bytes > protocol::MAX_PREFIX_BYTES
                    || gate.ready.len() >= protocol::MAX_PREFIX_EVENTS
                {
                    return Err(Failure::Local(
                        "Responses metadata prefix exceeds local limit",
                    ));
                }
                gate.ready.push_back(bytes);
                continue;
            }
            gate.queue_event(event, kind)?;
            return Ok(gate);
        }
    }

    async fn next_event(&mut self) -> Result<Value, Failure> {
        loop {
            if let Some(mut event) = self.decoder.next().map_err(|_| Failure::Protocol)? {
                if let Some(headers) = event.get_mut("headers").and_then(Value::as_object_mut) {
                    headers.remove(protocol::TURN_STATE_HEADER);
                }
                return Ok(event);
            }
            let bytes = self
                .upstream
                .next()
                .await
                .ok_or(Failure::Stream)?
                .map_err(|_| Failure::Stream)?;
            if is_plugin_stream_error_chunk(&bytes) {
                return Err(Failure::Local("Response blocked by gateway plugin"));
            }
            self.decoder.push(&bytes).map_err(Failure::Local)?;
        }
    }

    fn queue_event(&mut self, event: Value, kind: EventKind) -> Result<(), Failure> {
        // Some providers return the full output only in the terminal snapshot.
        // Emit missing done items once so the CLI receives tools/text as events.
        if matches!(kind, EventKind::Completed | EventKind::Incomplete) {
            if let Some(items) = event.pointer("/response/output").and_then(Value::as_array) {
                let count = self.output.count;
                if items.len() >= count
                    && self.output.is_recoverable()
                    && HistoryDigest::from_items(&items[..count]) == self.output
                {
                    for (index, item) in items.iter().enumerate().skip(count) {
                        let done = serde_json::json!({"type":"response.output_item.done","output_index":index,"item":item});
                        let bytes = protocol::sse_bytes(&done);
                        self.check_queue_limit(bytes.len())?;
                        self.record(&done, EventKind::Content);
                        self.ready.push_back(bytes);
                    }
                } else if !items.is_empty() {
                    self.request
                        .generation
                        .lock_or_recover()
                        .expected
                        .disable_recovery();
                }
            }
        }
        let bytes = protocol::sse_bytes(&event);
        self.check_queue_limit(bytes.len())?;
        self.record(&event, kind);
        if matches!(
            kind,
            EventKind::Completed | EventKind::Incomplete | EventKind::Failed
        ) {
            self.terminal = true;
            self.pending_terminal = Some((
                kind,
                event
                    .pointer("/response/id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ));
        }
        self.ready.push_back(bytes);
        Ok(())
    }

    fn check_queue_limit(&self, bytes: usize) -> Result<(), Failure> {
        let buffered: usize = self.ready.iter().map(Bytes::len).sum();
        if self.ready.len() >= protocol::MAX_PREFIX_EVENTS
            || buffered.saturating_add(bytes) > protocol::MAX_MESSAGE_BYTES
        {
            return Err(Failure::Local(
                "Responses pending output exceeds local limit",
            ));
        }
        Ok(())
    }

    fn record(&mut self, event: &Value, kind: EventKind) {
        let mut generation = self.request.generation.lock_or_recover();
        if kind != EventKind::Metadata {
            generation.committed = true;
        }
        if event.get("type").and_then(Value::as_str) == Some("response.output_item.done") {
            if let Some(item) = event.get("item") {
                generation.expected.append(std::slice::from_ref(item));
                self.output.append(std::slice::from_ref(item));
            }
        }
    }

    fn pop_ready(&mut self) -> Option<Bytes> {
        let bytes = self.ready.pop_front()?;
        if self.ready.is_empty() {
            if let Some((kind, response_id)) = self.pending_terminal.take() {
                let mut generation = self.request.generation.lock_or_recover();
                generation.terminal = true;
                generation.incomplete = kind == EventKind::Incomplete;
                generation.failed = kind == EventKind::Failed;
                if kind != EventKind::Failed {
                    if let (Some(response_id), Some(provider_id)) =
                        (response_id, generation.budget.provider_id)
                    {
                        *self.request.connection.continuation.lock_or_recover() =
                            Some(Continuation {
                                identity: generation.identity.clone(),
                                response_id,
                                provider_id,
                                upstream_ws: generation.upstream_ws,
                                history: generation.expected.clone(),
                            });
                    }
                }
            }
        }
        Some(bytes)
    }

    pub(in crate::gateway) fn into_stream(self) -> UpstreamByteStream {
        Box::pin(futures_util::stream::unfold(
            Some(self),
            |state| async move {
                let mut gate = state?;
                if let Some(bytes) = gate.pop_ready() {
                    return Some((Ok(bytes), Some(gate)));
                }
                if gate.terminal {
                    return None;
                }
                match gate.next_event().await {
                    Ok(event) => {
                        let kind = match protocol::event_kind(&event) {
                            Ok(kind) => kind,
                            Err(_) => return Some((Err(stream_error()), None)),
                        };
                        if let Err(failure) = gate.queue_event(event, kind) {
                            return Some((Err(failure_error(failure)), None));
                        }
                        Some((Ok(gate.pop_ready().unwrap()), Some(gate)))
                    }
                    Err(failure) => Some((Err(failure_error(failure)), None)),
                }
            },
        ))
    }
}

/// Adapt a bounded, terminal Responses JSON body to the same event pipeline.
pub(in crate::gateway) fn json_stream(mut upstream: UpstreamByteStream) -> UpstreamByteStream {
    Box::pin(futures_util::stream::once(async move {
        let mut body = Vec::new();
        while let Some(bytes) = upstream.next().await {
            let bytes = bytes?;
            if body.len().saturating_add(bytes.len()) > protocol::MAX_MESSAGE_BYTES {
                return Err(stream_error());
            }
            body.extend_from_slice(&bytes);
        }
        let response: Value = serde_json::from_slice(&body).map_err(|_| stream_error())?;
        let event_type = match response.get("status").and_then(Value::as_str) {
            Some("completed") => "response.completed",
            Some("incomplete") => "response.incomplete",
            Some("failed") => "response.failed",
            _ => return Err(stream_error()),
        };
        if response.get("id").and_then(Value::as_str).is_none()
            || !response.get("output").is_some_and(Value::is_array)
        {
            return Err(stream_error());
        }
        Ok(protocol::sse_bytes(
            &serde_json::json!({"type":event_type,"response":response}),
        ))
    }))
}

fn stream_error() -> UpstreamStreamError {
    std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        "Responses stream ended without a valid terminal event",
    )
    .into()
}

/// Error events carry a semantic status; this is not the WebSocket handshake status.
pub(in crate::gateway) fn error_status(event: &Value) -> reqwest::StatusCode {
    let status = event
        .get("status")
        .or_else(|| event.get("status_code"))
        .and_then(Value::as_u64)
        .and_then(|n| u16::try_from(n).ok());
    if let Some(status) = status
        .and_then(|n| reqwest::StatusCode::from_u16(n).ok())
        .filter(|status| status.is_client_error() || status.is_server_error())
    {
        return status;
    }
    let code = event
        .pointer("/error/code")
        .or_else(|| event.pointer("/response/error/code"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match code {
        "invalid_api_key" | "authentication_error" => reqwest::StatusCode::UNAUTHORIZED,
        "insufficient_quota" | "billing_hard_limit_reached" => {
            reqwest::StatusCode::PAYMENT_REQUIRED
        }
        "model_not_found" => reqwest::StatusCode::NOT_FOUND,
        "rate_limit_exceeded" => reqwest::StatusCode::TOO_MANY_REQUESTS,
        "server_error" | "internal_error" => reqwest::StatusCode::BAD_GATEWAY,
        _ => reqwest::StatusCode::BAD_REQUEST,
    }
}

pub(in crate::gateway) fn error_response(event: Value) -> UpstreamResponse {
    let status = error_status(&event);
    let body = event.get("response").cloned().unwrap_or(event);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    UpstreamResponse::new(
        status,
        headers,
        futures_util::stream::once(async move { Ok(Bytes::from(body.to_string())) }),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{
        protocol::Owner,
        state::{Budget, Generation, RecoveryIdentity, Runtime},
    };
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    fn request() -> RequestState {
        let runtime = Arc::new(Runtime::new(true));
        let owner = Owner::parse(&json!({"session_id":"s","thread_id":"t","window_id":"w","context_window_id":"c","turn_id":"g"}).to_string()).unwrap();
        let nonce = runtime.issue_nonce(&owner).unwrap();
        let request = RequestState {
            connection: runtime.connection().unwrap(),
            client_ws: true,
            generation: Arc::new(Mutex::new(Generation {
                identity: Some(RecoveryIdentity { owner, nonce }),
                expected: HistoryDigest::default(),
                properties: HistoryDigest::default(),
                previous: None,
                committed: false,
                terminal: false,
                incomplete: false,
                failed: false,
                recovered: false,
                from_trace: None,
                trace_id: "trace".into(),
                budget: Budget {
                    provider_id: Some(1),
                    ..Budget::default()
                },
                upstream_ws: true,
            })),
        };
        runtime.begin_generation(&request).unwrap();
        request
    }

    fn events(values: Vec<Value>) -> UpstreamByteStream {
        Box::pin(futures_util::stream::iter(
            values
                .into_iter()
                .map(|value| Ok(protocol::sse_bytes(&value))),
        ))
    }

    async fn collect(gate: Gate) -> Vec<Value> {
        let mut stream = gate.into_stream();
        let mut decoder = EventDecoder::default();
        let mut result = Vec::new();
        while let Some(bytes) = stream.next().await {
            decoder.push(&bytes.unwrap()).unwrap();
            while let Some(event) = decoder.next().unwrap() {
                result.push(event);
            }
        }
        result
    }

    #[tokio::test]
    async fn metadata_before_error_or_eof_never_commits() {
        let metadata = json!({"type":"response.created","response":{"id":"resp_a","output":[]}});
        let request = request();
        let result = Gate::prepare(
            events(vec![
                metadata.clone(),
                protocol::error_event("rate_limit_exceeded", "busy"),
            ]),
            request.clone(),
        )
        .await;
        assert!(matches!(result, Err(Failure::Event(_))));
        assert!(!request.generation.lock_or_recover().committed);
        assert!(matches!(
            Gate::prepare(events(vec![metadata]), request.clone()).await,
            Err(Failure::Stream)
        ));
        assert!(!request.generation.lock_or_recover().committed);
    }

    #[tokio::test]
    async fn semantic_output_then_eof_is_a_stream_error_without_replay() {
        let request = request();
        let gate = Gate::prepare(
            events(vec![
                json!({"type":"response.output_text.delta","delta":"partial"}),
            ]),
            request.clone(),
        )
        .await
        .unwrap_or_else(|_| panic!("content must commit"));
        let mut stream = gate.into_stream();
        assert!(stream.next().await.unwrap().is_ok());
        assert!(stream.next().await.unwrap().is_err());
        assert!(stream.next().await.is_none());
        let generation = request.generation.lock_or_recover();
        assert!(generation.committed);
        assert!(!generation.terminal);
    }

    #[tokio::test]
    async fn malformed_upstream_event_is_a_protocol_failure_before_output() {
        for wire in [
            b"data: {\"message\":\"missing type\"}\n\n".as_slice(),
            b"data: {not-json}\n\n".as_slice(),
        ] {
            let request = request();
            let stream: UpstreamByteStream = Box::pin(futures_util::stream::iter(vec![Ok(
                Bytes::copy_from_slice(wire),
            )]));
            assert!(matches!(
                Gate::prepare(stream, request.clone()).await,
                Err(Failure::Protocol)
            ));
            assert!(!request.generation.lock_or_recover().committed);
        }
    }

    #[tokio::test]
    async fn plugin_marker_inside_response_text_is_not_a_plugin_block() {
        let request = request();
        let gate = Gate::prepare(
            events(vec![
                json!({"type":"response.output_text.delta","delta":": aio-plugin-error"}),
                json!({"type":"response.output_text.delta","delta":": aio-plugin-error\n"}),
                json!({"type":"response.completed","response":{"id":"resp_a","output":[]}}),
            ]),
            request.clone(),
        )
        .await
        .unwrap_or_else(|_| panic!("ordinary output must not be blocked"));
        let output = collect(gate).await;
        assert_eq!(output.len(), 3);
        assert_eq!(output[0]["delta"], ": aio-plugin-error");
        assert!(request.generation.lock_or_recover().terminal);
    }

    #[tokio::test]
    async fn plugin_block_after_output_preserves_local_failure_class() {
        let stream: UpstreamByteStream = Box::pin(futures_util::stream::iter(vec![
            Ok(protocol::sse_bytes(
                &json!({"type":"response.output_text.delta","delta":"text"}),
            )),
            Ok(Bytes::from_static(
                b": aio-plugin-error\nevent: error\ndata: {}\n\n",
            )),
        ]));
        let gate = Gate::prepare(stream, request())
            .await
            .unwrap_or_else(|_| panic!("content"));
        let mut stream = gate.into_stream();
        assert!(stream.next().await.unwrap().is_ok());
        let error = stream.next().await.unwrap().unwrap_err();
        assert!(error.downcast_ref::<LocalStreamFailure>().is_some());
    }

    #[tokio::test]
    async fn terminal_failed_incomplete_and_empty_completion_stay_distinct() {
        for (kind, failed, incomplete) in [
            ("response.failed", true, false),
            ("response.incomplete", false, true),
            ("response.completed", false, false),
        ] {
            let request = request();
            let gate = Gate::prepare(
                events(vec![
                    json!({"type":"response.output_text.delta","delta":"text"}),
                    json!({"type":kind,"response":{"id":"resp_a","output":[]}}),
                ]),
                request.clone(),
            )
            .await
            .unwrap_or_else(|_| panic!("content must commit"));
            let output = collect(gate).await;
            assert_eq!(output.last().unwrap()["type"], kind);
            let generation = request.generation.lock_or_recover();
            assert!(generation.terminal);
            assert_eq!(generation.failed, failed);
            assert_eq!(generation.incomplete, incomplete);
        }
        let request = request();
        let gate = Gate::prepare(
            events(vec![
                json!({"type":"response.completed","response":{"id":"resp_empty","output":[]}}),
            ]),
            request.clone(),
        )
        .await
        .unwrap_or_else(|_| panic!("empty completion is valid"));
        assert_eq!(collect(gate).await.len(), 1);
        assert!(request.generation.lock_or_recover().terminal);
    }

    #[tokio::test]
    async fn queued_terminal_is_not_completion_until_delivered_and_queue_is_bounded() {
        let request = request();
        let item =
            json!({"type":"function_call","name":"tool","arguments":"{}","call_id":"call_a"});
        let gate = Gate::prepare(events(vec![json!({"type":"response.completed","response":{"id":"resp_a","output":[item.clone()]}})]), request.clone()).await.unwrap_or_else(|_| panic!("valid terminal"));
        assert!(!request.generation.lock_or_recover().terminal);
        let mut stream = gate.into_stream();
        assert!(stream.next().await.unwrap().is_ok());
        assert!(!request.generation.lock_or_recover().terminal);
        drop(stream);
        assert!(!request.generation.lock_or_recover().terminal);
        let flood = vec![item; protocol::MAX_PREFIX_EVENTS + 1];
        assert!(matches!(
            Gate::prepare(
                events(vec![
                    json!({"type":"response.completed","response":{"id":"resp_a","output":flood}})
                ]),
                request
            )
            .await,
            Err(Failure::Local(_))
        ));
    }

    #[tokio::test]
    async fn neutral_prefix_has_event_byte_and_deadline_limits() {
        let request = request();
        let flood = (0..=protocol::MAX_PREFIX_EVENTS)
            .map(|_| json!({"type":"response.metadata"}))
            .collect();
        assert!(matches!(
            Gate::prepare(events(flood), request.clone()).await,
            Err(Failure::Local(_))
        ));
        let flood =
            json!({"type":"response.metadata","padding":"x".repeat(protocol::MAX_PREFIX_BYTES)});
        assert!(matches!(
            Gate::prepare(events(vec![flood]), request.clone()).await,
            Err(Failure::Local(_))
        ));
        request.generation.lock_or_recover().budget.deadline = Some(Instant::now());
        assert!(matches!(
            Gate::prepare(Box::pin(futures_util::stream::pending()), request.clone()).await,
            Err(Failure::Timeout)
        ));
        assert!(!request.generation.lock_or_recover().committed);
    }

    #[tokio::test]
    async fn json_terminal_synthesizes_done_items_and_stream_snapshot_does_not_duplicate() {
        let item =
            json!({"type":"function_call","name":"tool","arguments":"{}","call_id":"call_a"});
        let response = json!({"id":"resp_a","status":"completed","output":[item.clone()]});
        let stream: UpstreamByteStream = Box::pin(futures_util::stream::iter(vec![Ok(
            Bytes::from(response.to_string()),
        )]));
        let request = request();
        let gate = Gate::prepare(json_stream(stream), request.clone())
            .await
            .unwrap_or_else(|_| panic!("valid JSON response"));
        let output = collect(gate).await;
        assert_eq!(output.len(), 2);
        assert_eq!(output[0]["type"], "response.output_item.done");
        assert_eq!(request.generation.lock_or_recover().expected.count, 1);
        let stream = events(vec![
            output[0].clone(),
            json!({"type":"response.completed","response":response}),
        ]);
        let next = super::tests::request();
        let gate = Gate::prepare(stream, next.clone())
            .await
            .unwrap_or_else(|_| panic!("stream response"));
        assert_eq!(collect(gate).await.len(), 2);
        assert_eq!(next.generation.lock_or_recover().expected.count, 1);
        let invalid: UpstreamByteStream = Box::pin(futures_util::stream::iter(vec![Ok(
            Bytes::from_static(b"<html>bad gateway</html>"),
        )]));
        assert!(matches!(
            Gate::prepare(json_stream(invalid), next).await,
            Err(Failure::Stream)
        ));
    }

    #[tokio::test]
    async fn split_events_preserve_order_and_strip_upstream_routing_state() {
        let mut wire = protocol::sse_bytes(&protocol::metadata_event("upstream-private")).to_vec();
        wire.extend_from_slice(&protocol::sse_bytes(
            &json!({"type":"response.completed","response":{"id":"resp_a","output":[]}}),
        ));
        let stream: UpstreamByteStream = Box::pin(futures_util::stream::iter(
            wire.into_iter().map(|byte| Ok(Bytes::from(vec![byte]))),
        ));
        let gate = Gate::prepare(stream, request())
            .await
            .unwrap_or_else(|_| panic!("split events"));
        let output = collect(gate).await;
        assert_eq!(output.len(), 2);
        assert!(output[0].pointer("/headers/x-codex-turn-state").is_none());
    }
}
