//! Shared types for the WSL module.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WslDetection {
    pub detected: bool,
    pub distros: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WslDistroConfigStatus {
    pub distro: String,
    pub claude: bool,
    pub codex: bool,
    pub gemini: bool,
    pub claude_mcp: bool,
    pub codex_mcp: bool,
    pub gemini_mcp: bool,
    pub claude_prompt: bool,
    pub codex_prompt: bool,
    pub gemini_prompt: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WslConfigureCliReport {
    pub cli_key: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WslConfigureDistroReport {
    pub distro: String,
    pub ok: bool,
    pub results: Vec<WslConfigureCliReport>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WslConfigureReport {
    pub ok: bool,
    pub message: String,
    pub distros: Vec<WslConfigureDistroReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslCliBackup {
    pub cli_key: String,
    /// Keys injected by AIO and the values written.
    pub injected_keys: std::collections::HashMap<String, String>,
    /// Original values before injection. `None` means key did not exist.
    pub original_values: std::collections::HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslDistroManifest {
    pub schema_version: u32,
    pub distro: String,
    pub configured: bool,
    pub proxy_origin: String,
    pub configured_at: i64,
    /// Cached UNC path to WSL home dir, so restore doesn't need to spawn wsl.exe.
    #[serde(default)]
    pub wsl_home_unc: Option<String>,
    pub cli_backups: Vec<WslCliBackup>,
}

/// MCP sync data for all CLIs, used when syncing to WSL.
pub struct WslMcpSyncData {
    pub claude: Vec<crate::mcp_sync::McpServerForSync>,
    pub codex: Vec<crate::mcp_sync::McpServerForSync>,
    pub gemini: Vec<crate::mcp_sync::McpServerForSync>,
}

/// Prompt sync data for all CLIs, used when syncing to WSL.
pub struct WslPromptSyncData {
    pub claude_content: Option<String>,
    pub codex_content: Option<String>,
    pub gemini_content: Option<String>,
}

/// One synced skill file inside a skill directory.
pub struct WslSkillFileSyncEntry {
    pub relative_path: String,
    pub content: Vec<u8>,
}

/// One synced skill directory.
pub struct WslSkillSyncEntry {
    pub skill_key: String,
    pub files: Vec<WslSkillFileSyncEntry>,
}

/// Skills sync data for all CLIs, used when syncing to WSL.
pub struct WslSkillsSyncData {
    pub claude: Vec<WslSkillSyncEntry>,
    pub codex: Vec<WslSkillSyncEntry>,
    pub gemini: Vec<WslSkillSyncEntry>,
}
