use super::{
    build_claude_probe_response_body, compute_observe_request, is_claude_count_tokens_request,
    is_internal_forwarded_request, should_observe_request, should_seed_in_progress_request_log,
};
use axum::http::HeaderMap;
use serde_json::json;

#[test]
fn count_tokens_request_is_detected_only_for_claude_and_exact_path() {
    assert!(is_claude_count_tokens_request(
        "claude",
        "/v1/messages/count_tokens"
    ));
    assert!(!is_claude_count_tokens_request("claude", "/v1/messages"));
    assert!(!is_claude_count_tokens_request(
        "claude",
        "/v1/messages/count_tokens/"
    ));
    assert!(!is_claude_count_tokens_request(
        "codex",
        "/v1/messages/count_tokens"
    ));
}

#[test]
fn claude_observation_matches_vendor_default_log_contract() {
    assert!(should_observe_request("claude", "/v1/messages"));
    assert!(!should_observe_request(
        "claude",
        "/v1/messages/count_tokens"
    ));
    assert!(!should_observe_request("claude", "/v1/other"));
    assert!(should_observe_request("codex", "/v1/responses"));
}

#[test]
fn codex_model_discovery_requests_are_not_observed() {
    assert!(!should_observe_request("codex", "/v1/models"));
    assert!(!should_observe_request("codex", "/v1/models/"));
    assert!(!should_observe_request("codex", "/models"));
    assert!(should_observe_request("codex", "/v1/responses"));
}

#[test]
fn claude_probe_requests_are_not_observed() {
    let headers = HeaderMap::new();
    let probe = json!({
        "messages": [
            {
                "role": "user",
                "content": " count "
            }
        ]
    });

    assert!(!compute_observe_request(
        "claude",
        "/v1/messages",
        &headers,
        Some(&probe)
    ));
}

#[test]
fn internally_forwarded_claude_requests_are_not_observed() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-aio-gateway-forwarded",
        "aio-coding-hub".parse().expect("valid header"),
    );

    assert!(is_internal_forwarded_request(&headers));
    assert!(!compute_observe_request(
        "claude",
        "/v1/messages",
        &headers,
        None
    ));
}

#[test]
fn internally_forwarded_codex_requests_are_not_observed() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-aio-gateway-forwarded",
        "aio-coding-hub".parse().expect("valid header"),
    );

    assert!(is_internal_forwarded_request(&headers));
    assert!(!compute_observe_request(
        "codex",
        "/v1/responses",
        &headers,
        None
    ));
}

#[test]
fn normal_claude_message_requests_remain_observed() {
    let headers = HeaderMap::new();
    let body = json!({
        "messages": [
            {
                "role": "user",
                "content": "hello"
            }
        ]
    });

    assert!(compute_observe_request(
        "claude",
        "/v1/messages",
        &headers,
        Some(&body)
    ));
}

#[test]
fn only_observed_claude_message_requests_seed_in_progress_request_logs() {
    assert!(should_seed_in_progress_request_log(
        "claude",
        "/v1/messages",
        true
    ));
    assert!(!should_seed_in_progress_request_log(
        "claude",
        "/v1/messages",
        false
    ));
    assert!(!should_seed_in_progress_request_log(
        "claude",
        "/v1/messages/count_tokens",
        true
    ));
    assert!(!should_seed_in_progress_request_log(
        "codex",
        "/v1/responses",
        true
    ));
}

#[test]
fn internal_forward_marker_requires_expected_value() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-aio-gateway-forwarded",
        "other-proxy".parse().expect("valid header"),
    );

    assert!(!is_internal_forwarded_request(&headers));
}

#[test]
fn claude_probe_response_matches_vendor_shape() {
    let body = build_claude_probe_response_body();
    assert_eq!(
        body.get("input_tokens").and_then(|value| value.as_i64()),
        Some(0)
    );
}
