//! Tavily Search API backend.
//!
//! Calls `https://api.tavily.com/search` with the configured API key and maps
//! results to [`SearchHit`]. Tavily returns LLM-friendly snippets out of the
//! box, which makes it a good match for the `web_search_tool_result` shape
//! that Claude Code expects.
//!
//! Reference: <https://docs.tavily.com/docs/api-reference/endpoint/search>

use crate::gateway::web_search::backend::{
    SearchError, SearchHit, SearchOptions, DEFAULT_SEARCH_TIMEOUT,
};
use serde::Deserialize;
use std::time::Duration;

/// Built-in backend that proxies queries to the Tavily Search API.
#[derive(Debug, Clone)]
pub struct TavilySearchBackend {
    pub api_key: String,
    pub timeout: Duration,
    /// Optional upstream proxy URL (matches `AppSettings::upstream_proxy_url`).
    pub proxy_url: Option<String>,
}

impl TavilySearchBackend {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout: DEFAULT_SEARCH_TIMEOUT,
            proxy_url: None,
        }
    }

    pub fn with_proxy(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    fn build_client(&self) -> Result<reqwest::Client, SearchError> {
        let mut builder = reqwest::Client::builder().timeout(self.timeout);
        if let Some(proxy) = self.proxy_url.as_deref().filter(|s| !s.is_empty()) {
            let reqwest_proxy =
                reqwest::Proxy::all(proxy).map_err(|e| SearchError::InvalidConfig {
                    message: format!("invalid tavily backend proxy url: {e}"),
                })?;
            builder = builder.proxy(reqwest_proxy);
        }
        builder.build().map_err(|e| SearchError::Transport {
            message: format!("failed to build reqwest client: {e}"),
        })
    }

    /// Perform a search. Returns normalized [`SearchHit`]s.
    pub async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchHit>, SearchError> {
        if self.api_key.trim().is_empty() {
            return Err(SearchError::InvalidConfig {
                message: "tavily api key is empty".to_string(),
            });
        }

        let client = self.build_client()?;
        let count = opts.max_results.max(1) as u32;

        let body = serde_json::json!({
            "api_key": self.api_key,
            "query": query,
            "max_results": count,
            "topic": "general",
            "include_answer": false,
            "include_raw_content": false,
            "include_images": false,
        });

        let resp = client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
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
                message: format!("tavily returned {status}: {}", truncate(&body, 512)),
            });
        }

        let parsed: TavilyResponse = resp.json().await.map_err(|e| SearchError::Transport {
            message: format!("tavily returned non-JSON body: {e}"),
        })?;

        let mut hits: Vec<SearchHit> = parsed
            .results
            .into_iter()
            .map(|r| SearchHit {
                title: r.title,
                url: r.url,
                snippet: r.content,
                published_at: if r.published_date.is_empty() {
                    None
                } else {
                    Some(r.published_date)
                },
            })
            .collect();

        // Post-filter for blocked_domains (Tavily does not natively support it).
        if !opts.blocked_domains.is_empty() {
            hits.retain(|h| {
                let host = host_of(&h.url).unwrap_or_default();
                let host = host.strip_prefix("www.").unwrap_or(&host);
                !opts.blocked_domains.iter().any(|d| d == host)
            });
        }
        // Post-filter for allowed_domains (intersection semantics).
        if !opts.allowed_domains.is_empty() {
            hits.retain(|h| {
                let host = host_of(&h.url).unwrap_or_default();
                let host = host.strip_prefix("www.").unwrap_or(&host);
                opts.allowed_domains.iter().any(|d| {
                    let d_stripped = d.strip_prefix("www.").unwrap_or(d);
                    host == d_stripped || host.ends_with(&format!(".{d_stripped}"))
                })
            });
        }

        hits.truncate(count as usize);
        Ok(hits)
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

fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://")?.1;
    let host_end = after_scheme
        .find(['/', '?', '#', ':'])
        .unwrap_or(after_scheme.len());
    let host = &after_scheme[..host_end];
    if host.is_empty() {
        return None;
    }
    Some(host.trim_start_matches("www.").to_string())
}

// --- Response shapes --------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    published_date: String,
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_opts() -> SearchOptions {
        SearchOptions {
            max_results: 5,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn missing_api_key_returns_invalid_config() {
        let backend = TavilySearchBackend::new("");
        let err = backend.search("rust", &dummy_opts()).await.unwrap_err();
        match err {
            SearchError::InvalidConfig { message } => assert!(message.contains("api key")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
