#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReactiveRectifierKind {
    ThinkingEffortConflict,
    ThinkingSignature,
    ThinkingBudget,
    GeminiFunctionId,
}

impl ReactiveRectifierKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::ThinkingEffortConflict => "thinking_effort_conflict_rectifier",
            Self::ThinkingSignature => "thinking_signature_rectifier",
            Self::ThinkingBudget => "thinking_budget_rectifier",
            Self::GeminiFunctionId => "gemini_function_id_rectifier",
        }
    }

    fn detect(self, message: &str) -> Option<&'static str> {
        match self {
            Self::ThinkingEffortConflict => {
                super::thinking_effort_conflict_rectifier::detect_trigger(message)
            }
            Self::ThinkingSignature => super::thinking_signature_rectifier::detect_trigger(message),
            Self::ThinkingBudget => super::thinking_budget_rectifier::detect_trigger(message),
            Self::GeminiFunctionId => super::gemini_function_id_rectifier::detect_trigger(message),
        }
    }
}

const ANTHROPIC_REGISTRY: [ReactiveRectifierKind; 3] = [
    ReactiveRectifierKind::ThinkingEffortConflict,
    ReactiveRectifierKind::ThinkingSignature,
    ReactiveRectifierKind::ThinkingBudget,
];
const GEMINI_REGISTRY: [ReactiveRectifierKind; 1] = [ReactiveRectifierKind::GeminiFunctionId];

#[derive(Debug, Clone, Copy)]
pub(super) struct ReactiveRectifierSettings {
    pub(super) thinking_effort_conflict: bool,
    pub(super) thinking_signature: bool,
    pub(super) thinking_budget: bool,
    pub(super) gemini_function_id: bool,
}

