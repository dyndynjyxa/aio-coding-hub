use super::provider_order;
use crate::gateway::proxy::failover::should_reuse_provider;
use crate::gateway::runtime::GatewayAppState;
use crate::providers;
use crate::{circuit_breaker, session_manager};
use std::collections::HashSet;

pub(super) struct ProviderSelection {
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) providers: Vec<providers::ProviderForGateway>,
    pub(super) bound_provider_order: Option<Vec<i64>>,
    pub(super) active_sort_mode_id: Option<i64>,
    pub(super) session_bound_sort_mode_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelPolicyFilterResult {
    pub(super) original_provider_ids: Vec<i64>,
    pub(super) ineligible_provider_ids: Vec<i64>,
    pub(super) invalid_provider_ids: Vec<i64>,
}

pub(super) fn filter_providers_by_model_policy(
    providers: &mut Vec<providers::ProviderForGateway>,
    requested_model: Option<&str>,
    forced_provider_id: Option<i64>,
) -> ModelPolicyFilterResult {
    let original_provider_ids = providers.iter().map(|provider| provider.id).collect();
    let Some(requested_model) = requested_model.filter(|model| !model.is_empty()) else {
        return ModelPolicyFilterResult {
            original_provider_ids,
            ineligible_provider_ids: Vec::new(),
            invalid_provider_ids: Vec::new(),
        };
    };

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum CandidateClass {
        Blocked,
        Explicit,
        Fallback,
        Invalid,
    }

    let classified = providers
        .iter()
        .map(|provider| {
            let class = match provider.model_policy_status {
                providers::ProviderModelPolicyStatus::Legacy => CandidateClass::Fallback,
                providers::ProviderModelPolicyStatus::Ready => match provider.model_policy.as_ref()
                {
                    Some(policy) => match policy.eligibility(requested_model) {
                        providers::ProviderModelEligibility::Blocked => CandidateClass::Blocked,
                        providers::ProviderModelEligibility::Explicit => CandidateClass::Explicit,
                        providers::ProviderModelEligibility::Fallback => CandidateClass::Fallback,
                    },
                    None => CandidateClass::Invalid,
                },
                providers::ProviderModelPolicyStatus::Invalid => CandidateClass::Invalid,
            };
            (provider.id, class)
        })
        .collect::<Vec<_>>();
    // Explicit-first narrowing is a routing preference, not an eligibility rule:
    // a forced provider (x-aio-provider-id) must only be rejected when its own
    // policy blocks the model, not because a sibling declared it explicitly.
    let use_explicit = forced_provider_id.is_none()
        && classified
            .iter()
            .any(|(_, class)| *class == CandidateClass::Explicit);
    let is_eligible = |class: CandidateClass| {
        class == CandidateClass::Explicit || (!use_explicit && class == CandidateClass::Fallback)
    };

    let invalid_provider_ids = classified
        .iter()
        .filter_map(|(id, class)| (*class == CandidateClass::Invalid).then_some(*id))
        .collect();
    let ineligible_provider_ids = classified
        .iter()
        .filter_map(|(id, class)| {
            (*class != CandidateClass::Invalid && !is_eligible(*class)).then_some(*id)
        })
        .collect();
    let eligible_ids = classified
        .iter()
        .filter_map(|(id, class)| is_eligible(*class).then_some(*id))
        .collect::<HashSet<_>>();
    providers.retain(|provider| eligible_ids.contains(&provider.id));

    ModelPolicyFilterResult {
        original_provider_ids,
        ineligible_provider_ids,
        invalid_provider_ids,
    }
}

pub(super) fn select_providers_with_session_binding<R: tauri::Runtime>(
    state: &GatewayAppState<R>,
    cli_key: &str,
    session_id: Option<&str>,
    created_at: i64,
) -> crate::shared::error::AppResult<ProviderSelection> {
    let bound_sort_mode_id = session_id.and_then(|sid| {
        state
            .session
            .get_bound_sort_mode_id(cli_key, sid, created_at)
    });

    let (active_sort_mode_id, effective_sort_mode_id, mut providers) = match bound_sort_mode_id {
        Some(sort_mode_id) => {
            let active_sort_mode_id =
                providers::active_sort_mode_id_for_gateway(&state.db, cli_key)?;
            let providers =
                providers::list_enabled_for_gateway_in_mode(&state.db, cli_key, sort_mode_id)?;
            (active_sort_mode_id, sort_mode_id, providers)
        }
        None => {
            let selection =
                providers::list_enabled_for_gateway_using_active_mode(&state.db, cli_key)?;
            (
                selection.sort_mode_id,
                selection.sort_mode_id,
                selection.providers,
            )
        }
    };

    let mut bound_provider_order: Option<Vec<i64>> = None;
    if let Some(sid) = session_id {
        let provider_order: Vec<i64> = providers.iter().map(|p| p.id).collect();
        state.session.bind_sort_mode(
            cli_key,
            sid,
            effective_sort_mode_id,
            Some(provider_order),
            created_at,
        );

        bound_provider_order = state
            .session
            .get_bound_provider_order(cli_key, sid, created_at);

        if let Some(order) = bound_provider_order.as_deref() {
            provider_order::reorder_providers_by_bound_order(&mut providers, order);
        }
    }

    Ok(ProviderSelection {
        effective_sort_mode_id,
        providers,
        bound_provider_order,
        active_sort_mode_id,
        session_bound_sort_mode_id: bound_sort_mode_id,
    })
}

