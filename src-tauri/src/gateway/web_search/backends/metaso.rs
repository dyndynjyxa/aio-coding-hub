//! Metaso Search API backend.
//!
//! Calls `https://metaso.cn/api/v1/search` with the configured API key and maps
//! results to [`SearchHit`]. Metaso returns LLM-friendly results with an
//! optional AI-generated `summary` per hit, which makes it a good match for
//! the `web_search_tool_result` shape that Claude Code expects.
//!
//! Reference: <https://metaso.cn>
//!
//! Request body shape (excerpted from public docs / sample calls):
//! ```json
//! {
//!   "q": "<query>",
//!   "scope": "webpage",
//!   "size": 10,
//!   "includeSummary": true,
//!   "includeRawContent": false,
//!   "conciseSnippet": true
//! }
//! ```
//!
//! Response shape (see `MetasoResponse` below).

use crate::gateway::web_search::backend::{
    SearchError, SearchHit, SearchOptions, DEFAULT_SEARCH_TIMEOUT,
};
use serde::Deserialize;
use std::time::Duration;

/// Built-in backend that proxies queries to the Metaso Search API.
#[derive(Debug, Clone)]
pub struct MetasoSearchBackend {
    pub api_key: String,
    pub timeout: Duration,
    /// Optional upstream proxy URL (matches `AppSettings::upstream_proxy_url`).
    pub proxy_url: Option<String>,
    /// Include Metaso's AI-generated `summary` field on each hit when true.
    pub include_summary: bool,
    /// Ask Metaso to return a shorter `snippet` field.
    pub concise_snippet: bool,
}

impl MetasoSearchBackend {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout: DEFAULT_SEARCH_TIMEOUT,
            proxy_url: None,
            include_summary: false,
            concise_snippet: false,
        }
    }

    pub fn with_proxy(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    pub fn with_include_summary(mut self, include_summary: bool) -> Self {
        self.include_summary = include_summary;
        self
    }

    pub fn with_concise_snippet(mut self, concise_snippet: bool) -> Self {
        self.concise_snippet = concise_snippet;
        self
    }

    fn build_client(&self) -> Result<reqwest::Client, SearchError> {
        let mut builder = reqwest::Client::builder().timeout(self.timeout);
        if let Some(proxy) = self.proxy_url.as_deref().filter(|s| !s.is_empty()) {
            let reqwest_proxy =
                reqwest::Proxy::all(proxy).map_err(|e| SearchError::InvalidConfig {
                    message: format!("invalid metaso backend proxy url: {e}"),
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
                message: "metaso api key is empty".to_string(),
            });
        }

        let client = self.build_client()?;
        let count = opts.max_results.max(1) as u32;

        // Metaso expects `size` as a string in the public sample call.
        let body = serde_json::json!({
            "q": query,
            "scope": "webpage",
            "size": count.to_string(),
            "includeSummary": self.include_summary,
            "includeRawContent": false,
            "conciseSnippet": self.concise_snippet,
        });

        let resp = client
            .post("https://metaso.cn/api/v1/search")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
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
                message: format!("metaso returned {status}: {}", truncate(&body, 512)),
            });
        }

        let parsed: MetasoResponse = resp.json().await.map_err(|e| SearchError::Transport {
            message: format!("metaso returned non-JSON body: {e}"),
        })?;

        let mut hits: Vec<SearchHit> = parsed
            .webpages
            .into_iter()
            .map(|r| {
                // Prefer the AI summary (richer) when include_summary is on
                // and the upstream actually returned one. Fall back to the
                // search-engine snippet, then to an empty string.
                let snippet = if !r.summary.is_empty() {
                    r.summary
                } else {
                    r.snippet
                };
                SearchHit {
                    title: r.title,
                    url: r.link,
                    snippet,
                    published_at: if r.date.is_empty() {
                        None
                    } else {
                        Some(r.date)
                    },
                }
            })
            .collect();

        // Post-filter for blocked_domains (Metaso does not natively support it).
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
struct MetasoResponse {
    #[serde(default)]
    webpages: Vec<MetasoWebpage>,
}

#[derive(Debug, Deserialize)]
struct MetasoWebpage {
    title: String,
    link: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    date: String,
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
        let backend = MetasoSearchBackend::new("");
        let err = backend.search("rust", &dummy_opts()).await.unwrap_err();
        match err {
            SearchError::InvalidConfig { message } => assert!(message.contains("api key")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn parses_sample_response() {
        let body = serde_json::json!({
            "webpages": [
                {
                    "title": "梦Meng梦辞汐",
                    "link": "https://www.douyin.com/user/MS4wLjABAAAA",
                    "score": "high",
                    "summary": "梦Meng梦辞汐是抖音平台上的用户",
                    "position": 1,
                    "date": "2023年04月27日"
                },
                {
                    "title": "唐凌汐",
                    "link": "https://m.qidian.com/book/1030298928/711275406.html",
                    "score": "medium",
                    "snippet": "唐凌汐望着眼前的羲和，她实在太漂亮了",
                    "position": 2,
                    "date": "2021年08月20日"
                }
            ],
            "total": 2
        })
        .to_string();

        let parsed: MetasoResponse = serde_json::from_str(&body).expect("parses");
        assert_eq!(parsed.webpages.len(), 2);
        assert_eq!(parsed.webpages[0].title, "梦Meng梦辞汐");
        assert!(!parsed.webpages[0].summary.is_empty());
        assert_eq!(parsed.webpages[0].date, "2023年04月27日");
        assert!(parsed.webpages[1].snippet.starts_with("唐凌汐"));
    }
}
