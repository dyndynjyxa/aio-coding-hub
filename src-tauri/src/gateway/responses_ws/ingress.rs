//! Local Responses WebSocket ingress. Each create re-enters the ordinary proxy chain.

use super::protocol::{self, EventDecoder, EventKind, HistoryDigest, Owner};
use super::state::{self, Connection, Generation, RecoveryIdentity, RequestState};
use crate::gateway::proxy::proxy_impl;
use crate::gateway::proxy::request_end::{
    emit_request_event_and_enqueue_request_log, RequestCompletion, RequestEndArgs,
    RequestEndContextArgs, RequestEndDeps,
};
use crate::gateway::proxy::GatewayErrorCode;
use crate::gateway::runtime::GatewayAppState;
use crate::shared::mutex_ext::MutexExt;
use axum::body::Body;
use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    FromRequestParts,
};
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(in crate::gateway) async fn dispatch<R>(
    state: GatewayAppState<R>,
    cli_key: String,
    path: String,
    req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let is_upgrade = req.method() == Method::GET
        && cli_key == "codex"
        && matches!(path.trim_end_matches('/'), "/v1/responses" | "/responses")
        && req
            .headers()
            .get(header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    if !is_upgrade {
        return proxy_impl(state, cli_key, path, req).await;
    }
    if req.headers().contains_key(header::ORIGIN) {
        return (
            StatusCode::FORBIDDEN,
            "Browser-origin Responses WebSocket is not supported",
        )
            .into_response();
    }
    let forced =
        crate::gateway::proxy::handler::early_error::extract_forced_provider_id(req.headers());
    if forced.is_none()
        && !crate::gateway::proxy::cli_proxy_guard::cli_proxy_enabled_cached(&state.app, "codex")
            .enabled
    {
        return (StatusCode::FORBIDDEN, "Codex CLI proxy is disabled").into_response();
    }
    if !state.responses_ws.enabled()
        || req
            .headers()
            .get("session-id")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|session| state.responses_ws.prefers_http(session))
    {
        return StatusCode::UPGRADE_REQUIRED.into_response();
    }
    let connection = match state.responses_ws.connection() {
        Ok(connection) => connection,
        Err(message) => return (StatusCode::UPGRADE_REQUIRED, message).into_response(),
    };
    let (mut parts, _) = req.into_parts();
    let upgrade = match WebSocketUpgrade::from_request_parts(&mut parts, &state).await {
        Ok(upgrade) => upgrade,
        Err(error) => return error.into_response(),
    };
    let headers = parts.headers;
    let uri = parts.uri;
    upgrade
        .max_message_size(crate::gateway::util::max_request_body_bytes())
        .max_frame_size(crate::gateway::util::max_request_body_bytes())
        .on_upgrade(move |socket| serve(socket, state, path, headers, uri, connection))
        .into_response()
}

async fn send_event(socket: &mut WebSocket, event: &Value) -> Result<(), ()> {
    tokio::time::timeout(
        Duration::from_secs(5),
        socket.send(Message::Text(event.to_string())),
    )
    .await
    .map_err(|_| ())?
    .map_err(|_| ())
}

