//! Web search interception subsystem.
//!
//! Detect Anthropic-format requests that originate from Claude Code's
//! internal `WebSearchTool` and either short-circuit them with a synthesized
//! SSE response (when `force_replace` is enabled) or let them passthrough
//! to the upstream provider unchanged.
//!
//! The detection logic and SSE synthesizer live in submodules so they can be
//! unit-tested in isolation.

pub mod backend;
pub mod backends;
pub mod detection;
pub mod factory;
pub mod sse;
