use crate::gateway::proxy::gemini_oauth::GeminiOAuthResponseMode;
use serde_json::Value;

fn text_at<'a>(body: &'a Value, path: &[&str]) -> Option<&'a str> {
    let value = path.iter().try_fold(body, |value, key| value.get(*key))?;
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn extract(
    body: &[u8],
    upstream_path: &str,
    gemini_oauth_mode: Option<GeminiOAuthResponseMode>,
) -> Option<String> {
    if matches!(
        gemini_oauth_mode,
        Some(GeminiOAuthResponseMode::CountTokens)
    ) {
        return None;
    }
    let body: Value = serde_json::from_slice(body).ok()?;
    let value = match gemini_oauth_mode {
        Some(GeminiOAuthResponseMode::GenerateContent)
        | Some(GeminiOAuthResponseMode::StreamGenerateContent) => text_at(
            &body,
            &[
                "request",
                "generationConfig",
                "thinkingConfig",
                "thinkingLevel",
            ],
        ),
        Some(GeminiOAuthResponseMode::CountTokens) => None,
        None => {
            let path = upstream_path.trim_end_matches('/');
            if path.ends_with("/responses") {
                text_at(&body, &["reasoning", "effort"])
            } else if path.ends_with("/chat/completions") {
                text_at(&body, &["reasoning_effort"])
            } else if path.ends_with("/messages") {
                text_at(&body, &["output_config", "effort"])
            } else if path.ends_with(":generateContent") || path.ends_with(":streamGenerateContent")
            {
                text_at(
                    &body,
                    &["generationConfig", "thinkingConfig", "thinkingLevel"],
                )
            } else {
                None
            }
        }
    }?;
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_value(
        body: &Value,
        upstream_path: &str,
        gemini_oauth_mode: Option<GeminiOAuthResponseMode>,
    ) -> Option<String> {
        extract(
            &serde_json::to_vec(body).expect("serialize request body"),
            upstream_path,
            gemini_oauth_mode,
        )
    }

    #[test]
    fn extracts_only_the_final_protocol_field() {
        let body = serde_json::json!({
            "reasoning": { "effort": " high " },
            "reasoning_effort": "low",
            "output_config": { "effort": "max" },
            "generationConfig": {
                "thinkingConfig": { "thinkingLevel": "medium", "thinkingBudget": 8192 }
            }
        });
        assert_eq!(
            extract_value(&body, "/v1/responses", None).as_deref(),
            Some("high")
        );
        assert_eq!(
            extract_value(&body, "/v1/chat/completions", None).as_deref(),
            Some("low")
        );
        assert_eq!(
            extract_value(&body, "/v1/messages", None).as_deref(),
            Some("max")
        );
        assert_eq!(
            extract_value(&body, "/v1beta/models/gemini:generateContent", None).as_deref(),
            Some("medium")
        );
    }

    #[test]
    fn extracts_wrapped_gemini_and_ignores_count_tokens_or_numeric_budget() {
        let wrapped = serde_json::json!({
            "request": {
                "generationConfig": {
                    "thinkingConfig": { "thinkingLevel": "xhigh", "thinkingBudget": 8192 }
                }
            }
        });
        assert_eq!(
            extract_value(
                &wrapped,
                "/v1internal:generateContent",
                Some(GeminiOAuthResponseMode::GenerateContent)
            )
            .as_deref(),
            Some("xhigh")
        );
        assert_eq!(
            extract_value(
                &wrapped,
                "/v1internal:countTokens",
                Some(GeminiOAuthResponseMode::CountTokens)
            ),
            None
        );
        assert_eq!(
            extract_value(
                &serde_json::json!({"reasoning": {"effort": 8}, "thinkingBudget": 8192}),
                "/v1/responses",
                None
            ),
            None
        );
    }

    #[test]
    fn preserves_future_explicit_string_values() {
        let body = serde_json::json!({"reasoning": {"effort": "ultra"}});
        assert_eq!(
            extract_value(&body, "/responses", None).as_deref(),
            Some("ultra")
        );
    }
}
