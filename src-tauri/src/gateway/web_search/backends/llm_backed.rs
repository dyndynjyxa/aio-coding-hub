//! LLM-backed search backend.
//!
//! Reuses another configured upstream provider's native `web_search_20250305`
//! tool: posts a `Perform a web search for the query: ...` message to the
//! chosen provider, parses the streamed SSE response, and unwraps the
//! `web_search_tool_result` content into normalized [`SearchHit`]s.
//!
//! This lets aio-coding-hub transparently fan out web search work to *any*
//! provider that natively supports Anthropic-style server-tool web search,
//! without making that provider the main conversation path for the user.

use crate::gateway::web_search::backend::{
    SearchError, SearchHit, SearchOptions, DEFAULT_SEARCH_TIMEOUT,
};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LlmBackedSearchBackend {
    /// Friendly provider name used in error messages.
    pub provider_name: String,
    /// Base URL of the upstream Anthropic-compatible API
    /// (e.g. `https://api.anthropic.com` or a third-party relay).
    pub base_url: String,
    /// API key (or `Bearer` token) for the upstream.
    pub api_key: String,
    /// Model to invoke (e.g. `claude-sonnet-4-5`). The model must support
    /// the `web_search_20250305` server tool.
    pub model: String,
    pub timeout: Duration,
}

impl LlmBackedSearchBackend {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            provider_name: provider_name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            timeout: DEFAULT_SEARCH_TIMEOUT,
        }
    }

    fn build_client(&self) -> Result<reqwest::Client, SearchError> {
        reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| SearchError::Transport {
                message: format!("failed to build reqwest client: {e}"),
            })
    }

    /// Perform a search by delegating to a LLM-native web search.
    pub async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchHit>, SearchError> {
        if self.api_key.trim().is_empty() {
            return Err(SearchError::InvalidConfig {
                message: format!("provider '{}' api key is empty", self.provider_name),
            });
        }
        if self.base_url.trim().is_empty() {
            return Err(SearchError::InvalidConfig {
                message: format!("provider '{}' base url is empty", self.provider_name),
            });
        }
        if self.model.trim().is_empty() {
            return Err(SearchError::InvalidConfig {
                message: format!("provider '{}' model is empty", self.provider_name),
            });
        }

        let client = self.build_client()?;

        // Build the request body. Mirrors the shape Claude Code's
        // WebSearchTool sends internally.
        let mut site_filter: Option<Value> = None;
        if !opts.allowed_domains.is_empty() {
            site_filter = Some(Value::Array(
                opts.allowed_domains
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ));
        }
        let web_search_tool = serde_json::json!({
            "type": "web_search_20250305",
            "name": "web_search",
            "max_uses": 1,
        });
        let mut web_search_tool = web_search_tool;
        if let Some(sf) = site_filter {
            web_search_tool["allowed_domains"] = sf;
        }
        if !opts.blocked_domains.is_empty() {
            web_search_tool["blocked_domains"] = Value::Array(
                opts.blocked_domains
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            );
        }

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 256,
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": format!("Perform a web search for the query: {query}")
                }
            ],
            "tools": [web_search_tool],
        });

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| SearchError::Transport {
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SearchError::Upstream {
                status: status.as_u16(),
                message: format!(
                    "llm-backed search upstream returned {status}: {}",
                    truncate(&body, 512)
                ),
            });
        }

        let bytes = resp.bytes().await.map_err(|e| SearchError::Transport {
            message: e.to_string(),
        })?;

        parse_web_search_result_blocks(&bytes)
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

/// Walk an Anthropic SSE stream and collect every `web_search_tool_result`
/// block's `content[]` entries of type `web_search_result`.
///
/// Accepts three input shapes:
///   1. Non-streamed JSON: top-level `content[]` of result-bearing blocks.
///   2. SSE: one JSON object per `data:` line, terminated by `\n\n`.
fn parse_web_search_result_blocks(raw: &[u8]) -> Result<Vec<SearchHit>, SearchError> {
    let text = std::str::from_utf8(raw).map_err(|e| SearchError::Transport {
        message: format!("llm-backed search returned non-utf8 body: {e}"),
    })?;

    if let Ok(json) = serde_json::from_str::<Value>(text) {
        if let Some(arr) = json.get("content").and_then(|v| v.as_array()) {
            return Ok(extract_hits_recursive(arr));
        }
    }

    let mut hits: Vec<SearchHit> = Vec::new();
    let mut current_event: Option<String> = None;
    let mut current_data = String::new();

    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(ev) = line.strip_prefix("event: ") {
            current_event = Some(ev.trim().to_string());
            current_data.clear();
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            current_data.push_str(data);
            continue;
        }
        if line.is_empty() && !current_data.is_empty() {
            if matches!(
                current_event.as_deref(),
                Some("content_block_start" | "content_block_delta")
            ) {
                if let Ok(val) = serde_json::from_str::<Value>(&current_data) {
                    if let Some(block) = val.get("content_block") {
                        if let Some(arr) = block.get("content").and_then(|v| v.as_array()) {
                            hits.extend(extract_hits_recursive(arr));
                        }
                    }
                    if let Some(arr) = val.get("content").and_then(|v| v.as_array()) {
                        hits.extend(extract_hits_recursive(arr));
                    }
                }
            }
            current_data.clear();
            current_event = None;
        }
    }
    Ok(hits)
}

/// Recursively scan a `content[]` array for `web_search_result` entries,
/// descending into `web_search_tool_result` blocks (which carry their hits
/// under a nested `content` array).
fn extract_hits_recursive(arr: &[Value]) -> Vec<SearchHit> {
    let mut out = Vec::new();
    for item in arr {
        match item.get("type").and_then(|v| v.as_str()) {
            Some("web_search_result") => {
                if let Some(hit) = hit_from_value(item) {
                    out.push(hit);
                }
            }
            Some("web_search_tool_result") => {
                if let Some(nested) = item.get("content").and_then(|v| v.as_array()) {
                    out.extend(extract_hits_recursive(nested));
                }
            }
            _ => {}
        }
    }
    out
}

fn hit_from_value(item: &Value) -> Option<SearchHit> {
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let url = item
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let snippet = item
        .get("snippet")
        .or_else(|| item.get("description"))
        .or_else(|| item.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if title.is_empty() && url.is_empty() {
        return None;
    }
    Some(SearchHit {
        title,
        url,
        snippet,
        published_at: None,
    })
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_non_stream_json_message() {
        let body = serde_json::json!({
            "content": [
                {
                    "type": "web_search_tool_result",
                    "content": [
                        {"type": "web_search_result", "title": "A", "url": "https://a.com"},
                        {"type": "web_search_result", "title": "B", "url": "https://b.com"}
                    ]
                }
            ]
        })
        .to_string();

        let hits = parse_web_search_result_blocks(body.as_bytes()).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "A");
        assert_eq!(hits[1].url, "https://b.com");
    }

    #[test]
    fn parse_sse_event_stream() {
        let sse = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\"}\n",
            "\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"web_search_tool_result\",\"content\":[{\"type\":\"web_search_result\",\"title\":\"X\",\"url\":\"https://x.com\"}]}}\n",
            "\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n",
            "\n",
        );
        let hits = parse_web_search_result_blocks(sse.as_bytes()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "X");
    }

    #[tokio::test]
    async fn missing_config_returns_invalid_config() {
        let backend =
            LlmBackedSearchBackend::new("demo", "https://example.com", "", "claude-sonnet");
        let err = backend
            .search("rust", &SearchOptions::default())
            .await
            .unwrap_err();
        match err {
            SearchError::InvalidConfig { message } => assert!(message.contains("api key")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
