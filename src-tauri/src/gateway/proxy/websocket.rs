//! Usage: WebSocket upgrade proxy path for gateway requests.

use super::cli_proxy_guard::cli_proxy_enabled_cached;
use super::errors::error_response;
use super::failover::select_provider_base_url_for_request;
use super::logging::enqueue_request_log_placeholder;
use super::provider_router;
use super::{
    compute_observe_request, mark_internal_forwarded_request,
    spawn_enqueue_request_log_with_backpressure, ErrorCategory, GatewayErrorCode,
    RequestLogEnqueueArgs,
};
use crate::gateway::events::{
    decision_chain as dc, emit_attempt_event, emit_gateway_debug_log_lazy,
    emit_request_start_event, FailoverAttempt, GatewayAttemptEvent,
};
use crate::gateway::runtime::GatewayAppState;
use crate::gateway::util::{
    build_target_url, clear_all_auth_headers, ensure_cli_required_headers,
    infer_requested_model_info, now_unix_millis, now_unix_seconds, redacted_headers_for_debug,
};
use axum::extract::ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

type UpstreamWsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const WEBSOCKET_TRACE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const WEBSOCKET_UPSTREAM_DRAIN_TIMEOUT: Duration = Duration::from_millis(1200);
const WEBSOCKET_SPECIAL_SETTINGS_JSON: &str = r#"[{"type":"websocket_proxy"}]"#;
const WEBSOCKET_RECONNECT_WINDOW_MS: i64 = 15 * 60 * 1000;
const WEBSOCKET_UNSUPPORTED_CACHE_SECS: i64 = 10 * 60;

static WEBSOCKET_CONNECTION_REGISTRY: OnceLock<Mutex<HashMap<String, WebsocketConnectionStats>>> =
    OnceLock::new();
static WEBSOCKET_UNSUPPORTED_CACHE: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
static WEBSOCKET_ACTIVE_CONNECTIONS: OnceLock<
    Mutex<HashMap<i64, HashMap<String, watch::Sender<u64>>>>,
> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct WebsocketConnectionStats {
    count: usize,
    last_seen_ms: i64,
}

struct WebsocketObservation<R: tauri::Runtime = tauri::Wry> {
    state: GatewayAppState<R>,
    trace_id: String,
    cli_key: String,
    method: String,
    path: String,
    query: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
    observe_request: bool,
    created_at_ms: i64,
    created_at: i64,
    started: Instant,
    provider_id: i64,
    provider_name: String,
    provider_base_url: String,
}

struct PreparedWebsocketUpstream {
    provider_id: i64,
    provider_name: String,
    provider_base_url: String,
    upstream_socket: UpstreamWsStream,
}

struct WebsocketCompletion {
    outcome: &'static str,
    status: Option<u16>,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
    reason: Option<String>,
    ttfb_ms: Option<u128>,
}

#[derive(Default)]
struct WebsocketRelayMetadata {
    requested_model: Option<String>,
    usage: Option<crate::usage::UsageExtract>,
    first_token_ms: Option<u128>,
    completed_turns: usize,
}

#[derive(Default)]
struct WebsocketRelayState {
    metadata: WebsocketRelayMetadata,
    turn_timings: HashMap<String, WebsocketTurnTiming>,
    active_response_id: Option<String>,
}

struct WebsocketTurnTiming {
    started_ms: u128,
    first_token_ms: Option<u128>,
}

struct WebsocketTurnCompletion {
    response_id: Option<String>,
    terminal_event: String,
    turn_index: usize,
    started_ms: u128,
    duration_ms: u128,
    ttfb_ms: Option<u128>,
    requested_model: Option<String>,
    usage: Option<crate::usage::UsageExtract>,
}

impl WebsocketCompletion {
    fn success(ttfb_ms: Option<u128>) -> Self {
        Self {
            outcome: "success",
            status: Some(StatusCode::SWITCHING_PROTOCOLS.as_u16()),
            error_category: None,
            error_code: None,
            reason: None,
            ttfb_ms,
        }
    }
}

pub(in crate::gateway) async fn proxy_websocket_impl<R>(
    state: GatewayAppState<R>,
    cli_key: String,
    forwarded_path: String,
    ws: WebSocketUpgrade,
    req: Request<axum::body::Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let started = Instant::now();
    let created_at_ms = now_unix_millis() as i64;
    let created_at = (created_at_ms / 1000).max(0);
    let method = req.method().to_string();
    let query = req.uri().query().map(str::to_string);
    let headers = req.headers().clone();
    let session_id = extract_websocket_session_id(&headers);
    let requested_model = infer_requested_model_info(&forwarded_path, query.as_deref(), None).model;
    let observe_request = compute_observe_request(&cli_key, &forwarded_path, &headers, None);
    let forced_provider_id = headers
        .get("x-aio-provider-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0);
    let trace_id = crate::gateway::util::new_trace_id();

    emit_gateway_debug_log_lazy(&state.app, || {
        format!(
            "[WS_REQ] trace_id={} cli_key={} method={} path={} model={}\n  headers={}",
            trace_id,
            cli_key,
            method,
            forwarded_path,
            requested_model.as_deref().unwrap_or("-"),
            redacted_headers_for_debug(&headers),
        )
    });

    let cli_proxy = cli_proxy_enabled_cached(&state.app, &cli_key);
    if !cli_proxy.enabled {
        return error_response(
            StatusCode::FORBIDDEN,
            trace_id,
            GatewayErrorCode::CliProxyDisabled.as_str(),
            format!("CLI proxy is disabled for cli_key={cli_key}"),
            vec![],
        );
    }

    let prepared = match prepare_websocket_upstream(
        &state,
        &cli_key,
        forced_provider_id,
        &forwarded_path,
        query.as_deref(),
        &headers,
        &trace_id,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(err) => {
            return error_response(err.0, trace_id, err.1.as_str(), err.2, vec![]);
        }
    };
    let observation = WebsocketObservation {
        state,
        trace_id,
        cli_key,
        method,
        path: forwarded_path,
        query,
        session_id,
        requested_model,
        observe_request,
        created_at_ms,
        created_at,
        started,
        provider_id: prepared.provider_id,
        provider_name: prepared.provider_name,
        provider_base_url: prepared.provider_base_url,
    };
    log_websocket_connection_start(&observation);
    emit_websocket_request_start(&observation);
    enqueue_websocket_request_placeholder(&observation).await;

    ws.on_upgrade(move |socket| async move {
        relay_websocket(socket, prepared.upstream_socket, observation).await;
    })
}

fn extract_websocket_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("session_id")
        .or_else(|| headers.get("x-session-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_websocket_provider<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    cli_key: &str,
    forced_provider_id: Option<i64>,
) -> Result<crate::providers::ProviderForGateway, (StatusCode, GatewayErrorCode, String)> {
    let mut providers =
        crate::providers::list_enabled_for_gateway_using_active_mode(&state.db, cli_key)
            .map(|selection| selection.providers)
            .map_err(|err| {
                (
                    StatusCode::BAD_REQUEST,
                    GatewayErrorCode::InvalidCliKey,
                    err.to_string(),
                )
            })?;
    if providers.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            GatewayErrorCode::NoEnabledProvider,
            format!("no enabled provider for cli_key={cli_key}"),
        ));
    }
    if let Some(provider_id) = forced_provider_id {
        return providers
            .into_iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    GatewayErrorCode::NoEnabledProvider,
                    format!("no enabled provider for cli_key={cli_key}, provider_id={provider_id}"),
                )
            });
    }
    Ok(providers.remove(0))
}

