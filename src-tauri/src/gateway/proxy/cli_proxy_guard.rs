//! Usage: CLI proxy enabled guard (cached lookup to protect the gateway endpoints).

use crate::cli_proxy;
use crate::gateway::util::now_unix_millis;
use crate::shared::mutex_ext::MutexExt;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const CLI_PROXY_ENABLED_CACHE_TTL_MS_OK: i64 = 500;
const CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR: i64 = 5_000;
const CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES: usize = 16;

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

pub(super) fn cli_proxy_enabled_cached<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
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
        Err(err) => (false, Some(err.to_string())),
    };
    let cache_ttl_ms = if error.is_some() {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR
    } else {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_OK
    };

    {
        let mut cache = cache.lock_or_recover();
        prune_cli_proxy_enabled_cache(&mut cache, now_unix_ms);
        evict_oldest_cli_proxy_enabled_cache_entry_if_full(&mut cache, cli_key);
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

fn prune_cli_proxy_enabled_cache(
    cache: &mut HashMap<String, CliProxyEnabledCacheEntry>,
    now_unix_ms: i64,
) {
    cache.retain(|_, entry| entry.expires_at_unix_ms > now_unix_ms);
}

fn evict_oldest_cli_proxy_enabled_cache_entry_if_full(
    cache: &mut HashMap<String, CliProxyEnabledCacheEntry>,
    cli_key: &str,
) {
    if cache.len() < CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES || cache.contains_key(cli_key) {
        return;
    }

    let Some(oldest_key) = cache
        .iter()
        .min_by_key(|(_, entry)| entry.expires_at_unix_ms)
        .map(|(key, _)| key.clone())
    else {
        return;
    };
    cache.remove(&oldest_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache_entry(expires_at_unix_ms: i64) -> CliProxyEnabledCacheEntry {
        CliProxyEnabledCacheEntry {
            enabled: true,
            error: None,
            expires_at_unix_ms,
        }
    }

    #[test]
    fn prune_cli_proxy_enabled_cache_removes_expired_entries() {
        let mut cache = HashMap::new();
        cache.insert("expired".to_string(), cache_entry(100));
        cache.insert("fresh".to_string(), cache_entry(101));

        prune_cli_proxy_enabled_cache(&mut cache, 100);

        assert!(!cache.contains_key("expired"));
        assert!(cache.contains_key("fresh"));
    }

    #[test]
    fn evict_oldest_cli_proxy_enabled_cache_entry_keeps_capacity() {
        let mut cache = HashMap::new();
        for index in 0..CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES {
            cache.insert(format!("cli-{index}"), cache_entry(1_000 + index as i64));
        }

        evict_oldest_cli_proxy_enabled_cache_entry_if_full(&mut cache, "new-cli");
        cache.insert("new-cli".to_string(), cache_entry(2_000));

        assert_eq!(cache.len(), CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES);
        assert!(!cache.contains_key("cli-0"));
        assert!(cache.contains_key("cli-1"));
        assert!(cache.contains_key("new-cli"));
    }

    #[test]
    fn evict_oldest_cli_proxy_enabled_cache_entry_does_not_evict_existing_key() {
        let mut cache = HashMap::new();
        for index in 0..CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES {
            cache.insert(format!("cli-{index}"), cache_entry(1_000 + index as i64));
        }

        evict_oldest_cli_proxy_enabled_cache_entry_if_full(&mut cache, "cli-0");

        assert_eq!(cache.len(), CLI_PROXY_ENABLED_CACHE_MAX_ENTRIES);
        assert!(cache.contains_key("cli-0"));
    }
}
