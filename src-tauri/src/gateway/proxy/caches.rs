//! Usage: In-memory caches for gateway proxy behavior (error dedupe, base_url latency picks).

use axum::http::StatusCode;
use std::collections::HashMap;

const RECENT_ERROR_CACHE_MAX_ENTRIES: usize = 512;
const PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES: usize = 1024;

#[derive(Debug, Clone)]
pub(super) struct CachedGatewayError {
    pub(super) trace_id: String,
    pub(super) status: StatusCode,
    pub(super) error_code: &'static str,
    pub(super) message: String,
    pub(super) retry_after_seconds: Option<u64>,
    pub(super) expires_at_unix: i64,
    pub(super) fingerprint_debug: String,
}

#[derive(Debug, Default)]
pub(in crate::gateway) struct RecentErrorCache {
    errors: HashMap<u64, CachedGatewayError>,
}

impl RecentErrorCache {
    pub(super) fn get_error(
        &mut self,
        now_unix: i64,
        fingerprint_key: u64,
        fingerprint_debug: &str,
    ) -> Option<CachedGatewayError> {
        self.prune_expired(now_unix);

        match self.errors.get(&fingerprint_key) {
            Some(entry)
                if entry.expires_at_unix > now_unix
                    && entry.fingerprint_debug == fingerprint_debug =>
            {
                let mut out = entry.clone();
                let remaining = out.expires_at_unix.saturating_sub(now_unix);
                out.retry_after_seconds = if remaining > 0 {
                    Some(remaining as u64)
                } else {
                    None
                };
                Some(out)
            }
            Some(_) => {
                self.errors.remove(&fingerprint_key);
                None
            }
            None => None,
        }
    }

    pub(super) fn insert_error(
        &mut self,
        now_unix: i64,
        fingerprint_key: u64,
        entry: CachedGatewayError,
    ) {
        self.prune_expired(now_unix);

        if self.errors.len() >= RECENT_ERROR_CACHE_MAX_ENTRIES
            && !self.errors.contains_key(&fingerprint_key)
        {
            if let Some((oldest_key, _)) = self
                .errors
                .iter()
                .min_by_key(|(_, v)| v.expires_at_unix)
                .map(|(k, v)| (*k, v.expires_at_unix))
            {
                self.errors.remove(&oldest_key);
            }
        }

        self.errors.insert(fingerprint_key, entry);
    }

    pub(in crate::gateway) fn clear(&mut self) -> usize {
        let count = self.errors.len();
        self.errors.clear();
        count
    }

    fn prune_expired(&mut self, now_unix: i64) {
        self.errors.retain(|_, v| v.expires_at_unix > now_unix);
    }

    #[cfg(test)]
    pub(in crate::gateway) fn has_active_error_for_tests(
        &self,
        now_unix: i64,
        fingerprint_key: u64,
        fingerprint_debug: &str,
    ) -> bool {
        self.errors.get(&fingerprint_key).is_some_and(|entry| {
            entry.expires_at_unix > now_unix && entry.fingerprint_debug == fingerprint_debug
        })
    }

    #[cfg(test)]
    pub(in crate::gateway) fn insert_unavailable_for_tests(
        &mut self,
        now_unix: i64,
        fingerprint_key: u64,
        fingerprint_debug: &str,
        retry_after_seconds: u64,
    ) {
        self.insert_error(
            now_unix,
            fingerprint_key,
            CachedGatewayError {
                trace_id: "trace-test".to_string(),
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
                message: "cached unavailable".to_string(),
                retry_after_seconds: Some(retry_after_seconds),
                expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                fingerprint_debug: fingerprint_debug.to_string(),
            },
        );
    }
}

#[derive(Debug, Clone)]
struct CachedProviderBaseUrlPing {
    best_base_url: String,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Default)]
pub(in crate::gateway) struct ProviderBaseUrlPingCache {
    entries: HashMap<i64, CachedProviderBaseUrlPing>,
}

impl ProviderBaseUrlPingCache {
    fn prune_expired(&mut self, now_unix_ms: u64) {
        self.entries
            .retain(|_, v| v.expires_at_unix_ms > now_unix_ms);
    }