pub(super) fn resolve_session_routing_decision(
    headers: &axum::http::HeaderMap,
    introspection_json: Option<&serde_json::Value>,
    is_claude_count_tokens: bool,
) -> SessionRoutingDecision {
    let extracted_session_id =
        session_manager::SessionManager::extract_session_id_from_json(headers, introspection_json);

    let session_id = if is_claude_count_tokens {
        None
    } else {
        extracted_session_id
    };

    let allow_session_reuse = if is_claude_count_tokens {
        false
    } else {
        should_reuse_provider(introspection_json)
    };

    SessionRoutingDecision {
        session_id,
        allow_session_reuse,
    }
}

pub(super) fn apply_session_reuse_provider_binding(
    allow_session_reuse: bool,
    providers: &mut [providers::ProviderForGateway],
    bound_provider_id: Option<i64>,
    bound_provider_order: Option<&[i64]>,
) -> Option<i64> {
    if !allow_session_reuse {
        return None;
    }
    let bound_provider_id = bound_provider_id?;

    provider_order::apply_session_provider_preference(
        providers,
        bound_provider_id,
        bound_provider_order,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_session_bound_provider_id(
    session: &session_manager::SessionManager,
    circuit: &circuit_breaker::CircuitBreaker,
    cli_key: &str,
    session_id: Option<&str>,
    created_at: i64,
    allow_session_reuse: bool,
    forced_provider_id: Option<i64>,
    providers: &mut Vec<providers::ProviderForGateway>,
    bound_provider_order: Option<&[i64]>,
) -> SessionBoundResult {
    let bound_provider_id =
        session_id.and_then(|sid| session.get_bound_provider(cli_key, sid, created_at));

    if allow_session_reuse && forced_provider_id.is_none() {
        if let (Some(session_id), Some(bound_provider_id)) = (session_id, bound_provider_id) {
            if !providers.iter().any(|p| p.id == bound_provider_id) {
                // The bound provider is no longer eligible for the current session's provider list
                // (e.g. sort_mode/provider membership changed). Clear the stale binding so it
                // cannot bypass selection constraints.
                session.clear_bound_provider(cli_key, session_id, created_at);
            } else {
                let check = circuit.should_allow(bound_provider_id, created_at);
                if !check.allow {
                    providers.retain(|provider| provider.id != bound_provider_id);
                    return SessionBoundResult::DeniedByCircuit {
                        provider_id: bound_provider_id,
                        snapshot: check.after,
                    };
                }
            }
        }
    }

    match apply_session_reuse_provider_binding(
        allow_session_reuse,
        providers,
        bound_provider_id,
        bound_provider_order,
    ) {
        Some(id) => SessionBoundResult::Preferred(id),
        None => SessionBoundResult::NoPreference,
    }
}

/// Outcome of resolving session-bound provider preference.
///
/// This makes the reason a bound provider was (or was not) applied explicit,
/// which is important for observability (especially single-provider + circuit open cases).
#[derive(Debug, Clone)]
pub(super) enum SessionBoundResult {
    /// A provider id was selected/preferred for session reuse (the list may have been rotated).
    Preferred(i64),
    /// No session preference was applied for this request.
    NoPreference,
    /// The session had a bound provider that was still in the candidate list,
    /// but it was removed because the circuit breaker denied it (open or active cooldown).
    /// The provider has already been filtered out of `providers`.
    DeniedByCircuit {
        provider_id: i64,
        snapshot: crate::circuit_breaker::CircuitSnapshot,
    },
}

pub(super) struct SessionRoutingDecision {
    pub(super) session_id: Option<String>,
    pub(super) allow_session_reuse: bool,
}

#[cfg(test)]
mod tests;
