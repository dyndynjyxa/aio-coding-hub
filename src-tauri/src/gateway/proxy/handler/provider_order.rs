use crate::gateway::proxy::failover::select_next_provider_id_from_order;
use crate::providers;
use std::collections::{HashMap, HashSet};

pub(super) fn reorder_providers_by_bound_order(
    providers: &mut Vec<providers::ProviderForGateway>,
    order: &[i64],
) {
    if order.is_empty() || providers.len() <= 1 {
        return;
    }

    let mut by_id: HashMap<i64, providers::ProviderForGateway> =
        HashMap::with_capacity(providers.len());
    let mut original_ids: Vec<i64> = Vec::with_capacity(providers.len());
    for item in providers.drain(..) {
        original_ids.push(item.id);
        by_id.insert(item.id, item);
    }

    let mut reordered: Vec<providers::ProviderForGateway> = Vec::with_capacity(original_ids.len());
    for provider_id in order {
        if let Some(item) = by_id.remove(provider_id) {
            reordered.push(item);
        }
    }
    for provider_id in original_ids {
        if let Some(item) = by_id.remove(&provider_id) {
            reordered.push(item);
        }
    }

    *providers = reordered;
}

pub(super) fn apply_session_provider_preference(
    providers: &mut Vec<providers::ProviderForGateway>,
    bound_provider_id: i64,
    bound_provider_order: Option<&[i64]>,
) -> Option<i64> {
    if providers.is_empty() {
        return None;
    }

    if let Some(idx) = providers.iter().position(|p| p.id == bound_provider_id) {
        if idx > 0 {
            let chosen = providers.remove(idx);
            providers.insert(0, chosen);
        }
        return Some(bound_provider_id);
    }

    let order = bound_provider_order?;
    if order.is_empty() || providers.len() <= 1 {
        return None;
    }

    let current_provider_ids: HashSet<i64> = providers.iter().map(|p| p.id).collect();
    let next_provider_id =
        select_next_provider_id_from_order(bound_provider_id, order, &current_provider_ids)?;

    if let Some(idx) = providers.iter().position(|p| p.id == next_provider_id) {
        if idx > 0 {
            providers.rotate_left(idx);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{apply_session_provider_preference, reorder_providers_by_bound_order};
    use crate::providers;

    fn provider(id: i64) -> providers::ProviderForGateway {
        providers::ProviderForGateway {
            id,
            name: format!("p{id}"),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: String::new(),
            claude_models: providers::ClaudeModels::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
        }
    }

    fn ids(items: &[providers::ProviderForGateway]) -> Vec<i64> {
        items.iter().map(|item| item.id).collect()
    }

    #[test]
    fn reorder_by_bound_order_preserves_unspecified_tail() {
        let mut providers = vec![provider(1), provider(2), provider(3), provider(4)];
        reorder_providers_by_bound_order(&mut providers, &[3, 1]);
        assert_eq!(ids(&providers), vec![3, 1, 2, 4]);
    }

    #[test]
    fn apply_session_preference_promotes_bound_provider_when_present() {
        let mut providers = vec![provider(11), provider(22), provider(33)];
        let selected = apply_session_provider_preference(&mut providers, 22, Some(&[11, 22, 33]));
        assert_eq!(selected, Some(22));
        assert_eq!(ids(&providers), vec![22, 11, 33]);
    }

    #[test]
    fn apply_session_preference_rotates_to_next_when_bound_missing() {
        let mut providers = vec![provider(10), provider(20), provider(30)];
        let selected = apply_session_provider_preference(&mut providers, 99, Some(&[99, 30, 20]));
        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![30, 10, 20]);
    }

    #[test]
    fn apply_session_preference_is_noop_without_bound_order() {
        let mut providers = vec![provider(10), provider(20), provider(30)];
        let selected = apply_session_provider_preference(&mut providers, 99, None);
        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![10, 20, 30]);
    }
}
