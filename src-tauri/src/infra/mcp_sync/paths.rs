//! Usage: Resolve MCP config paths and sync storage paths.

use crate::app_paths;
use crate::codex_paths;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

pub(super) fn mcp_target_path(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;

    match cli_key {
        // cc-switch: Claude MCP uses ~/.claude.json
        "claude" => Ok(home.join(".claude.json")),
        // cc-switch: Codex MCP uses $CODEX_HOME/config.toml (default: ~/.codex/config.toml)
        "codex" => codex_paths::codex_config_toml_path(app),
        // cc-switch: Gemini MCP uses ~/.gemini/settings.json
        "gemini" => Ok(home.join(".gemini").join("settings.json")),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

pub(super) fn backup_file_name(cli_key: &str) -> &'static str {
    match cli_key {
        "claude" => "claude.json",
        "codex" => "config.toml",
        "gemini" => "settings.json",
        _ => "config",
    }
}

pub(super) fn mcp_sync_root_dir(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join("mcp-sync").join(cli_key))
}

pub(super) fn mcp_sync_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

pub(super) fn mcp_sync_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

pub(super) fn legacy_mcp_sync_roots(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<Vec<PathBuf>, String> {
    let home = home_dir(app)?;
    Ok(super::LEGACY_APP_DOTDIR_NAMES
        .iter()
        .map(|dir_name| home.join(dir_name).join("mcp-sync").join(cli_key))
        .collect())
}
