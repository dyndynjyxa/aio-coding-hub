//! Responses WebSocket upgrades through the gateway's existing HTTP transport.

use futures_util::{SinkExt, StreamExt};
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response, StatusCode, Url, Version};
use std::time::Duration;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub(crate) const MAX_MESSAGE_BYTES: usize = super::protocol::MAX_MESSAGE_BYTES;
const MAX_CONNECT_DURATION: Duration = Duration::from_secs(5);
/// OpenAI ends a Responses WebSocket at 60 minutes; no turn starts on one this old.
const MAX_REUSE_AGE: Duration = Duration::from_secs(55 * 60);
/// An idle socket pings this often, which keeps NAT and proxy mappings open, and is
/// dropped when a whole interval passes without a frame: such a drop is silent.
const KEEPALIVE: Duration = Duration::from_secs(20);
/// A peer that stopped reading must not hold the next turn.
const PING_WRITE_TIMEOUT: Duration = Duration::from_secs(1);

pub(crate) type UpstreamWebSocket = WebSocketStream<reqwest::Upgraded>;

#[derive(Debug)]
pub(crate) struct UpstreamConnection {
    pub(crate) socket: UpstreamWebSocket,
    pub(crate) headers: HeaderMap,
    reuse_until: Instant,
}

/// A socket kept between turns. Reading it answers pings (tungstenite queues the
/// pong without blocking), so the upstream keeps it open.
pub(crate) struct IdleConnection {
    resume: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Option<UpstreamConnection>>,
}

impl UpstreamConnection {
    pub(crate) fn park(self) -> IdleConnection {
        self.park_with(KEEPALIVE)
    }

    fn park_with(mut self, keepalive: Duration) -> IdleConnection {
        let (resume, mut resumed) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut ticks = tokio::time::interval_at(Instant::now() + keepalive, keepalive);
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut answered = true;
            loop {
                tokio::select! {
                    biased;
                    // Also taken when the idle connection is dropped, which ends the task.
                    _ = &mut resumed => return Some(self),
                    message = self.socket.next() => match message {
                        Some(Ok(Message::Close(_)) | Err(_)) | None => return None,
                        // Pings are answered by the read itself; frames of the finished
                        // response say nothing of the next one.
                        Some(Ok(_)) => answered = true,
                    },
                    _ = ticks.tick() => {
                        if !answered {
                            return None;
                        }
                        answered = false;
                        let ping = self.socket.send(Message::Ping(Vec::new()));
                        tokio::time::timeout(PING_WRITE_TIMEOUT, ping).await.ok()?.ok()?;
                    }
                }
            }
        });
        IdleConnection { resume, task }
    }
}

