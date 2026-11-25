//! WSL MCP sync: sync MCP server configurations to WSL distros.

use crate::mcp_sync::McpServerForSync;
use crate::shared::error::AppResult;
use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};

use super::shell::{
    bash_single_quote, read_wsl_file_with_max_len, remove_wsl_file, run_wsl_bash_script_capture,
    write_wsl_file, wsl_resolve_codex_home_script,
};

const WSL_MCP_SYNC_TARGET_MAX_BYTES: usize = 1024 * 1024;
const WSL_MCP_SYNC_MANIFEST_MAX_BYTES: usize = 256 * 1024;

fn ensure_wsl_mcp_sync_bytes_within_limit(
    bytes: &[u8],
    max_len: usize,
    label: &str,
) -> AppResult<()> {
    if bytes.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {label} too large (max {max_len} bytes)").into());
    }
    Ok(())
}

// ── WSL MCP manifest ──

/// Tracks which MCP server keys were synced to a WSL distro for a specific CLI,
/// so we can properly remove them on the next sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslMcpManifest {
    distro: String,
    cli_key: String,
    managed_keys: Vec<String>,
    updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

fn wsl_mcp_manifest_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    let dir = crate::app_paths::app_data_dir(app)?
        .join("wsl-mcp-sync")
        .join(distro)
        .join(cli_key);
    Ok(dir.join("manifest.json"))
}

// ── WSL MCP sync ──

#[derive(Debug, PartialEq, Eq)]
pub(super) struct WslMcpSyncResult {
    pub(super) count: usize,
    pub(super) legacy_cleanup_skipped: bool,
}

/// Sync MCP configuration for a single CLI to a WSL distro.
/// Uses the existing `build_next_bytes` to merge servers into the config.
pub(super) fn sync_wsl_mcp_for_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
    cli_key: &str,
    servers: &[McpServerForSync],
) -> AppResult<Option<WslMcpSyncResult>> {
    if !matches!(cli_key, "claude" | "codex" | "gemini") {
        return Err(format!("unknown cli_key: {cli_key}").into());
    }
    let manifest_path = wsl_mcp_manifest_path(app, distro, cli_key)?;
    let record = read_optional_file_with_max_len(&manifest_path, WSL_MCP_SYNC_MANIFEST_MAX_BYTES)
        .and_then(|raw| {
            raw.map(|bytes| {
                let manifest: WslMcpManifest = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("WSL MCP manifest is invalid: {e}"))?;
                Ok((bytes, manifest))
            })
            .transpose()
        });
    // Only Gemini needs an intact ownership journal to migrate between two files.
    let record = if cli_key == "gemini" {
        record?
    } else {
        record.unwrap_or(None)
    };
    let previous = record.as_ref().map(|(_, m)| m);
    let owned = previous.map_or(&[][..], |m| m.managed_keys.as_slice());
    if owned.is_empty() && servers.is_empty() {
        return Ok(None);
    }

    // A pre-Antigravity manifest owns keys in settings.json, never the new file.
    let migrating = cli_key == "gemini" && previous.is_some_and(|m| m.target.is_none());
    let managed = if migrating { &[][..] } else { owned };

    // Resolve the config file path inside WSL (handles CODEX_HOME, etc.)
    let resolve_script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
{resolver}
case {cli_key} in
  claude) echo "$HOME/.claude.json" ;;
  codex) echo "$p/config.toml" ;;
  gemini)
    agy_mcp="$HOME/.gemini/config/mcp_config.json"
    if [ -L "$agy_mcp" ] || {{ [ -e "$agy_mcp" ] && [ ! -f "$agy_mcp" ]; }}; then
      echo "WSL MCP: config must be a regular file" >&2
      exit 1
    fi
    old="$HOME/.gemini/settings.json"
    if [ "{migrate}" = true ] && {{ [ -L "$old" ] || {{ [ -e "$old" ] && [ ! -f "$old" ]; }}; }}; then
      echo AIO_WSL_LEGACY_UNSAFE
    fi
    echo "$agy_mcp" ;;

