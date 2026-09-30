use super::*;
use serde_json::json;

#[test]
fn detect_trigger_budget_tokens_too_low() {
    let msg = "thinking.enabled.budget_tokens: Input should be greater than or equal to 1024";
    assert_eq!(detect_trigger(msg), Some(TRIGGER_BUDGET_TOKENS_TOO_LOW));

    let msg2 = "budget_tokens must be >= 1024 when thinking is enabled";
    assert_eq!(detect_trigger(msg2), Some(TRIGGER_BUDGET_TOKENS_TOO_LOW));
}

#[test]
fn detect_trigger_unrelated_error() {
    assert_eq!(detect_trigger("invalid signature in thinking block"), None);
    assert_eq!(detect_trigger(""), None);
}

#[test]
fn rectify_raises_only_to_minimum_budget_and_output_limit() {
    for max_tokens in [0, 10, 1024] {
        let mut message = json!({
            "model": "claude-test",
            "messages": [ { "role": "user", "content": [ { "type": "text", "text": "hi" } ] } ],
            "max_tokens": max_tokens,
            "thinking": { "type": "enabled", "budget_tokens": 512 }
        });

        let result = rectify_anthropic_request_message(&mut message);
        assert!(result.applied);
        assert_eq!(result.before.thinking_budget_tokens, Some(512));
        assert_eq!(result.after.thinking_budget_tokens, Some(1024));
        assert_eq!(message["thinking"]["type"].as_str(), Some("enabled"));
        assert_eq!(message["thinking"]["budget_tokens"].as_u64(), Some(1024));
        assert_eq!(message["max_tokens"].as_u64(), Some(1025));
    }
}

#[test]
fn rectify_skips_adaptive_thinking() {
    let mut message = json!({
        "model": "claude-test",
        "messages": [ { "role": "user", "content": [ { "type": "text", "text": "hi" } ] } ],
        "thinking": { "type": "adaptive", "budget_tokens": 512 }
    });

    let result = rectify_anthropic_request_message(&mut message);
    assert!(!result.applied);
    assert_eq!(message["thinking"]["type"].as_str(), Some("adaptive"));
    assert_eq!(message["thinking"]["budget_tokens"].as_u64(), Some(512));
}

#[test]
fn rectify_preserves_sufficient_output_limits_and_other_fields() {
    for max_tokens in [1025, 4096, 64000] {
        for budget in [json!(512), json!(-1), json!(1023.5)] {
            let mut message = json!({"max_tokens":max_tokens,"thinking":{"type":"enabled","budget_tokens":budget,"display":"summarized"},"metadata":{"keep":true}});
            let mut expected = message.clone();
            expected["thinking"]["budget_tokens"] = json!(1024);
            assert!(rectify_anthropic_request_message(&mut message).applied);
            assert_eq!(message, expected);
            assert!(!rectify_anthropic_request_message(&mut message).applied);
        }
    }
}

#[test]
fn rectify_does_not_enable_thinking_or_invent_missing_parameters() {
    for mut message in [
        json!(null),
        json!([]),
        json!({"max_tokens":4096}),
        json!({"max_tokens":4096,"thinking":{"type":"disabled","budget_tokens":512}}),
        json!({"max_tokens":4096,"thinking":{"type":"enabled","budget_tokens":1024}}),
        json!({"max_tokens":4096,"thinking":{"type":"enabled","budget_tokens":2048}}),
        json!({"max_tokens":4096,"thinking":{"type":"enabled"}}),
        json!({"max_tokens":4096,"thinking":{"type":"enabled","budget_tokens":"512"}}),
        json!({"thinking":{"type":"enabled","budget_tokens":512}}),
    ] {
        let original = message.clone();
        assert!(!rectify_anthropic_request_message(&mut message).applied);
        assert_eq!(message, original);
    }
}
