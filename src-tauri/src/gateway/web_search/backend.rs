//! Web search backend abstraction.
//!
//! Defines a uniform interface that every pluggable search backend
//! (Brave, Tavily, SerpAPI, LLM-backed, etc.) must implement. The gateway
//! interceptor consumes this interface so new backends can be added without
//! touching the request pipeline.
//!
//! The codebase favors concrete enum dispatch over `dyn Trait` to avoid
//! pulling in `async-trait` or `BoxFuture` machinery. Backends are wired in
//! via [`SearchBackendImpl`] (an enum) and selected by
//! [`crate::gateway::web_search::factory::build_backend`].

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// A single search result returned by a backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub snippet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

/// Normalized search options that are independent of the underlying provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return. Backends should cap to this value.
    #[serde(default)]
    pub max_results: usize,
    /// Restrict results to these domains (Anthropic `allowed_domains`).
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Exclude these domains (Anthropic `blocked_domains`).
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    /// ISO language code, e.g. "zh-CN" / "en-US". Backend-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Region code, e.g. "US" / "CN". Backend-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// Search backend kinds, used in settings to drive factory selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchBackendKind {
    /// Built-in: queries the Brave Search Web API.
    #[default]
    Brave,
    /// Built-in: queries the Tavily Search API (LLM-friendly results).
    Tavily,
    /// Built-in: queries the Metaso Search API (LLM-friendly results with
    /// optional AI-generated summaries).
    Metaso,
    /// Recursive: invokes another configured LLM provider that natively
    /// supports `web_search_20250305`, then unwraps the search results.
    LlmBacked,
}

impl fmt::Display for SearchBackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Brave => f.write_str("brave"),
            Self::Tavily => f.write_str("tavily"),
            Self::Metaso => f.write_str("metaso"),
            Self::LlmBacked => f.write_str("llm_backed"),
        }
    }
}

/// Errors that backends can return. The `kind` tag lets the interceptor
/// decide whether to surface a `web_search_tool_result` error block (so the
/// model can react gracefully) or fail the request entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchError {
    /// Backend rejected the request before sending (e.g. missing API key).
    /// Surfaces as a `web_search_tool_result` error block in the SSE response.
    InvalidConfig { message: String },
    /// Upstream search provider returned an error response.
    Upstream { status: u16, message: String },
    /// Network or transport failure.
    Transport { message: String },
    /// Backend does not support one of the requested options.
    Unsupported { message: String },
    /// Catch-all.
    Other { message: String },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig { message }
            | Self::Upstream { message, .. }
            | Self::Transport { message }
            | Self::Unsupported { message }
            | Self::Other { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for SearchError {}

/// Per-call timeout for a search backend. Backends may override; this is a
/// reasonable default used by the interceptor and by built-in backends.
pub const DEFAULT_SEARCH_TIMEOUT: Duration = Duration::from_secs(15);

/// Concrete enum dispatching to one of the built-in backends.
///
/// New backends are added by:
///   1. Implementing the search methods in `crate::gateway::web_search::backends`.
///   2. Adding a variant here.
///   3. Wiring the variant in [`crate::gateway::web_search::factory::build_backend`].
#[derive(Debug, Clone)]
pub enum SearchBackendImpl {
    Brave(crate::gateway::web_search::backends::brave::BraveSearchBackend),
    Tavily(crate::gateway::web_search::backends::tavily::TavilySearchBackend),
    Metaso(crate::gateway::web_search::backends::metaso::MetasoSearchBackend),
    LlmBacked(crate::gateway::web_search::backends::llm_backed::LlmBackedSearchBackend),
}

impl SearchBackendImpl {
    /// Short stable tag used in special_settings_json (e.g. "brave" / "tavily").
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Brave(_) => "brave",
            Self::Tavily(_) => "tavily",
            Self::Metaso(_) => "metaso",
            Self::LlmBacked(_) => "llm_backed",
        }
    }
}
