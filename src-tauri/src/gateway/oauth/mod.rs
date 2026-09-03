//! Usage: OAuth adapter pattern for multi-CLI OAuth login support.

pub(crate) mod adapters;
pub(crate) mod callback_server;
pub(crate) mod pkce;
pub(crate) mod provider_trait;
pub(crate) mod refresh;
pub(crate) mod refresh_loop;
pub(crate) mod registry;
pub(crate) mod token_exchange;

use std::sync::Mutex;
use tokio::sync::watch;

struct ActiveOAuthFlow {
    flow_id: String,
    _abort: watch::Sender<()>,
}

pub(crate) struct OAuthFlowLifecycle {
    pub(crate) flow_id: String,
    pub(crate) abort_rx: watch::Receiver<()>,
}

/// Global lifecycle handle for in-progress OAuth flows.
/// When a new flow starts, it cancels any prior pending flow so the old callback
/// listener is dropped immediately (frees the port) and stale device-code polls
/// can no longer persist tokens.
static ACTIVE_FLOW: Mutex<Option<ActiveOAuthFlow>> = Mutex::new(None);

fn generate_flow_id() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Cancel any in-progress OAuth flow and return a receiver that the new flow
/// should select on so it can itself be cancelled by a future invocation.
pub(crate) fn begin_flow_lifecycle() -> OAuthFlowLifecycle {
    let mut guard = ACTIVE_FLOW.lock().unwrap_or_else(|e| e.into_inner());
    // Dropping the old sender causes the old receiver to see a channel-closed signal,
    // which aborts the old `wait_for_callback` via the tokio::select! in the caller.
    let (tx, rx) = watch::channel(());
    let flow_id = generate_flow_id();
    *guard = Some(ActiveOAuthFlow {
        flow_id: flow_id.clone(),
        _abort: tx,
    });
    OAuthFlowLifecycle {
        flow_id,
        abort_rx: rx,
    }
}

pub(crate) fn is_current_flow(flow_id: &str) -> bool {
    let guard = ACTIVE_FLOW.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .is_some_and(|active| active.flow_id == flow_id)
}

pub(crate) fn cancel_flow(flow_id: &str) -> bool {
    let mut guard = ACTIVE_FLOW.lock().unwrap_or_else(|e| e.into_inner());
    if guard
        .as_ref()
        .is_some_and(|active| active.flow_id == flow_id)
    {
        *guard = None;
        true
    } else {
        false
    }
}

pub(crate) fn complete_current_flow<T>(
    flow_id: &str,
    complete: impl FnOnce() -> crate::shared::error::AppResult<T>,
) -> crate::shared::error::AppResult<T> {
    let mut guard = ACTIVE_FLOW.lock().unwrap_or_else(|e| e.into_inner());
    if guard
        .as_ref()
        .is_none_or(|active| active.flow_id != flow_id)
    {
        return Err(crate::shared::error::AppError::from(
            "OAuth flow cancelled: login attempt is no longer current".to_string(),
        ));
    }

    let result = complete();
    if result.is_ok() {
        *guard = None;
    }
    result
}

/// Default User-Agent for OAuth HTTP requests (mirrors the supported Codex CLI).
pub(crate) const DEFAULT_OAUTH_USER_AGENT: &str =
    crate::gateway::upstream_identity::CODEX_CLI_USER_AGENT;
/// Default request timeout in seconds for OAuth HTTP requests.
pub(crate) const DEFAULT_OAUTH_TIMEOUT_SECS: u64 = 30;
/// Default connect timeout in seconds for OAuth HTTP requests.
pub(crate) const DEFAULT_OAUTH_CONNECT_TIMEOUT_SECS: u64 = 15;

/// Build an HTTP client with default OAuth settings, honoring the app's
/// configured upstream proxy (Settings → 上游代理) in addition to env overrides.
pub(crate) fn build_default_oauth_http_client<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<reqwest::Client, String> {
    build_oauth_http_client(
        app,
        DEFAULT_OAUTH_USER_AGENT,
        DEFAULT_OAUTH_TIMEOUT_SECS,
        DEFAULT_OAUTH_CONNECT_TIMEOUT_SECS,
    )
}

fn resolve_app_configured_proxy_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    let settings = crate::settings::read(app)
        .map_err(|err| format!("oauth upstream proxy settings unavailable: {err}"))?;

    super::http_client::validate_proxy_for_settings(&settings)
        .map_err(|err| format!("invalid app upstream proxy settings: {err}"))?;

    super::http_client::effective_proxy_url(&settings)
        .map_err(|err| format!("invalid app upstream proxy settings: {err}"))
}

