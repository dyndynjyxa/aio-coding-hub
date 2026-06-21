//! Usage: Domain modules (business concepts and use-cases).
//!
//! Note: Some modules may still call infra helpers during the migration; Phase 3 focuses on
//! physical structure + stable API boundaries first.

pub(crate) mod claude_model_validation;
pub(crate) mod claude_model_validation_history;
pub(crate) mod claude_plugins;
pub(crate) mod cli_sessions;
pub(crate) mod cost;
pub(crate) mod cost_stats;
pub(crate) mod mcp;
pub(crate) mod plugins;
pub(crate) mod prompts;
pub(crate) mod provider_availability;
pub(crate) mod provider_limit_usage;
pub(crate) mod provider_oauth_limits;
pub(crate) mod providers;
pub(crate) mod skills;
pub(crate) mod sort_modes;
pub(crate) mod usage;
pub(crate) mod usage_stats;
pub(crate) mod workspace_switch;
pub(crate) mod workspaces;
