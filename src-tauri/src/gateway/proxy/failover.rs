//! Usage: Failover policy helpers (retry/switch decisions, provider selection, base_url picking).

use crate::providers;
use crate::shared::mutex_ext::MutexExt;
use std::collections::HashSet;
use std::future::Future;
use std::time::Duration;

use crate::gateway::runtime::GatewayAppState;
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
    let code = status.as_u16();

    // 5xx: brief pause before switching provider to avoid rapid-fire exhaustion
    if (500..600).contains(&code) && !matches!(code, 408 | 429) {
        return Some(Duration::from_millis(100));
    }

    if !matches!(code, 408 | 429) {
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

async fn first_successful_base_url_probe<F, Fut>(
    base_urls: &[String],
    mut probe: F,
) -> Option<(String, u64)>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<u64, String>> + Send + 'static,
{
    let mut join_set = tokio::task::JoinSet::new();
    for base_url in base_urls
        .iter()
        .map(|base_url| base_url.trim())
        .filter(|base_url| !base_url.is_empty())
    {
        let base_url = base_url.to_string();
        let task = probe(base_url.clone());
        join_set.spawn(async move { (base_url, task.await) });
    }

    while let Some(joined) = join_set.join_next().await {
        let Ok((base_url, result)) = joined else {
            continue;
        };
        let Ok(ms) = result else {
            continue;
        };
        return Some((base_url, ms));
    }

    None
}

pub(super) fn resolve_primary_provider_base_url(
    provider: &providers::ProviderForGateway,
    cli_key: &str,
) -> Result<String, String> {
    if provider.auth_mode == "oauth" {
        let registry = crate::gateway::oauth::registry::global_registry();
        let provider_type = provider
            .oauth_provider_type
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let adapter = if provider_type.is_empty() {
            registry.get_by_cli_key(cli_key).ok_or_else(|| {
                format!(
                    "SEC_INVALID_INPUT: no OAuth adapter for cli_key={cli_key} (provider_id={})",
                    provider.id
                )
            })?
        } else {
            registry.get_by_provider_type(provider_type).ok_or_else(|| {
                format!(
                    "SEC_INVALID_INPUT: no OAuth adapter for provider_type={provider_type} (provider_id={}, cli_key={cli_key})",
                    provider.id
                )
            })?
        };

        if adapter.cli_key() != cli_key {
            return Err(format!(
                "SEC_INVALID_STATE: oauth adapter mismatch for provider_id={} (cli_key={cli_key}, provider_type={}, resolved_cli_key={})",
                provider.id,
                if provider_type.is_empty() {
                    "<empty>"
                } else {
                    provider_type
                },
                adapter.cli_key()
            ));
        }

        return Ok(adapter.default_base_url().to_string());
    }

    // Skip empty strings — legacy DB rows may store base_url="" which causes
    // `build_target_url` to fail with "relative URL without a base".
    Ok(provider
        .base_urls
        .iter()
        .find(|u| !u.trim().is_empty())
        .cloned()
        .unwrap_or_default())
}

pub(super) async fn select_provider_base_url_for_request<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    provider: &providers::ProviderForGateway,
    cli_key: &str,
    cache_ttl_seconds: u32,
) -> Result<String, String> {
    let primary = resolve_primary_provider_base_url(provider, cli_key)?;

    // OAuth providers always use adapter.default_base_url(); ignore legacy/base-url mode.
    if provider.auth_mode == "oauth" {
        return Ok(primary);
    }

    if !matches!(provider.base_url_mode, providers::ProviderBaseUrlMode::Ping) {
        return Ok(primary);
    }

    if provider.base_urls.len() <= 1 {
        return Ok(primary);
    }

    let now_unix_ms = now_unix_millis();
    {
        let mut cache = state.latency_cache.lock_or_recover();
        if let Some(best) =
            cache.get_valid_best_base_url(provider.id, now_unix_ms, &provider.base_urls)
        {
            return Ok(best);
        }
    }

    let ttl_ms = (cache_ttl_seconds.max(1) as u64).saturating_mul(1000);
    let expires_at_unix_ms = now_unix_ms.saturating_add(ttl_ms);
    let timeout = Duration::from_millis(PROVIDER_BASE_URL_PING_TIMEOUT_MS);

    let client = state.client();
    let Some((best_base_url, _best_latency_ms)) =
        first_successful_base_url_probe(&provider.base_urls, |base_url| {
            let client = client.clone();
            async move { crate::base_url_probe::probe_base_url_ms(&client, &base_url, timeout).await }
        })
        .await
    else {
        return Ok(primary);
    };

    {
        let mut cache = state.latency_cache.lock_or_recover();
        cache.put_best_base_url(
            provider.id,
            best_base_url.clone(),
            expires_at_unix_ms,
            now_unix_ms,
        );
    }

    Ok(best_base_url)
}

#[cfg(test)]
mod tests;
