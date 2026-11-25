//! Usage: Filesystem helpers for MCP sync operations.

use super::paths;

pub(super) use crate::shared::fs::{
    copy_dir_recursive_if_missing, copy_file_if_missing, read_file_with_max_len,
    read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed,
};

pub(super) fn write_config_file(
    cli_key: &str,
    path: &std::path::Path,
    bytes: &[u8],
) -> crate::shared::error::AppResult<()> {
    if cli_key != "gemini" {
        write_file_atomic_if_changed(path, bytes)?;
        return Ok(());
    }
    if crate::shared::fs::regular_file_metadata(path)?
        .is_some_and(|m| m.len() == bytes.len() as u64)
        && read_file_with_max_len(path, super::MCP_SYNC_TARGET_MAX_BYTES)? == bytes
    {
        return Ok(());
    }
    crate::shared::fs::write_private_file_atomic(path, bytes)
}

pub fn read_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> Result<Option<Vec<u8>>, String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    if cli_key == "grok" {
        return Ok(crate::grok_config::read_bytes_path(&path)?);
    }
    if cli_key == "gemini" {
        crate::shared::fs::regular_file_metadata(&path)?;
    }
    Ok(read_optional_file_with_max_len(
        &path,
        super::MCP_SYNC_TARGET_MAX_BYTES,
    )?)
}

pub fn restore_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    if cli_key == "grok" {
        return Ok(crate::grok_config::restore_bytes_path(&path, bytes)?);
    }
    if cli_key == "gemini" {
        crate::shared::fs::regular_file_metadata(&path)?;
    }
    match bytes {
        Some(content) => {
            if content.len() > super::MCP_SYNC_TARGET_MAX_BYTES {
                return Err(format!(
                    "SEC_INVALID_INPUT: MCP sync target restore too large (max {} bytes)",
                    super::MCP_SYNC_TARGET_MAX_BYTES
                ));
            }
            Ok(write_config_file(cli_key, &path, &content)?)
        }
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}
