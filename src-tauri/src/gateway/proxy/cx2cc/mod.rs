//! CX2CC (Codex-to-Claude-Code) translation layer.
//!
//! Translates between Anthropic Messages API and OpenAI Responses API,
//! enabling Claude Code CLI to use Codex/OpenAI providers transparently.

pub mod models;
pub mod streaming;
pub mod transform;