async fn prepare_websocket_upstream<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    cli_key: &str,
    forced_provider_id: Option<i64>,
    forwarded_path: &str,
    query: Option<&str>,
    inbound_headers: &HeaderMap,
    trace_id: &str,
) -> Result<PreparedWebsocketUpstream, (StatusCode, GatewayErrorCode, String)> {
    let providers = match forced_provider_id {
        Some(_) => vec![resolve_websocket_provider(
            state,
            cli_key,
            forced_provider_id,
        )?],
        None => crate::providers::list_enabled_for_gateway_using_active_mode(&state.db, cli_key)
            .map(|selection| selection.providers)
            .map_err(|err| {
                (
                    StatusCode::BAD_REQUEST,
                    GatewayErrorCode::InvalidCliKey,
                    err.to_string(),
                )
            })?,
    };
    if providers.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            GatewayErrorCode::NoEnabledProvider,
            format!("no enabled provider for cli_key={cli_key}"),
        ));
    }

    let mut earliest_available_unix = None;
    let mut skipped_open = 0usize;
    let mut skipped_cooldown = 0usize;
    let mut skipped_websocket_disabled = 0usize;
    let mut attempted_websocket_provider = false;
    let mut last_error = None;

    for provider in providers {
        if !provider.supports_websockets {
            skipped_websocket_disabled += 1;
            let reason = format!(
                "websocket disabled for provider={} (id={})",
                provider.name, provider.id
            );
            tracing::info!(
                target: "gateway_debug",
                trace_id = %trace_id,
                provider_id = provider.id,
                provider = %provider.name,
                "[WS_PROVIDER_SKIP] provider websocket disabled"
            );
            emit_gateway_debug_log_lazy(&state.app, || {
                format!(
                    "[WS_PROVIDER_SKIP] trace_id={} provider={} (id={}) reason=websocket_disabled",
                    trace_id, provider.name, provider.id,
                )
            });
            last_error = Some((GatewayErrorCode::NoEnabledProvider, reason));
            continue;
        }
        attempted_websocket_provider = true;

        let base_url =
            match select_provider_base_url_for_request(state, &provider, cli_key, 1).await {
                Ok(value) => value,
                Err(err) => {
                    last_error = Some((GatewayErrorCode::InvalidBaseUrl, err));
                    continue;
                }
            };
        let now_unix = now_unix_seconds() as i64;
        if websocket_unsupported_cache_hit(provider.id, &base_url, now_unix) {
            let reason = format!(
                "websocket unsupported by upstream cache provider={} (id={}) base_url={}",
                provider.name, provider.id, base_url
            );
            tracing::info!(
                target: "gateway_debug",
                trace_id = %trace_id,
                provider_id = provider.id,
                provider = %provider.name,
                base_url = %base_url,
                "[WS_UNSUPPORTED] cached websocket unsupported provider skipped"
            );
            last_error = Some((GatewayErrorCode::UpstreamConnectFailed, reason));
            continue;
        }
        let gate = provider_router::gate_provider(provider_router::GateProviderArgs {
            app: Some(&state.app),
            circuit: state.circuit.as_ref(),
            trace_id,
            cli_key,
            provider_id: provider.id,
            provider_name: provider.name.as_str(),
            provider_base_url_display: base_url.as_str(),
            now_unix,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
        });
        if gate.is_none() {
            continue;
        }

        let target_url = match build_websocket_url(&base_url, forwarded_path, query) {
            Ok(value) => value,
            Err(err) => {
                last_error = Some((GatewayErrorCode::InvalidBaseUrl, err));
                continue;
            }
        };
        let upstream_headers = match build_upstream_websocket_headers(
            state,
            cli_key,
            inbound_headers,
            &provider,
        )
        .await
        {
            Ok(headers) => headers,
            Err(err) => {
                last_error = Some((GatewayErrorCode::InternalError, err));
                continue;
            }
        };
        let request = match build_tungstenite_request(target_url, upstream_headers) {
            Ok(request) => request,
            Err(err) => {
                last_error = Some((GatewayErrorCode::InternalError, err));
                continue;
            }
        };

        match tokio_tungstenite::connect_async(request).await {
            Ok((upstream_socket, response)) => {
                emit_gateway_debug_log_lazy(&state.app, || {
                    format!(
                        "[WS_UPSTREAM_RESP] trace_id={} status={} provider={} (id={})\n  headers={}",
                        trace_id,
                        response.status().as_u16(),
                        provider.name,
                        provider.id,
                        redacted_headers_for_debug(response.headers()),
                    )
                });
                provider_router::record_success_and_emit_transition(
                    provider_router::RecordCircuitArgs::from_state(
                        state,
                        trace_id,
                        cli_key,
                        provider.id,
                        provider.name.as_str(),
                        base_url.as_str(),
                        now_unix_seconds() as i64,
                    ),
                );
                return Ok(PreparedWebsocketUpstream {
                    provider_id: provider.id,
                    provider_name: provider.name,
                    provider_base_url: base_url,
                    upstream_socket,
                });
            }
            Err(err) => {
                let reason = err.to_string();
                if let Some(status) = websocket_unsupported_status_from_error(&err) {
                    remember_websocket_unsupported(provider.id, &base_url, now_unix);
                    tracing::info!(
                        target: "gateway_debug",
                        trace_id = %trace_id,
                        provider_id = provider.id,
                        provider = %provider.name,
                        base_url = %base_url,
                        status,
                        "[WS_UNSUPPORTED] upstream does not support websocket"
                    );
                }
                emit_gateway_debug_log_lazy(&state.app, || {
                    format!(
                        "[WS_UPSTREAM_FAIL] trace_id={} provider={} (id={}) reason={}",
                        trace_id, provider.name, provider.id, reason,
                    )
                });
                tracing::warn!(
                    trace_id = %trace_id,
                    provider_id = provider.id,
                    provider_name = %provider.name,
                    "websocket upstream connect failed without circuit failure, trying next provider: {reason}"
                );
                last_error = Some((GatewayErrorCode::UpstreamConnectFailed, reason));
            }
        }
    }

    let no_websocket_provider_enabled =
        skipped_websocket_disabled > 0 && !attempted_websocket_provider;
    let message = if skipped_open > 0 || skipped_cooldown > 0 {
        match earliest_available_unix {
            Some(until) => format!(
                "no websocket provider available for cli_key={cli_key}; skipped_open={skipped_open}, skipped_cooldown={skipped_cooldown}, earliest_available_unix={until}"
            ),
            None => format!(
                "no websocket provider available for cli_key={cli_key}; skipped_open={skipped_open}, skipped_cooldown={skipped_cooldown}"
            ),
        }
    } else if no_websocket_provider_enabled {
        match forced_provider_id {
            Some(provider_id) => format!(
                "no websocket-enabled provider for cli_key={cli_key}, provider_id={provider_id}; skipped_websocket_disabled={skipped_websocket_disabled}"
            ),
            None => format!(
                "no websocket-enabled provider for cli_key={cli_key}; skipped_websocket_disabled={skipped_websocket_disabled}"
            ),
        }
    } else {
        last_error
            .as_ref()
            .map(|(_, err)| err.clone())
            .unwrap_or_else(|| format!("no websocket provider available for cli_key={cli_key}"))
    };
    let code = last_error
        .map(|(code, _)| code)
        .unwrap_or(GatewayErrorCode::NoEnabledProvider);
    let status = if no_websocket_provider_enabled {
        StatusCode::UPGRADE_REQUIRED
    } else {
        StatusCode::BAD_GATEWAY
    };
    emit_gateway_debug_log_lazy(&state.app, || {
        format!(
            "[WS_RESP] trace_id={} status={} error_code={} message={}",
            trace_id,
            status.as_u16(),
            code.as_str(),
            message,
        )
    });
    Err((status, code, message))
}

fn build_websocket_url(
    base_url: &str,
    forwarded_path: &str,
    query: Option<&str>,
) -> Result<reqwest::Url, String> {
    let mut url = build_target_url(base_url, forwarded_path, query)?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        "ws" => "ws",
        "wss" => "wss",
        other => return Err(format!("unsupported websocket base_url scheme: {other}")),
    };
    url.set_scheme(scheme)
        .map_err(|_| format!("failed to set websocket URL scheme: {scheme}"))?;
    Ok(url)
}

async fn build_upstream_websocket_headers<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    cli_key: &str,
    inbound: &HeaderMap,
    provider: &crate::providers::ProviderForGateway,
) -> Result<HeaderMap, String> {
    let mut headers = build_base_upstream_websocket_headers(cli_key, inbound);
    inject_websocket_provider_auth(state, cli_key, provider, &mut headers).await?;
    Ok(headers)
}

fn build_base_upstream_websocket_headers(cli_key: &str, inbound: &HeaderMap) -> HeaderMap {
    let mut headers = inbound.clone();
    clear_hop_and_websocket_handshake_headers(&mut headers);
    headers.remove(header::HOST);
    headers.remove(header::CONTENT_LENGTH);
    clear_all_auth_headers(&mut headers);
    ensure_cli_required_headers(cli_key, &mut headers);
    if cli_key == "claude" {
        mark_internal_forwarded_request(&mut headers);
    }
    headers
}

