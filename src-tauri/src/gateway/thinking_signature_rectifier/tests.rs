use super::*;
use serde_json::json;

#[test]
fn detect_trigger_invalid_signature_in_thinking_block() {
    let trigger = detect_trigger("messages.1.content.0: Invalid `signature` in `thinking` block");
    assert_eq!(trigger, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));

    let trigger2 = detect_trigger("Messages.1.Content.0: invalid signature in thinking block");
    assert_eq!(trigger2, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));
}

#[test]
fn detect_trigger_signature_field_required_variants() {
    let trigger = detect_trigger("messages.0.content.1.signature: Field required");
    assert_eq!(trigger, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));

    let trigger2 = detect_trigger("messages.0.content.1.signature: field required");
    assert_eq!(trigger2, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));
}

#[test]
fn detect_trigger_signature_extra_inputs_variants() {
    let trigger = detect_trigger("messages.0.content.1.signature: Extra inputs are not permitted");
    assert_eq!(trigger, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));
}

#[test]
fn detect_trigger_thinking_block_cannot_be_modified_variants() {
    let trigger = detect_trigger("thinking or redacted_thinking blocks cannot be modified");
    assert_eq!(trigger, Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK));
}

#[test]
fn detect_trigger_missing_thinking_prefix() {
    let trigger = detect_trigger(
        "messages.69.content.0.type: Expected `thinking` or `redacted_thinking`, but found `tool_use`. When `thinking` is enabled, a final `assistant` message must start with a thinking block (preceeding the lastmost set of `tool_use` and `tool_result` blocks). To avoid this requirement, disable `thinking`.",
    );
    assert_eq!(
        trigger,
        Some(TRIGGER_ASSISTANT_MESSAGE_MUST_START_WITH_THINKING)
    );
}

#[test]
fn detect_trigger_invalid_request_with_thinking_context() {
    assert_eq!(
        detect_trigger("非法请求: thinking block signature mismatch"),
        Some(TRIGGER_INVALID_REQUEST)
    );
    assert_eq!(
        detect_trigger("illegal request: invalid thinking parameter"),
        Some(TRIGGER_INVALID_REQUEST)
    );
    assert_eq!(
        detect_trigger("invalid request: signature verification failed"),
        Some(TRIGGER_INVALID_REQUEST)
    );
    assert_eq!(
        detect_trigger("invalid request: redacted block error"),
        Some(TRIGGER_INVALID_REQUEST)
    );
}

#[test]
fn detect_trigger_generic_invalid_request_matches_cch_fallback() {
    assert_eq!(detect_trigger("非法请求"), Some(TRIGGER_INVALID_REQUEST));
    assert_eq!(
        detect_trigger("illegal request format"),
        Some(TRIGGER_INVALID_REQUEST)
    );
    assert_eq!(
        detect_trigger("invalid request: malformed JSON"),
        Some(TRIGGER_INVALID_REQUEST)
    );
}

#[test]
fn detect_trigger_unrelated_error() {
    assert_eq!(detect_trigger("Request timeout"), None);
}

#[test]
fn rectify_removes_thinking_blocks_and_signature_fields() {
    let mut message = json!({
        "model": "claude-test",
        "messages": [
            {
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "t", "signature": "sig_thinking" },
                    { "type": "text", "text": "hello", "signature": "sig_text_should_remove" },
                    { "type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": { "query": "q" }, "signature": "sig_tool_should_remove" },
                    { "type": "redacted_thinking", "data": "r", "signature": "sig_redacted" }
                ]
            },
            {
                "role": "user",
                "content": [ { "type": "text", "text": "hi" } ]
            }
        ]
    });

    let result = rectify_anthropic_request_message(&mut message);
    assert!(result.applied);
    assert_eq!(result.removed_thinking_blocks, 1);
    assert_eq!(result.removed_redacted_thinking_blocks, 1);
    assert_eq!(result.removed_signature_fields, 2);

    let content = message["messages"][0]["content"]
        .as_array()
        .expect("content should be array");
    let types: Vec<_> = content
        .iter()
        .map(|v| v["type"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(types, vec!["text", "tool_use"]);
    assert!(content[0].get("signature").is_none());
    assert!(content[1].get("signature").is_none());
}

#[test]
fn rectify_no_messages_should_not_modify() {
    let mut message = json!({ "model": "claude-test" });
    let result = rectify_anthropic_request_message(&mut message);
    assert!(!result.applied);
    assert_eq!(result.removed_thinking_blocks, 0);
    assert_eq!(result.removed_redacted_thinking_blocks, 0);
    assert_eq!(result.removed_signature_fields, 0);
}

#[test]
fn rectify_removes_top_level_thinking_when_tool_use_without_thinking_prefix() {
    let mut message = json!({
        "model": "claude-test",
        "thinking": { "type": "enabled", "budget_tokens": 1024 },
        "messages": [
            {
                "role": "assistant",
                "content": [
                    { "type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": { "query": "q" } }
                ]
            },
            {
                "role": "user",
                "content": [ { "type": "tool_result", "tool_use_id": "toolu_1", "content": "ok" } ]
            }
        ]
    });

    let result = rectify_anthropic_request_message(&mut message);
    assert!(result.applied);
    assert!(result.removed_top_level_thinking);
    assert!(message.get("thinking").is_none());
}