    fn evict_oldest_if_full(&mut self, provider_id: i64) {
        if self.entries.len() < PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES {
            return;
        }
        if self.entries.contains_key(&provider_id) {
            return;
        }
        if let Some((oldest_provider_id, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.expires_at_unix_ms)
            .map(|(k, v)| (*k, v.expires_at_unix_ms))
        {
            self.entries.remove(&oldest_provider_id);
        }
    }

    pub(super) fn get_valid_best_base_url(
        &mut self,
        provider_id: i64,
        now_unix_ms: u64,
        base_urls: &[String],
    ) -> Option<String> {
        self.prune_expired(now_unix_ms);

        let entry = self.entries.get(&provider_id)?;
        if entry.expires_at_unix_ms <= now_unix_ms {
            self.entries.remove(&provider_id);
            return None;
        }

        if !base_urls.iter().any(|u| u == &entry.best_base_url) {
            self.entries.remove(&provider_id);
            return None;
        }

        Some(entry.best_base_url.clone())
    }

    pub(super) fn put_best_base_url(
        &mut self,
        provider_id: i64,
        best_base_url: String,
        expires_at_unix_ms: u64,
        now_unix_ms: u64,
    ) {
        self.prune_expired(now_unix_ms);
        self.evict_oldest_if_full(provider_id);
        self.entries.insert(
            provider_id,
            CachedProviderBaseUrlPing {
                best_base_url,
                expires_at_unix_ms,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CachedGatewayError, ProviderBaseUrlPingCache, RecentErrorCache,
        PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES, RECENT_ERROR_CACHE_MAX_ENTRIES,
    };
    use axum::http::StatusCode;

    fn cached_error(expires_at_unix: i64, fingerprint_debug: &str) -> CachedGatewayError {
        CachedGatewayError {
            trace_id: "trace_1".to_string(),
            status: StatusCode::SERVICE_UNAVAILABLE,
            error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
            message: "cached unavailable".to_string(),
            retry_after_seconds: Some(30),
            expires_at_unix,
            fingerprint_debug: fingerprint_debug.to_string(),
        }
    }

    #[test]
    fn get_error_returns_remaining_retry_after_seconds() {
        let mut cache = RecentErrorCache::default();
        cache.insert_error(100, 10, cached_error(130, "fp-a"));

        let got = cache
            .get_error(110, 10, "fp-a")
            .expect("cached error should exist");

        assert_eq!(got.retry_after_seconds, Some(20));
        assert_eq!(got.trace_id, "trace_1");
        assert_eq!(got.error_code, "GW_ALL_PROVIDERS_UNAVAILABLE");
    }

    #[test]
    fn get_error_returns_none_after_expiration() {
        let mut cache = RecentErrorCache::default();
        cache.insert_error(100, 11, cached_error(130, "fp-b"));

        let got = cache.get_error(130, 11, "fp-b");
        assert!(got.is_none());
    }

    #[test]
    fn get_error_mismatched_debug_removes_stale_entry() {
        let mut cache = RecentErrorCache::default();
        cache.insert_error(100, 12, cached_error(140, "fp-correct"));

        let mismatch = cache.get_error(110, 12, "fp-other");
        assert!(mismatch.is_none());

        let second_read = cache.get_error(110, 12, "fp-correct");
        assert!(second_read.is_none());
    }

    #[test]
    fn clear_removes_all_cached_errors() {
        let mut cache = RecentErrorCache::default();
        cache.insert_error(100, 21, cached_error(140, "fp-one"));
        cache.insert_error(100, 22, cached_error(140, "fp-two"));

        cache.clear();

        assert!(cache.get_error(110, 21, "fp-one").is_none());
        assert!(cache.get_error(110, 22, "fp-two").is_none());
    }

    #[test]
    fn insert_error_updates_existing_key_without_evicting_another_entry() {
        let mut cache = RecentErrorCache::default();
        for key in 0..RECENT_ERROR_CACHE_MAX_ENTRIES {
            cache.insert_error(
                100,
                key as u64,
                cached_error(1_000 + key as i64, &format!("fp-{key}")),
            );
        }

        let update_key = (RECENT_ERROR_CACHE_MAX_ENTRIES - 1) as u64;
        cache.insert_error(100, update_key, cached_error(9_999, "fp-updated"));

        assert_eq!(cache.errors.len(), RECENT_ERROR_CACHE_MAX_ENTRIES);
        assert!(cache.errors.contains_key(&0));
        assert_eq!(
            cache
                .errors
                .get(&update_key)
                .map(|entry| entry.fingerprint_debug.as_str()),
            Some("fp-updated")
        );
    }

    #[test]
    fn base_url_ping_cache_returns_only_fresh_matching_urls() {
        let mut cache = ProviderBaseUrlPingCache::default();
        let base_url = "https://fast.example".to_string();

        cache.put_best_base_url(1, base_url.clone(), 2000, 1000);

        assert_eq!(
            cache.get_valid_best_base_url(1, 1500, std::slice::from_ref(&base_url)),
            Some(base_url.clone())
        );
        assert_eq!(
            cache.get_valid_best_base_url(1, 1500, &["https://other.example".to_string()]),
            None
        );

        cache.put_best_base_url(1, base_url.clone(), 2000, 1500);
        assert_eq!(
            cache.get_valid_best_base_url(1, 2000, std::slice::from_ref(&base_url)),
            None
        );
    }

    #[test]
    fn base_url_ping_cache_prunes_expired_entries_on_write() {
        let mut cache = ProviderBaseUrlPingCache::default();
        let expired_url = "https://expired.example".to_string();
        let fresh_url = "https://fresh.example".to_string();

        cache.put_best_base_url(1, expired_url.clone(), 1000, 0);
        cache.put_best_base_url(2, fresh_url.clone(), 3000, 1000);

        assert_eq!(
            cache.get_valid_best_base_url(1, 1000, std::slice::from_ref(&expired_url)),
            None
        );
        assert_eq!(
            cache.get_valid_best_base_url(2, 1000, std::slice::from_ref(&fresh_url)),
            Some(fresh_url)
        );
    }

    #[test]
    fn base_url_ping_cache_evicts_oldest_entry_when_full() {
        let mut cache = ProviderBaseUrlPingCache::default();

        for provider_id in 0..PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES {
            cache.put_best_base_url(
                provider_id as i64,
                format!("https://p{provider_id}.example"),
                10_000 + provider_id as u64,
                0,
            );
        }

        cache.put_best_base_url(
            PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES as i64,
            "https://new.example".to_string(),
            99_999,
            0,
        );

        assert_eq!(
            cache.get_valid_best_base_url(0, 1, &["https://p0.example".to_string()]),
            None
        );
        assert_eq!(
            cache.get_valid_best_base_url(1, 1, &["https://p1.example".to_string()]),
            Some("https://p1.example".to_string())
        );
        assert_eq!(
            cache.get_valid_best_base_url(
                PROVIDER_BASE_URL_PING_CACHE_MAX_ENTRIES as i64,
                1,
                &["https://new.example".to_string()]
            ),
            Some("https://new.example".to_string())
        );
    }
}
