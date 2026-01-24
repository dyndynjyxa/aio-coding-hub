//! Usage: CLI proxy enabled guard (cached lookup to protect the gateway endpoints).

use crate::cli_proxy;
use crate::gateway::util::now_unix_millis;
use crate::shared::mutex_ext::MutexExt;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const CLI_PROXY_ENABLED_CACHE_TTL_MS_OK: i64 = 500;
const CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR: i64 = 5_000;

#[derive(Debug, Clone)]
struct CliProxyEnabledCacheEntry {
    enabled: bool,
    error: Option<String>,
    expires_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub(super) struct CliProxyEnabledSnapshot {
    pub(super) enabled: bool,
    pub(super) error: Option<String>,
    pub(super) cache_hit: bool,
    pub(super) cache_ttl_ms: i64,
}

pub(super) fn cli_proxy_enabled_cached(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> CliProxyEnabledSnapshot {
    static CLI_PROXY_ENABLED_CACHE: OnceLock<Mutex<HashMap<String, CliProxyEnabledCacheEntry>>> =
        OnceLock::new();

    let now_unix_ms = now_unix_millis().min(i64::MAX as u64) as i64;
    let cache = CLI_PROXY_ENABLED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock_or_recover();
        if let Some(entry) = cache.get(cli_key) {
            if entry.expires_at_unix_ms > now_unix_ms {
                let cache_ttl_ms = if entry.error.is_some() {
                    CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR
                } else {
                    CLI_PROXY_ENABLED_CACHE_TTL_MS_OK
                };
                return CliProxyEnabledSnapshot {
                    enabled: entry.enabled,
                    error: entry.error.clone(),
                    cache_hit: true,
                    cache_ttl_ms,
                };
            }
        }
    }

    let (enabled, error) = match cli_proxy::is_enabled(app, cli_key) {
        Ok(v) => (v, None),
        Err(err) => (false, Some(err)),
    };
    let cache_ttl_ms = if error.is_some() {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR
    } else {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_OK
    };

    {
        let mut cache = cache.lock_or_recover();
        cache.insert(
            cli_key.to_string(),
            CliProxyEnabledCacheEntry {
                enabled,
                error: error.clone(),
                expires_at_unix_ms: now_unix_ms.saturating_add(cache_ttl_ms.max(1)),
            },
        );
    }

    CliProxyEnabledSnapshot {
        enabled,
        error,
        cache_hit: false,
        cache_ttl_ms,
    }
}
