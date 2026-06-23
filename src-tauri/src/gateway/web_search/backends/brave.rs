//! Brave Search API backend.
//!
//! Calls `https://api.search.brave.com/res/v1/web/search` with the configured
//! API key (`X-Subscription-Token`) and maps results to [`SearchHit`].
//!
//! Reference: <https://api.search.brave.com/app/documentation/web-search/get-started>

use crate::gateway::web_search::backend::{
    SearchError, SearchHit, SearchOptions, DEFAULT_SEARCH_TIMEOUT,
};
use serde::Deserialize;
use std::time::Duration;

/// Built-in backend that proxies queries to the Brave Search Web API.
#[derive(Debug, Clone)]
pub struct BraveSearchBackend {
    pub api_key: String,
    pub timeout: Duration,
    /// Optional upstream proxy URL (matches `AppSettings::upstream_proxy_url`).
    pub proxy_url: Option<String>,
}

impl BraveSearchBackend {
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
                    message: format!("invalid brave backend proxy url: {e}"),
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
                message: "brave api key is empty".to_string(),
            });
        }

        let client = self.build_client()?;
        let count = opts.max_results.max(1) as u32;

        // Brave does not have a direct `blocked_domains` param. Approximate it
        // by appending `-site:domain` to the query string.
        let effective_query = if opts.blocked_domains.is_empty() {
            query.to_string()
        } else {
            let suffix: String = opts
                .blocked_domains
                .iter()
                .map(|d| format!(" -site:{d}"))
                .collect();
            format!("{query}{suffix}")
        };

        let mut params: Vec<(String, String)> = vec![
            ("q".into(), effective_query),
            ("count".into(), count.to_string()),
        ];
        if !opts.allowed_domains.is_empty() {
            params.push(("site_filter".into(), opts.allowed_domains.join(",")));
        }
        if let Some(lang) = opts.language.as_deref() {
            params.push(("search_lang".into(), lang.to_string()));
        }
        if let Some(region) = opts.region.as_deref() {
            params.push(("country".into(), region.to_string()));
        }

        let resp = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&params)
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
                message: format!("brave returned {status}: {}", truncate(&body, 512)),
            });
        }

        let parsed: BraveResponse = resp.json().await.map_err(|e| SearchError::Transport {
            message: format!("brave returned non-JSON body: {e}"),
        })?;

        let mut hits: Vec<SearchHit> = parsed
            .web
            .map(|w| w.results)
            .unwrap_or_default()
            .into_iter()
            .map(|r| SearchHit {
                title: r.title,
                url: r.url,
                snippet: r.description.unwrap_or_default(),
                published_at: r.age,
            })
            .collect();

        // Defense-in-depth: if query rewriting was bypassed (e.g. URL has no
        // recognizable host), drop hits whose host is in `blocked_domains`.
        if !opts.blocked_domains.is_empty() {
            hits.retain(|h| {
                let host = host_of(&h.url).unwrap_or_default();
                let host = host.strip_prefix("www.").unwrap_or(&host);
                !opts.blocked_domains.iter().any(|d| d == host)
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

/// Extract host (without `www.` prefix) from a URL string. Pure string ops so
/// we don't pull in the `url` crate.
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

// --- Response shapes (subset) -----------------------------------------------

#[derive(Debug, Deserialize)]
struct BraveResponse {
    #[serde(default)]
    web: Option<BraveWeb>,
}

#[derive(Debug, Deserialize)]
struct BraveWeb {
    #[serde(default)]
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    age: Option<String>,
}

// --- Tests -------------------------------------------------------------------

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
        let backend = BraveSearchBackend::new("");
        let err = backend.search("rust", &dummy_opts()).await.unwrap_err();
        match err {
            SearchError::InvalidConfig { message } => {
                assert!(message.contains("api key"));
            }
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let s = "中文测试 字符串";
        let t = truncate(s, 4);
        assert!(t.is_char_boundary(t.len()));
    }

    #[test]
    fn host_of_handles_common_shapes() {
        assert_eq!(
            host_of("https://example.com/path"),
            Some("example.com".into())
        );
        assert_eq!(
            host_of("https://www.example.com/"),
            Some("example.com".into())
        );
        assert_eq!(
            host_of("https://api.search.brave.com/res/v1/web/search?q=hi"),
            Some("api.search.brave.com".into())
        );
        assert_eq!(
            host_of("https://example.com:8080/x"),
            Some("example.com".into())
        );
        assert_eq!(host_of("not a url"), None);
    }
}
