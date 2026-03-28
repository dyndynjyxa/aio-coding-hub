//! Usage: Tauri IPC command modules.
//!
//! This layer is the stable interface called by the frontend via `invoke()`.
//! Command names / args / return JSON shapes are considered a frozen contract.

pub(crate) mod app;
pub(crate) mod claude_model_validation;
pub(crate) mod cli_manager;
pub(crate) mod cli_proxy;
pub(crate) mod cli_sessions;
pub(crate) mod cli_update;
pub(crate) mod config_migrate;
pub(crate) mod cost;
pub(crate) mod data_management;
pub(crate) mod env_conflicts;
pub(crate) mod gateway;
pub(crate) mod limit;
pub(crate) mod mcp;
pub(crate) mod model_prices;
pub(crate) mod notice;
pub(crate) mod prompts;
pub(crate) mod provider_limit_usage;
pub(crate) mod providers;
pub(crate) mod request_logs;
pub(crate) mod settings;
pub(crate) mod skills;
pub(crate) mod sort_modes;
pub(crate) mod usage;
pub(crate) mod workspaces;
pub(crate) mod wsl;

pub(crate) use app::*;
pub(crate) use claude_model_validation::*;
pub(crate) use cli_manager::*;
pub(crate) use cli_proxy::*;
pub(crate) use cli_sessions::*;
pub(crate) use cli_update::*;
pub(crate) use config_migrate::*;
pub(crate) use cost::*;
pub(crate) use data_management::*;
pub(crate) use env_conflicts::*;
pub(crate) use gateway::*;
pub(crate) use mcp::*;
pub(crate) use model_prices::*;
pub(crate) use notice::*;
pub(crate) use prompts::*;
pub(crate) use provider_limit_usage::*;
pub(crate) use providers::*;
pub(crate) use request_logs::*;
pub(crate) use settings::*;
pub(crate) use skills::*;
pub(crate) use sort_modes::*;
pub(crate) use usage::*;
pub(crate) use workspaces::*;
pub(crate) use wsl::*;
