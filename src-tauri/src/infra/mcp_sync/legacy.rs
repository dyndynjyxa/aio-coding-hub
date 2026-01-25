//! Usage: Best-effort migration of legacy MCP sync directories.

use super::fs::{copy_dir_recursive_if_missing, copy_file_if_missing};
use super::paths::{
    legacy_mcp_sync_roots, mcp_sync_files_dir, mcp_sync_manifest_path, mcp_sync_root_dir,
};

pub(super) fn try_migrate_legacy_mcp_sync_dir(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<bool, String> {
    let new_root = mcp_sync_root_dir(app, cli_key)?;
    let new_manifest_path = mcp_sync_manifest_path(&new_root);
    if new_manifest_path.exists() {
        return Ok(false);
    }

    for legacy_root in legacy_mcp_sync_roots(app, cli_key)? {
        let legacy_manifest_path = mcp_sync_manifest_path(&legacy_root);
        if !legacy_manifest_path.exists() {
            continue;
        }

        std::fs::create_dir_all(&new_root)
            .map_err(|e| format!("failed to create {}: {e}", new_root.display()))?;

        let _ = copy_file_if_missing(&legacy_manifest_path, &new_manifest_path)?;

        let legacy_files_dir = mcp_sync_files_dir(&legacy_root);
        if legacy_files_dir.exists() {
            let new_files_dir = mcp_sync_files_dir(&new_root);
            copy_dir_recursive_if_missing(&legacy_files_dir, &new_files_dir)?;
        }

        return Ok(true);
    }

    Ok(false)
}