fn clear_hop_and_websocket_handshake_headers(headers: &mut HeaderMap) {
    headers.remove(header::CONNECTION);
    headers.remove(header::UPGRADE);
    headers.remove(header::SEC_WEBSOCKET_ACCEPT);
    headers.remove(header::SEC_WEBSOCKET_KEY);
    headers.remove(header::SEC_WEBSOCKET_VERSION);
    headers.remove("sec-websocket-extensions");
    headers.remove("proxy-connection");
    headers.remove(header::PROXY_AUTHENTICATE);
    headers.remove(header::PROXY_AUTHORIZATION);
    headers.remove(header::TE);
    headers.remove(header::TRAILER);
    headers.remove(header::TRANSFER_ENCODING);
}

async fn inject_websocket_provider_auth<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    cli_key: &str,
    provider: &crate::providers::ProviderForGateway,
    headers: &mut HeaderMap,
) -> Result<(), String> {
    if provider.auth_mode == "oauth" {
        let details = crate::providers::get_oauth_details(&state.db, provider.id)
            .map_err(|err| err.to_string())?;
        if details.cli_key != cli_key {
            return Err(format!(
                "SEC_INVALID_STATE: oauth details cli_key mismatch for provider_id={} (expected={cli_key}, actual={})",
                provider.id, details.cli_key
            ));
        }

        let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter(
            cli_key,
            provider.id,
            Some(details.oauth_provider_type.as_str()),
        )?;
        let token = details.oauth_access_token.trim();
        if token.is_empty() {
            return Err("SEC_INVALID_INPUT: oauth access_token is empty".to_string());
        }
        adapter.inject_upstream_headers(headers, token)?;
        return Ok(());
    }

    if provider.api_key_plaintext.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: provider api_key is empty".to_string());
    }

    crate::gateway::util::inject_provider_auth(cli_key, &provider.api_key_plaintext, headers);
    Ok(())
}

async fn relay_websocket<R: tauri::Runtime>(
    client_socket: WebSocket,
    upstream_socket: UpstreamWsStream,
    observation: WebsocketObservation<R>,
) {
    let mut provider_close_rx =
        register_active_websocket_connection(observation.provider_id, &observation.trace_id);
    let relay_metadata = relay_websocket_messages(
        client_socket,
        upstream_socket,
        &observation,
        &mut provider_close_rx,
    )
    .await;
    unregister_active_websocket_connection(observation.provider_id, &observation.trace_id);
    let ttfb_ms = relay_metadata.first_token_ms;
    record_websocket_request_end(
        observation,
        WebsocketCompletion::success(ttfb_ms),
        relay_metadata,
    );
}

pub(in crate::gateway) fn close_active_provider_websockets(provider_id: i64) -> usize {
    let registry = WEBSOCKET_ACTIVE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut registry) = registry.lock() else {
        return 0;
    };
    let Some(connections) = registry.remove(&provider_id) else {
        return 0;
    };
    let count = connections.len();
    for tx in connections.into_values() {
        let _ = tx.send(1);
    }
    count
}

fn register_active_websocket_connection(provider_id: i64, trace_id: &str) -> watch::Receiver<u64> {
    let (tx, rx) = watch::channel(0);
    let registry = WEBSOCKET_ACTIVE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut registry) = registry.lock() {
        registry
            .entry(provider_id)
            .or_default()
            .insert(trace_id.to_string(), tx);
    }
    rx
}

fn unregister_active_websocket_connection(provider_id: i64, trace_id: &str) {
    let Some(registry) = WEBSOCKET_ACTIVE_CONNECTIONS.get() else {
        return;
    };
    let Ok(mut registry) = registry.lock() else {
        return;
    };
    let Some(connections) = registry.get_mut(&provider_id) else {
        return;
    };
    connections.remove(trace_id);
    if connections.is_empty() {
        registry.remove(&provider_id);
    }
}

fn emit_websocket_request_start<R: tauri::Runtime>(observation: &WebsocketObservation<R>) {
    if !observation.observe_request {
        return;
    }

    emit_request_start_event(
        &observation.state.app,
        observation.trace_id.clone(),
        observation.cli_key.clone(),
        observation.session_id.clone(),
        observation.method.clone(),
        observation.path.clone(),
        observation.query.clone(),
        observation.requested_model.clone(),
        observation.created_at,
    );

    let attempt = build_websocket_attempt(observation, "started", None, None, None, None, 0);
    emit_websocket_attempt_event(observation, &attempt, 0);
}

async fn enqueue_websocket_request_placeholder<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
) {
    if !observation.observe_request {
        return;
    }

    let attempt = build_websocket_attempt(observation, "started", None, None, None, None, 0);
    let attempts_json = serde_json::to_string(&[attempt]).unwrap_or_else(|_| "[]".to_string());
    let args = RequestLogEnqueueArgs {
        trace_id: observation.trace_id.clone(),
        cli_key: observation.cli_key.clone(),
        session_id: observation.session_id.clone(),
        method: observation.method.clone(),
        path: observation.path.clone(),
        query: observation.query.clone(),
        excluded_from_stats: true,
        special_settings_json: Some(WEBSOCKET_SPECIAL_SETTINGS_JSON.to_string()),
        status: None,
        error_code: None,
        duration_ms: 0,
        ttfb_ms: None,
        attempts_json,
        requested_model: observation.requested_model.clone(),
        created_at_ms: observation.created_at_ms,
        created_at: observation.created_at,
        usage_metrics: None,
        usage: None,
        provider_chain_json: None,
        error_details_json: None,
    };

    enqueue_request_log_placeholder(&observation.state.app, &observation.state.log_tx, args).await;
}

fn log_websocket_connection_start<R: tauri::Runtime>(observation: &WebsocketObservation<R>) {
    let reconnect_count = register_websocket_connection(
        &observation.cli_key,
        observation.session_id.as_deref(),
        observation.created_at_ms,
    );
    tracing::info!(
        target: "gateway_debug",
        trace_id = %observation.trace_id,
        cli_key = %observation.cli_key,
        session_id = observation.session_id.as_deref().unwrap_or("-"),
        provider_id = observation.provider_id,
        provider = %observation.provider_name,
        reconnect_count = reconnect_count.unwrap_or(0),
        "[WS_CONN] websocket connection started"
    );
    if let Some(reconnect_count) = reconnect_count.filter(|value| *value > 0) {
        tracing::info!(
            target: "gateway_debug",
            trace_id = %observation.trace_id,
            cli_key = %observation.cli_key,
            session_id = observation.session_id.as_deref().unwrap_or("-"),
            provider_id = observation.provider_id,
            provider = %observation.provider_name,
            reconnect_count,
            "[WS_RECONNECT] websocket reconnect detected"
        );
    }
}

fn register_websocket_connection(
    cli_key: &str,
    session_id: Option<&str>,
    now_ms: i64,
) -> Option<usize> {
    let session_id = session_id?.trim();
    if session_id.is_empty() {
        return None;
    }
    let key = format!("{cli_key}:{session_id}");
    let registry = WEBSOCKET_CONNECTION_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut registry) = registry.lock() else {
        return None;
    };
    registry.retain(|_, stats| {
        now_ms.saturating_sub(stats.last_seen_ms) <= WEBSOCKET_RECONNECT_WINDOW_MS
    });
    let entry = registry.entry(key).or_insert(WebsocketConnectionStats {
        count: 0,
        last_seen_ms: now_ms,
    });
    let reconnect_count = entry.count;
    entry.count = entry.count.saturating_add(1);
    entry.last_seen_ms = now_ms;
    Some(reconnect_count)
}

fn websocket_unsupported_status_from_error(err: &tungstenite::Error) -> Option<u16> {
    let status = match err {
        tungstenite::Error::Http(response) => response.status(),
        _ => return None,
    };
    matches!(
        status,
        StatusCode::BAD_REQUEST
            | StatusCode::NOT_FOUND
            | StatusCode::METHOD_NOT_ALLOWED
            | StatusCode::UPGRADE_REQUIRED
            | StatusCode::NOT_IMPLEMENTED
    )
    .then_some(status.as_u16())
}

