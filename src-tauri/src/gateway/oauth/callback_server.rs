//! Usage: One-shot localhost callback listener for OAuth authorization code flow.

use crate::shared::error::AppResult;
use crate::shared::security::constant_time_eq;
use reqwest::Url;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SUCCESS_HTML: &str =
    "<html><body><h1>Authentication successful</h1><p>You may close this window.</p></body></html>";
const ERROR_HTML: &str = "<html><body><h1>Authentication failed</h1><p>You may close this window and retry.</p></body></html>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OAuthCallbackPayload {
    pub(crate) code: Option<String>,
    pub(crate) state: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) error_description: Option<String>,
}

#[derive(Debug)]
pub(crate) struct BoundOAuthCallbackListener {
    port: u16,
    listener_v4: Option<TcpListener>,
    listener_v6: Option<TcpListener>,
}

impl BoundOAuthCallbackListener {
    pub(crate) fn port(&self) -> u16 {
        self.port
    }
}

pub(crate) async fn bind_callback_listener(
    preferred_port: u16,
) -> AppResult<BoundOAuthCallbackListener> {
    match try_bind_on_port(preferred_port).await {
        Ok(bound) => Ok(bound),
        Err(preferred_err) if preferred_port == 0 => Err(format!(
            "SYSTEM_ERROR: oauth callback bind failed: {preferred_err}"
        )
        .into()),
        Err(preferred_err) => match try_bind_on_port(0).await {
            Ok(bound) => Ok(bound),
            Err(fallback_err) => Err(format!(
                "SYSTEM_ERROR: oauth callback bind failed: {preferred_err}; fallback_dynamic_port: {fallback_err}"
            )
            .into()),
        },
    }
}

async fn try_bind_on_port(port: u16) -> Result<BoundOAuthCallbackListener, String> {
    if port == 0 {
        return try_bind_dynamic_port().await;
    }

    let mut bind_errors: Vec<String> = Vec::new();
    let listener_v4 = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => Some(listener),
        Err(err) => {
            bind_errors.push(format!("127.0.0.1:{port} ({err})"));
            None
        }
    };
    let listener_v6 = match TcpListener::bind(("::1", port)).await {
        Ok(listener) => Some(listener),
        Err(err) => {
            bind_errors.push(format!("::1:{port} ({err})"));
            None
        }
    };
    if listener_v4.is_none() && listener_v6.is_none() {
        return Err(bind_errors.join("; "));
    }

    Ok(BoundOAuthCallbackListener {
        port,
        listener_v4,
        listener_v6,
    })
}

async fn try_bind_dynamic_port() -> Result<BoundOAuthCallbackListener, String> {
    let mut bind_errors: Vec<String> = Vec::new();

    match TcpListener::bind(("127.0.0.1", 0)).await {
        Ok(listener_v4) => {
            let port = listener_v4
                .local_addr()
                .map_err(|e| format!("127.0.0.1:0 (local_addr failed: {e})"))?
                .port();
            let listener_v6 = match TcpListener::bind(("::1", port)).await {
                Ok(listener) => Some(listener),
                Err(err) => {
                    bind_errors.push(format!("::1:{port} ({err})"));
                    None
                }
            };
            return Ok(BoundOAuthCallbackListener {
                port,
                listener_v4: Some(listener_v4),
                listener_v6,
            });
        }
        Err(err) => bind_errors.push(format!("127.0.0.1:0 ({err})")),
    }

    match TcpListener::bind(("::1", 0)).await {
        Ok(listener_v6) => {
            let port = listener_v6
                .local_addr()
                .map_err(|e| format!("::1:0 (local_addr failed: {e})"))?
                .port();
            let listener_v4 = match TcpListener::bind(("127.0.0.1", port)).await {
                Ok(listener) => Some(listener),
                Err(err) => {
                    bind_errors.push(format!("127.0.0.1:{port} ({err})"));
                    None
                }
            };
            return Ok(BoundOAuthCallbackListener {
                port,
                listener_v4,
                listener_v6: Some(listener_v6),
            });
        }
        Err(err) => bind_errors.push(format!("::1:0 ({err})")),
    }

    Err(bind_errors.join("; "))
}

