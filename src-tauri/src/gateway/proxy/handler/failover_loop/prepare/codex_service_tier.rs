//! Usage: Codex service_tier detection and priority billing decision.
//!
//! Detects `service_tier` from Codex requests and responses, and determines
//! whether priority pricing should be applied for billing.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::gateway::response_fixer;

/// Result of Codex service tier detection and billing decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexServiceTierResult {
    #[serde(rename = "type")]
    pub type_name: String,
    pub scope: String,
    pub hit: bool,
    #[serde(rename = "requestedServiceTier")]
    pub requested_service_tier: Option<String>,
    #[serde(rename = "actualServiceTier")]
    pub actual_service_tier: Option<String>,
    #[serde(
        rename = "billingSourcePreference",
        skip_serializing_if = "Option::is_none"
    )]
    pub billing_source_preference: Option<String>,
    #[serde(rename = "resolvedFrom", skip_serializing_if = "Option::is_none")]
    pub resolved_from: Option<String>,
    #[serde(rename = "effectivePriority")]
    pub effective_priority: bool,
}

impl CodexServiceTierResult {
    /// Create a new billing decision result.
    ///
    /// # Arguments
    /// - `requested`: service_tier from request body
    /// - `actual`: service_tier from response
    /// - `billing_source`: "requested" or "actual" preference from settings
    pub fn new(requested: Option<String>, actual: Option<String>, billing_source: &str) -> Self {
        let (resolved_from, effective_priority) =
            resolve_priority(requested.as_deref(), actual.as_deref(), billing_source);

        Self {
            type_name: "codex_service_tier_result".to_string(),
            scope: "response".to_string(),
            hit: requested.is_some() || actual.is_some(),
            requested_service_tier: requested,
            actual_service_tier: actual,
            billing_source_preference: Some(billing_source.to_string()),
            resolved_from: resolved_from.map(|s| s.to_string()),
            effective_priority,
        }
    }
}

/// Determine effective priority based on billing source preference.
fn resolve_priority(
    requested: Option<&str>,
    actual: Option<&str>,
    billing_source: &str,
) -> (Option<&'static str>, bool) {
    match billing_source {
        "requested" => {
            let is_priority = requested == Some("priority");
            if requested.is_some() {
                (Some("requested"), is_priority)
            } else {
                (None, false)
            }
        }
        _ => {
            if let Some(actual_tier) = actual {
                (Some("actual"), actual_tier == "priority")
            } else if let Some(requested_tier) = requested {
                (Some("requested"), requested_tier == "priority")
            } else {
                (None, false)
            }
        }
    }
}