fn websocket_unsupported_cache_key(provider_id: i64, base_url: &str) -> String {
    format!("{provider_id}:{}", base_url.trim_end_matches('/'))
}

fn websocket_unsupported_cache_hit(provider_id: i64, base_url: &str, now_unix: i64) -> bool {
    let cache = WEBSOCKET_UNSUPPORTED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut cache) = cache.lock() else {
        return false;
    };
    cache.retain(|_, expires_at| *expires_at > now_unix);
    cache.contains_key(&websocket_unsupported_cache_key(provider_id, base_url))
}

fn remember_websocket_unsupported(provider_id: i64, base_url: &str, now_unix: i64) {
    let cache = WEBSOCKET_UNSUPPORTED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut cache) = cache.lock() {
        cache.insert(
            websocket_unsupported_cache_key(provider_id, base_url),
            now_unix.saturating_add(WEBSOCKET_UNSUPPORTED_CACHE_SECS),
        );
    }
}

fn record_websocket_request_end<R: tauri::Runtime>(
    observation: WebsocketObservation<R>,
    completion: WebsocketCompletion,
    relay_metadata: WebsocketRelayMetadata,
) {
    let duration_ms = observation.started.elapsed().as_millis();
    tracing::info!(
        target: "gateway_debug",
        trace_id = %observation.trace_id,
        cli_key = %observation.cli_key,
        session_id = observation.session_id.as_deref().unwrap_or("-"),
        provider_id = observation.provider_id,
        provider = %observation.provider_name,
        status = completion
            .status
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        outcome = completion.outcome,
        duration_ms,
        ttfb_ms = completion
            .ttfb_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        completed_turns = relay_metadata.completed_turns,
        "[WS_CONN] websocket connection ended"
    );
    emit_gateway_debug_log_lazy(&observation.state.app, || {
        format!(
            "[WS_RESP] trace_id={} status={} outcome={} duration_ms={} ttfb_ms={} completed_turns={} provider={} (id={})",
            observation.trace_id,
            completion
                .status
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            completion.outcome,
            duration_ms,
            completion
                .ttfb_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            relay_metadata.completed_turns,
            observation.provider_name,
            observation.provider_id,
        )
    });

    if !observation.observe_request {
        return;
    }

    let attempt = build_websocket_attempt(
        &observation,
        completion.outcome,
        completion.status,
        completion.error_category,
        completion.error_code,
        completion.reason,
        duration_ms,
    );
    emit_websocket_attempt_event(&observation, &attempt, duration_ms);

    let requested_model = relay_metadata
        .requested_model
        .or(observation.requested_model.clone());
    let usage = if relay_metadata.completed_turns > 0 {
        None
    } else {
        relay_metadata.usage
    };
    let usage_metrics = usage.as_ref().map(|extract| extract.metrics.clone());

    let excluded_from_stats = completion.status == Some(StatusCode::SWITCHING_PROTOCOLS.as_u16());
    let (log_args, attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
        &observation.trace_id,
        &observation.cli_key,
        observation.session_id,
        &observation.method,
        &observation.path,
        observation.query.as_deref(),
        excluded_from_stats,
        Some(WEBSOCKET_SPECIAL_SETTINGS_JSON.to_string()),
        completion.status,
        completion.error_code,
        duration_ms,
        completion.ttfb_ms,
        &[attempt],
        requested_model,
        observation.created_at_ms,
        observation.created_at,
        usage_metrics.clone(),
        usage,
    );

    log_args.emit_gateway_request_event(
        &observation.state.app,
        completion.error_category,
        completion.ttfb_ms,
        attempts,
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        observation.state.app,
        observation.state.db,
        observation.state.log_tx,
        log_args,
    );
}

fn build_websocket_attempt<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
    outcome: &'static str,
    status: Option<u16>,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
    reason: Option<String>,
    duration_ms: u128,
) -> FailoverAttempt {
    let reason_code = if outcome == "success" {
        Some(dc::success_reason_code(1, 1))
    } else {
        error_category.and_then(|_| {
            error_code.map(|_| {
                if error_category == Some(ErrorCategory::ProviderError.as_str()) {
                    ErrorCategory::ProviderError.reason_code()
                } else {
                    ErrorCategory::SystemError.reason_code()
                }
            })
        })
    };

    FailoverAttempt {
        provider_id: observation.provider_id,
        provider_name: observation.provider_name.clone(),
        base_url: observation.provider_base_url.clone(),
        outcome: outcome.to_string(),
        status,
        provider_index: Some(1),
        retry_index: Some(1),
        session_reuse: Some(false),
        error_category,
        error_code,
        decision: None,
        reason,
        selection_method: dc::selection_method(1, 1, Some(false)),
        reason_code,
        attempt_started_ms: Some(0),
        attempt_duration_ms: Some(duration_ms),
        circuit_state_before: None,
        circuit_state_after: None,
        circuit_failure_count: None,
        circuit_failure_threshold: None,
    }
}

fn emit_websocket_attempt_event<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
    attempt: &FailoverAttempt,
    duration_ms: u128,
) {
    emit_attempt_event(
        &observation.state.app,
        GatewayAttemptEvent {
            trace_id: observation.trace_id.clone(),
            cli_key: observation.cli_key.clone(),
            session_id: observation.session_id.clone(),
            method: observation.method.clone(),
            path: observation.path.clone(),
            query: observation.query.clone(),
            requested_model: observation.requested_model.clone(),
            attempt_index: 1,
            provider_id: attempt.provider_id,
            session_reuse: attempt.session_reuse,
            provider_name: attempt.provider_name.clone(),
            base_url: attempt.base_url.clone(),
            outcome: attempt.outcome.clone(),
            status: attempt.status,
            attempt_started_ms: 0,
            attempt_duration_ms: duration_ms,
            circuit_state_before: None,
            circuit_state_after: None,
            circuit_failure_count: None,
            circuit_failure_threshold: None,
            claude_model_mapping: None,
        },
    );
}

fn build_tungstenite_request(
    target_url: reqwest::Url,
    headers: HeaderMap,
) -> Result<tungstenite::handshake::client::Request, String> {
    let mut request = target_url
        .as_str()
        .into_client_request()
        .map_err(|err| err.to_string())?;

    for (name, value) in headers.iter() {
        request.headers_mut().insert(name.clone(), value.clone());
    }

    Ok(request)
}

