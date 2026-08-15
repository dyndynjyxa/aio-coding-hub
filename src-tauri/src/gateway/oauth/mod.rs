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
        DEFAULT_OAUTH_USER_AGENT,
        DEFAULT_OAUTH_TIMEOUT_SECS,
        DEFAULT_OAUTH_CONNECT_TIMEOUT_SECS,
        resolve_app_configured_proxy_url(app).as_deref(),
    )
}

/// Resolve the app's configured upstream proxy URL (Settings → 上游代理) for use
/// by OAuth HTTP clients. Returns `None` (after logging) if settings can't be
/// read or the configured proxy is invalid, so a local settings hiccup never
/// hard-fails a login/refresh attempt — it just falls back to no explicit proxy.
pub(crate) fn resolve_app_configured_proxy_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<String> {
    let settings = match crate::settings::read(app) {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!("oauth: failed to read app settings for proxy resolution: {err}");
            return None;
        }
    };

    // Run the same validation the gateway applies when it installs the proxy
    // (scheme/format plus the self-loop guard), so a hand-edited settings.json
    // can never point OAuth traffic back at the gateway itself.
    if let Err(err) = super::http_client::validate_proxy_for_settings(&settings) {
        tracing::warn!("oauth: ignoring invalid configured upstream proxy: {err}");
        return None;
    }

    match super::http_client::effective_proxy_url(&settings) {
        Ok(url) => url,
        Err(err) => {
            tracing::warn!("oauth: ignoring invalid configured upstream proxy: {err}");
            None
        }
    }
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
/// 2. `configured_proxy_url` — the app's Settings → 上游代理 (Upstream Proxy),
///    resolved via [`resolve_app_configured_proxy_url`]. This is the same proxy
///    the gateway uses for upstream API calls (supports `http(s)://` and
///    `socks5(h)://`), so enabling it also routes OAuth login/refresh/reset
///    traffic — no separate proxy setup is needed to log in from behind a firewall.
/// 3. Standard proxy env vars (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), picked
///    up automatically via reqwest defaults.
pub(crate) fn build_oauth_http_client(
    user_agent: &str,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    configured_proxy_url: Option<&str>,
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
    } else if let Some(proxy_url) = configured_proxy_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
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
        // Log which standard proxy env vars are active for diagnostics.
        for var in [
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "ALL_PROXY",
            "https_proxy",
            "http_proxy",
            "all_proxy",
        ] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    tracing::debug!(
                        env_var = var,
                        value = %mask_oauth_proxy_env_value(&val),
                        "oauth: detected proxy env var"
                    );
                }
            }
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
    use std::sync::{Mutex, MutexGuard, OnceLock};

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

        let err = build_oauth_http_client("test-agent", 1, 1, None)
            .expect_err("invalid explicit proxy should fail")
            .to_string();

        assert!(err.contains("[redacted]"));
        assert!(!err.contains("super-secret"));
        assert!(!err.contains("user:"));
    }

    #[test]
    fn configured_proxy_url_is_applied_when_no_env_override() {
        let _env_lock = crate::test_support::test_env_lock();
        let _restore = EnvVarRestore::unset("AIO_OAUTH_PROXY_URL");

        let result = build_oauth_http_client("test-agent", 1, 1, Some("socks5://127.0.0.1:1080"));

        assert!(
            result.is_ok(),
            "configured socks5 proxy should build a client"
        );
    }

    #[test]
    fn env_override_takes_priority_over_configured_proxy_url() {
        let _env_lock = crate::test_support::test_env_lock();
        let _restore = EnvVarRestore::set("AIO_OAUTH_PROXY_URL", "http://user:super-secret@");

        // Even though a valid configured proxy is supplied, the (invalid) env
        // override must still win so `AIO_OAUTH_PROXY_URL` stays authoritative.
        let err = build_oauth_http_client("test-agent", 1, 1, Some("socks5://127.0.0.1:1080"))
            .expect_err("invalid env override should fail even with a valid configured proxy");

        assert!(err.contains("AIO_OAUTH_PROXY_URL"));
    }

    #[test]
    fn empty_env_override_falls_through_to_configured_proxy() {
        let _env_lock = crate::test_support::test_env_lock();
        let _restore = EnvVarRestore::set("AIO_OAUTH_PROXY_URL", "   ");

        // The configured proxy is invalid, so surfacing *its* error proves the
        // blank env override did not shadow it into a silent direct connection.
        let err = build_oauth_http_client("test-agent", 1, 1, Some("http://user:super-secret@"))
            .expect_err("invalid configured proxy should fail once the env override is blank");

        assert!(err.contains("upstream proxy"));
        assert!(!err.contains("AIO_OAUTH_PROXY_URL"));
        assert!(err.contains("[redacted]"));
        assert!(!err.contains("super-secret"));
    }

    #[test]
    fn resolve_app_configured_proxy_url_reflects_settings() {
        let _lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let home_dir = home.path().as_os_str().to_os_string();
        // Drop guards, so a failed assertion cannot leak these into the rest of
        // the (single-threaded) test binary and break every later settings read.
        let _home_restore = EnvVarRestore::set("AIO_CODING_HUB_HOME_DIR", home_dir);
        let dotdir = ".aio-coding-hub-oauth-proxy-test";
        let _dotdir_restore = EnvVarRestore::set("AIO_CODING_HUB_DOTDIR_NAME", dotdir);
        crate::test_support::clear_settings_cache();

        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        assert_eq!(resolve_app_configured_proxy_url(&handle), None);

        let mut settings = crate::settings::read(&handle).expect("read default settings");
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "socks5://ssh-proxy:1080".to_string();
        crate::settings::write(&handle, &settings).expect("persist settings");
        crate::test_support::clear_settings_cache();

        let resolved = resolve_app_configured_proxy_url(&handle).expect("proxy should resolve");
        let parsed = reqwest::Url::parse(&resolved).expect("resolved proxy url should parse");
        assert_eq!(parsed.scheme(), "socks5");
        assert_eq!(parsed.host_str(), Some("ssh-proxy"));
        assert_eq!(parsed.port(), Some(1080));
    }

    #[test]
    fn resolve_app_configured_proxy_url_rejects_gateway_self_loop() {
        let _lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let home_dir = home.path().as_os_str().to_os_string();
        let _home_restore = EnvVarRestore::set("AIO_CODING_HUB_HOME_DIR", home_dir);
        let dotdir = ".aio-coding-hub-oauth-self-loop-test";
        let _dotdir_restore = EnvVarRestore::set("AIO_CODING_HUB_DOTDIR_NAME", dotdir);
        crate::test_support::clear_settings_cache();

        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        let mut settings = crate::settings::read(&handle).expect("read default settings");
        let gateway_proxy_url = format!("http://127.0.0.1:{}", settings.preferred_port);
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = gateway_proxy_url;
        crate::settings::write(&handle, &settings).expect("persist settings");
        crate::test_support::clear_settings_cache();

        // A hand-edited settings.json pointing at the gateway must not be used
        // for OAuth traffic either — the gateway rejects it for the same reason.
        assert_eq!(resolve_app_configured_proxy_url(&handle), None);
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
