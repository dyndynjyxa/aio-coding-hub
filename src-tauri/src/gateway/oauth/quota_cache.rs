//! Usage: Short-lived in-memory cache for OAuth quota cooldown provider ids.

use crate::shared::error::AppResult;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

const QUOTA_CACHE_TTL_SECS: i64 = 5;

#[derive(Debug, Clone)]
struct QuotaCacheEntry {
    provider_ids: HashSet<i64>,
    expires_at_unix: i64,
}

#[derive(Debug, Default)]
pub(crate) struct QuotaCache {
    inner: Mutex<HashMap<String, QuotaCacheEntry>>,
}

impl QuotaCache {
    pub(crate) fn load_quota_exceeded_provider_ids_for_cli(
        &self,
        db: &crate::db::Db,
        cli_key: &str,
        now_unix: i64,
    ) -> AppResult<HashSet<i64>> {
        if let Some(cached) = self.get_if_fresh(cli_key, now_unix) {
            return Ok(cached);
        }

        let conn = db.open_connection()?;
        let provider_ids =
            crate::providers::list_oauth_quota_exceeded_provider_ids(&conn, cli_key, now_unix)?;
        self.store(cli_key, provider_ids.clone(), now_unix);
        Ok(provider_ids)
    }

    pub(crate) fn invalidate_cli(&self, cli_key: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.remove(cli_key);
        }
    }

    pub(crate) fn invalidate_all(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.clear();
        }
    }

    fn get_if_fresh(&self, cli_key: &str, now_unix: i64) -> Option<HashSet<i64>> {
        let mut inner = self.inner.lock().ok()?;
        if let Some(entry) = inner.get(cli_key) {
            if entry.expires_at_unix > now_unix {
                return Some(entry.provider_ids.clone());
            }
        }
        inner.remove(cli_key);
        None
    }

    fn store(&self, cli_key: &str, provider_ids: HashSet<i64>, now_unix: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.insert(
                cli_key.to_string(),
                QuotaCacheEntry {
                    provider_ids,
                    expires_at_unix: now_unix.saturating_add(QUOTA_CACHE_TTL_SECS.max(1)),
                },
            );
        }
    }
}

fn global_quota_cache() -> &'static QuotaCache {
    static QUOTA_CACHE: OnceLock<QuotaCache> = OnceLock::new();
    QUOTA_CACHE.get_or_init(QuotaCache::default)
}

pub(crate) fn load_quota_exceeded_provider_ids_for_cli(
    db: &crate::db::Db,
    cli_key: &str,
    now_unix: i64,
) -> AppResult<HashSet<i64>> {
    global_quota_cache().load_quota_exceeded_provider_ids_for_cli(db, cli_key, now_unix)
}

pub(crate) fn invalidate_cli(cli_key: &str) {
    global_quota_cache().invalidate_cli(cli_key);
}

pub(crate) fn invalidate_all() {
    global_quota_cache().invalidate_all();
}