async fn relay_websocket_messages<R: tauri::Runtime>(
    client_socket: WebSocket,
    upstream_socket: UpstreamWsStream,
    observation: &WebsocketObservation<R>,
    provider_close_rx: &mut watch::Receiver<u64>,
) -> WebsocketRelayMetadata {
    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut upstream_tx, mut upstream_rx) = upstream_socket.split();
    let mut relay_state = WebsocketRelayState::default();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + WEBSOCKET_TRACE_HEARTBEAT_INTERVAL,
        WEBSOCKET_TRACE_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut drop_downstream = false;
    let mut drain_deadline: Option<tokio::time::Instant> = None;

    loop {
        tokio::select! {
            changed = provider_close_rx.changed() => {
                if changed.is_ok() {
                    emit_websocket_relay_debug(
                        observation,
                        "websocket provider config changed, closing active connection",
                    );
                }
                break;
            }
            _ = heartbeat.tick() => {
                emit_websocket_progress_event(observation);
            }
            _ = wait_websocket_drain_deadline(drain_deadline), if drain_deadline.is_some() => {
                emit_websocket_relay_debug(
                    observation,
                    "websocket upstream drain timeout after client disconnect",
                );
                break;
            }
            msg = client_rx.next(), if !drop_downstream => {
                let Some(msg) = msg else {
                    start_websocket_upstream_drain(
                        observation,
                        "client stream ended",
                        &mut drop_downstream,
                        &mut drain_deadline,
                    );
                    continue;
                };
                match msg {
                    Ok(message) => {
                        let is_close = matches!(message, AxumWsMessage::Close(_));
                        if is_close {
                            start_websocket_upstream_drain(
                                observation,
                                "client close frame",
                                &mut drop_downstream,
                                &mut drain_deadline,
                            );
                            continue;
                        }
                        if let Err(err) = upstream_tx.send(axum_to_tungstenite(message)).await {
                            emit_websocket_relay_debug(
                                observation,
                                format!("websocket client-to-upstream send failed: {err}"),
                            );
                            break;
                        }
                    }
                    Err(err) => {
                        emit_websocket_relay_debug(
                            observation,
                            format!("websocket client receive failed: {err}"),
                        );
                        start_websocket_upstream_drain(
                            observation,
                            "client receive failed",
                            &mut drop_downstream,
                            &mut drain_deadline,
                        );
                        continue;
                    }
                }
            }
            msg = upstream_rx.next() => {
                let Some(msg) = msg else {
                    let _ = client_tx.close().await;
                    break;
                };
                match msg {
                    Ok(message) => {
                        let Some(message) = tungstenite_to_axum(message) else {
                            continue;
                        };
                        let turn_completion = capture_websocket_upstream_metadata(
                            &message,
                            &observation.cli_key,
                            observation.started.elapsed().as_millis(),
                            &mut relay_state,
                        );
                        if let Some(turn_completion) = turn_completion {
                            record_websocket_turn_end(observation, turn_completion);
                            if drop_downstream {
                                emit_websocket_relay_debug(
                                    observation,
                                    "websocket upstream drain captured terminal event",
                                );
                                break;
                            }
                        }
                        if drop_downstream {
                            continue;
                        }
                        let is_close = matches!(message, AxumWsMessage::Close(_));
                        if let Err(err) = client_tx.send(message).await {
                            emit_websocket_relay_debug(
                                observation,
                                format!("websocket upstream-to-client send failed: {err}"),
                            );
                            start_websocket_upstream_drain(
                                observation,
                                "client send failed",
                                &mut drop_downstream,
                                &mut drain_deadline,
                            );
                            continue;
                        }
                        if is_close {
                            break;
                        }
                    }
                    Err(err) => {
                        emit_websocket_relay_debug(
                            observation,
                            format!("websocket upstream receive failed: {err}"),
                        );
                        break;
                    }
                }
            }
        }
    }

    relay_state.metadata
}

async fn wait_websocket_drain_deadline(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending::<()>().await,
    }
}

fn start_websocket_upstream_drain<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
    reason: &str,
    drop_downstream: &mut bool,
    drain_deadline: &mut Option<tokio::time::Instant>,
) {
    if *drop_downstream {
        return;
    }

    *drop_downstream = true;
    *drain_deadline = Some(tokio::time::Instant::now() + WEBSOCKET_UPSTREAM_DRAIN_TIMEOUT);
    emit_websocket_relay_debug(
        observation,
        format!("websocket client disconnected, draining upstream: {reason}"),
    );
}

fn emit_websocket_relay_debug<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
    message: impl Into<String>,
) {
    let message = message.into();
    emit_gateway_debug_log_lazy(&observation.state.app, || {
        format!("[WS_RELAY] trace_id={} {}", observation.trace_id, message)
    });
}

fn capture_websocket_upstream_metadata(
    message: &AxumWsMessage,
    cli_key: &str,
    elapsed_ms: u128,
    state: &mut WebsocketRelayState,
) -> Option<WebsocketTurnCompletion> {
    let bytes = match message {
        AxumWsMessage::Text(value) => value.as_bytes(),
        AxumWsMessage::Binary(value) => value.as_slice(),
        _ => return None,
    };

    if let Some(model) = crate::usage::parse_model_from_json_bytes(bytes) {
        state.metadata.requested_model = Some(model);
    }

    let Some(event_type) = parse_websocket_event_type(bytes) else {
        return None;
    };

    let response_id = parse_websocket_response_id(bytes, &event_type);
    if event_type == "response.created" {
        if let Some(response_id) = response_id.as_deref() {
            init_websocket_turn_timing(state, response_id, elapsed_ms);
        }
    }

    if is_websocket_token_event(&event_type) {
        if state.metadata.first_token_ms.is_none() {
            state.metadata.first_token_ms = Some(elapsed_ms);
        }
        mark_websocket_turn_first_token(state, response_id.as_deref(), elapsed_ms);
    }

    let usage = if should_parse_websocket_usage(&event_type) {
        crate::usage::parse_usage_from_json_or_sse_bytes(cli_key, bytes)
    } else {
        None
    };
    if let Some(usage) = usage.as_ref() {
        accumulate_websocket_usage(&mut state.metadata.usage, usage);
    }

    if !is_websocket_terminal_event(&event_type) {
        return None;
    }

    state.metadata.completed_turns = state.metadata.completed_turns.saturating_add(1);
    let turn_index = state.metadata.completed_turns;
    let timing = response_id
        .as_deref()
        .and_then(|id| state.turn_timings.remove(id))
        .unwrap_or(WebsocketTurnTiming {
            started_ms: elapsed_ms,
            first_token_ms: None,
        });
    if response_id
        .as_deref()
        .is_some_and(|id| state.active_response_id.as_deref() == Some(id))
    {
        state.active_response_id = None;
    }

    let duration_ms = elapsed_ms.saturating_sub(timing.started_ms);
    Some(WebsocketTurnCompletion {
        response_id,
        terminal_event: event_type,
        turn_index,
        started_ms: timing.started_ms,
        duration_ms,
        ttfb_ms: timing.first_token_ms,
        requested_model: state.metadata.requested_model.clone(),
        usage,
    })
}

fn record_websocket_turn_end<R: tauri::Runtime>(
    observation: &WebsocketObservation<R>,
    turn: WebsocketTurnCompletion,
) {
    if !observation.observe_request {
        return;
    }

    let trace_id = format!("{}-ws-{}", observation.trace_id, turn.turn_index);
    let mut attempt = build_websocket_attempt(
        observation,
        "success",
        Some(StatusCode::SWITCHING_PROTOCOLS.as_u16()),
        None,
        None,
        None,
        turn.duration_ms,
    );
    attempt.attempt_started_ms = Some(turn.started_ms);
    attempt.attempt_duration_ms = Some(turn.duration_ms);

    let special_settings_json = websocket_special_settings_json(&turn);
    let created_at_ms = observation
        .created_at_ms
        .saturating_add(turn.started_ms.min(i64::MAX as u128) as i64);
    let created_at = (created_at_ms / 1000).max(0);
    let usage_metrics = turn.usage.as_ref().map(|extract| extract.metrics.clone());
    let excluded_from_stats = websocket_turn_excluded_from_stats(&turn);
    let input_tokens = usage_metrics
        .as_ref()
        .and_then(|metrics| metrics.input_tokens);
    let output_tokens = usage_metrics
        .as_ref()
        .and_then(|metrics| metrics.output_tokens);
    let total_tokens = usage_metrics
        .as_ref()
        .and_then(|metrics| metrics.total_tokens);
    let requested_model = turn.requested_model.or(observation.requested_model.clone());
    tracing::info!(
        target: "gateway_debug",
        trace_id = %trace_id,
        parent_trace_id = %observation.trace_id,
        cli_key = %observation.cli_key,
        session_id = observation.session_id.as_deref().unwrap_or("-"),
        provider_id = observation.provider_id,
        provider = %observation.provider_name,
        model = requested_model.as_deref().unwrap_or("-"),
        terminal_event = %turn.terminal_event,
        response_id = turn.response_id.as_deref().unwrap_or("-"),
        duration_ms = turn.duration_ms,
        ttfb_ms = turn.ttfb_ms.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
        input_tokens = input_tokens.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
        output_tokens = output_tokens.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
        total_tokens = total_tokens.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
        excluded_from_stats,
        "[WS_TURN] websocket request log recorded"
    );
    let (log_args, attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
        &trace_id,
        &observation.cli_key,
        observation.session_id.clone(),
        &observation.method,
        &observation.path,
        observation.query.as_deref(),
        excluded_from_stats,
        Some(special_settings_json),
        Some(StatusCode::SWITCHING_PROTOCOLS.as_u16()),
        None,
        turn.duration_ms,
        turn.ttfb_ms,
        &[attempt],
        requested_model,
        created_at_ms,
        created_at,
        usage_metrics.clone(),
        turn.usage,
    );

    log_args.emit_gateway_request_event(
        &observation.state.app,
        None,
        turn.ttfb_ms,
        attempts,
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        observation.state.app.clone(),
        observation.state.db.clone(),
        observation.state.log_tx.clone(),
        log_args,
    );
}

