//! Detect requests that originate from Claude Code's `WebSearchTool`.
//!
//! Claude Code's WebSearchTool makes its own LLM sub-call (rather than
//! receiving a request from the model), and it has a recognizable shape:
//!
//!   - `messages[0].role == "user"`
//!   - `messages[0].content` is a single text block whose text starts with
//!     `"Perform a web search for the query: "`
//!   - the request's `tools` array contains an entry with
//!     `type == "web_search_20250305"`
//!
//! Both signals are checked (the message prefix is the strongest one and
//! the only one Claude Code itself emits today; the tool-presence check is
//! belt-and-braces in case a future client uses a different prefix).

use serde_json::Value;

/// Marker text that Claude Code's WebSearchTool prepends to its internal
/// LLM call's user message. Exposed publicly so tests can build fixtures
/// without duplicating the literal.
pub const WEB_SEARCH_USER_MESSAGE_PREFIX: &str = "Perform a web search for the query: ";

/// Anthropic server tool type identifier.
pub const WEB_SEARCH_TOOL_TYPE: &str = "web_search_20250305";

/// Result of a detection pass. The `query` field is set when detection
/// succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchDetection {
    pub query: String,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    /// `max_uses` value declared by the caller. We always emit exactly one
    /// search result block per intercepted request, so this is informational
    /// only.
    pub max_uses: u32,
}

/// Returns `Some(detection)` if `body` is a WebSearchTool sub-call, `None`
/// otherwise. Performs no logging; callers decide how to react.
pub fn detect_web_search_request(body: &Value) -> Option<WebSearchDetection> {
    if !has_web_search_tool(body) {
        return None;
    }
    let user_text = extract_user_message_text(body)?;
    let rest = user_text.strip_prefix(WEB_SEARCH_USER_MESSAGE_PREFIX)?;
    let query = rest.trim();
    if query.is_empty() {
        return None;
    }

    let (allowed_domains, blocked_domains, max_uses) = extract_tool_options(body);

    Some(WebSearchDetection {
        query: query.to_string(),
        allowed_domains,
        blocked_domains,
        max_uses,
    })
}

fn has_web_search_tool(body: &Value) -> bool {
    body.get("tools")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .any(|t| t.get("type").and_then(|v| v.as_str()) == Some(WEB_SEARCH_TOOL_TYPE))
        })
        .unwrap_or(false)
}

fn extract_user_message_text(body: &Value) -> Option<&str> {
    let messages = body.get("messages").and_then(|m| m.as_array())?;
    let first = messages.first()?.as_object()?;
    if first.get("role").and_then(|v| v.as_str()) != Some("user") {
        return None;
    }
    let content = first.get("content")?;
    match content {
        Value::String(s) => Some(s.as_str()),
        Value::Array(arr) => {
            // Prefer a string-typed text block. We only consider the first
            // text block — WebSearchTool emits exactly one.
            for block in arr {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_tool_options(body: &Value) -> (Vec<String>, Vec<String>, u32) {
    let Some(tools) = body.get("tools").and_then(|t| t.as_array()) else {
        return (Vec::new(), Vec::new(), 1);
    };
    let Some(tool) = tools
        .iter()
        .find(|t| t.get("type").and_then(|v| v.as_str()) == Some(WEB_SEARCH_TOOL_TYPE))
    else {
        return (Vec::new(), Vec::new(), 1);
    };

    let allowed_domains = tool
        .get("allowed_domains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let blocked_domains = tool
        .get("blocked_domains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let max_uses = tool
        .get("max_uses")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(1);

    (allowed_domains, blocked_domains, max_uses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_request(query: &str) -> Value {
        json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 256,
            "messages": [
                {"role": "user", "content": format!("{WEB_SEARCH_USER_MESSAGE_PREFIX}{query}")}
            ],
            "tools": [
                {
                    "type": "web_search_20250305",
                    "name": "web_search",
                    "max_uses": 8
                }
            ]
        })
    }

    #[test]
    fn detects_a_typical_websearch_request() {
        let body = sample_request("rust async");
        let det = detect_web_search_request(&body).expect("should detect");
        assert_eq!(det.query, "rust async");
        assert_eq!(det.max_uses, 8);
        assert!(det.allowed_domains.is_empty());
        assert!(det.blocked_domains.is_empty());
    }

    #[test]
    fn extracts_allowed_and_blocked_domains() {
        let body = json!({
            "model": "claude-sonnet-4-5",
            "messages": [
                {"role": "user", "content": format!("{WEB_SEARCH_USER_MESSAGE_PREFIX}rust")}
            ],
            "tools": [
                {
                    "type": "web_search_20250305",
                    "name": "web_search",
                    "allowed_domains": ["anthropic.com", "docs.anthropic.com"],
                    "blocked_domains": ["example.com"],
                    "max_uses": 3
                }
            ]
        });
        let det = detect_web_search_request(&body).unwrap();
        assert_eq!(
            det.allowed_domains,
            vec!["anthropic.com", "docs.anthropic.com"]
        );
        assert_eq!(det.blocked_domains, vec!["example.com"]);
        assert_eq!(det.max_uses, 3);
    }

    #[test]
    fn handles_user_content_as_array() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": format!("{WEB_SEARCH_USER_MESSAGE_PREFIX}hello")
                }]
            }],
            "tools": [{"type": "web_search_20250305", "name": "web_search"}]
        });
        let det = detect_web_search_request(&body).unwrap();
        assert_eq!(det.query, "hello");
    }

    #[test]
    fn does_not_detect_a_normal_message() {
        let body = json!({
            "messages": [{"role": "user", "content": "hello world"}],
            "tools": []
        });
        assert!(detect_web_search_request(&body).is_none());
    }

    #[test]
    fn does_not_detect_when_user_message_lacks_prefix() {
        let body = json!({
            "messages": [{"role": "user", "content": "what is rust"}],
            "tools": [{"type": "web_search_20250305", "name": "web_search"}]
        });
        assert!(detect_web_search_request(&body).is_none());
    }

    #[test]
    fn does_not_detect_when_web_search_tool_is_absent() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": format!("{WEB_SEARCH_USER_MESSAGE_PREFIX}hello")
            }],
            "tools": [{"name": "Bash", "type": "function"}]
        });
        assert!(detect_web_search_request(&body).is_none());
    }

    #[test]
    fn rejects_empty_query() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": WEB_SEARCH_USER_MESSAGE_PREFIX
            }],
            "tools": [{"type": "web_search_20250305", "name": "web_search"}]
        });
        assert!(detect_web_search_request(&body).is_none());
    }
}
