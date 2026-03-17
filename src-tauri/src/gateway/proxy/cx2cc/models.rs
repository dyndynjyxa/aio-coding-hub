//! Claude model name → OpenAI model name mapping for the CX2CC translation layer.

pub fn map_claude_to_openai(
    claude_model: &str,
    claude_models: &crate::domain::providers::ClaudeModels,
) -> String {
    if claude_model.contains("opus") {
        if let Some(ref m) = claude_models.opus_model {
            return m.clone();
        }
        return "o3".to_string();
    }

    if claude_model.contains("haiku") {
        if let Some(ref m) = claude_models.haiku_model {
            return m.clone();
        }
        return "gpt-4.1-mini".to_string();
    }

    if claude_model.contains("sonnet") {
        if let Some(ref m) = claude_models.sonnet_model {
            return m.clone();
        }
        return "gpt-4.1".to_string();
    }

    if let Some(ref m) = claude_models.main_model {
        return m.clone();
    }
    "gpt-4.1".to_string()
}