fn websocket_turn_excluded_from_stats(turn: &WebsocketTurnCompletion) -> bool {
    let Some(usage) = turn.usage.as_ref() else {
        return true;
    };
    usage.metrics.output_tokens.unwrap_or(0) <= 0
}

fn websocket_special_settings_json(turn: &WebsocketTurnCompletion) -> String {
    let mut turn_obj = serde_json::json!({
        "type": "websocket_turn",
        "turn_index": turn.turn_index,
        "terminal_event": turn.terminal_event,
    });
    if let Some(response_id) = turn.response_id.as_deref() {
        if let Some(obj) = turn_obj.as_object_mut() {
            obj.insert("response_id".to_string(), serde_json::json!(response_id));
        }
    }
    serde_json::json!([
        {"type": "websocket_proxy"},
        turn_obj
    ])
    .to_string()
}

fn init_websocket_turn_timing(
    state: &mut WebsocketRelayState,
    response_id: &str,
    elapsed_ms: u128,
) {
    state
        .turn_timings
        .entry(response_id.to_string())
        .or_insert(WebsocketTurnTiming {
            started_ms: elapsed_ms,
            first_token_ms: None,
        });
    state.active_response_id = Some(response_id.to_string());
}

fn mark_websocket_turn_first_token(
    state: &mut WebsocketRelayState,
    response_id: Option<&str>,
    elapsed_ms: u128,
) {
    let timing = response_id
        .map(str::to_string)
        .or_else(|| state.active_response_id.clone())
        .map(|id| {
            state
                .turn_timings
                .entry(id.clone())
                .or_insert(WebsocketTurnTiming {
                    started_ms: 0,
                    first_token_ms: None,
                })
        });

    if let Some(timing) = timing {
        if timing.first_token_ms.is_none() {
            timing.first_token_ms = Some(elapsed_ms.saturating_sub(timing.started_ms));
        }
    }
}

fn parse_websocket_event_type(bytes: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    value
        .get("type")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_websocket_response_id(bytes: &[u8], event_type: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let response_id = value
        .get("response")
        .and_then(|response| response.get("id"))
        .and_then(|value| value.as_str())
        .or_else(|| value.get("response_id").and_then(|value| value.as_str()))
        .or_else(|| {
            is_websocket_terminal_event(event_type)
                .then(|| value.get("id").and_then(|value| value.as_str()))
                .flatten()
        })?;

    let response_id = response_id.trim();
    (!response_id.is_empty()).then(|| response_id.to_string())
}

fn is_websocket_terminal_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "response.completed"
            | "response.done"
            | "response.failed"
            | "response.incomplete"
            | "response.cancelled"
            | "response.canceled"
    )
}

fn should_parse_websocket_usage(event_type: &str) -> bool {
    is_websocket_terminal_event(event_type)
}

fn is_websocket_token_event(event_type: &str) -> bool {
    if matches!(
        event_type,
        "" | "response.created"
            | "response.in_progress"
            | "response.output_item.added"
            | "response.output_item.done"
    ) {
        return false;
    }

    event_type.contains(".delta")
        || event_type.starts_with("response.output_text")
        || event_type.starts_with("response.output")
        || matches!(event_type, "response.completed" | "response.done")
}

fn accumulate_websocket_usage(
    aggregate: &mut Option<crate::usage::UsageExtract>,
    usage: &crate::usage::UsageExtract,
) {
    let Some(existing) = aggregate else {
        *aggregate = Some(usage.clone());
        return;
    };

    add_metric(
        &mut existing.metrics.input_tokens,
        usage.metrics.input_tokens,
    );
    add_metric(
        &mut existing.metrics.output_tokens,
        usage.metrics.output_tokens,
    );
    add_metric(
        &mut existing.metrics.total_tokens,
        usage.metrics.total_tokens,
    );
    add_metric(
        &mut existing.metrics.cache_read_input_tokens,
        usage.metrics.cache_read_input_tokens,
    );
    add_metric(
        &mut existing.metrics.cache_creation_input_tokens,
        usage.metrics.cache_creation_input_tokens,
    );
    add_metric(
        &mut existing.metrics.cache_creation_5m_input_tokens,
        usage.metrics.cache_creation_5m_input_tokens,
    );
    add_metric(
        &mut existing.metrics.cache_creation_1h_input_tokens,
        usage.metrics.cache_creation_1h_input_tokens,
    );
    existing.usage_json = websocket_usage_json(&existing.metrics);
}

fn add_metric(base: &mut Option<i64>, patch: Option<i64>) {
    if let Some(patch) = patch {
        *base = Some(base.unwrap_or(0).saturating_add(patch));
    }
}

fn websocket_usage_json(metrics: &crate::usage::UsageMetrics) -> String {
    let mut obj = serde_json::Map::new();
    if let Some(value) = metrics.input_tokens {
        obj.insert("input_tokens".to_string(), serde_json::json!(value));
    }
    if let Some(value) = metrics.output_tokens {
        obj.insert("output_tokens".to_string(), serde_json::json!(value));
    }
    if let Some(value) = metrics.total_tokens {
        obj.insert("total_tokens".to_string(), serde_json::json!(value));
    }
    if let Some(value) = metrics.cache_read_input_tokens {
        obj.insert(
            "cache_read_input_tokens".to_string(),
            serde_json::json!(value),
        );
    }
    if let Some(value) = metrics.cache_creation_input_tokens {
        obj.insert(
            "cache_creation_input_tokens".to_string(),
            serde_json::json!(value),
        );
    }
    if let Some(value) = metrics.cache_creation_5m_input_tokens {
        obj.insert(
            "cache_creation_5m_input_tokens".to_string(),
            serde_json::json!(value),
        );
    }
    if let Some(value) = metrics.cache_creation_1h_input_tokens {
        obj.insert(
            "cache_creation_1h_input_tokens".to_string(),
            serde_json::json!(value),
        );
    }
    serde_json::Value::Object(obj).to_string()
}

fn emit_websocket_progress_event<R: tauri::Runtime>(observation: &WebsocketObservation<R>) {
    if !observation.observe_request {
        return;
    }

    let duration_ms = observation.started.elapsed().as_millis();
    let attempt =
        build_websocket_attempt(observation, "started", None, None, None, None, duration_ms);
    emit_websocket_attempt_event(observation, &attempt, duration_ms);
}

fn axum_to_tungstenite(message: AxumWsMessage) -> tungstenite::Message {
    match message {
        AxumWsMessage::Text(value) => tungstenite::Message::Text(value),
        AxumWsMessage::Binary(value) => tungstenite::Message::Binary(value),
        AxumWsMessage::Ping(value) => tungstenite::Message::Ping(value),
        AxumWsMessage::Pong(value) => tungstenite::Message::Pong(value),
        AxumWsMessage::Close(value) => {
            tungstenite::Message::Close(value.map(|frame| tungstenite::protocol::CloseFrame {
                code: frame.code.into(),
                reason: frame.reason,
            }))
        }
    }
}

