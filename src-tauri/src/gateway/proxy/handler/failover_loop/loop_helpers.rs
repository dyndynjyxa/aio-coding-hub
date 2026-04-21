//! Usage: Helper types and functions for the failover loop orchestrator.
//!
//! Contains `FinalizeOwnedCommon`, skip-attempt helpers, and the
//! "all providers unavailable" finalization predicate.

use super::*;

pub(super) struct FinalizeOwnedCommon {
    pub(super) cli_key: String,
    pub(super) method_hint: String,
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
}

pub(super) fn finalize_owned_from_input(input: &RequestContext) -> FinalizeOwnedCommon {
    FinalizeOwnedCommon {
        cli_key: input.cli_key.clone(),
        method_hint: input.method_hint.clone(),
        forwarded_path: input.forwarded_path.clone(),
        query: input.query.clone(),
        trace_id: input.trace_id.clone(),
        session_id: input.session_id.clone(),
        requested_model: input.requested_model.clone(),
        special_settings: input.special_settings.clone(),
    }
}

pub(super) struct SkippedProviderAttempt<'a> {
    pub(super) provider_id: i64,
    pub(super) provider_name: &'a str,
    pub(super) base_url: &'a str,
    pub(super) error_category: &'static str,
    pub(super) error_code: &'static str,
    pub(super) reason: String,
    pub(super) reason_code: Option<&'static str>,
    pub(super) attempt_started_ms: u128,
}

pub(super) fn push_skipped_provider_attempt(
    attempts: &mut Vec<FailoverAttempt>,
    skipped: SkippedProviderAttempt<'_>,
) {
    attempts.push(FailoverAttempt {
        provider_id: skipped.provider_id,
        provider_name: skipped.provider_name.to_string(),
        base_url: skipped.base_url.to_string(),
        outcome: "skipped".to_string(),
        status: None,
        provider_index: None,
        retry_index: None,
        session_reuse: None,
        error_category: Some(skipped.error_category),
        error_code: Some(skipped.error_code),
        decision: Some("skip"),
        reason: Some(skipped.reason),
        selection_method: Some(dc::SELECTION_METHOD_FILTERED),
        reason_code: skipped.reason_code,
        attempt_started_ms: Some(skipped.attempt_started_ms),
        attempt_duration_ms: Some(0),
        circuit_state_before: None,
        circuit_state_after: None,
        circuit_failure_count: None,
        circuit_failure_threshold: None,
    });
}

pub(super) fn is_gate_only_skipped_attempt(attempt: &FailoverAttempt) -> bool {
    if attempt.decision != Some("skip") {
        return false;
    }

    if attempt.provider_index.is_some() || attempt.retry_index.is_some() {
        return false;
    }

    matches!(
        attempt.reason_code,
        Some(dc::REASON_CIRCUIT_OPEN | dc::REASON_CIRCUIT_COOLDOWN | dc::REASON_RATE_LIMITED)
    )
}

pub(super) fn should_finalize_as_all_providers_unavailable(attempts: &[FailoverAttempt]) -> bool {
    attempts.is_empty() || attempts.iter().all(is_gate_only_skipped_attempt)
}

pub(super) fn apply_cx2cc_request_settings(
    responses_body: &mut serde_json::Value,
    cx2cc_settings: &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
) {
    if let Some(ref effort) = cx2cc_settings.model_reasoning_effort {
        responses_body["reasoning"] = serde_json::json!({ "effort": effort });
    }
    if let Some(ref tier) = cx2cc_settings.service_tier {
        responses_body["service_tier"] = serde_json::json!(tier);
    }
    if cx2cc_settings.disable_response_storage {
        responses_body["store"] = serde_json::json!(false);
    }
}