async fn serve<R>(
    mut socket: WebSocket,
    state: GatewayAppState<R>,
    path: String,
    mut headers: HeaderMap,
    uri: axum::http::Uri,
    connection: Arc<Connection>,
) where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let mut stop = state.responses_ws.shutdown.subscribe();
    let mut changed = state.responses_ws.changed.subscribe();
    loop {
        if !state.responses_ws.enabled() || connection.epoch != state.responses_ws.epoch() {
            break;
        }
        let idle_timeout_secs = crate::settings::read(&state.app)
            .map(|settings| settings.upstream_stream_idle_timeout_seconds)
            .unwrap_or(crate::settings::DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS);
        let message = tokio::select! {
            _ = stop.changed() => break,
            _ = changed.changed() => break,
            _ = tokio::time::sleep(Duration::from_secs(idle_timeout_secs.into())), if idle_timeout_secs > 0 => break,
            message = socket.next() => match message {
                Some(Ok(message)) => message, _ => break,
            },
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Ping(bytes) => {
                if socket.send(Message::Pong(bytes)).await.is_err() {
                    break;
                }
                continue;
            }
            Message::Pong(_) => continue,
            _ => break,
        };
        let mut body: Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(_) => {
                let _ = send_event(
                    &mut socket,
                    &protocol::error_event("invalid_request", "Invalid JSON response.create"),
                )
                .await;
                continue;
            }
        };
        drop(text);
        if body.get("type").and_then(Value::as_str) != Some("response.create")
            || !body.get("input").is_some_and(Value::is_array)
        {
            let _ = send_event(
                &mut socket,
                &protocol::error_event(
                    "invalid_request",
                    "Expected response.create with input array",
                ),
            )
            .await;
            continue;
        }
        let forced =
            crate::gateway::proxy::handler::early_error::extract_forced_provider_id(&headers);
        let started = Instant::now();
        let created_at_ms = crate::gateway::util::now_unix_millis() as i64;
        let prepared =
            prepare_request(connection.clone(), &headers, &body, forced).and_then(|prepared| {
                if let state::PreparedRequest::Dispatch(Some(request)) = &prepared {
                    connection.runtime.begin_generation(request)?;
                }
                Ok(prepared)
            });
        let request_state = match prepared {
            Ok(state::PreparedRequest::Dispatch(value)) => value,
            Ok(state::PreparedRequest::Failure(failure)) => {
                let event = crate::gateway::client_error::encode_failure(
                    crate::gateway::client_error::ClientProtocol::ResponsesWs,
                    &failure,
                );
                let _ = send_event(&mut socket, &event).await;
                break;
            }
            Err(message) => {
                let trace_id = crate::gateway::util::new_trace_id();
                emit_request_event_and_enqueue_request_log(
                    RequestEndArgs::from_context(RequestEndContextArgs {
                        deps: RequestEndDeps::new(
                            &state.app,
                            &state.db,
                            &state.log_tx,
                            &state.plugin_pipeline,
                            &state.active_requests,
                        ),
                        trace_id: &trace_id,
                        cli_key: "codex",
                        method: "POST",
                        path: &path,
                        observe: true,
                        query: uri.query(),
                        excluded_from_stats: false,
                        duration_ms: started.elapsed().as_millis(),
                        attempts: &[],
                        special_settings_json: Some(
                            serde_json::json!([{
                                "type":"codex_responses_transport", "scope":"request",
                                "client_transport":"responses_ws", "failure_class":"local",
                                "reason_code":"invalid_request", "upstream_sent":false,
                            }])
                            .to_string(),
                        ),
                        session_id: None,
                        requested_model: None,
                        created_at_ms,
                        created_at: created_at_ms / 1000,
                    })
                    .with_completion(RequestCompletion::failure(
                        400,
                        Some("local"),
                        GatewayErrorCode::RequestRejected.as_str(),
                    ))
                    .with_request_rejection("invalid_request", message),
                )
                .await;
                let mut event = protocol::error_event("invalid_request", message);
                event["trace_id"] = serde_json::json!(trace_id);
                let _ = send_event(&mut socket, &event).await;
                break;
            }
        };
        let _lease = state::GenerationLease(request_state.clone());
        if let Some(request_state) = &request_state {
            // Upgrade headers describe only the first generation; later turns
            // carry their state in each response.create frame.
            headers.remove(protocol::TURN_STATE_HEADER);
            let identity = request_state.generation.lock_or_recover().identity.clone();
            if let Some(identity) = identity {
                if send_event(&mut socket, &protocol::metadata_event(&identity.nonce))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
        let prewarm = body.get("generate") == Some(&Value::Bool(false));
        let mut warm_history =
            HistoryDigest::from_items(body.get("input").and_then(Value::as_array).unwrap());
        body.as_object_mut().unwrap().remove("type");
        body.as_object_mut()
            .unwrap()
            .insert("stream".into(), Value::Bool(true));
        let mut request = Request::builder()
            .method(Method::POST)
            .uri(uri.clone())
            .body(Body::from(body.to_string()))
            .unwrap();
        *request.headers_mut() = headers.clone();
        crate::gateway::util::strip_hop_headers(request.headers_mut());
        let handshake_headers: Vec<_> = request
            .headers()
            .keys()
            .filter(|name| name.as_str().starts_with("sec-websocket-"))
            .cloned()
            .collect();
        for name in handshake_headers {
            request.headers_mut().remove(name);
        }

        if let Some(metadata) = body
            .pointer("/client_metadata/x-codex-turn-metadata")
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<axum::http::HeaderValue>().ok())
        {
            request
                .headers_mut()
                .insert("x-codex-turn-metadata", metadata);
        }
        request.headers_mut().remove(header::CONTENT_LENGTH);
        request.headers_mut().remove(header::CONTENT_ENCODING);
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        request.extensions_mut().insert(connection.clone());
        if let Some(request_state) = &request_state {
            request.extensions_mut().insert(request_state.clone());
        }
        drop(body);
        let mut forward = Box::pin(proxy_impl(
            state.clone(),
            "codex".into(),
            path.clone(),
            request,
        ));
        let response = loop {
            tokio::select! {
                _ = stop.changed() => return,
                response = &mut forward => break response,
                message = socket.next() => if !handle_busy_message(&mut socket, message).await { return; },
            }
        };
        let status = response.status();
        let success = status.is_success();
        let trace = response.headers().get("x-trace-id").cloned();
        let retry_after = response.headers().get(header::RETRY_AFTER).cloned();
        let failure = response
            .extensions()
            .get::<crate::gateway::proxy::GatewayFailure>()
            .cloned();
        let mut bytes = response.into_body().into_data_stream();
        let mut decoder = EventDecoder::default();
        let mut terminal = false;
        let mut error = !success;
        let mut error_body = Vec::new();
        loop {
            let chunk = tokio::select! {
                _ = stop.changed() => return,
                chunk = bytes.next() => chunk,
                message = socket.next() => { if !handle_busy_message(&mut socket, message).await { return; } continue; },
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(_) => {
                    error = true;
                    break;
                }
            };
            if !success {
                let remaining = (16 * 1024usize).saturating_sub(error_body.len());
                error_body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                continue;
            }
            if decoder.push(&chunk).is_err() {
                error = true;
                break;
            }
            drop(chunk);
            loop {
                let mut event = match decoder.next() {
                    Ok(Some(event)) => event,
                    Ok(None) => break,
                    Err(_) => {
                        error = true;
                        break;
                    }
                };
                if event.get("type").and_then(Value::as_str) == Some("error") {
                    event = crate::gateway::client_error::normalize_ws_error(event, None);
                }
                let kind = protocol::event_kind(&event).unwrap();
                if prewarm {
                    if event.get("type").and_then(Value::as_str)
                        == Some("response.output_item.done")
                    {
                        if let Some(item) = event.get("item") {
                            warm_history.append(std::slice::from_ref(item));
                        }
                    }
                    if kind == EventKind::Completed {
                        if let (Some(id), Some(provider_id)) = (
                            event.pointer("/response/id").and_then(Value::as_str),
                            connection
                                .upstream
                                .lock_or_recover()
                                .as_ref()
                                .map(|socket| socket.provider_id),
                        ) {
                            *connection.prewarm.lock_or_recover() = Some(state::Prewarm {
                                response_id: id.into(),
                                provider_id,
                                history: warm_history.clone(),
                            });
                        }
                    }
                }
                if send_event(&mut socket, &event).await.is_err() {
                    return;
                }
                if matches!(
                    kind,
                    EventKind::Completed | EventKind::Incomplete | EventKind::Failed
                ) {
                    terminal = true;
                    error = kind == EventKind::Failed;
                    break;
                }
            }
            if terminal || error {
                break;
            }
        }
        if !terminal {
            let mut event = if let Some(failure) = failure {
                crate::gateway::client_error::encode_failure(
                    crate::gateway::client_error::ClientProtocol::ResponsesWs,
                    &failure,
                )
            } else {
                serde_json::from_slice::<Value>(&error_body)
                    .ok()
                    .filter(|value| {
                        value.get("error").is_some_and(Value::is_object)
                            || value.get("type").and_then(Value::as_str) == Some("response.failed")
                    })
                    .map(|event| {
                        crate::gateway::client_error::normalize_ws_error(event, Some(status))
                    })
                    .unwrap_or_else(|| {
                        crate::gateway::client_error::ws_error(
                            if success {
                                StatusCode::BAD_GATEWAY
                            } else {
                                status
                            },
                            "upstream_error",
                            "Responses generation failed before a valid terminal event",
                        )
                    })
            };
            if let Some(trace) = trace.and_then(|v| v.to_str().ok().map(str::to_owned)) {
                event["trace_id"] = Value::String(trace);
            }
            if let Some(retry_after) =
                retry_after.and_then(|value| value.to_str().ok().map(str::to_owned))
            {
                event["headers"]["retry-after"] = Value::String(retry_after);
            }
            let _ = send_event(&mut socket, &event).await;
            error = true;
        }
        if error {
            break;
        }
    }
    let _ = tokio::time::timeout(Duration::from_millis(250), socket.close()).await;
}

