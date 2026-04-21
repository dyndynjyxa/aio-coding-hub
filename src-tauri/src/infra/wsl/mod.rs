//! Usage: Windows WSL detection and per-distro client configuration helpers.

mod config_claude;
mod config_codex;
mod config_gemini;
mod constants;
mod data_gathering;
mod detection;
mod manifest;
mod mcp_adapt;
mod mcp_sync;
mod prompt_sync;
mod shell;
mod skills_sync;
mod status;
mod types;

#[cfg(test)]
#[path = "tests.rs"]
mod tests_mod;

// Re-export crate-facing entry points used outside this module.

pub use detection::{
    detect, host_ipv4_best_effort, resolve_wsl_home_unc, resolve_wsl_host, validate_distro,
};
pub use manifest::{restore_wsl_clients, startup_repair_wsl_manifests};
pub use status::{configure_clients, get_config_status};
pub use types::{
    WslConfigureReport, WslDetection, WslDistroConfigStatus, WslMcpSyncData, WslPromptSyncData,
    WslSkillsSyncData,
};

pub use data_gathering::{gather_mcp_sync_data, gather_prompt_sync_data, gather_skills_sync_data};
