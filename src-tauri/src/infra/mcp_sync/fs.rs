//! Usage: Filesystem helpers for MCP sync operations.

use super::paths;

pub(super) use crate::shared::fs::{
    copy_dir_recursive_if_missing, copy_file_if_missing, read_file_with_max_len,
    read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed,
};

pub fn read_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> Result<Option<Vec<u8>>, String> {
    let path = paths::mcp_target_path(app, cli_key)?;
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
    match bytes {
        Some(content) => {
            if content.len() > super::MCP_SYNC_TARGET_MAX_BYTES {
                return Err(format!(
                    "SEC_INVALID_INPUT: MCP sync target restore too large (max {} bytes)",
                    super::MCP_SYNC_TARGET_MAX_BYTES
                ));
            }
            Ok(write_file_atomic(&path, &content)?)
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
