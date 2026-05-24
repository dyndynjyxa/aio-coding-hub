//! Usage: Apply managed MCP server config to supported CLIs.

use crate::shared::time::now_unix_seconds;
use std::collections::HashSet;

use super::claude_json::build_claude_config_json;
use super::codex_toml::build_codex_config_toml;
use super::fs::{read_optional_file_with_max_len, write_file_atomic_if_changed};
use super::gemini_json::build_gemini_settings_json;
use super::manifest::{backup_for_enable, read_manifest, write_manifest};
use super::paths::{mcp_target_path, validate_cli_key};
use super::McpServerForSync;

pub(crate) fn build_next_bytes(
    cli_key: &str,
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    match cli_key {
        "claude" => build_claude_config_json(current, managed_keys, servers),
        "codex" => build_codex_config_toml(current, managed_keys, servers),
        "gemini" => build_gemini_settings_json(current, managed_keys, servers),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn normalized_keys(servers: &[McpServerForSync]) -> Vec<String> {
    let mut keys: Vec<String> = servers.iter().map(|s| s.server_key.to_string()).collect();
    keys.sort();
    keys.dedup();
    keys
}

pub fn sync_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);

    let desired_keys = normalized_keys(servers);

    let mut manifest = match if should_backup {
        backup_for_enable(app, cli_key, existing.clone())
    } else {
        Ok(existing.unwrap())
    } {
        Ok(m) => m,
        Err(err) => return Err(format!("MCP_SYNC_BACKUP_FAILED: {err}")),
    };

    if should_backup {
        // Persist snapshot before applying changes so we can restore on failure.
        manifest.enabled = false;
        manifest.managed_keys.clear();
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, cli_key, &manifest)?;
    }

    let target_path = mcp_target_path(app, cli_key)?;
    manifest.file.path = target_path.to_string_lossy().to_string();

    let current = read_optional_file_with_max_len(&target_path, super::MCP_SYNC_TARGET_MAX_BYTES)?;
    let managed_keys = manifest.managed_keys.clone();

    let next_bytes = build_next_bytes(cli_key, current, &managed_keys, servers)?;

    write_file_atomic_if_changed(&target_path, &next_bytes)?;

    manifest.enabled = true;
    manifest.managed_keys = desired_keys;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    // Best-effort: sanity check to avoid duplicated keys in manifest.
    let set: HashSet<String> = manifest.managed_keys.iter().cloned().collect();
    if set.len() != manifest.managed_keys.len() {
        tracing::warn!(cli_key = %cli_key, "MCP sync: duplicate entries in managed_keys");
    }

    Ok(())
}