pub(crate) async fn wait_for_callback(
    mut listener: BoundOAuthCallbackListener,
    expected_state: &str,
    timeout: Duration,
) -> AppResult<OAuthCallbackPayload> {
    let accept_future = async {
        match (listener.listener_v4.as_mut(), listener.listener_v6.as_mut()) {
            (Some(v4), Some(v6)) => {
                tokio::select! {
                    result = v4.accept() => result,
                    result = v6.accept() => result,
                }
            }
            (Some(v4), None) => v4.accept().await,
            (None, Some(v6)) => v6.accept().await,
            (None, None) => unreachable!("listeners checked above"),
        }
    };

    let (mut socket, _) = tokio::time::timeout(timeout, accept_future)
        .await
        .map_err(|_| "SYSTEM_ERROR: oauth callback timed out".to_string())?
        .map_err(|e| format!("SYSTEM_ERROR: oauth callback accept failed: {e}"))?;

    let mut buffer = vec![0u8; 8192];
    let size = socket
        .read(&mut buffer)
        .await
        .map_err(|e| format!("SYSTEM_ERROR: oauth callback read failed: {e}"))?;
    if size == 0 {
        return Err("SYSTEM_ERROR: oauth callback request is empty"
            .to_string()
            .into());
    }

    let request = String::from_utf8_lossy(&buffer[..size]);
    let target = extract_request_target(request.as_ref())?;
    let payload = parse_callback_target(target)?;
    validate_state(&payload, expected_state)?;

    let is_error = payload.error.is_some();
    let body = if is_error { ERROR_HTML } else { SUCCESS_HTML };
    let status = if is_error {
        "HTTP/1.1 400 Bad Request"
    } else {
        "HTTP/1.1 200 OK"
    };
    let response = format!(
        "{status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;

    Ok(payload)
}

fn extract_request_target(request: &str) -> AppResult<&str> {
    let mut lines = request.lines();
    let first = lines
        .next()
        .ok_or_else(|| "SYSTEM_ERROR: oauth callback malformed request".to_string())?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method != "GET" || target.is_empty() {
        return Err("SYSTEM_ERROR: oauth callback must be GET"
            .to_string()
            .into());
    }
    Ok(target)
}

pub(crate) fn parse_callback_target(target: &str) -> AppResult<OAuthCallbackPayload> {
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|e| format!("SYSTEM_ERROR: invalid oauth callback target: {e}"))?;
    if !matches!(
        url.path(),
        "/callback" | "/auth/callback" | "/oauth2callback"
    ) {
        return Err("SYSTEM_ERROR: invalid oauth callback path"
            .to_string()
            .into());
    }

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    let mut error_description: Option<String> = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(value.to_string()),
            "error_description" => error_description = Some(value.to_string()),
            _ => {}
        }
    }

    if code.is_none() && error.is_none() {
        return Err("SYSTEM_ERROR: oauth callback missing code/error"
            .to_string()
            .into());
    }

    Ok(OAuthCallbackPayload {
        code,
        state,
        error,
        error_description,
    })
}

fn validate_state(payload: &OAuthCallbackPayload, expected_state: &str) -> AppResult<()> {
    let state = payload
        .state
        .as_deref()
        .ok_or_else(|| "SYSTEM_ERROR: oauth callback missing state".to_string())?;
    if !constant_time_eq(state.as_bytes(), expected_state.as_bytes()) {
        return Err("SEC_INVALID_INPUT: oauth callback state mismatch"
            .to_string()
            .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_callback_target_extracts_code_and_state() {
        let payload = parse_callback_target("/callback?code=abc123&state=xyz").expect("payload");
        assert_eq!(payload.code.as_deref(), Some("abc123"));
        assert_eq!(payload.state.as_deref(), Some("xyz"));
        assert!(payload.error.is_none());
    }

    #[test]
    fn parse_callback_target_accepts_provider_error() {
        let payload =
            parse_callback_target("/callback?error=access_denied&error_description=nope&state=xyz")
                .expect("payload");
        assert_eq!(payload.error.as_deref(), Some("access_denied"));
        assert_eq!(payload.error_description.as_deref(), Some("nope"));
        assert_eq!(payload.state.as_deref(), Some("xyz"));
    }

    #[test]
    fn parse_callback_target_accepts_codex_callback_path() {
        let payload =
            parse_callback_target("/auth/callback?code=abc123&state=xyz").expect("payload");
        assert_eq!(payload.code.as_deref(), Some("abc123"));
        assert_eq!(payload.state.as_deref(), Some("xyz"));
    }

    #[test]
    fn parse_callback_target_accepts_gemini_callback_path() {
        let payload =
            parse_callback_target("/oauth2callback?code=abc123&state=xyz").expect("payload");
        assert_eq!(payload.code.as_deref(), Some("abc123"));
        assert_eq!(payload.state.as_deref(), Some("xyz"));
    }

    #[test]
    fn validate_state_rejects_mismatch() {
        let payload = OAuthCallbackPayload {
            code: Some("abc".to_string()),
            state: Some("foo".to_string()),
            error: None,
            error_description: None,
        };
        let err = validate_state(&payload, "bar").expect_err("should fail");
        assert!(err.to_string().contains("state mismatch"));
    }
}
