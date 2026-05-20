//! Usage: MCP sync manifest persistence (backup/restore).

use crate::shared::time::now_unix_seconds;
use serde::{Deserialize, Serialize};

use super::fs::{read_file_with_max_len, read_optional_file_with_max_len, write_file_atomic};
use super::legacy::try_migrate_legacy_mcp_sync_dir;
use super::paths::{
    backup_file_name, mcp_sync_files_dir, mcp_sync_manifest_path, mcp_sync_root_dir,
    mcp_target_path,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct McpSyncFileEntry {
    pub(super) path: String,
    pub(super) existed: bool,
    pub(super) backup_rel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct McpSyncManifest {
    pub(super) schema_version: u32,
    pub(super) managed_by: String,
    pub(super) cli_key: String,
    pub(super) enabled: bool,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) file: McpSyncFileEntry,
    pub(super) managed_keys: Vec<String>,
}

pub fn read_manifest_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);
    read_optional_file_with_max_len(&path, super::MCP_SYNC_MANIFEST_MAX_BYTES)
}

pub fn restore_manifest_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);
    match bytes {
        Some(content) => {
            if content.len() > super::MCP_SYNC_MANIFEST_MAX_BYTES {
                return Err(format!(
                    "SEC_INVALID_INPUT: MCP sync manifest restore too large (max {} bytes)",
                    super::MCP_SYNC_MANIFEST_MAX_BYTES
                )
                .into());
            }
            write_file_atomic(&path, &content)?;
            Ok(())
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

pub(super) fn read_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<McpSyncManifest>> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);

    if !path.exists() {
        if let Err(err) = try_migrate_legacy_mcp_sync_dir(app, cli_key) {
            tracing::warn!("MCP sync migration failed: {}", err);
        }
    }

    let Some(content) = read_optional_file_with_max_len(&path, super::MCP_SYNC_MANIFEST_MAX_BYTES)?
    else {
        return Ok(None);
    };

    let manifest: McpSyncManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse mcp manifest.json: {e}"))?;

    if manifest.managed_by != super::MANAGED_BY {
        return Err(format!(
            "mcp manifest managed_by mismatch: expected {}, got {}",
            super::MANAGED_BY,
            manifest.managed_by
        )
        .into());
    }

    Ok(Some(manifest))
}

pub(super) fn write_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    manifest: &McpSyncManifest,
) -> crate::shared::error::AppResult<()> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = mcp_sync_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize mcp manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

pub(super) fn backup_for_enable<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    existing: Option<McpSyncManifest>,
) -> crate::shared::error::AppResult<McpSyncManifest> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let files_dir = mcp_sync_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let target_path = mcp_target_path(app, cli_key)?;
    let now = now_unix_seconds();

    let existed = target_path.exists();
    let backup_rel = if existed {
        let bytes = read_file_with_max_len(&target_path, super::MCP_SYNC_TARGET_MAX_BYTES)?;
        let backup_name = backup_file_name(cli_key);
        let backup_path = files_dir.join(backup_name);
        write_file_atomic(&backup_path, &bytes)?;
        Some(backup_name.to_string())
    } else {
        None
    };

    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);

    Ok(McpSyncManifest {
        schema_version: super::MANIFEST_SCHEMA_VERSION,
        managed_by: super::MANAGED_BY.to_string(),
        cli_key: cli_key.to_string(),
        enabled: true,
        created_at,
        updated_at: now,
        file: McpSyncFileEntry {
            path: target_path.to_string_lossy().to_string(),
            existed,
            backup_rel,
        },
        managed_keys: Vec::new(),
    })
}