impl IdleConnection {
    /// None when the socket closed while idle or is too old for another turn.
    pub(crate) async fn resume(self) -> Option<UpstreamConnection> {
        let _ = self.resume.send(());
        let connection = self.task.await.ok()??;
        (Instant::now() < connection.reuse_until).then_some(connection)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectError {
    #[error("WebSocket HTTP client unavailable: {0}")]
    Client(String),
    #[error("WebSocket connection deadline exceeded")]
    Timeout,
    #[error("WebSocket transport failed")]
    Transport(#[source] reqwest::Error),
    #[error("WebSocket handshake invalid: {0}")]
    Protocol(&'static str),
    #[error("WebSocket upgrade rejected with HTTP {}", .0.status())]
    Rejected(Box<Response>),
}

/// `headers` must already contain the selected provider's credentials. The caller
/// retains ownership of cancellation and passes the remaining attempt deadline.
pub(crate) async fn connect(
    url: Url,
    headers: HeaderMap,
    deadline: Instant,
) -> Result<UpstreamConnection, ConnectError> {
    let client = crate::gateway::http_client::get_no_redirect().map_err(ConnectError::Client)?;
    connect_with_client(&client, url, headers, deadline).await
}

async fn connect_with_client(
    client: &Client,
    url: Url,
    mut headers: HeaderMap,
    deadline: Instant,
) -> Result<UpstreamConnection, ConnectError> {
    let deadline = deadline.min(Instant::now() + MAX_CONNECT_DURATION);
    if deadline <= Instant::now() {
        return Err(ConnectError::Timeout);
    }
    let url = handshake_url(url)?;
    prepare_headers(&mut headers);
    let key = generate_key();
    headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
    headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    headers.insert(
        header::SEC_WEBSOCKET_VERSION,
        HeaderValue::from_static("13"),
    );
    headers.insert(
        header::SEC_WEBSOCKET_KEY,
        HeaderValue::from_str(&key)
            .map_err(|_| ConnectError::Protocol("invalid generated handshake key"))?,
    );
    tokio::time::timeout_at(deadline, async {
        let response = client
            .get(url)
            .version(Version::HTTP_11)
            .headers(headers)
            .send()
            .await
            .map_err(transport_error)?;
        if response.status() != StatusCode::SWITCHING_PROTOCOLS {
            // Preserve status, headers and body for the existing HTTP classifier.
            return Err(ConnectError::Rejected(Box::new(response)));
        }
        validate_upgrade(&response, &key)?;
        let headers = response.headers().clone();
        let upgraded = response.upgrade().await.map_err(transport_error)?;
        let config = WebSocketConfig {
            max_message_size: Some(MAX_MESSAGE_BYTES),
            max_frame_size: Some(MAX_MESSAGE_BYTES),
            ..WebSocketConfig::default()
        };
        let socket = WebSocketStream::from_raw_socket(upgraded, Role::Client, Some(config)).await;
        Ok(UpstreamConnection {
            socket,
            headers,
            reuse_until: Instant::now() + MAX_REUSE_AGE,
        })
    })
    .await
    .map_err(|_| ConnectError::Timeout)?
}

fn transport_error(error: reqwest::Error) -> ConnectError {
    if error.is_timeout() {
        ConnectError::Timeout
    } else {
        ConnectError::Transport(error.without_url())
    }
}

fn handshake_url(mut url: Url) -> Result<Url, ConnectError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ConnectError::Protocol("URL credentials are not supported"));
    }
    let scheme = match url.scheme() {
        "http" | "ws" => "http",
        "https" | "wss" => "https",
        _ => return Err(ConnectError::Protocol("unsupported URL scheme")),
    };
    url.set_scheme(scheme)
        .map_err(|_| ConnectError::Protocol("invalid handshake URL"))?;
    Ok(url)
}

fn prepare_headers(headers: &mut HeaderMap) {
    let connection_headers: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    for name in connection_headers {
        headers.remove(name);
    }
    crate::gateway::util::strip_hop_headers(headers);
    let websocket_headers: Vec<HeaderName> = headers
        .keys()
        .filter(|name| name.as_str().starts_with("sec-websocket-"))
        .cloned()
        .collect();
    for name in websocket_headers {
        headers.remove(name);
    }
    for name in [
        header::HOST,
        header::CONTENT_LENGTH,
        header::CONTENT_TYPE,
        header::CONTENT_ENCODING,
    ] {
        headers.remove(name);
    }
}

fn header_has_token(headers: &HeaderMap, name: HeaderName, token: &str) -> bool {
    headers.get_all(name).iter().any(|value| {
        value.to_str().is_ok_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(token))
        })
    })
}