fn mask_oauth_proxy_env_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if reqwest::Url::parse(trimmed).is_err() && trimmed.contains('@') {
        return "[redacted]".to_string();
    }
    super::http_client::mask_url(trimmed)
}

/// Build an HTTP client suitable for OAuth token exchange and refresh requests.
///
/// Proxy resolution order:
/// 1. `AIO_OAUTH_PROXY_URL` env var — explicit override for advanced/dev setups.
///    An empty/whitespace value counts as unset so it cannot silently shadow the
///    app-configured proxy.
/// 2. The app's Settings → 上游代理 (Upstream Proxy). This is the same proxy
///    the gateway uses for upstream API calls (supports `http(s)://` and
///    `socks5(h)://`), so enabling it also routes OAuth login/refresh/reset
///    traffic — no separate proxy setup is needed to log in from behind a firewall.
/// 3. Standard proxy env vars (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), picked
///    up automatically via reqwest defaults.
pub(crate) fn build_oauth_http_client<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    user_agent: &str,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs));

    // Explicit proxy override from dedicated env var. An empty value is treated
    // as unset: otherwise `AIO_OAUTH_PROXY_URL=` (common in container/launcher
    // setups) would fall into this branch and silently drop the configured proxy.
    let env_override = std::env::var("AIO_OAUTH_PROXY_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(proxy_url) = env_override.as_deref() {
        let masked = mask_oauth_proxy_env_value(proxy_url);
        tracing::info!(
            proxy_url = %masked,
            "oauth: using explicit proxy from AIO_OAUTH_PROXY_URL"
        );
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| format!("invalid AIO_OAUTH_PROXY_URL={masked}: {e}"))?;
        builder = super::http_client::apply_socks5_local_dns_workaround(builder, proxy_url);
        builder = builder.proxy(proxy);
    } else {
        let configured_proxy_url = resolve_app_configured_proxy_url(app)?;
        if let Some(proxy_url) = configured_proxy_url.as_deref() {
            let masked = mask_oauth_proxy_env_value(proxy_url);
            tracing::info!(
                proxy_url = %masked,
                "oauth: using app-configured upstream proxy"
            );
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| format!("invalid upstream proxy '{masked}': {e}"))?;
            builder = super::http_client::apply_socks5_local_dns_workaround(builder, proxy_url);
            builder = builder.proxy(proxy);
        } else {
            builder = super::http_client::apply_system_proxy_self_loop_guard(builder);
        }
    }

    builder
        .build()
        .map_err(|e| format!("oauth HTTP client init failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{mpsc, Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::Duration;

    struct EnvVarRestore {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarRestore {
        fn set(key: &'static str, value: impl Into<OsString>) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value.into());
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    struct OAuthProxyTestGuard<'a> {
        _guard: MutexGuard<'a, ()>,
    }

    fn oauth_proxy_test_lock() -> OAuthProxyTestGuard<'static> {
        OAuthProxyTestGuard {
            _guard: crate::test_support::test_env_lock(),
        }
    }

    fn isolate_settings_env(
        home: &tempfile::TempDir,
        dotdir: &'static str,
    ) -> (EnvVarRestore, EnvVarRestore) {
        crate::test_support::clear_settings_cache();
        (
            EnvVarRestore::set(
                "AIO_CODING_HUB_HOME_DIR",
                home.path().as_os_str().to_os_string(),
            ),
            EnvVarRestore::set("AIO_CODING_HUB_DOTDIR_NAME", dotdir),
        )
    }

    fn unset_standard_proxy_env() -> Vec<EnvVarRestore> {
        [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
            "NO_PROXY",
            "no_proxy",
        ]
        .into_iter()
        .map(EnvVarRestore::unset)
        .collect()
    }

    fn spawn_recording_http_proxy() -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind proxy listener");
        let addr = listener.local_addr().expect("proxy listener addr");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept proxy request");
            let mut buf = [0_u8; 4096];
            let size = stream.read(&mut buf).expect("read proxy request");
            tx.send(String::from_utf8_lossy(&buf[..size]).to_string())
                .expect("record proxy request");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .expect("write proxy response");
        });

        (format!("http://127.0.0.1:{}", addr.port()), rx)
    }

    fn oauth_flow_test_lock() -> MutexGuard<'static, ()> {
        static FLOW_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        FLOW_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner())
    }

    fn reset_oauth_flow_for_test() {
        let mut guard = ACTIVE_FLOW.lock().unwrap_or_else(|err| err.into_inner());
        *guard = None;
    }

    #[test]
    fn oauth_proxy_env_mask_redacts_valid_url_credentials() {
        assert_eq!(
            mask_oauth_proxy_env_value("http://user:secret@proxy.example.com:7890"),
            "http://proxy.example.com:7890"
        );
    }

    #[test]
    fn oauth_proxy_env_mask_redacts_invalid_credential_like_values() {
        assert_eq!(
            mask_oauth_proxy_env_value("http://user:super-secret@"),
            "[redacted]"
        );
    }

    #[test]
    fn explicit_oauth_proxy_error_masks_env_value() {
        let _env_lock = crate::test_support::test_env_lock();
        let _restore = EnvVarRestore::set("AIO_OAUTH_PROXY_URL", "http://user:super-secret@");
        let app = tauri::test::mock_app();

        let err = build_oauth_http_client(app.handle(), "test-agent", 1, 1)
            .expect_err("invalid explicit proxy should fail")
            .to_string();

        assert!(err.contains("[redacted]"));
        assert!(!err.contains("super-secret"));
        assert!(!err.contains("user:"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blank_env_uses_app_proxy_for_real_request() {
        let _env_lock = oauth_proxy_test_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-real-proxy-test");
        let _oauth_proxy_restore = EnvVarRestore::set("AIO_OAUTH_PROXY_URL", "   ");
        let _system_proxy_restores = unset_standard_proxy_env();
        let (proxy_url, request_rx) = spawn_recording_http_proxy();
        let app = tauri::test::mock_app();

        let mut settings = crate::settings::read(app.handle()).expect("read default settings");
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = proxy_url;
        crate::settings::write(app.handle(), &settings).expect("persist settings");

        let client = build_oauth_http_client(app.handle(), "test-agent", 3, 1)
            .expect("configured proxy should build client");
        let response = client
            .get("http://oauth.example.test/token")
            .send()
            .await
            .expect("OAuth request should use configured proxy");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let request = request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("configured proxy should receive OAuth request");
        assert!(request.starts_with("GET http://oauth.example.test/token HTTP/1.1"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn env_override_bypasses_unreadable_app_settings() {
        let _env_lock = oauth_proxy_test_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-env-priority-test");
        let _system_proxy_restores = unset_standard_proxy_env();
        let (proxy_url, request_rx) = spawn_recording_http_proxy();
        let _oauth_proxy_restore = EnvVarRestore::set("AIO_OAUTH_PROXY_URL", proxy_url.as_str());
        let app = tauri::test::mock_app();
        let settings_path = crate::app_paths::app_data_dir(app.handle())
            .expect("resolve app data dir")
            .join("settings.json");
        std::fs::write(settings_path, b"{invalid-json").expect("write invalid settings");
        crate::test_support::clear_settings_cache();

        let client = build_oauth_http_client(app.handle(), "test-agent", 3, 1)
            .expect("env override should avoid reading app settings");
        let response = client
            .get("http://oauth.example.test/device")
            .send()
            .await
            .expect("OAuth request should use env proxy");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let request = request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("env proxy should receive OAuth request");
        assert!(request.starts_with("GET http://oauth.example.test/device HTTP/1.1"));
    }

    #[test]
    fn unreadable_app_settings_fail_closed_without_env_override() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-settings-error-test");
        let _oauth_proxy_restore = EnvVarRestore::unset("AIO_OAUTH_PROXY_URL");
        let app = tauri::test::mock_app();
        let settings_path = crate::app_paths::app_data_dir(app.handle())
            .expect("resolve app data dir")
            .join("settings.json");
        std::fs::write(settings_path, b"{invalid-json").expect("write invalid settings");
        crate::test_support::clear_settings_cache();

        let err = build_oauth_http_client(app.handle(), "test-agent", 1, 1)
            .expect_err("unreadable settings must not silently fall back")
            .to_string();

        assert!(err.contains("oauth upstream proxy settings unavailable"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn automatic_system_proxy_self_loop_is_bypassed() {
        let _env_lock = oauth_proxy_test_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-system-self-loop-test");
        let _oauth_proxy_restore = EnvVarRestore::unset("AIO_OAUTH_PROXY_URL");
        let _system_proxy_restores = unset_standard_proxy_env();
        let (server_url, request_rx) = spawn_recording_http_proxy();
        let port = reqwest::Url::parse(&server_url)
            .expect("parse server URL")
            .port()
            .expect("server URL port");
        let _http_proxy_restore = EnvVarRestore::set("HTTP_PROXY", server_url.as_str());
        super::super::http_client::sync_runtime_context(port, "127.0.0.1", "127.0.0.1");
        let app = tauri::test::mock_app();

        let client = build_oauth_http_client(app.handle(), "test-agent", 3, 1)
            .expect("system self-loop guard should build client");
        let response = client
            .get(format!("{server_url}/token"))
            .send()
            .await
            .expect("self-loop proxy should be bypassed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let request = request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("origin should receive direct request");
        assert!(request.starts_with("GET /token HTTP/1.1"));
    }

    #[test]
    fn enabled_invalid_app_proxy_fails_closed_without_leaking_credentials() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-fail-closed-test");
        let _oauth_proxy_restore = EnvVarRestore::unset("AIO_OAUTH_PROXY_URL");
        let app = tauri::test::mock_app();

        let mut settings = crate::settings::read(app.handle()).expect("read default settings");
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "http://user:super-secret@".to_string();
        crate::settings::write(app.handle(), &settings).expect("persist settings");

        let err = build_oauth_http_client(app.handle(), "test-agent", 1, 1)
            .expect_err("invalid enabled proxy must not fall back")
            .to_string();

        assert!(err.contains("invalid app upstream proxy settings"));
        assert!(!err.contains("AIO_OAUTH_PROXY_URL"));
        assert!(err.contains("[redacted]"));
        assert!(!err.contains("super-secret"));
    }

    #[test]
    fn resolve_app_configured_proxy_url_reflects_settings() {
        let _lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-proxy-test");

        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        assert_eq!(
            resolve_app_configured_proxy_url(&handle).expect("default settings should resolve"),
            None
        );

        let mut settings = crate::settings::read(&handle).expect("read default settings");
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "socks5://ssh-proxy:1080".to_string();
        crate::settings::write(&handle, &settings).expect("persist settings");
        crate::test_support::clear_settings_cache();

        let resolved = resolve_app_configured_proxy_url(&handle)
            .expect("settings should be valid")
            .expect("proxy should resolve");
        let parsed = reqwest::Url::parse(&resolved).expect("resolved proxy url should parse");
        assert_eq!(parsed.scheme(), "socks5");
        assert_eq!(parsed.host_str(), Some("ssh-proxy"));
        assert_eq!(parsed.port(), Some(1080));
    }

    #[test]
    fn resolve_app_configured_proxy_url_rejects_gateway_self_loop() {
        let _lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let (_home_restore, _dotdir_restore) =
            isolate_settings_env(&home, ".aio-coding-hub-oauth-self-loop-test");

        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        let mut settings = crate::settings::read(&handle).expect("read default settings");
        let gateway_proxy_url = format!("http://127.0.0.1:{}", settings.preferred_port);
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = gateway_proxy_url;
        crate::settings::write(&handle, &settings).expect("persist settings");
        crate::test_support::clear_settings_cache();

        let err = resolve_app_configured_proxy_url(&handle)
            .expect_err("gateway self-loop must fail closed");
        assert!(err.contains("self-loop"));
    }

    #[test]
    fn oauth_flow_lifecycle_replaces_current_flow() {
        let _flow_lock = oauth_flow_test_lock();
        reset_oauth_flow_for_test();

        let first = begin_flow_lifecycle();
        assert!(is_current_flow(&first.flow_id));

        let second = begin_flow_lifecycle();
        assert!(!is_current_flow(&first.flow_id));
        assert!(is_current_flow(&second.flow_id));

        assert!(!cancel_flow(&first.flow_id));
        assert!(cancel_flow(&second.flow_id));
        assert!(!is_current_flow(&second.flow_id));
    }

    #[test]
    fn oauth_flow_completion_rejects_stale_flow() {
        let _flow_lock = oauth_flow_test_lock();
        reset_oauth_flow_for_test();

        let first = begin_flow_lifecycle();
        let second = begin_flow_lifecycle();

        let stale = complete_current_flow(&first.flow_id, || {
            Ok::<_, crate::shared::error::AppError>(())
        });
        assert!(stale.is_err());

        let current = complete_current_flow(&second.flow_id, || {
            Ok::<_, crate::shared::error::AppError>(())
        });
        assert!(current.is_ok());
        assert!(!is_current_flow(&second.flow_id));
    }
}