async fn handle_busy_message(
    socket: &mut WebSocket,
    message: Option<Result<Message, axum::Error>>,
) -> bool {
    match message {
        Some(Ok(Message::Ping(bytes))) => socket.send(Message::Pong(bytes)).await.is_ok(),
        Some(Ok(Message::Pong(_))) => true,
        Some(Ok(Message::Text(_))) => send_event(
            socket,
            &protocol::error_event("response_in_progress", "A response is already in progress"),
        )
        .await
        .is_ok(),
        _ => false,
    }
}

fn prepare_request(
    connection: Arc<Connection>,
    headers: &HeaderMap,
    body: &Value,
    forced: Option<i64>,
) -> Result<state::PreparedRequest, &'static str> {
    if body.get("generate") == Some(&Value::Bool(false)) {
        return Ok(state::PreparedRequest::Dispatch(None));
    }
    if body.get("background") == Some(&Value::Bool(true)) {
        return Err("Background generation is not supported over this WebSocket");
    }
    // Turn metadata enables cross-connection recovery; the socket itself owns
    // ordinary Responses requests and their sequential continuations.
    let owner = body
        .pointer("/client_metadata/x-codex-turn-metadata")
        .and_then(Value::as_str)
        .and_then(Owner::parse);
    let input = body
        .get("input")
        .and_then(Value::as_array)
        .ok_or("missing input array")?;
    let previous = body
        .get("previous_response_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let nonce_in = state::recovery_nonce(headers, body)?;
    if nonce_in.is_some() && owner.is_none() {
        return Err("missing or invalid Codex turn metadata for recovery");
    }
    let properties = state::request_properties(body, forced);
    if let Some(owner) = &owner {
        if let Some(failure) = connection.runtime.known_failure(
            owner,
            nonce_in,
            input,
            previous.as_deref(),
            &properties,
        )? {
            return Ok(state::PreparedRequest::Failure(failure));
        }
    }
    let continuation = connection.continuation.lock_or_recover();
    // The cached WebSocket context spans turns; recovery nonces remain turn-scoped.
    let current =
        continuation.as_ref().filter(
            |current| match (current.identity.as_ref(), owner.as_ref()) {
                (Some(current), Some(owner)) => {
                    current.owner.session == owner.session
                        && current.owner.thread == owner.thread
                        && current.owner.window == owner.window
                        && current.owner.context_window == owner.context_window
                }
                (None, None) => true,
                _ => false,
            },
        );
    let mut expected = if previous.is_some() {
        current
            .filter(|current| Some(current.response_id.as_str()) == previous.as_deref())
            .map(|current| current.history.clone())
            .or_else(|| {
                connection
                    .prewarm
                    .lock_or_recover()
                    .as_ref()
                    .filter(|warm| Some(warm.response_id.as_str()) == previous.as_deref())
                    .map(|warm| warm.history.clone())
            })
            .ok_or("unknown response continuation")?
    } else {
        HistoryDigest::default()
    };
    let current_identity = current.and_then(|current| current.identity.as_ref());
    let nonce_in = if let Some(current) = current_identity {
        if nonce_in.is_some_and(|nonce| nonce != current.nonce) {
            return Err("Response continuation owner mismatch");
        }
        if Some(&current.owner) == owner.as_ref() {
            Some(current.nonce.as_str())
        } else if previous.is_some() {
            // A new turn may still echo the last turn's token on this socket.
            None
        } else {
            nonce_in
        }
    } else {
        nonce_in
    };
    let recovered = if previous.is_none() {
        match &owner {
            Some(owner) => connection
                .runtime
                .claim(owner, nonce_in, input, &properties)?,
            None => None,
        }
    } else {
        None
    };
    let identity = if let Some(owner) = owner {
        let nonce = match nonce_in {
            Some(nonce) => nonce.to_owned(),
            None => connection.runtime.issue_nonce(&owner)?,
        };
        Some(RecoveryIdentity { owner, nonce })
    } else {
        None
    };
    expected.append(input);
    drop(continuation);
    let (budget, from_trace) = recovered.map_or_else(
        || (state::Budget::default(), None),
        |record| (record.budget, Some(record.from_trace)),
    );
    Ok(state::PreparedRequest::Dispatch(Some(RequestState {
        connection,
        client_ws: true,
        generation: Arc::new(Mutex::new(Generation {
            identity,
            expected,
            input: HistoryDigest::from_items(input),
            properties,
            previous,
            committed: false,
            terminal: false,
            incomplete: false,
            failed: false,
            recovered: from_trace.is_some(),
            from_trace,
            trace_id: String::new(),
            budget,
            upstream_ws: false,
        })),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prepare_request(
        connection: Arc<Connection>,
        headers: &HeaderMap,
        body: &Value,
        forced: Option<i64>,
    ) -> Result<Option<RequestState>, &'static str> {
        match super::prepare_request(connection, headers, body, forced)? {
            state::PreparedRequest::Dispatch(request) => Ok(request),
            state::PreparedRequest::Failure(_) => panic!("expected generation dispatch"),
        }
    }
    #[test]
    fn continuation_can_cross_turns_but_not_session_window_or_context() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let connection = runtime.connection().unwrap();
        let metadata = json!({"session_id":"session","thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"first"});
        let mut body = json!({"type":"response.create","model":"model","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}],"client_metadata":{"x-codex-turn-metadata":metadata.to_string()}});
        let first = prepare_request(connection.clone(), &HeaderMap::new(), &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&first).unwrap();
        let first_nonce = {
            let generation = first.generation.lock_or_recover();
            *connection.continuation.lock_or_recover() = Some(state::Continuation {
                identity: generation.identity.clone(),
                response_id: "resp_first".into(),
                provider_id: 1,
                upstream_ws: true,
                history: generation.expected.clone(),
            });
            generation.identity.as_ref().unwrap().nonce.clone()
        };
        runtime.finish_generation(&first, true);
        body["previous_response_id"] = json!("resp_first");
        let mut next_metadata = metadata;
        next_metadata["turn_id"] = json!("second");
        body["client_metadata"]["x-codex-turn-metadata"] = json!(next_metadata.to_string());
        let next = prepare_request(connection.clone(), &HeaderMap::new(), &body, None)
            .unwrap()
            .unwrap();
        assert_ne!(
            next.generation
                .lock_or_recover()
                .identity
                .as_ref()
                .unwrap()
                .nonce,
            first_nonce
        );
        assert_eq!(next.generation.lock_or_recover().expected.count, 2);
        runtime.begin_generation(&next).unwrap();
        for field in ["session_id", "thread_id", "window_id", "context_window_id"] {
            let mut other_metadata = next_metadata.clone();
            other_metadata[field] = json!("other");
            body["client_metadata"]["x-codex-turn-metadata"] = json!(other_metadata.to_string());
            assert!(matches!(
                prepare_request(connection.clone(), &HeaderMap::new(), &body, None),
                Err("unknown response continuation")
            ));
        }
    }

    fn complete(request: &RequestState, response_id: &str) {
        let generation = request.generation.lock_or_recover();
        *request.connection.continuation.lock_or_recover() = Some(state::Continuation {
            identity: generation.identity.clone(),
            response_id: response_id.into(),
            provider_id: 1,
            upstream_ws: true,
            history: generation.expected.clone(),
        });
        drop(generation);
        request.connection.runtime.finish_generation(request, true);
    }

    #[test]
    fn ordinary_ws_continuation_is_socket_scoped_without_recovery_metadata() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let connection = runtime.connection().unwrap();
        let headers = HeaderMap::new();
        let mut body = json!({"type":"response.create","model":"model","input":[{"role":"user","content":"hello"}]});
        let first = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        assert!(first.generation.lock_or_recover().identity.is_none());
        runtime.begin_generation(&first).unwrap();
        assert!(runtime.suspend(&first).is_err());
        complete(&first, "resp_first");
        body["previous_response_id"] = json!("resp_first");
        // Partial metadata is insufficient for recovery, but not a transport error.
        body["client_metadata"] = json!({"x-codex-turn-metadata":"{\"turn_id\":\"turn\"}"});
        let next = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&next).unwrap();
        assert_eq!(next.generation.lock_or_recover().expected.count, 2);
        assert!(matches!(
            prepare_request(runtime.connection().unwrap(), &headers, &body, None),
            Err("unknown response continuation")
        ));
        runtime.invalidate();
        assert!(runtime.begin_generation(&next).is_err());
    }

    #[test]
    fn socket_owned_nonce_admits_full_resends_and_rotates_across_turns() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let connection = runtime.connection().unwrap();
        let headers = HeaderMap::new();
        let mut metadata = json!({"session_id":"s","thread_id":"t","window_id":"w","context_window_id":"c","turn_id":"first"});
        let input = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]});
        let mut body = json!({"type":"response.create","model":"model","input":[input],"client_metadata":{"x-codex-turn-metadata":metadata.to_string()}});
        let first = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&first).unwrap();
        complete(&first, "resp_first");
        let nonce = first
            .generation
            .lock_or_recover()
            .identity
            .as_ref()
            .unwrap()
            .nonce
            .clone();
        // A full resend without the echoed nonce is the client's authoritative history.
        let replay = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&replay).unwrap();
        runtime.finish_generation(&replay, false);
        body["input"].as_array_mut().unwrap().push(input);
        let extended = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            extended
                .generation
                .lock_or_recover()
                .identity
                .as_ref()
                .unwrap()
                .nonce,
            nonce
        );
        body["previous_response_id"] = json!("resp_first");
        body["client_metadata"]["x-codex-turn-state"] = json!("aio-ws-other");
        assert!(matches!(
            prepare_request(connection.clone(), &headers, &body, None),
            Err("Response continuation owner mismatch")
        ));
        metadata["turn_id"] = json!("second");
        body["client_metadata"]["x-codex-turn-metadata"] = json!(metadata.to_string());
        body["client_metadata"]["x-codex-turn-state"] = json!(nonce);
        let second = prepare_request(connection, &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&second).unwrap();
        assert_ne!(
            second
                .generation
                .lock_or_recover()
                .identity
                .as_ref()
                .unwrap()
                .nonce,
            nonce
        );
    }

    #[test]
    fn local_nonce_cannot_enter_as_an_unidentified_ws_request() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let connection = runtime.connection().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(protocol::TURN_STATE_HEADER, "aio-ws-local".parse().unwrap());
        let body = json!({"type":"response.create","input":[]});
        assert!(matches!(
            prepare_request(connection.clone(), &headers, &body, None),
            Err("missing or invalid Codex turn metadata for recovery")
        ));
        let mut body = body;
        body["client_metadata"] = json!({"x-codex-turn-state":"aio-ws-other"});
        assert!(matches!(
            prepare_request(connection.clone(), &headers, &body, None),
            Err("conflicting Responses owner nonce")
        ));
        assert!(matches!(
            prepare_request(connection, &HeaderMap::new(), &body, None),
            Err("missing or invalid Codex turn metadata for recovery")
        ));
    }
    #[test]
    fn unverified_history_allows_socket_continuation_and_a_fresh_full_rebuild() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let connection = runtime.connection().unwrap();
        let headers = HeaderMap::new();
        let metadata = json!({"session_id":"s","thread_id":"t","window_id":"w","context_window_id":"c","turn_id":"turn"});
        let input = json!({"type":"future_item","payload":"hello"});
        let mut body = json!({"type":"response.create","input":[input],"client_metadata":{"x-codex-turn-metadata":metadata.to_string()}});
        let first = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&first).unwrap();
        complete(&first, "resp_first");
        body["previous_response_id"] = json!("resp_first");
        let next = prepare_request(connection.clone(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&next).unwrap();
        assert!(runtime.suspend(&next).is_err());
        complete(&next, "resp_next");
        body.as_object_mut().unwrap().remove("previous_response_id");
        body["input"] = json!([input, input, input]);
        // Unverifiable history cannot claim a recovery grant, but still runs fresh.
        let rebuilt = prepare_request(connection, &headers, &body, None)
            .unwrap()
            .unwrap();
        assert!(!rebuilt.generation.lock_or_recover().recovered);
        runtime.begin_generation(&rebuilt).unwrap();
    }

    #[test]
    fn ws_recovery_header_retains_the_original_attempt_budget() {
        use serde_json::json;
        let runtime = Arc::new(state::Runtime::new(true));
        let metadata = json!({"session_id":"s","thread_id":"t","window_id":"w","context_window_id":"c","turn_id":"turn"});
        let body = json!({"type":"response.create","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}],"client_metadata":{"x-codex-turn-metadata":metadata.to_string()}});
        let first = prepare_request(
            runtime.connection().unwrap(),
            &HeaderMap::new(),
            &body,
            None,
        )
        .unwrap()
        .unwrap();
        runtime.begin_generation(&first).unwrap();
        let deadline = std::time::Instant::now();
        let nonce = {
            let mut generation = first.generation.lock_or_recover();
            generation.budget.deadline = Some(deadline);
            generation.budget.retry_index = 2;
            generation.trace_id = "first_trace".into();
            generation.identity.as_ref().unwrap().nonce.clone()
        };
        runtime.suspend(&first).unwrap();
        drop(first);
        let mut headers = HeaderMap::new();
        headers.insert(protocol::TURN_STATE_HEADER, nonce.parse().unwrap());
        let recovered = prepare_request(runtime.connection().unwrap(), &headers, &body, None)
            .unwrap()
            .unwrap();
        runtime.begin_generation(&recovered).unwrap();
        let generation = recovered.generation.lock_or_recover();
        assert!(generation.recovered);
        assert_eq!(generation.budget.deadline, Some(deadline));
        assert_eq!(generation.budget.retry_index, 2);
        assert_eq!(generation.from_trace.as_deref(), Some("first_trace"));
    }
}
