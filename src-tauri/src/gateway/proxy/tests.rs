use super::is_claude_count_tokens_request;

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
