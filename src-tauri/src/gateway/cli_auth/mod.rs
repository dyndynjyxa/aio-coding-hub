//! CLI-specific API-key authentication strategies.
//!
//! Each supported CLI (Claude, Codex, Gemini) implements [`CliAuthStrategy`]
//! to define how API-key credentials are injected into upstream request headers.
//! The global [`CliAuthRegistry`] provides a single lookup point used by
//! `inject_provider_auth` and `ensure_cli_required_headers` in `util.rs`.

mod claude;
mod codex;
mod gemini;
mod registry;
mod strategy;

pub(crate) use registry::global_cli_auth_registry;
// Re-export the trait for use by future callers (e.g. CliAuthStrategy in H1).
#[allow(unused_imports)]
pub(crate) use strategy::CliAuthStrategy;
