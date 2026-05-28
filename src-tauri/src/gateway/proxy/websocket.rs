//! Usage: WebSocket upgrade proxy path for gateway requests.

use super::cli_proxy_guard::cli_proxy_enabled_cached;
use super::errors::error_response;
use super::failover::select_provider_base_url_for_request;
use super::{mark_internal_forwarded_request, GatewayErrorCode};
use crate::gateway::runtime::GatewayAppState;
use crate::gateway::util::{build_target_url, clear_all_auth_headers, ensure_cli_required_headers};
use axum::extract::ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade};
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use std::borrow::Cow;
use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

type UpstreamWsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

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
    let query = req.uri().query().map(str::to_string);
    let headers = req.headers().clone();
    let forced_provider_id = headers
        .get("x-aio-provider-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0);
    let trace_id = crate::gateway::util::new_trace_id();

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

    let provider = match resolve_websocket_provider(&state, &cli_key, forced_provider_id) {
        Ok(provider) => provider,
        Err(err) => {
            return error_response(err.0, trace_id, err.1.as_str(), err.2, vec![]);
        }
    };

    let base_url = match select_provider_base_url_for_request(&state, &provider, &cli_key, 1).await
    {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                trace_id,
                GatewayErrorCode::InvalidBaseUrl.as_str(),
                err,
                vec![],
            );
        }
    };

    let target_url = match build_websocket_url(&base_url, &forwarded_path, query.as_deref()) {
        Ok(value) => value,
        Err(err) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                trace_id,
                GatewayErrorCode::InvalidBaseUrl.as_str(),
                err,
                vec![],
            );
        }
    };

    let upstream_headers =
        match build_upstream_websocket_headers(&state, &cli_key, &headers, &provider).await {
            Ok(headers) => headers,
            Err(err) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    trace_id,
                    GatewayErrorCode::InternalError.as_str(),
                    err,
                    vec![],
                );
            }
        };
    ws.on_upgrade(move |socket| async move {
        relay_websocket(socket, target_url, upstream_headers, trace_id).await;
    })
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

async fn relay_websocket(
    client_socket: WebSocket,
    target_url: reqwest::Url,
    headers: HeaderMap,
    trace_id: String,
) {
    let request = match build_tungstenite_request(target_url, headers) {
        Ok(request) => request,
        Err(err) => {
            tracing::warn!(trace_id = %trace_id, "websocket request build failed: {err}");
            close_client_socket(client_socket, err).await;
            return;
        }
    };

    let (upstream_socket, _response) = match tokio_tungstenite::connect_async(request).await {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(trace_id = %trace_id, "websocket upstream connect failed: {err}");
            close_client_socket(client_socket, err.to_string()).await;
            return;
        }
    };

    relay_websocket_messages(client_socket, upstream_socket, trace_id).await;
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

async fn close_client_socket(mut socket: WebSocket, reason: String) {
    let _ = socket
        .send(AxumWsMessage::Close(Some(axum::extract::ws::CloseFrame {
            code: axum::extract::ws::close_code::ERROR,
            reason: Cow::Owned(reason),
        })))
        .await;
}

async fn relay_websocket_messages(
    client_socket: WebSocket,
    upstream_socket: UpstreamWsStream,
    trace_id: String,
) {
    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut upstream_tx, mut upstream_rx) = upstream_socket.split();

    loop {
        tokio::select! {
            msg = client_rx.next() => {
                let Some(msg) = msg else {
                    let _ = upstream_tx.close().await;
                    break;
                };
                match msg {
                    Ok(message) => {
                        let is_close = matches!(message, AxumWsMessage::Close(_));
                        if let Err(err) = upstream_tx.send(axum_to_tungstenite(message)).await {
                            tracing::debug!(trace_id = %trace_id, "websocket client-to-upstream send failed: {err}");
                            break;
                        }
                        if is_close {
                            break;
                        }
                    }
                    Err(err) => {
                        tracing::debug!(trace_id = %trace_id, "websocket client receive failed: {err}");
                        break;
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
                        let is_close = matches!(message, AxumWsMessage::Close(_));
                        if let Err(err) = client_tx.send(message).await {
                            tracing::debug!(trace_id = %trace_id, "websocket upstream-to-client send failed: {err}");
                            break;
                        }
                        if is_close {
                            break;
                        }
                    }
                    Err(err) => {
                        tracing::debug!(trace_id = %trace_id, "websocket upstream receive failed: {err}");
                        break;
                    }
                }
            }
        }
    }
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
    use axum::http::HeaderValue;

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
