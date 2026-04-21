//! Usage: In-memory caches for gateway proxy behavior (error dedupe, base_url latency picks).

use axum::http::StatusCode;
use std::collections::HashMap;

const RECENT_ERROR_CACHE_MAX_ENTRIES: usize = 512;

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

        if self.errors.len() >= RECENT_ERROR_CACHE_MAX_ENTRIES {
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

    pub(in crate::gateway) fn clear(&mut self) {
        self.errors.clear();
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
    pub(super) fn get_valid_best_base_url(
        &mut self,
        provider_id: i64,
        now_unix_ms: u64,
        base_urls: &[String],
    ) -> Option<String> {
        self.entries
            .retain(|_, v| v.expires_at_unix_ms > now_unix_ms);

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
    ) {
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
    use super::{CachedGatewayError, RecentErrorCache};
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
}
