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

/// Global abort handle for in-progress OAuth flows.
/// When a new flow starts, it cancels any prior pending flow so the old callback
/// listener is dropped immediately (frees the port).
static ACTIVE_FLOW_ABORT: Mutex<Option<watch::Sender<()>>> = Mutex::new(None);

/// Cancel any in-progress OAuth flow and return a receiver that the new flow
/// should select on so it can itself be cancelled by a future invocation.
pub(crate) fn cancel_previous_flow() -> watch::Receiver<()> {
    let mut guard = ACTIVE_FLOW_ABORT.lock().unwrap_or_else(|e| e.into_inner());
    // Dropping the old sender causes the old receiver to see a channel-closed signal,
    // which aborts the old `wait_for_callback` via the tokio::select! in the caller.
    let (tx, rx) = watch::channel(());
    *guard = Some(tx);
    rx
}

/// Default User-Agent for OAuth HTTP requests (mirrors official Codex CLI).
pub(crate) const DEFAULT_OAUTH_USER_AGENT: &str = "codex_cli_rs/0.76.0";
/// Default request timeout in seconds for OAuth HTTP requests.
pub(crate) const DEFAULT_OAUTH_TIMEOUT_SECS: u64 = 30;
/// Default connect timeout in seconds for OAuth HTTP requests.
pub(crate) const DEFAULT_OAUTH_CONNECT_TIMEOUT_SECS: u64 = 15;

/// Build an HTTP client with default OAuth settings.
pub(crate) fn build_default_oauth_http_client() -> Result<reqwest::Client, String> {
    build_oauth_http_client(
        DEFAULT_OAUTH_USER_AGENT,
        DEFAULT_OAUTH_TIMEOUT_SECS,
        DEFAULT_OAUTH_CONNECT_TIMEOUT_SECS,
    )
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
/// Respects standard proxy environment variables (`HTTPS_PROXY`, `HTTP_PROXY`,
/// `ALL_PROXY`) automatically via reqwest defaults.  Additionally, if the user
/// has set `AIO_OAUTH_PROXY_URL`, that URL will be configured as an explicit
/// "all traffic" proxy, which is useful in corporate environments where system
/// proxy detection is insufficient.
pub(crate) fn build_oauth_http_client(
    user_agent: &str,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs));

    // Explicit proxy override from dedicated env var.
    if let Ok(proxy_url) = std::env::var("AIO_OAUTH_PROXY_URL") {
        let trimmed = proxy_url.trim();
        if !trimmed.is_empty() {
            let masked = mask_oauth_proxy_env_value(trimmed);
            tracing::info!(
                proxy_url = %masked,
                "oauth: using explicit proxy from AIO_OAUTH_PROXY_URL"
            );
            let proxy = reqwest::Proxy::all(trimmed)
                .map_err(|e| format!("invalid AIO_OAUTH_PROXY_URL={masked}: {e}"))?;
            builder = builder.proxy(proxy);
        }
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

    struct EnvVarRestore {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarRestore {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
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

        let err = build_oauth_http_client("test-agent", 1, 1)
            .expect_err("invalid explicit proxy should fail")
            .to_string();

        assert!(err.contains("[redacted]"));
        assert!(!err.contains("super-secret"));
        assert!(!err.contains("user:"));
    }
}
