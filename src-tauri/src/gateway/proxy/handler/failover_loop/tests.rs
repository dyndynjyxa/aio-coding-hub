use super::context::{AttemptOutcome, FailoverRunState};
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

fn timeout_attempt(
    provider_id: i64,
    provider_index: u32,
    session_reuse: Option<bool>,
) -> FailoverAttempt {
    FailoverAttempt {
        provider_id,
        provider_name: format!("provider-{provider_id}"),
        base_url: "https://example.com".to_string(),
        outcome: "request_timeout: category=SYSTEM_ERROR code=GW_UPSTREAM_TIMEOUT decision=switch timeout_secs=30".to_string(),
        status: None,
        provider_index: Some(provider_index),
        retry_index: Some(1),
        session_reuse,
        error_category: Some("SYSTEM_ERROR"),
        error_code: Some(GatewayErrorCode::UpstreamTimeout.as_str()),
        decision: Some("switch"),
        reason: Some("request timeout".to_string()),
        selection_method: dc::selection_method(provider_index, 1, session_reuse),
        reason_code: Some(dc::REASON_SYSTEM_ERROR),
        attempt_started_ms: Some(1),
        attempt_duration_ms: Some(30_000),
        circuit_state_before: Some("CLOSED"),
        circuit_state_after: Some("OPEN"),
        circuit_failure_count: Some(5),
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
fn timeout_storm_attempts_finalize_as_failed_not_unavailable() {
    let attempts = vec![
        timeout_attempt(10, 1, Some(true)),
        timeout_attempt(20, 2, None),
    ];

    assert!(!should_finalize_as_all_providers_unavailable(&attempts));
    assert!(attempts
        .iter()
        .all(|attempt| attempt.retry_index == Some(1)));
    assert!(attempts
        .iter()
        .all(|attempt| attempt.error_code == Some(GatewayErrorCode::UpstreamTimeout.as_str())));
    assert_eq!(attempts[0].session_reuse, Some(true));
}

#[test]
fn non_gate_skip_attempts_do_not_finalize_as_unavailable() {
    let attempts = vec![skipped_attempt(None)];

    assert!(!should_finalize_as_all_providers_unavailable(&attempts));
}

#[test]
fn failover_run_state_owns_attempts_failed_ids_and_last_outcome() {
    let mut state = FailoverRunState::new();
    state.attempts.push(real_attempt());
    state.failed_provider_ids.insert(42);
    state.last_outcome = Some(AttemptOutcome::new(
        "provider_error",
        GatewayErrorCode::Upstream5xx.as_str(),
    ));

    let outcome = state.last_outcome.expect("last outcome");

    assert_eq!(state.attempts.len(), 1);
    assert!(state.failed_provider_ids.contains(&42));
    assert_eq!(outcome.error_category, "provider_error");
    assert_eq!(outcome.error_code, GatewayErrorCode::Upstream5xx.as_str());
}

#[test]
fn attempt_outcome_preserves_terminal_error_pair() {
    let outcome = AttemptOutcome::new(
        "system_error",
        GatewayErrorCode::UpstreamConnectFailed.as_str(),
    );

    assert_eq!(outcome.error_category, "system_error");
    assert_eq!(
        outcome.error_code,
        GatewayErrorCode::UpstreamConnectFailed.as_str()
    );
}

#[test]
fn stream_flag_from_raw_body_detects_compact_and_spaced_json_flags() {
    assert!(super::stream_flag_from_raw_body(br#"{"stream":true}"#));
    assert!(super::stream_flag_from_raw_body(
        br#"{"model":"claude","stream": true}"#
    ));
}

#[test]
fn stream_flag_from_raw_body_only_scans_first_two_kb() {
    let mut body = vec![b' '; 2048];
    body.extend_from_slice(br#"{"stream":true}"#);

    assert!(!super::stream_flag_from_raw_body(&body));
}

#[test]
fn stream_flag_from_raw_body_ignores_non_utf8_payloads() {
    assert!(!super::stream_flag_from_raw_body(&[0xff, 0xfe, b'{']));
}
