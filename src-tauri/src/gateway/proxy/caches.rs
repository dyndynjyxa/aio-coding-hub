//! Usage: In-memory caches for gateway proxy behavior (error dedupe, base_url latency picks).

use axum::http::StatusCode;
use std::collections::HashMap;

const RECENT_ERROR_CACHE_MAX_ENTRIES: usize = 512;
const RECENT_TRACE_DEDUP_MAX_ENTRIES: usize = 1024;
pub(super) const RECENT_TRACE_DEDUP_TTL_SECS: i64 = 10;

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
    traces: HashMap<u64, CachedTraceId>,
}

#[derive(Debug, Clone)]
struct CachedTraceId {
    trace_id: String,
    expires_at_unix: i64,
    fingerprint_debug: String,
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

    pub(super) fn get_trace_id(
        &mut self,
        now_unix: i64,
        fingerprint_key: u64,
        fingerprint_debug: &str,
    ) -> Option<String> {
        self.prune_expired(now_unix);
        match self.traces.get(&fingerprint_key) {
            Some(entry)
                if entry.expires_at_unix > now_unix
                    && entry.fingerprint_debug == fingerprint_debug =>
            {
                Some(entry.trace_id.clone())
            }
            Some(_) => {
                self.traces.remove(&fingerprint_key);
                None
            }
            None => None,
        }
    }

    pub(super) fn upsert_trace_id(
        &mut self,
        now_unix: i64,
        fingerprint_key: u64,
        trace_id: String,
        fingerprint_debug: String,
        ttl_secs: i64,
    ) {
        self.prune_expired(now_unix);
        if self.traces.len() >= RECENT_TRACE_DEDUP_MAX_ENTRIES {
            if let Some((oldest_key, _)) = self
                .traces
                .iter()
                .min_by_key(|(_, v)| v.expires_at_unix)
                .map(|(k, v)| (*k, v.expires_at_unix))
            {
                self.traces.remove(&oldest_key);
            }
        }

        self.traces.insert(
            fingerprint_key,
            CachedTraceId {
                trace_id,
                expires_at_unix: now_unix.saturating_add(ttl_secs.max(1)),
                fingerprint_debug,
            },
        );
    }

    fn prune_expired(&mut self, now_unix: i64) {
        self.errors.retain(|_, v| v.expires_at_unix > now_unix);
        self.traces.retain(|_, v| v.expires_at_unix > now_unix);
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