/// Extract `service_tier` from Codex request body.
///
/// OpenAI API format: `{ "service_tier": "priority" | "default" | ... }`
pub fn extract_requested_service_tier(body: &Value) -> Option<String> {
    body.get("service_tier")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parse `service_tier` from JSON response body.
///
/// Checks both top-level and nested `response.service_tier` (for Responses API).
pub fn parse_service_tier_from_json(body: &Value) -> Option<String> {
    // Try top-level service_tier
    if let Some(tier) = body.get("service_tier").and_then(|v| v.as_str()) {
        let tier = tier.trim();
        if !tier.is_empty() {
            return Some(tier.to_string());
        }
    }

    // Try response.service_tier (OpenAI Responses API format)
    if let Some(tier) = body
        .get("response")
        .and_then(|r| r.get("service_tier"))
        .and_then(|v| v.as_str())
    {
        let tier = tier.trim();
        if !tier.is_empty() {
            return Some(tier.to_string());
        }
    }

    None
}

/// Parse `service_tier` from a single SSE data line.
///
/// Expected format: `data: {...}` or `data: [DONE]`
pub fn parse_service_tier_from_sse_line(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data.trim() == "[DONE]" {
        return None;
    }

    let json: Value = serde_json::from_str(data).ok()?;
    parse_service_tier_from_json(&json)
}

/// Check if text looks like SSE format.
pub fn is_sse_text(text: &str) -> bool {
    text.lines()
        .any(|line| line.starts_with("data: ") || line.starts_with("event: "))
}

/// Parse service_tier from complete response text (JSON or SSE).
///
/// For SSE, returns the last seen service_tier value.
pub fn parse_service_tier_from_response_text(response_text: &str) -> Option<String> {
    let mut last_seen: Option<String> = None;

    // Try direct JSON parse first
    if let Ok(json) = serde_json::from_str::<Value>(response_text) {
        if let Some(tier) = parse_service_tier_from_json(&json) {
            return Some(tier);
        }
    }

    // Fallback: SSE stream parsing
    if is_sse_text(response_text) {
        for line in response_text.lines() {
            if let Some(tier) = parse_service_tier_from_sse_line(line) {
                last_seen = Some(tier);
            }
        }
    }

    last_seen
}

/// Build a `codex_service_tier_result` setting from request/response bytes.
pub(super) fn build_result_from_bodies(
    cli_key: &str,
    request_body: &[u8],
    response_body: Option<&[u8]>,
) -> Option<CodexServiceTierResult> {
    if cli_key != "codex" {
        return None;
    }

    let requested = serde_json::from_slice::<Value>(request_body)
        .ok()
        .and_then(|body| extract_requested_service_tier(&body));
    let actual = response_body
        .and_then(|body| std::str::from_utf8(body).ok())
        .and_then(parse_service_tier_from_response_text);

    let result = CodexServiceTierResult::new(requested, actual, "actual");
    result.hit.then_some(result)
}

/// Append a Codex service tier decision to request-log special settings.
pub(super) fn append_result_if_detected(
    cli_key: &str,
    request_body: &[u8],
    response_body: Option<&[u8]>,
    special_settings: &Arc<Mutex<Vec<Value>>>,
) {
    let Some(result) = build_result_from_bodies(cli_key, request_body, response_body) else {
        return;
    };

    let Ok(value) = serde_json::to_value(result) else {
        return;
    };

    response_fixer::push_special_setting(special_settings, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_requested_service_tier() {
        let body = serde_json::json!({ "service_tier": "priority" });
        assert_eq!(
            extract_requested_service_tier(&body),
            Some("priority".to_string())
        );

        let body = serde_json::json!({ "service_tier": "default" });
        assert_eq!(
            extract_requested_service_tier(&body),
            Some("default".to_string())
        );

        let body = serde_json::json!({ "model": "gpt-5.4" });
        assert_eq!(extract_requested_service_tier(&body), None);

        let body = serde_json::json!({ "service_tier": "" });
        assert_eq!(extract_requested_service_tier(&body), None);
    }

    #[test]
    fn test_parse_service_tier_from_json() {
        // Top-level
        let body = serde_json::json!({ "service_tier": "priority", "id": "123" });
        assert_eq!(
            parse_service_tier_from_json(&body),
            Some("priority".to_string())
        );

        // Nested in response
        let body = serde_json::json!({
            "response": { "service_tier": "default" }
        });
        assert_eq!(
            parse_service_tier_from_json(&body),
            Some("default".to_string())
        );

        // No service_tier
        let body = serde_json::json!({ "id": "123" });
        assert_eq!(parse_service_tier_from_json(&body), None);
    }

    #[test]
    fn test_parse_service_tier_from_sse_line() {
        let line = r#"data: {"id":"123","service_tier":"priority"}"#;
        assert_eq!(
            parse_service_tier_from_sse_line(line),
            Some("priority".to_string())
        );

        let line = "data: [DONE]";
        assert_eq!(parse_service_tier_from_sse_line(line), None);

        let line = "event: message";
        assert_eq!(parse_service_tier_from_sse_line(line), None);
    }

    #[test]
    fn test_resolve_priority_actual_preference() {
        // Actual takes precedence
        let (from, priority) = resolve_priority(Some("priority"), Some("default"), "actual");
        assert_eq!(from, Some("actual"));
        assert!(!priority);

        // Fall back to requested if no actual
        let (from, priority) = resolve_priority(Some("priority"), None, "actual");
        assert_eq!(from, Some("requested"));
        assert!(priority);
    }

    #[test]
    fn test_resolve_priority_requested_preference() {
        // Only use requested
        let (from, priority) = resolve_priority(Some("priority"), Some("default"), "requested");
        assert_eq!(from, Some("requested"));
        assert!(priority);

        // Ignore actual even if present
        let (from, priority) = resolve_priority(Some("default"), Some("priority"), "requested");
        assert_eq!(from, Some("requested"));
        assert!(!priority);
    }

    #[test]
    fn test_codex_service_tier_result_serialization() {
        let result = CodexServiceTierResult::new(
            Some("priority".to_string()),
            Some("priority".to_string()),
            "actual",
        );

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["type"], "codex_service_tier_result");
        assert_eq!(json["scope"], "response");
        assert_eq!(json["hit"], true);
        assert_eq!(json["requestedServiceTier"], "priority");
        assert_eq!(json["actualServiceTier"], "priority");
        assert_eq!(json["billingSourcePreference"], "actual");
        assert_eq!(json["resolvedFrom"], "actual");
        assert_eq!(json["effectivePriority"], true);
    }

    #[test]
    fn test_build_result_from_bodies_uses_response_actual_tier() {
        let request = br#"{"service_tier":"priority"}"#;
        let response = br#"{"service_tier":"default"}"#;

        let result = build_result_from_bodies("codex", request, Some(response)).unwrap();

        assert_eq!(result.requested_service_tier.as_deref(), Some("priority"));
        assert_eq!(result.actual_service_tier.as_deref(), Some("default"));
        assert!(!result.effective_priority);
        assert_eq!(result.resolved_from.as_deref(), Some("actual"));
    }

    #[test]
    fn test_build_result_from_bodies_falls_back_to_requested_tier() {
        let request = br#"{"service_tier":"priority"}"#;

        let result = build_result_from_bodies("codex", request, None).unwrap();

        assert_eq!(result.requested_service_tier.as_deref(), Some("priority"));
        assert_eq!(result.actual_service_tier, None);
        assert!(result.effective_priority);
        assert_eq!(result.resolved_from.as_deref(), Some("requested"));
    }

    #[test]
    fn test_build_result_from_bodies_ignores_non_codex() {
        let request = br#"{"service_tier":"priority"}"#;

        assert!(build_result_from_bodies("claude", request, None).is_none());
    }
}