impl ReactiveRectifierSettings {
    fn enabled(self, kind: ReactiveRectifierKind) -> bool {
        match kind {
            ReactiveRectifierKind::ThinkingEffortConflict => self.thinking_effort_conflict,
            ReactiveRectifierKind::ThinkingSignature => self.thinking_signature,
            ReactiveRectifierKind::ThinkingBudget => self.thinking_budget,
            ReactiveRectifierKind::GeminiFunctionId => self.gemini_function_id,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ReactiveRectifierMatch {
    pub(super) kind: ReactiveRectifierKind,
    pub(super) trigger: &'static str,
    pub(super) enabled: bool,
}

pub(super) fn detect(
    cli_key: &str,
    error_message: &str,
    settings: ReactiveRectifierSettings,
) -> Option<ReactiveRectifierMatch> {
    let registry = match cli_key {
        "claude" | "claude_desktop" => ANTHROPIC_REGISTRY.as_slice(),
        "gemini" => GEMINI_REGISTRY.as_slice(),
        _ => return None,
    };

    let messages = error_messages(error_message);
    for kind in registry {
        if let Some(trigger) = messages.iter().find_map(|message| kind.detect(message)) {
            return Some(ReactiveRectifierMatch {
                kind: *kind,
                trigger,
                enabled: settings.enabled(*kind),
            });
        }
    }
    None
}

// Read only error fields: validation responses may echo a complete request in `input`.
// Keep each violation separate so a field name cannot match another violation's message.
fn error_messages(body: &str) -> Vec<String> {
    fn collect(value: &serde_json::Value, messages: &mut Vec<String>) {
        match value {
            serde_json::Value::String(message) => messages.push(message.clone()),
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, messages);
                }
            }
            serde_json::Value::Object(object) => {
                if let Some(message) = object.get("message").and_then(|v| v.as_str()) {
                    messages.push(message.to_owned());
                }
                if let Some(message) = object.get("msg").and_then(|v| v.as_str()) {
                    let path = object.get("loc").and_then(|v| v.as_array()).map(|parts| {
                        parts
                            .iter()
                            .map(|part| match part.as_str() {
                                Some(part) => part.to_owned(),
                                None => part.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(".")
                    });
                    messages.push(match path {
                        Some(path) => format!("{path}: {message}"),
                        None => message.to_owned(),
                    });
                }
                for key in ["error", "detail"] {
                    if let Some(value) = object.get(key) {
                        collect(value, messages);
                    }
                }
            }
            _ => {}
        }
    }

    match serde_json::from_str(body) {
        Ok(value) => {
            let mut messages = Vec::new();
            collect(&value, &mut messages);
            messages
        }
        // The error reader may truncate JSON at its scan limit. Do not let echoed
        // request content become an error message when that JSON cannot be parsed.
        Err(_) if matches!(body.trim_start().chars().next(), Some('{' | '[' | '"')) => Vec::new(),
        Err(_) => vec![body.to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_enabled() -> ReactiveRectifierSettings {
        ReactiveRectifierSettings {
            thinking_effort_conflict: true,
            thinking_signature: true,
            thinking_budget: true,
            gemini_function_id: true,
        }
    }

    #[test]
    fn ignores_echoed_input_and_keeps_validation_errors_separate() {
        for error in [
            serde_json::json!({"detail":[{
                "loc":["body","max_tokens"], "msg":"Field required",
                "input":{"messages":[{"content":[{"type":"thinking","signature":"valid"}]}]}
            }]}),
            serde_json::json!({"detail":[
                {"loc":["body","messages",0,"content",0,"signature"],"msg":"Unexpected value"},
                {"loc":["body","max_tokens"],"msg":"Field required"}
            ]}),
            serde_json::json!({"error":{"message":"max_tokens: Input should be greater than or equal to 1024"},
                "input":{"thinking":{"budget_tokens":512}}}),
            serde_json::json!({"input":{"message":"Invalid signature in thinking block"}}),
        ] {
            assert!(
                detect("claude", &error.to_string(), all_enabled()).is_none(),
                "{error}"
            );
        }
    }

    #[test]
    fn truncated_or_malformed_json_does_not_scan_echoed_input() {
        let body = serde_json::json!({
            "error":{"message":"invalid request: unsupported model"},
            "input":{"messages":[{"content":[{
                "type":"thinking", "thinking":"Inspect the code block", "signature":"valid"
            }]}]},
            "padding":"x".repeat(128 * 1024)
        })
        .to_string();
        assert!(detect("claude", &body, all_enabled()).is_none());
        assert!(detect("claude", &body[..64 * 1024], all_enabled()).is_none());
        for malformed in [
            " \n{\"error\":{\"message\":\"Invalid signature in thinking block\"}",
            " [\"Invalid signature in thinking block\"",
            " \n\"Invalid signature in thinking block",
        ] {
            assert!(
                detect("claude", malformed, all_enabled()).is_none(),
                "{malformed}"
            );
        }
    }

    #[test]
    fn plain_text_errors_still_trigger_each_rectifier() {
        for (cli, message, kind) in [
            (
                "claude",
                " \nInvalid signature in thinking block",
                ReactiveRectifierKind::ThinkingSignature,
            ),
            (
                "claude",
                "thinking.budget_tokens must be greater than or equal to 1024",
                ReactiveRectifierKind::ThinkingBudget,
            ),
            (
                "claude",
                "thinking cannot be disabled when reasoning_effort is set",
                ReactiveRectifierKind::ThinkingEffortConflict,
            ),
            (
                "gemini",
                "Unknown name \"id\" at 'contents[0].parts[0].function_call'",
                ReactiveRectifierKind::GeminiFunctionId,
            ),
        ] {
            assert_eq!(
                detect(cli, message, all_enabled()).map(|m| m.kind),
                Some(kind)
            );
        }
    }

    #[test]
    fn detects_errors_in_supported_json_envelopes() {
        for error in [
            serde_json::json!({"error":{"message":"Invalid signature in thinking block"}}),
            serde_json::json!({"message":"Invalid signature in thinking block"}),
            serde_json::json!({"detail":"Invalid signature in thinking block"}),
            serde_json::json!({"detail":[{"loc":["body","messages",0,"content",0,"thinking","signature"],"msg":"Field required","input":{}}]}),
        ] {
            assert_eq!(
                detect("claude", &error.to_string(), all_enabled()).map(|m| m.kind),
                Some(ReactiveRectifierKind::ThinkingSignature),
                "{error}"
            );
        }
        let gemini = serde_json::json!({"error":{"message":"Unknown name \"id\" at 'contents[0].parts[0].function_call'"}});
        assert_eq!(
            detect("gemini", &gemini.to_string(), all_enabled()).map(|m| m.kind),
            Some(ReactiveRectifierKind::GeminiFunctionId)
        );
    }

    #[test]
    fn anthropic_registry_prioritizes_effort_before_generic_signature() {
        let matched = detect(
            "claude",
            "invalid request: thinking cannot be disabled when reasoning_effort is set",
            all_enabled(),
        )
        .expect("rectifier match");

        assert_eq!(matched.kind, ReactiveRectifierKind::ThinkingEffortConflict);
    }

    #[test]
    fn claude_desktop_uses_anthropic_registry() {
        let matched = detect(
            "claude_desktop",
            "invalid request: thinking cannot be disabled when reasoning_effort is set",
            all_enabled(),
        )
        .expect("rectifier match");

        assert_eq!(matched.kind, ReactiveRectifierKind::ThinkingEffortConflict);
    }

    #[test]
    fn invalid_request_prefix_does_not_hide_budget_error() {
        let matched = detect(
            "claude",
            "invalid request: thinking.budget_tokens must be greater than or equal to 1024",
            all_enabled(),
        )
        .expect("budget rectifier match");

        assert_eq!(matched.kind, ReactiveRectifierKind::ThinkingBudget);
    }

    #[test]
    fn disabled_first_match_does_not_fall_through_to_later_descriptor() {
        let matched = detect(
            "claude",
            "invalid request: thinking cannot be disabled when reasoning_effort is set",
            ReactiveRectifierSettings {
                thinking_effort_conflict: false,
                ..all_enabled()
            },
        )
        .expect("rectifier match");

        assert_eq!(matched.kind, ReactiveRectifierKind::ThinkingEffortConflict);
        assert!(!matched.enabled);
    }

    #[test]
    fn routes_gemini_and_excludes_unrelated_cli() {
        let message = r#"Unknown name "id" at 'contents[0].parts[0].function_call'"#;
        assert_eq!(
            detect("gemini", message, all_enabled()).map(|matched| matched.kind),
            Some(ReactiveRectifierKind::GeminiFunctionId)
        );
        assert!(detect("grok", message, all_enabled()).is_none());
    }
}
