pub(super) type ThinkingEffortConflictRectifierTrigger = &'static str;

pub(super) const TRIGGER_THINKING_DISABLED_WITH_REASONING_EFFORT:
    ThinkingEffortConflictRectifierTrigger = "thinking_disabled_with_reasoning_effort";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ThinkingEffortConflictRectifierResult {
    pub(super) applied: bool,
    pub(super) removed_output_config_effort: bool,
    pub(super) removed_reasoning_effort: bool,
    pub(super) thinking_type: Option<String>,
    pub(super) effort: Option<String>,
}

pub(super) fn detect_trigger(
    error_message: &str,
) -> Option<ThinkingEffortConflictRectifierTrigger> {
    if error_message.trim().is_empty() {
        return None;
    }

    let lower = error_message.to_ascii_lowercase();
    let mentions_disable_conflict =
        lower.contains("cannot be disabled") || lower.contains("can not be disabled");
    if !mentions_disable_conflict {
        return None;
    }

    if lower.contains("reasoning_effort")
        || (lower.contains("output_config") && lower.contains("thinking"))
    {
        return Some(TRIGGER_THINKING_DISABLED_WITH_REASONING_EFFORT);
    }

    None
}

pub(super) fn rectify(message: &mut serde_json::Value) -> ThinkingEffortConflictRectifierResult {
    let thinking_type = message
        .get("thinking")
        .and_then(serde_json::Value::as_object)
        .and_then(|thinking| thinking.get("type"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let mut result = ThinkingEffortConflictRectifierResult {
        applied: false,
        removed_output_config_effort: false,
        removed_reasoning_effort: false,
        thinking_type: thinking_type.clone(),
        effort: None,
    };

    if thinking_type
        .as_deref()
        .is_some_and(|kind| kind != "disabled")
    {
        return result;
    }

    let Some(message) = message.as_object_mut() else {
        return result;
    };

    let mut remove_empty_output_config = false;
    if let Some(output_config) = message
        .get_mut("output_config")
        .and_then(serde_json::Value::as_object_mut)
    {
        if let Some(effort) = output_config.remove("effort") {
            result.effort = effort.as_str().map(str::to_string);
            result.removed_output_config_effort = true;
            result.applied = true;
            remove_empty_output_config = output_config.is_empty();
        }
    }
    if remove_empty_output_config {
        message.remove("output_config");
    }

    if let Some(reasoning_effort) = message.remove("reasoning_effort") {
        if result.effort.is_none() {
            result.effort = reasoning_effort.as_str().map(str::to_string);
        }
        result.removed_reasoning_effort = true;
        result.applied = true;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_only_disabled_thinking_effort_conflicts() {
        for message in [
            "thinking options type cannot be disabled when reasoning_effort is set",
            "Thinking can not be disabled when output_config.effort is set",
            r#"{\"error\":{\"message\":\"thinking cannot be disabled when reasoning_effort is set\"}}"#,
        ] {
            assert_eq!(
                detect_trigger(message),
                Some(TRIGGER_THINKING_DISABLED_WITH_REASONING_EFFORT)
            );
        }

        for message in [
            "reasoning_effort must be one of low or high",
            "invalid request: malformed",
            "thinking cannot be disabled",
            "output_config is invalid",
        ] {
            assert_eq!(detect_trigger(message), None);
        }
    }

    #[test]
    fn removes_effort_fields_and_preserves_siblings() {
        let mut body = json!({
            "thinking": {"type": "disabled"},
            "output_config": {"effort": "max", "verbosity": "high", "future": true},
            "reasoning_effort": "high",
            "messages": [],
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert!(result.removed_output_config_effort);
        assert!(result.removed_reasoning_effort);
        assert_eq!(result.thinking_type.as_deref(), Some("disabled"));
        assert_eq!(result.effort.as_deref(), Some("max"));
        assert_eq!(body["thinking"]["type"], "disabled");
        assert_eq!(
            body["output_config"],
            json!({"verbosity":"high","future":true})
        );
        assert!(body.get("reasoning_effort").is_none());
        assert_eq!(body["messages"], json!([]));
    }

    #[test]
    fn removes_empty_output_config_when_effort_was_only_field() {
        let mut body = json!({"output_config":{"effort":"low"}});

        let result = rectify(&mut body);

        assert!(result.applied);
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn keeps_explicitly_enabled_or_adaptive_thinking_unchanged() {
        for kind in ["enabled", "adaptive"] {
            let mut body = json!({
                "thinking":{"type":kind},
                "output_config":{"effort":"max"},
                "reasoning_effort":"high"
            });
            let before = body.clone();

            let result = rectify(&mut body);

            assert!(!result.applied);
            assert_eq!(body, before);
        }
    }

    #[test]
    fn leaves_effortless_payload_unchanged() {
        let mut body = json!({
            "thinking":{"type":"disabled"},
            "output_config":{"verbosity":"high"}
        });
        let before = body.clone();

        let result = rectify(&mut body);

        assert!(!result.applied);
        assert_eq!(body, before);
    }
}