fn validate_upgrade(response: &Response, key: &str) -> Result<(), ConnectError> {
    let headers = response.headers();
    if response.version() != Version::HTTP_11
        || !header_has_token(headers, header::UPGRADE, "websocket")
        || !header_has_token(headers, header::CONNECTION, "upgrade")
    {
        return Err(ConnectError::Protocol("missing HTTP/1.1 upgrade headers"));
    }
    let expected = derive_accept_key(key.as_bytes());
    let mut accepted = headers.get_all(header::SEC_WEBSOCKET_ACCEPT).iter();
    if accepted.next().map(HeaderValue::as_bytes) != Some(expected.as_bytes())
        || accepted.next().is_some()
    {
        return Err(ConnectError::Protocol("invalid Sec-WebSocket-Accept"));
    }
    if headers.contains_key(header::SEC_WEBSOCKET_EXTENSIONS)
        || headers.contains_key(header::SEC_WEBSOCKET_PROTOCOL)
    {
        return Err(ConnectError::Protocol("unsolicited WebSocket negotiation"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ws::WebSocketUpgrade;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::tungstenite::Message;

    async fn serve(router: Router) -> (Url, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!(
            "http://{}/responses",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (url, task)
    }

    fn client() -> Client {
        Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap()
    }

    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }

    #[tokio::test]
    async fn upgrade_uses_fresh_headers_and_exchanges_codec_messages() {
        let router = Router::new().route(
            "/responses",
            get(|headers: HeaderMap, ws: WebSocketUpgrade| async move {
                assert_eq!(headers[header::AUTHORIZATION], "Bearer provider-secret");
                assert_eq!(headers["x-tenant"], "oauth-tenant");
                assert_ne!(headers[header::SEC_WEBSOCKET_KEY], "downstream-key");
                assert_eq!(headers[header::SEC_WEBSOCKET_VERSION], "13");
                assert_eq!(headers[header::UPGRADE], "websocket");
                assert_ne!(headers[header::HOST], "downstream.test");
                for name in [
                    "sec-websocket-extensions",
                    "sec-websocket-protocol",
                    "sec-websocket-accept",
                    "x-downstream-hop",
                    "proxy-authorization",
                    "content-length",
                    "content-encoding",
                ] {
                    assert!(!headers.contains_key(name), "unexpected header: {name}");
                }
                ws.on_upgrade(|mut socket| async move {
                    let message = socket.recv().await.unwrap().unwrap();
                    socket.send(message).await.unwrap();
                })
            }),
        );
        let (mut url, server) = serve(router).await;
        url.set_scheme("ws").unwrap();
        let mut headers = HeaderMap::new();
        for (name, value) in [
            ("authorization", "Bearer provider-secret"),
            ("host", "downstream.test"),
            ("connection", "keep-alive, X-Downstream-Hop"),
            ("x-downstream-hop", "private"),
            ("proxy-authorization", "downstream-proxy-secret"),
            ("upgrade", "other"),
            ("sec-websocket-key", "downstream-key"),
            ("sec-websocket-version", "8"),
            ("sec-websocket-extensions", "permessage-deflate"),
            ("sec-websocket-protocol", "downstream-protocol"),
            ("sec-websocket-accept", "downstream-accept"),
            ("content-length", "100"),
            ("content-encoding", "gzip"),
        ] {
            headers.insert(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
        use crate::gateway::oauth::provider_trait::OAuthProvider;
        crate::gateway::oauth::adapters::codex::CodexOAuthProvider::new()
            .inject_upstream_headers(&mut headers, "provider-secret")
            .unwrap();
        headers.extend(
            crate::providers::custom_headers_to_map(&[crate::providers::ProviderCustomHeader {
                name: "x-tenant".into(),
                value: "oauth-tenant".into(),
            }])
            .unwrap(),
        );
        let mut connection = connect_with_client(&client(), url, headers, deadline())
            .await
            .unwrap();
        assert_eq!(connection.headers[header::UPGRADE], "websocket");
        assert_eq!(
            connection.socket.get_config().max_message_size,
            Some(MAX_MESSAGE_BYTES)
        );
        assert_eq!(
            connection.socket.get_config().max_frame_size,
            Some(MAX_MESSAGE_BYTES)
        );
        connection
            .socket
            .send(Message::Text("response.create".into()))
            .await
            .unwrap();
        assert_eq!(
            connection.socket.next().await.unwrap().unwrap(),
            Message::Text("response.create".into())
        );
        server.abort();
    }

    #[tokio::test]
    async fn upgrade_rejects_bad_accept() {
        let router = Router::new().route(
            "/responses",
            get(|| async {
                (
                    StatusCode::SWITCHING_PROTOCOLS,
                    [
                        (header::UPGRADE, "websocket"),
                        (header::CONNECTION, "Upgrade"),
                        (header::SEC_WEBSOCKET_ACCEPT, "incorrect-accept"),
                    ],
                )
            }),
        );
        let (url, server) = serve(router).await;
        assert!(matches!(
            connect_with_client(&client(), url, HeaderMap::new(), deadline()).await,
            Err(ConnectError::Protocol("invalid Sec-WebSocket-Accept"))
        ));
        server.abort();
    }

    #[tokio::test]
    async fn rejected_upgrade_preserves_status_headers_and_body_without_redirecting() {
        for status in [
            StatusCode::NOT_FOUND,
            StatusCode::UPGRADE_REQUIRED,
            StatusCode::TEMPORARY_REDIRECT,
        ] {
            let router = Router::new().route(
                "/responses",
                get(move || async move {
                    (
                        status,
                        [(header::LOCATION, "http://127.0.0.1:1/never-follow")],
                        "provider rejection",
                    )
                        .into_response()
                }),
            );
            let (url, server) = serve(router).await;
            let Err(ConnectError::Rejected(response)) =
                connect_with_client(&client(), url, HeaderMap::new(), deadline()).await
            else {
                panic!("expected original HTTP rejection");
            };
            assert_eq!(response.status(), status);
            assert_eq!(
                response.headers()[header::LOCATION],
                "http://127.0.0.1:1/never-follow"
            );
            assert_eq!(response.text().await.unwrap(), "provider rejection");
            server.abort();
        }
    }

    #[tokio::test]
    async fn connect_honors_remaining_deadline() {
        let router = Router::new().route(
            "/responses",
            get(|| async {
                std::future::pending::<()>().await;
                StatusCode::OK
            }),
        );
        let (url, server) = serve(router).await;
        let started = Instant::now();
        assert!(matches!(
            connect_with_client(
                &client(),
                url.clone(),
                HeaderMap::new(),
                started + Duration::from_millis(30)
            )
            .await,
            Err(ConnectError::Timeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(matches!(
            connect_with_client(&client(), url, HeaderMap::new(), Instant::now()).await,
            Err(ConnectError::Timeout)
        ));
        server.abort();
    }

    #[tokio::test]
    async fn upgraded_codec_rejects_oversized_incoming_frames() {
        let router = Router::new().route(
            "/responses",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(|mut socket| async move {
                    let _ = socket
                        .send(axum::extract::ws::Message::Text(
                            "x".repeat(MAX_MESSAGE_BYTES + 1),
                        ))
                        .await;
                })
            }),
        );
        let (url, server) = serve(router).await;
        let mut connection = connect_with_client(&client(), url, HeaderMap::new(), deadline())
            .await
            .unwrap();
        assert!(matches!(
            connection.socket.next().await.unwrap(),
            Err(tokio_tungstenite::tungstenite::Error::Capacity(_))
        ));
        server.abort();
    }

    #[tokio::test]
    async fn idle_connection_answers_pings_and_resumes_for_the_next_turn() {
        let ponged = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify = ponged.clone();
        let router = Router::new().route(
            "/responses",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |mut socket| async move {
                    use axum::extract::ws::Message as Server;
                    socket.send(Server::Ping(vec![7])).await.unwrap();
                    socket
                        .send(Server::Text("late frame of the last turn".into()))
                        .await
                        .unwrap();
                    assert_eq!(socket.recv().await.unwrap().unwrap(), Server::Pong(vec![7]));
                    notify.notify_one();
                    let next = socket.recv().await.unwrap().unwrap();
                    socket.send(next).await.unwrap();
                })
            }),
        );
        let (url, server) = serve(router).await;
        let connection = connect_with_client(&client(), url, HeaderMap::new(), deadline())
            .await
            .unwrap();
        let idle = connection.park();
        tokio::time::timeout(Duration::from_secs(2), ponged.notified())
            .await
            .expect("idle socket answers the upstream ping");
        let mut connection = idle.resume().await.expect("open socket resumes");
        connection
            .socket
            .send(Message::Text("next turn".into()))
            .await
            .unwrap();
        assert_eq!(
            connection.socket.next().await.unwrap().unwrap(),
            Message::Text("next turn".into())
        );
        server.abort();
    }

    #[tokio::test]
    async fn idle_connection_is_not_resumed_after_close_or_at_its_age_limit() {
        let router = Router::new().route(
            "/responses",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(|mut socket| async move {
                    if socket.recv().await.is_none() {
                        return;
                    }
                    let _ = socket.close().await;
                })
            }),
        );
        let (url, server) = serve(router).await;
        let mut closing = connect_with_client(&client(), url.clone(), HeaderMap::new(), deadline())
            .await
            .unwrap();
        closing
            .socket
            .send(Message::Text("close".into()))
            .await
            .unwrap();
        let idle = closing.park();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !idle.task.is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("idle socket sees the close");
        assert!(idle.resume().await.is_none());

        let mut old = connect_with_client(&client(), url, HeaderMap::new(), deadline())
            .await
            .unwrap();
        old.reuse_until = Instant::now();
        assert!(old.park().resume().await.is_none());
        server.abort();
    }

    #[tokio::test]
    async fn idle_connection_pings_and_drops_a_peer_that_stops_answering() {
        let router = Router::new()
            .route(
                "/answering",
                get(|ws: WebSocketUpgrade| async move {
                    // Reading answers the client's pings.
                    ws.on_upgrade(|mut socket| async move {
                        while let Some(Ok(_)) = socket.recv().await {}
                    })
                }),
            )
            .route(
                "/silent",
                get(|ws: WebSocketUpgrade| async move {
                    ws.on_upgrade(|socket| async move {
                        let _held = socket;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    })
                }),
            );
        let (url, server) = serve(router).await;
        let keepalive = Duration::from_millis(50);
        let answering = connect_with_client(
            &client(),
            url.join("answering").unwrap(),
            HeaderMap::new(),
            deadline(),
        )
        .await
        .unwrap()
        .park_with(keepalive);
        let silent = connect_with_client(
            &client(),
            url.join("silent").unwrap(),
            HeaderMap::new(),
            deadline(),
        )
        .await
        .unwrap()
        .park_with(keepalive);
        tokio::time::timeout(Duration::from_secs(2), async {
            while !silent.task.is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("a peer that never answers the keepalive is dropped");
        assert!(silent.resume().await.is_none());
        tokio::time::sleep(keepalive * 6).await;
        assert!(!answering.task.is_finished());
        assert!(answering.resume().await.is_some());
        server.abort();
    }

    #[test]
    fn handshake_url_keeps_tls_authority_path_and_query() {
        let url = Url::parse("wss://example.test:8443/v1/responses?model=test").unwrap();
        assert_eq!(
            handshake_url(url).unwrap().as_str(),
            "https://example.test:8443/v1/responses?model=test"
        );
        for value in [
            "ftp://example.test/responses",
            "https://user:secret@example.test/responses",
        ] {
            assert!(matches!(
                handshake_url(Url::parse(value).unwrap()),
                Err(ConnectError::Protocol(_))
            ));
        }
    }
}
