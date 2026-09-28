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
        "claude" => ANTHROPIC_REGISTRY.as_slice(),
        "gemini" => GEMINI_REGISTRY.as_slice(),
        _ => return None,
    };

    for kind in registry {
        if let Some(trigger) = kind.detect(error_message) {
            return Some(ReactiveRectifierMatch {
                kind: *kind,
                trigger,
                enabled: settings.enabled(*kind),
            });
        }
    }
    None
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
