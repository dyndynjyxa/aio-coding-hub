use super::loop_helpers::should_finalize_as_all_providers_unavailable;
use crate::gateway::events::{decision_chain as dc, FailoverAttempt};
use crate::gateway::proxy::GatewayErrorCode;

fn skipped_attempt(reason_code: Option<&'static str>) -> FailoverAttempt {
    FailoverAttempt {
        provider_id: 1,
        provider_name: "provider".to_string(),
        base_url: "https://example.com".to_string(),
        outcome: "skipped".to_string(),
        status: None,
        provider_index: None,
        retry_index: None,
        session_reuse: None,
        error_category: Some("circuit_breaker"),
        error_code: Some(GatewayErrorCode::ProviderCircuitOpen.as_str()),
        decision: Some("skip"),
        reason: Some("provider skipped by circuit breaker (cooldown)".to_string()),
        selection_method: Some(dc::SELECTION_METHOD_FILTERED),
        reason_code,
        attempt_started_ms: Some(1),
        attempt_duration_ms: Some(0),
        circuit_state_before: None,
        circuit_state_after: None,
        circuit_failure_count: None,
        circuit_failure_threshold: None,
    }
}

fn real_attempt() -> FailoverAttempt {
    FailoverAttempt {
        provider_id: 1,
        provider_name: "provider".to_string(),
        base_url: "https://example.com".to_string(),
        outcome: "request_error".to_string(),
        status: Some(502),
        provider_index: Some(1),
        retry_index: Some(1),
        session_reuse: Some(false),
        error_category: Some("SYSTEM_ERROR"),
        error_code: Some(GatewayErrorCode::UpstreamConnectFailed.as_str()),
        decision: Some("switch"),
        reason: Some("reqwest connect error".to_string()),
        selection_method: Some("ordered"),
        reason_code: Some(dc::REASON_SYSTEM_ERROR),
        attempt_started_ms: Some(1),
        attempt_duration_ms: Some(10),
        circuit_state_before: Some("CLOSED"),
        circuit_state_after: None,
        circuit_failure_count: Some(0),
        circuit_failure_threshold: Some(5),
    }
}

#[test]
fn skip_only_gate_attempts_finalize_as_unavailable() {
    let attempts = vec![
        skipped_attempt(Some(dc::REASON_CIRCUIT_COOLDOWN)),
        skipped_attempt(Some(dc::REASON_RATE_LIMITED)),
    ];

    assert!(should_finalize_as_all_providers_unavailable(&attempts));
}

#[test]
fn empty_attempts_still_finalize_as_unavailable() {
    assert!(should_finalize_as_all_providers_unavailable(&[]));
}

#[test]
fn real_attempts_do_not_finalize_as_unavailable() {
    let attempts = vec![
        skipped_attempt(Some(dc::REASON_CIRCUIT_OPEN)),
        real_attempt(),
    ];

    assert!(!should_finalize_as_all_providers_unavailable(&attempts));
}

#[test]
fn non_gate_skip_attempts_do_not_finalize_as_unavailable() {
    let attempts = vec![skipped_attempt(None)];

    assert!(!should_finalize_as_all_providers_unavailable(&attempts));
}
