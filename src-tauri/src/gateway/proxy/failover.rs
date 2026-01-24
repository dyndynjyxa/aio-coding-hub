//! Usage: Failover policy helpers (retry/switch decisions, provider selection, base_url picking).

use crate::providers;
use crate::shared::mutex_ext::MutexExt;
use std::collections::HashSet;
use std::time::Duration;

use crate::gateway::manager::GatewayAppState;
use crate::gateway::util::now_unix_millis;

#[derive(Debug, Clone, Copy)]
pub(super) enum FailoverDecision {
    RetrySameProvider,
    SwitchProvider,
    Abort,
}

impl FailoverDecision {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::RetrySameProvider => "retry",
            Self::SwitchProvider => "switch",
            Self::Abort => "abort",
        }
    }
}

pub(super) fn retry_backoff_delay(
    status: reqwest::StatusCode,
    retry_index: u32,
) -> Option<Duration> {
    if !matches!(status.as_u16(), 408 | 429) {
        return None;
    }

    let retry_index = retry_index.max(1);
    let base_ms = 80u64;
    let max_ms = 800u64;
    let ms = base_ms.saturating_mul(retry_index as u64).min(max_ms);
    Some(Duration::from_millis(ms))
}

pub(super) fn should_reuse_provider(body_json: Option<&serde_json::Value>) -> bool {
    let Some(value) = body_json else {
        return false;
    };

    let len = value
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .or_else(|| {
            value
                .get("input")
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .or_else(|| {
            value
                .get("contents")
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .or_else(|| {
            value
                .get("request")
                .and_then(|v| v.get("contents"))
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .unwrap_or(0);

    len > 1
}

pub(super) fn select_next_provider_id_from_order(
    bound_provider_id: i64,
    provider_order: &[i64],
    current_provider_ids: &HashSet<i64>,
) -> Option<i64> {
    if provider_order.is_empty() || current_provider_ids.is_empty() {
        return None;
    }

    let start = match provider_order
        .iter()
        .position(|provider_id| *provider_id == bound_provider_id)
    {
        Some(idx) => idx.saturating_add(1),
        None => 0,
    };

    for offset in 0..provider_order.len() {
        let idx = (start + offset) % provider_order.len();
        let candidate = provider_order[idx];
        if current_provider_ids.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

const PROVIDER_BASE_URL_PING_TIMEOUT_MS: u64 = 2000;

pub(super) async fn select_provider_base_url_for_request(
    state: &GatewayAppState,
    provider: &providers::ProviderForGateway,
    cache_ttl_seconds: u32,
) -> String {
    let primary = provider
        .base_urls
        .first()
        .cloned()
        .unwrap_or_else(String::new);

    if !matches!(provider.base_url_mode, providers::ProviderBaseUrlMode::Ping) {
        return primary;
    }

    if provider.base_urls.len() <= 1 {
        return primary;
    }

    let now_unix_ms = now_unix_millis();
    {
        let mut cache = state.latency_cache.lock_or_recover();
        if let Some(best) =
            cache.get_valid_best_base_url(provider.id, now_unix_ms, &provider.base_urls)
        {
            return best;
        }
    }

    let ttl_ms = (cache_ttl_seconds.max(1) as u64).saturating_mul(1000);
    let expires_at_unix_ms = now_unix_ms.saturating_add(ttl_ms);
    let timeout = Duration::from_millis(PROVIDER_BASE_URL_PING_TIMEOUT_MS);

    let mut join_set = tokio::task::JoinSet::new();
    for base_url in provider.base_urls.iter().cloned() {
        let client = state.client.clone();
        join_set.spawn(async move {
            let result =
                crate::base_url_probe::probe_base_url_ms(&client, &base_url, timeout).await;
            (base_url, result)
        });
    }

    let mut best: Option<(String, u64)> = None;
    while let Some(joined) = join_set.join_next().await {
        let Ok((base_url, result)) = joined else {
            continue;
        };
        let Ok(ms) = result else {
            continue;
        };

        match best.as_ref() {
            Some((_, best_ms)) if ms >= *best_ms => {}
            _ => best = Some((base_url, ms)),
        }
    }

    let Some((best_base_url, _best_latency_ms)) = best else {
        return primary;
    };

    {
        let mut cache = state.latency_cache.lock_or_recover();
        cache.put_best_base_url(provider.id, best_base_url.clone(), expires_at_unix_ms);
    }

    best_base_url
}

#[cfg(test)]
mod tests {
    use super::select_next_provider_id_from_order;
    use std::collections::HashSet;

    fn set(ids: &[i64]) -> HashSet<i64> {
        ids.iter().copied().collect()
    }

    #[test]
    fn select_next_provider_id_wraps_and_skips_missing() {
        let order = vec![1, 2, 3, 4];
        let current = set(&[2, 4]);

        assert_eq!(
            select_next_provider_id_from_order(4, &order, &current),
            Some(2)
        );
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &current),
            Some(4)
        );
    }

    #[test]
    fn select_next_provider_id_returns_none_when_no_candidate() {
        let order = vec![1, 2, 3];
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &set(&[])),
            None
        );
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &set(&[99])),
            None
        );
    }

    #[test]
    fn select_next_provider_id_starts_from_head_when_bound_missing() {
        let order = vec![10, 20, 30];
        let current = set(&[30]);
        assert_eq!(
            select_next_provider_id_from_order(999, &order, &current),
            Some(30)
        );
    }
}
