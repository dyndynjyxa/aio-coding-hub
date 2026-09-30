pub(super) type ThinkingBudgetRectifierTrigger = &'static str;

pub(super) const TRIGGER_BUDGET_TOKENS_TOO_LOW: ThinkingBudgetRectifierTrigger =
    "budget_tokens_too_low";

const MIN_THINKING_BUDGET: u64 = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ThinkingBudgetRectifierSnapshot {
    pub(super) max_tokens: Option<u64>,
    pub(super) thinking_type: Option<String>,
    pub(super) thinking_budget_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub(super) struct ThinkingBudgetRectifierResult {
    pub(super) applied: bool,
    pub(super) before: ThinkingBudgetRectifierSnapshot,
    pub(super) after: ThinkingBudgetRectifierSnapshot,
}

pub(super) fn detect_trigger(error_message: &str) -> Option<ThinkingBudgetRectifierTrigger> {
    if error_message.trim().is_empty() {
        return None;
    }

    let lower = error_message.to_lowercase();
    let has_budget_tokens_ref = lower.contains("budget_tokens") || lower.contains("budget tokens");
    let has_thinking_ref = lower.contains("thinking");
    let has_1024_constraint = lower.contains("greater than or equal to 1024")
        || lower.contains(">= 1024")
        || (lower.contains("1024") && lower.contains("input should be"));

    if has_budget_tokens_ref && has_thinking_ref && has_1024_constraint {
        return Some(TRIGGER_BUDGET_TOKENS_TOO_LOW);
    }

    None
}

fn snapshot(message: &serde_json::Value) -> ThinkingBudgetRectifierSnapshot {
    let message_obj = message.as_object();
    let max_tokens = message_obj
        .and_then(|v| v.get("max_tokens"))
        .and_then(|v| v.as_u64());

    let thinking_obj = message_obj
        .and_then(|v| v.get("thinking"))
        .and_then(|v| v.as_object());

    let thinking_type = thinking_obj
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let thinking_budget_tokens = thinking_obj
        .and_then(|v| v.get("budget_tokens"))
        .and_then(|v| v.as_u64());

    ThinkingBudgetRectifierSnapshot {
        max_tokens,
        thinking_type,
        thinking_budget_tokens,
    }
}

pub(super) fn rectify_anthropic_request_message(
    message: &mut serde_json::Value,
) -> ThinkingBudgetRectifierResult {
    let before = snapshot(message);

    let budget_is_too_low = message
        .get("thinking")
        .and_then(|v| v.get("budget_tokens"))
        .and_then(|v| v.as_f64())
        .is_some_and(|budget| budget < MIN_THINKING_BUDGET as f64);
    if before.thinking_type.as_deref() != Some("enabled")
        || !budget_is_too_low
        || before.max_tokens.is_none()
    {
        return ThinkingBudgetRectifierResult {
            applied: false,
            before: before.clone(),
            after: before,
        };
    }

    message["thinking"]["budget_tokens"] = serde_json::json!(MIN_THINKING_BUDGET);
    if before
        .max_tokens
        .is_some_and(|max| max <= MIN_THINKING_BUDGET)
    {
        message["max_tokens"] = serde_json::json!(MIN_THINKING_BUDGET + 1);
    }

    let after = snapshot(message);
    let applied = before != after;

    ThinkingBudgetRectifierResult {
        applied,
        before,
        after,
    }
}

#[cfg(test)]
mod tests;
