//! CX2CC bridge runtime settings, read from AppSettings.

use crate::infra::settings;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Cx2ccSettings {
    pub fallback_model_opus: String,
    pub fallback_model_sonnet: String,
    pub fallback_model_haiku: String,
    pub fallback_model_main: String,
    pub model_reasoning_effort: Option<String>,
    pub service_tier: Option<String>,
    pub disable_response_storage: bool,
    pub enable_reasoning_to_thinking: bool,
    pub drop_stop_sequences: bool,
    pub clean_schema: bool,
    pub filter_batch_tool: bool,
}

impl Cx2ccSettings {
    pub fn from_app_settings(s: &settings::AppSettings) -> Self {
        Self {
            fallback_model_opus: s.cx2cc_fallback_model_opus.clone(),
            fallback_model_sonnet: s.cx2cc_fallback_model_sonnet.clone(),
            fallback_model_haiku: s.cx2cc_fallback_model_haiku.clone(),
            fallback_model_main: s.cx2cc_fallback_model_main.clone(),
            model_reasoning_effort: non_empty(&s.cx2cc_model_reasoning_effort),
            service_tier: non_empty(&s.cx2cc_service_tier),
            disable_response_storage: s.cx2cc_disable_response_storage,
            enable_reasoning_to_thinking: s.cx2cc_enable_reasoning_to_thinking,
            drop_stop_sequences: s.cx2cc_drop_stop_sequences,
            clean_schema: s.cx2cc_clean_schema,
            filter_batch_tool: s.cx2cc_filter_batch_tool,
        }
    }
}

impl Default for Cx2ccSettings {
    fn default() -> Self {
        Self {
            fallback_model_opus: settings::DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            fallback_model_sonnet: settings::DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            fallback_model_haiku: settings::DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            fallback_model_main: settings::DEFAULT_CX2CC_FALLBACK_MODEL.to_string(),
            model_reasoning_effort: None,
            service_tier: None,
            disable_response_storage: true,
            enable_reasoning_to_thinking: true,
            drop_stop_sequences: true,
            clean_schema: true,
            filter_batch_tool: true,
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Cx2ccSettings;
    use crate::infra::settings::AppSettings;

    #[test]
    fn default_uses_expected_values() {
        let cfg = Cx2ccSettings::default();

        assert_eq!(cfg.fallback_model_opus, "gpt-5.4");
        assert_eq!(cfg.fallback_model_sonnet, "gpt-5.4");
        assert_eq!(cfg.fallback_model_haiku, "gpt-5.4");
        assert_eq!(cfg.fallback_model_main, "gpt-5.4");
        assert_eq!(cfg.model_reasoning_effort, None);
        assert_eq!(cfg.service_tier, None);
        assert!(cfg.disable_response_storage);
        assert!(cfg.enable_reasoning_to_thinking);
        assert!(cfg.drop_stop_sequences);
        assert!(cfg.clean_schema);
        assert!(cfg.filter_batch_tool);
    }

    #[test]
    fn from_app_settings_trims_optional_strings() {
        let app = AppSettings {
            cx2cc_fallback_model_opus: "o3".to_string(),
            cx2cc_fallback_model_sonnet: "gpt-4.1".to_string(),
            cx2cc_fallback_model_haiku: "gpt-4.1-mini".to_string(),
            cx2cc_fallback_model_main: "gpt-5.4".to_string(),
            cx2cc_model_reasoning_effort: " medium ".to_string(),
            cx2cc_service_tier: "  flex ".to_string(),
            cx2cc_disable_response_storage: false,
            cx2cc_enable_reasoning_to_thinking: false,
            cx2cc_drop_stop_sequences: false,
            cx2cc_clean_schema: false,
            cx2cc_filter_batch_tool: false,
            ..Default::default()
        };

        let cfg = Cx2ccSettings::from_app_settings(&app);

        assert_eq!(cfg.fallback_model_opus, "o3");
        assert_eq!(cfg.fallback_model_sonnet, "gpt-4.1");
        assert_eq!(cfg.fallback_model_haiku, "gpt-4.1-mini");
        assert_eq!(cfg.fallback_model_main, "gpt-5.4");
        assert_eq!(cfg.model_reasoning_effort.as_deref(), Some("medium"));
        assert_eq!(cfg.service_tier.as_deref(), Some("flex"));
        assert!(!cfg.disable_response_storage);
        assert!(!cfg.enable_reasoning_to_thinking);
        assert!(!cfg.drop_stop_sequences);
        assert!(!cfg.clean_schema);
        assert!(!cfg.filter_batch_tool);
    }
}
