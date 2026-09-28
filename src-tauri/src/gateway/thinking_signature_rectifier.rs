pub(super) type ThinkingSignatureRectifierTrigger = &'static str;

pub(super) const TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK: ThinkingSignatureRectifierTrigger =
    "invalid_signature_in_thinking_block";
pub(super) const TRIGGER_ASSISTANT_MESSAGE_MUST_START_WITH_THINKING:
    ThinkingSignatureRectifierTrigger = "assistant_message_must_start_with_thinking";
pub(super) const TRIGGER_INVALID_REQUEST: ThinkingSignatureRectifierTrigger = "invalid_request";

#[derive(Debug, Clone, Copy)]
pub(super) struct ThinkingSignatureRectifierResult {
    pub(super) applied: bool,
    pub(super) removed_thinking_blocks: usize,
    pub(super) removed_redacted_thinking_blocks: usize,
    pub(super) removed_signature_fields: usize,
    pub(super) removed_top_level_thinking: bool,
}

pub(super) fn detect_trigger(error_message: &str) -> Option<ThinkingSignatureRectifierTrigger> {
    if error_message.trim().is_empty() {
        return None;
    }

    let lower = error_message.to_lowercase();

    let looks_like_thinking_enabled_but_missing_thinking_prefix = lower
        .contains("must start with a thinking block")
        || (lower.contains("expected")
            && lower.contains("thinking")
            && (lower.contains("redacted_thinking") || lower.contains("redacted thinking"))
            && lower.contains("found")
            && (lower.contains("tool_use") || lower.contains("tool use")));

    if looks_like_thinking_enabled_but_missing_thinking_prefix {
        return Some(TRIGGER_ASSISTANT_MESSAGE_MUST_START_WITH_THINKING);
    }

    let looks_like_invalid_signature_in_thinking_block = lower.contains("invalid")
        && lower.contains("signature")
        && lower.contains("thinking")
        && lower.contains("block");
    if looks_like_invalid_signature_in_thinking_block {
        return Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK);
    }

    let looks_like_missing_signature_field =
        lower.contains("signature") && lower.contains("field required");
    if looks_like_missing_signature_field {
        return Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK);
    }

    let looks_like_extra_signature_field =
        lower.contains("signature") && lower.contains("extra inputs are not permitted");
    if looks_like_extra_signature_field {
        return Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK);
    }

    let looks_like_thinking_block_modified = (lower.contains("thinking")
        || lower.contains("redacted_thinking"))
        && lower.contains("cannot be modified");
    if looks_like_thinking_block_modified {
        return Some(TRIGGER_INVALID_SIGNATURE_IN_THINKING_BLOCK);
    }

    let looks_like_generic_invalid_request = error_message.contains("非法请求")
        || lower.contains("illegal request")
        || lower.contains("invalid request");
    if looks_like_generic_invalid_request {
        return Some(TRIGGER_INVALID_REQUEST);
    }

    None
}

pub(super) fn rectify_anthropic_request_message(
    message: &mut serde_json::Value,
) -> ThinkingSignatureRectifierResult {
    let mut removed_thinking_blocks = 0usize;
    let mut removed_redacted_thinking_blocks = 0usize;
    let mut removed_signature_fields = 0usize;
    let mut removed_top_level_thinking = false;
    let mut applied = false;

    let Some(message_obj) = message.as_object_mut() else {
        return ThinkingSignatureRectifierResult {
            applied: false,
            removed_thinking_blocks,
            removed_redacted_thinking_blocks,
            removed_signature_fields,
            removed_top_level_thinking,
        };
    };

    let thinking_enabled = message_obj
        .get("thinking")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("type"))
        .and_then(|v| v.as_str())
        == Some("enabled");

    let mut should_remove_top_level_thinking = false;

    {
        let Some(messages) = message_obj
            .get_mut("messages")
            .and_then(|v| v.as_array_mut())
        else {
            return ThinkingSignatureRectifierResult {
                applied: false,
                removed_thinking_blocks,
                removed_redacted_thinking_blocks,
                removed_signature_fields,
                removed_top_level_thinking,
            };
        };

        for msg in messages.iter_mut() {
            let Some(msg_obj) = msg.as_object_mut() else {
                continue;
            };

            let Some(content) = msg_obj.get_mut("content").and_then(|v| v.as_array_mut()) else {
                continue;
            };

            let original = std::mem::take(content);
            let mut new_content: Vec<serde_json::Value> = Vec::with_capacity(original.len());
            let mut content_modified = false;

            for mut block in original {
                let Some(block_obj) = block.as_object_mut() else {
                    new_content.push(block);
                    continue;
                };

                match block_obj.get("type").and_then(|v| v.as_str()) {
                    Some("thinking") => {
                        removed_thinking_blocks += 1;
                        content_modified = true;
                        continue;
                    }
                    Some("redacted_thinking") => {
                        removed_redacted_thinking_blocks += 1;
                        content_modified = true;
                        continue;
                    }
                    _ => {}
                }

                if block_obj.remove("signature").is_some() {
                    removed_signature_fields += 1;
                    content_modified = true;
                }

                new_content.push(block);
            }

            if content_modified {
                applied = true;
            }
            *content = new_content;
        }

        // Fallback: if top-level thinking is enabled, but the final assistant message doesn't start
        // with thinking/redacted_thinking AND contains tool_use, remove top-level thinking to avoid
        // Anthropic 400 "Expected thinking..., but found tool_use".
        if thinking_enabled {
            let last_assistant_content = messages.iter().rev().find_map(|msg| {
                let msg_obj = msg.as_object()?;
                if msg_obj.get("role").and_then(|v| v.as_str()) != Some("assistant") {
                    return None;
                }
                msg_obj.get("content").and_then(|v| v.as_array())
            });

            if let Some(content) = last_assistant_content {
                if let Some(first_block) = content.first() {
                    let first_block_type = first_block
                        .as_object()
                        .and_then(|obj| obj.get("type"))
                        .and_then(|v| v.as_str());

                    let missing_thinking_prefix = first_block_type != Some("thinking")
                        && first_block_type != Some("redacted_thinking");

                    if missing_thinking_prefix {
                        let has_tool_use = content.iter().any(|block| {
                            block
                                .as_object()
                                .and_then(|obj| obj.get("type"))
                                .and_then(|v| v.as_str())
                                == Some("tool_use")
                        });

                        if has_tool_use {
                            should_remove_top_level_thinking = true;
                        }
                    }
                }
            }
        }
    }

    if should_remove_top_level_thinking && message_obj.remove("thinking").is_some() {
        removed_top_level_thinking = true;
        applied = true;
    }

    ThinkingSignatureRectifierResult {
        applied,
        removed_thinking_blocks,
        removed_redacted_thinking_blocks,
        removed_signature_fields,
        removed_top_level_thinking,
    }
}

#[cfg(test)]
mod tests;
