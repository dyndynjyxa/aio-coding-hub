//! Usage: Resolve MCP config paths and sync storage paths.

use crate::app_paths;
use crate::codex_paths;
use std::path::{Path, PathBuf};

pub(super) fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

pub(super) fn mcp_target_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;

    match cli_key {
        "claude" => Ok(home.join(".claude.json")),
        "codex" => codex_paths::codex_config_toml_path(app),
        "gemini" => Ok(home.join(".gemini").join("settings.json")),
        "grok" => crate::grok_config::config_path(app),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
    }
}

pub(super) fn backup_file_name(cli_key: &str) -> &'static str {
    match cli_key {
        "claude" => "claude.json",
        "codex" => "config.toml",
        "gemini" => "settings.json",
        "grok" => "config.toml",
        _ => "config",
    }
}

pub(super) fn mcp_sync_root_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join("mcp-sync").join(cli_key))
}

pub(super) fn mcp_sync_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

pub(super) fn mcp_sync_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

pub(super) fn legacy_mcp_sync_roots<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<PathBuf>> {
    let home = home_dir(app)?;
    Ok(super::LEGACY_APP_DOTDIR_NAMES
        .iter()
        .map(|dir_name| home.join(dir_name).join("mcp-sync").join(cli_key))
        .collect())
}