esac
"#,
        resolver = wsl_resolve_codex_home_script("p"),
        migrate = migrating && !owned.is_empty(),
        cli_key = bash_single_quote(cli_key)
    );

    let resolved_path = run_wsl_bash_script_capture(distro, &resolve_script)?;
    let (mut legacy_cleanup_skipped, resolved_path) =
        match resolved_path.trim().strip_prefix("AIO_WSL_LEGACY_UNSAFE\n") {
            Some(path) => (true, path),
            None => (false, resolved_path.trim()),
        };

    let current = read_wsl_file_with_max_len(distro, resolved_path, WSL_MCP_SYNC_TARGET_MAX_BYTES)?;
    let (next, keys) = if cli_key == "gemini" {
        let merge = crate::mcp_sync::merge_antigravity_mcp(current.clone(), managed, servers)?;
        (merge.bytes, merge.managed_keys)
    } else {
        let next = crate::mcp_sync::build_next_bytes(cli_key, current.clone(), managed, servers)
            .map_err(|e| format!("WSL MCP build failed for {cli_key}: {e}"))?;
        let mut keys: Vec<_> = servers.iter().map(|s| s.server_key.clone()).collect();
        keys.sort();
        keys.dedup();
        (next, keys)
    };
    ensure_wsl_mcp_sync_bytes_within_limit(&next, WSL_MCP_SYNC_TARGET_MAX_BYTES, "WSL MCP target")?;

    let mut legacy = if migrating && !owned.is_empty() && !legacy_cleanup_skipped {
        let prefix = resolved_path
            .strip_suffix("/config/mcp_config.json")
            .ok_or("WSL MCP: unexpected Antigravity target")?;
        let path = format!("{prefix}/settings.json");
        let cleanup = read_wsl_file_with_max_len(distro, &path, WSL_MCP_SYNC_TARGET_MAX_BYTES)
            .and_then(|bytes| {
                if let Some(old) = bytes {
                    Ok(crate::mcp_sync::release_legacy_entries(&old, None, owned)?
                        .map(|next| (path, old, next)))
                } else {
                    Ok(None)
                }
            });
        match cleanup {
            Ok(plan) => plan,
            Err(_) => {
                legacy_cleanup_skipped = true;
                None
            }
        }
    } else {
        None
    };
    let manifest = WslMcpManifest {
        distro: distro.into(),
        cli_key: cli_key.into(),
        managed_keys: keys,
        updated_at: crate::shared::time::now_unix_seconds(),
        target: (cli_key == "gemini").then(|| "antigravity".into()),
    };
    let json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("WSL MCP manifest: {e}"))?;
    ensure_wsl_mcp_sync_bytes_within_limit(
        &json,
        WSL_MCP_SYNC_MANIFEST_MAX_BYTES,
        "WSL MCP manifest",
    )?;

    let write_target =
        (!migrating || !servers.is_empty()) && current.as_deref() != Some(next.as_slice());
    if migrating {
        // Release the old keys first. A restart before switching the manifest
        // can retry that cleanup. Once it switches, the next sync owns the new keys.
        if let Some((path, _, bytes)) = &legacy {
            if write_wsl_file(distro, path, bytes).is_err() {
                legacy_cleanup_skipped = true;
                legacy = None;
            }
        }
        let apply = || -> AppResult<()> {
            write_file_atomic(&manifest_path, &json)?;
            if write_target {
                write_wsl_file(distro, resolved_path, &next)?;
            }
            Ok(())
        };
        if let Err(error) = apply() {
            if let Some((path, old, _)) = &legacy {
                write_wsl_file(distro, path, old)?;
            }
            if let Some((original, _)) = &record {
                write_file_atomic(&manifest_path, original)?;
            }
            return Err(error);
        }
    } else {
        if write_target {
            write_wsl_file(distro, resolved_path, &next)?;
        }
        if let Err(error) = write_file_atomic(&manifest_path, &json) {
            if write_target {
                match current {
                    Some(bytes) => write_wsl_file(distro, resolved_path, &bytes)?,
                    None => remove_wsl_file(distro, resolved_path)?,
                }
            }
            return Err(error);
        }
    }
    Ok(Some(WslMcpSyncResult {
        count: manifest.managed_keys.len(),
        legacy_cleanup_skipped,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_mcp_sync_bytes_limit_rejects_oversized_content() {
        let bytes = vec![b'x'; WSL_MCP_SYNC_TARGET_MAX_BYTES + 1];

        let err = ensure_wsl_mcp_sync_bytes_within_limit(
            &bytes,
            WSL_MCP_SYNC_TARGET_MAX_BYTES,
            "WSL MCP target",
        )
        .expect_err("oversized WSL MCP content should fail");

        assert!(err.to_string().contains("too large"));
    }
}