fn tungstenite_to_axum(message: tungstenite::Message) -> Option<AxumWsMessage> {
    match message {
        tungstenite::Message::Text(value) => Some(AxumWsMessage::Text(value)),
        tungstenite::Message::Binary(value) => Some(AxumWsMessage::Binary(value)),
        tungstenite::Message::Ping(value) => Some(AxumWsMessage::Ping(value)),
        tungstenite::Message::Pong(value) => Some(AxumWsMessage::Pong(value)),
        tungstenite::Message::Close(value) => Some(AxumWsMessage::Close(value.map(|frame| {
            axum::extract::ws::CloseFrame {
                code: frame.code.into(),
                reason: frame.reason,
            }
        }))),
        tungstenite::Message::Frame(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
    use axum::http::HeaderValue;
    use axum::routing::get;
    use axum::Router;
    use std::borrow::Cow;
    use std::sync::{Arc, Mutex};

    fn websocket_test_state(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: crate::db::Db,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        let (log_tx, _log_rx) =
            tokio::sync::mpsc::channel::<crate::request_logs::RequestLogInsert>(8);
        GatewayAppState {
            app,
            db,
            log_tx,
            circuit: Arc::new(crate::circuit_breaker::CircuitBreaker::new(
                crate::circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: 1,
                    open_duration_secs: 60,
                },
                HashMap::new(),
                None,
            )),
            session: Arc::new(crate::session_manager::SessionManager::new()),
            codex_session_cache: Arc::new(Mutex::new(
                crate::gateway::codex_session_id::CodexSessionIdCache::default(),
            )),
            recent_errors: Arc::new(Mutex::new(RecentErrorCache::default())),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
        }
    }

    fn insert_codex_provider(
        db: &crate::db::Db,
        name: &str,
        base_url: String,
        priority: i64,
    ) -> i64 {
        insert_codex_provider_with_websockets(db, name, base_url, priority, true)
    }

    fn insert_codex_provider_with_websockets(
        db: &crate::db::Db,
        name: &str,
        base_url: String,
        priority: i64,
        supports_websockets: bool,
    ) -> i64 {
        crate::providers::upsert(
            db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: name.to_string(),
                base_urls: vec![base_url],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
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
                supports_websockets: Some(supports_websockets),
            },
        )
        .expect("insert provider")
        .id
    }

    async fn spawn_websocket_upstream() -> (String, tokio::task::JoinHandle<()>) {
        async fn handler(ws: WebSocketUpgrade) -> Response {
            ws.on_upgrade(|mut socket| async move { while socket.recv().await.is_some() {} })
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind websocket upstream");
        let addr = listener.local_addr().expect("local addr");
        let app = Router::new().route("/v1/realtime", get(handler));
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{addr}"), task)
    }

    async fn spawn_plain_http_upstream() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind plain upstream");
        let addr = listener.local_addr().expect("local addr");
        let app = Router::new().fallback(|| async { "not websocket" });
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{addr}"), task)
    }

    #[test]
    fn build_websocket_url_rewrites_http_scheme() {
        let url = build_websocket_url(
            "https://api.example.com/v1",
            "/v1/realtime",
            Some("model=x"),
        )
        .expect("websocket url");

        assert_eq!(url.as_str(), "wss://api.example.com/v1/realtime?model=x");
    }

    #[test]
    fn websocket_success_completion_uses_switching_protocols_status() {
        let completion = WebsocketCompletion::success(Some(12));

        assert_eq!(completion.outcome, "success");
        assert_eq!(
            completion.status,
            Some(StatusCode::SWITCHING_PROTOCOLS.as_u16())
        );
        assert_eq!(completion.ttfb_ms, Some(12));
    }

    #[test]
    fn captures_realtime_response_done_usage_from_upstream_text_frame() {
        let mut state = WebsocketRelayState::default();
        let message = AxumWsMessage::Text(
            r#"{"type":"response.done","response":{"model":"gpt-realtime","usage":{"input_tokens":11,"output_tokens":7,"total_tokens":18,"input_token_details":{"cached_tokens":3}}}}"#
                .to_string(),
        );

        let turn = capture_websocket_upstream_metadata(&message, "codex", 42, &mut state)
            .expect("turn captured");

        let usage = state.metadata.usage.expect("usage captured");
        assert_eq!(
            state.metadata.requested_model.as_deref(),
            Some("gpt-realtime")
        );
        assert_eq!(usage.metrics.input_tokens, Some(11));
        assert_eq!(usage.metrics.output_tokens, Some(7));
        assert_eq!(usage.metrics.total_tokens, Some(18));
        assert_eq!(usage.metrics.cache_read_input_tokens, Some(3));
        assert_eq!(turn.turn_index, 1);
        assert_eq!(turn.terminal_event, "response.done");
        assert_eq!(turn.duration_ms, 0);
    }

    #[test]
    fn websocket_turn_timing_uses_first_token_event_not_terminal_event() {
        let mut state = WebsocketRelayState::default();
        let created = AxumWsMessage::Text(
            r#"{"type":"response.created","response":{"id":"resp_1","model":"gpt-realtime"}}"#
                .to_string(),
        );
        let delta = AxumWsMessage::Text(
            r#"{"type":"response.output_text.delta","delta":"hi"}"#.to_string(),
        );
        let done = AxumWsMessage::Text(
            r#"{"type":"response.completed","response":{"id":"resp_1","usage":{"input_tokens":2,"output_tokens":3,"total_tokens":5}}}"#
                .to_string(),
        );

        assert!(capture_websocket_upstream_metadata(&created, "codex", 10, &mut state).is_none());
        assert!(capture_websocket_upstream_metadata(&delta, "codex", 25, &mut state).is_none());
        let turn = capture_websocket_upstream_metadata(&done, "codex", 70, &mut state)
            .expect("turn captured");

        assert_eq!(state.metadata.first_token_ms, Some(25));
        assert_eq!(turn.started_ms, 10);
        assert_eq!(turn.duration_ms, 60);
        assert_eq!(turn.ttfb_ms, Some(15));
        assert_eq!(
            turn.usage
                .as_ref()
                .and_then(|usage| usage.metrics.output_tokens),
            Some(3)
        );
    }

    #[test]
    fn websocket_usage_accumulates_multiple_terminal_turns() {
        let mut state = WebsocketRelayState::default();
        let first = AxumWsMessage::Text(
            r#"{"type":"response.completed","response":{"id":"resp_1","usage":{"input_tokens":2,"output_tokens":1,"total_tokens":3}}}"#
                .to_string(),
        );
        let second = AxumWsMessage::Text(
            r#"{"type":"response.failed","response":{"id":"resp_2","usage":{"input_tokens":3,"output_tokens":4,"total_tokens":7}}}"#
                .to_string(),
        );

        let turn1 = capture_websocket_upstream_metadata(&first, "codex", 100, &mut state)
            .expect("first turn");
        let turn2 = capture_websocket_upstream_metadata(&second, "codex", 200, &mut state)
            .expect("second turn");

        let usage = state.metadata.usage.expect("aggregate usage");
        assert_eq!(turn1.turn_index, 1);
        assert_eq!(turn2.turn_index, 2);
        assert_eq!(usage.metrics.input_tokens, Some(5));
        assert_eq!(usage.metrics.output_tokens, Some(5));
        assert_eq!(usage.metrics.total_tokens, Some(10));
    }

    #[test]
    fn websocket_turn_stats_require_output_tokens() {
        let mut state = WebsocketRelayState::default();
        let input_only = AxumWsMessage::Text(
            r#"{"type":"response.completed","response":{"id":"resp_input_only","usage":{"input_tokens":12828,"output_tokens":0,"total_tokens":12828}}}"#
                .to_string(),
        );
        let answered = AxumWsMessage::Text(
            r#"{"type":"response.completed","response":{"id":"resp_answered","usage":{"input_tokens":10,"output_tokens":2,"total_tokens":12}}}"#
                .to_string(),
        );

        let input_only_turn =
            capture_websocket_upstream_metadata(&input_only, "codex", 100, &mut state)
                .expect("input-only turn");
        let answered_turn =
            capture_websocket_upstream_metadata(&answered, "codex", 200, &mut state)
                .expect("answered turn");

        assert!(websocket_turn_excluded_from_stats(&input_only_turn));
        assert!(!websocket_turn_excluded_from_stats(&answered_turn));
    }

    #[test]
    fn active_provider_websocket_registry_closes_registered_connections() {
        let provider_id = 9_001_001;
        let mut close_rx = register_active_websocket_connection(provider_id, "trace-close");

        assert_eq!(close_active_provider_websockets(provider_id), 1);
        assert!(close_rx.has_changed().expect("close signal available"));
        assert_eq!(*close_rx.borrow_and_update(), 1);
        assert_eq!(close_active_provider_websockets(provider_id), 0);
    }

    #[test]
    fn active_provider_websocket_registry_unregisters_finished_connections() {
        let provider_id = 9_001_002;
        let close_rx = register_active_websocket_connection(provider_id, "trace-finished");
        unregister_active_websocket_connection(provider_id, "trace-finished");

        assert_eq!(close_active_provider_websockets(provider_id), 0);
        assert!(close_rx.has_changed().is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn websocket_upstream_selection_skips_open_circuit_provider() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("ws-open-circuit.sqlite"))
            .expect("init db");
        let state = websocket_test_state(app.handle().clone(), db.clone());
        let (plain_base_url, plain_task) = spawn_plain_http_upstream().await;
        let (ws_base_url, ws_task) = spawn_websocket_upstream().await;
        let first_provider_id = insert_codex_provider(&db, "open circuit", plain_base_url, 0);
        let second_provider_id = insert_codex_provider(&db, "ws ok", ws_base_url, 1);
        state
            .circuit
            .record_failure(first_provider_id, now_unix_seconds() as i64);

        let prepared = prepare_websocket_upstream(
            &state,
            "codex",
            None,
            "/v1/realtime",
            None,
            &HeaderMap::new(),
            "trace-ws-open-circuit",
        )
        .await
        .expect("prepared websocket upstream");

        assert_eq!(prepared.provider_id, second_provider_id);
        assert_eq!(
            state
                .circuit
                .snapshot(first_provider_id, now_unix_seconds() as i64)
                .state,
            crate::circuit_breaker::CircuitState::Open
        );

        drop(prepared);
        plain_task.abort();
        ws_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn websocket_upstream_selection_tries_next_when_provider_rejects_ws() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("ws-provider-failover.sqlite"))
            .expect("init db");
        let state = websocket_test_state(app.handle().clone(), db.clone());
        let (plain_one_base_url, plain_one_task) = spawn_plain_http_upstream().await;
        let (plain_two_base_url, plain_two_task) = spawn_plain_http_upstream().await;
        let (ws_base_url, ws_task) = spawn_websocket_upstream().await;
        let first_provider_id = insert_codex_provider(&db, "plain one", plain_one_base_url, 0);
        let second_provider_id = insert_codex_provider(&db, "plain two", plain_two_base_url, 1);
        let third_provider_id = insert_codex_provider(&db, "ws ok", ws_base_url, 2);

        let prepared = prepare_websocket_upstream(
            &state,
            "codex",
            None,
            "/v1/realtime",
            None,
            &HeaderMap::new(),
            "trace-ws-failover",
        )
        .await
        .expect("prepared websocket upstream");

        assert_eq!(prepared.provider_id, third_provider_id);
        let now = now_unix_seconds() as i64;
        assert_eq!(
            state.circuit.snapshot(first_provider_id, now).state,
            crate::circuit_breaker::CircuitState::Closed
        );
        assert_eq!(
            state.circuit.snapshot(second_provider_id, now).state,
            crate::circuit_breaker::CircuitState::Closed
        );

        drop(prepared);
        plain_one_task.abort();
        plain_two_task.abort();
        ws_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn websocket_upstream_selection_skips_provider_without_websocket_flag() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("ws-provider-flag.sqlite"))
            .expect("init db");
        let state = websocket_test_state(app.handle().clone(), db.clone());
        let (plain_base_url, plain_task) = spawn_plain_http_upstream().await;
        let (ws_base_url, ws_task) = spawn_websocket_upstream().await;
        let first_provider_id =
            insert_codex_provider_with_websockets(&db, "ws disabled", plain_base_url, 0, false);
        let second_provider_id =
            insert_codex_provider_with_websockets(&db, "ws enabled", ws_base_url, 1, true);

        let prepared = prepare_websocket_upstream(
            &state,
            "codex",
            None,
            "/v1/realtime",
            None,
            &HeaderMap::new(),
            "trace-ws-provider-flag",
        )
        .await
        .expect("prepared websocket upstream");

        assert_eq!(prepared.provider_id, second_provider_id);
        assert_eq!(
            state
                .circuit
                .snapshot(first_provider_id, now_unix_seconds() as i64)
                .state,
            crate::circuit_breaker::CircuitState::Closed
        );

        drop(prepared);
        plain_task.abort();
        ws_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn websocket_forced_provider_without_websocket_flag_fails_without_circuit() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("ws-forced-provider-flag.sqlite"))
            .expect("init db");
        let state = websocket_test_state(app.handle().clone(), db.clone());
        let (plain_base_url, plain_task) = spawn_plain_http_upstream().await;
        let provider_id =
            insert_codex_provider_with_websockets(&db, "ws disabled", plain_base_url, 0, false);

        let err = match prepare_websocket_upstream(
            &state,
            "codex",
            Some(provider_id),
            "/v1/realtime",
            None,
            &HeaderMap::new(),
            "trace-ws-forced-provider-flag",
        )
        .await
        {
            Ok(_) => panic!("forced provider should not be attempted"),
            Err(err) => err,
        };

        assert_eq!(err.0, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(err.1, GatewayErrorCode::NoEnabledProvider);
        assert!(err.2.contains("no websocket-enabled provider"));
        assert_eq!(
            state
                .circuit
                .snapshot(provider_id, now_unix_seconds() as i64)
                .state,
            crate::circuit_breaker::CircuitState::Closed
        );

        plain_task.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn websocket_all_providers_without_websocket_flag_fail_fast() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("ws-all-provider-flag.sqlite"))
            .expect("init db");
        let state = websocket_test_state(app.handle().clone(), db.clone());
        let (plain_one_base_url, plain_one_task) = spawn_plain_http_upstream().await;
        let (plain_two_base_url, plain_two_task) = spawn_plain_http_upstream().await;
        let first_provider_id = insert_codex_provider_with_websockets(
            &db,
            "ws disabled one",
            plain_one_base_url,
            0,
            false,
        );
        let second_provider_id = insert_codex_provider_with_websockets(
            &db,
            "ws disabled two",
            plain_two_base_url,
            1,
            false,
        );

        let err = match prepare_websocket_upstream(
            &state,
            "codex",
            None,
            "/v1/realtime",
            None,
            &HeaderMap::new(),
            "trace-ws-all-provider-flag",
        )
        .await
        {
            Ok(_) => panic!("websocket preparation should fail fast"),
            Err(err) => err,
        };

        assert_eq!(err.0, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(err.1, GatewayErrorCode::NoEnabledProvider);
        assert!(err.2.contains("no websocket-enabled provider"));

        let now = now_unix_seconds() as i64;
        assert_eq!(
            state.circuit.snapshot(first_provider_id, now).state,
            crate::circuit_breaker::CircuitState::Closed
        );
        assert_eq!(
            state.circuit.snapshot(second_provider_id, now).state,
            crate::circuit_breaker::CircuitState::Closed
        );

        plain_one_task.abort();
        plain_two_task.abort();
    }

    #[test]
    fn extract_websocket_session_id_prefers_primary_header() {
        let mut headers = HeaderMap::new();
        headers.insert("session_id", HeaderValue::from_static(" sess-primary "));
        headers.insert("x-session-id", HeaderValue::from_static("sess-fallback"));

        assert_eq!(
            extract_websocket_session_id(&headers).as_deref(),
            Some("sess-primary")
        );
    }

    #[test]
    fn upstream_headers_remove_client_handshake_and_keep_protocol() {
        let mut inbound = HeaderMap::new();
        inbound.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
        inbound.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        inbound.insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("realtime"),
        );
        inbound.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer old"),
        );

        let provider = crate::providers::ProviderForGateway {
            id: 1,
            name: "p1".to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: "sk-test".to_string(),
            claude_models: crate::providers::ClaudeModels::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: crate::providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            supports_websockets: true,
        };

        let mut headers = build_base_upstream_websocket_headers("codex", &inbound);
        crate::gateway::util::inject_provider_auth(
            "codex",
            &provider.api_key_plaintext,
            &mut headers,
        );

        assert!(!headers.contains_key(header::CONNECTION));
        assert!(!headers.contains_key(header::UPGRADE));
        assert!(!headers.contains_key(header::SEC_WEBSOCKET_KEY));
        assert_eq!(
            headers
                .get(header::SEC_WEBSOCKET_PROTOCOL)
                .and_then(|v| v.to_str().ok()),
            Some("realtime")
        );
        assert!(headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .starts_with("Bearer sk-test"));
    }

    #[test]
    fn message_conversion_preserves_close_frame() {
        let input = AxumWsMessage::Close(Some(axum::extract::ws::CloseFrame {
            code: axum::extract::ws::close_code::NORMAL,
            reason: Cow::Borrowed("done"),
        }));

        let output = tungstenite_to_axum(axum_to_tungstenite(input)).expect("converted message");

        assert!(matches!(
            output,
            AxumWsMessage::Close(Some(frame))
                if frame.code == axum::extract::ws::close_code::NORMAL
                    && frame.reason == "done"
        ));
    }
}
