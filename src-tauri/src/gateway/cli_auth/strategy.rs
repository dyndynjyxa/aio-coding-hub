//! Trait definition for CLI-specific API-key authentication injection.

use axum::http::HeaderMap;

/// Strategy for injecting API-key authentication headers for a specific CLI.
///
/// Each CLI (Claude, Codex, Gemini) has its own header conventions.
/// Implementations are registered in [`super::registry::CliAuthRegistry`].
pub(crate) trait CliAuthStrategy: Send + Sync {
    /// The CLI key string this strategy handles (e.g. `"claude"`, `"codex"`, `"gemini"`).
    fn cli_key_str(&self) -> &'static str;

    /// Inject API-key authentication headers into the given header map.
    ///
    /// Called **after** `clear_all_auth_headers` has already stripped all
    /// existing auth headers (fail-closed pattern).
    fn inject_api_key_auth(&self, headers: &mut HeaderMap, api_key: &str);

    /// Ensure CLI-required headers are present (e.g. `anthropic-version` for Claude).
    ///
    /// Called independently of `inject_api_key_auth` to fix up headers that
    /// may be needed even when auth was injected through a different path
    /// (e.g. OAuth).
    fn ensure_required_headers(&self, headers: &mut HeaderMap);
}
