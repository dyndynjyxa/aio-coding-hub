//! Usage: Filesystem helpers for MCP sync operations.

use super::paths;

pub(super) use crate::shared::fs::{
    copy_dir_recursive_if_missing, copy_file_if_missing, read_optional_file, write_file_atomic,
    write_file_atomic_if_changed,
};

pub fn read_target_bytes(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<Vec<u8>>, String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    read_optional_file(&path)
}

pub fn restore_target_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    match bytes {
        Some(content) => write_file_atomic(&path, &content),
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}
