//! WSL prompt sync: sync prompt files to WSL distros.

use crate::prompt_sync;
use crate::shared::error::AppResult;
use crate::shared::fs::{
    read_file_with_max_len, read_optional_file_with_max_len, write_file_atomic,
};

use super::shell::{
    bash_single_quote, read_wsl_file_with_max_len, remove_wsl_file, run_wsl_bash_script_capture,
    write_wsl_file, wsl_resolve_codex_home_script,
};

const WSL_PROMPT_MANIFEST_SCHEMA_VERSION: u32 = 1;
const WSL_PROMPT_MANAGED_BY: &str = "aio-coding-hub";
const WSL_PROMPT_TARGET_MAX_BYTES: usize = 1024 * 1024;
const WSL_PROMPT_MANIFEST_MAX_BYTES: usize = 256 * 1024;

fn ensure_wsl_prompt_sync_bytes_within_limit(
    bytes: &[u8],
    max_len: usize,
    label: &str,
) -> AppResult<()> {
    if bytes.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {label} too large (max {max_len} bytes)").into());
    }
    Ok(())
}

// ── WSL prompt manifest ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslPromptSyncFileEntry {
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslPromptManifest {
    schema_version: u32,
    managed_by: String,
    distro: String,
    cli_key: String,
    enabled: bool,
    created_at: i64,
    updated_at: i64,
    file: WslPromptSyncFileEntry,
}

fn wsl_prompt_sync_root_dir(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    Ok(crate::app_paths::app_data_dir(app)?
        .join("wsl-prompt-sync")
        .join(distro)
        .join(cli_key))
}

fn wsl_prompt_files_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("files")
}

fn wsl_prompt_manifest_path(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    Ok(wsl_prompt_sync_root_dir(app, distro, cli_key)?.join("manifest.json"))
}

fn read_wsl_prompt_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<Option<WslPromptManifest>> {
    let path = wsl_prompt_manifest_path(app, distro, cli_key)?;
    let bytes = match read_optional_file_with_max_len(&path, WSL_PROMPT_MANIFEST_MAX_BYTES)? {
        Some(content) => content,
        None => return Ok(None),
    };

    let manifest: WslPromptManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("failed to parse WSL prompt manifest: {e}"))?;
    if manifest.managed_by != WSL_PROMPT_MANAGED_BY {
        return Err(format!(
            "WSL prompt manifest managed_by mismatch: expected {WSL_PROMPT_MANAGED_BY}, got {}",
            manifest.managed_by
        )
        .into());
    }
    Ok(Some(manifest))
}

fn write_wsl_prompt_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    manifest: &WslPromptManifest,
) -> AppResult<()> {
    let root = wsl_prompt_sync_root_dir(app, distro, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create WSL prompt sync dir: {e}"))?;
    let path = wsl_prompt_manifest_path(app, distro, cli_key)?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize WSL prompt manifest: {e}"))?;
    ensure_wsl_prompt_sync_bytes_within_limit(
        json.as_bytes(),
        WSL_PROMPT_MANIFEST_MAX_BYTES,
        "WSL prompt manifest",
    )?;
    write_file_atomic(&path, json.as_bytes())?;
    Ok(())
}

/// Resolve the prompt target file path inside a WSL distro.
/// Returns an absolute path like `/home/user/.claude/CLAUDE.md`.
fn resolve_wsl_prompt_path(distro: &str, cli_key: &str) -> AppResult<String> {
    let resolve_script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
{resolver}
case {cli_key} in
  claude) echo "$HOME/.claude/CLAUDE.md" ;;
  codex) echo "$p/AGENTS.md" ;;
  gemini) echo "$HOME/.gemini/GEMINI.md" ;;
esac
"#,
        resolver = wsl_resolve_codex_home_script("p"),
        cli_key = bash_single_quote(cli_key)
    );
    let resolved = run_wsl_bash_script_capture(distro, &resolve_script)?;
    let resolved = resolved.trim().to_string();
    if resolved.is_empty() || !resolved.starts_with('/') {
        return Err(format!("failed to resolve prompt path for {cli_key}: {resolved}").into());
    }
    Ok(resolved)
}

/// Sync a prompt file for a single CLI to a WSL distro.
fn backup_wsl_prompt_for_enable(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    target_path: &str,
    existing: Option<WslPromptManifest>,
) -> AppResult<WslPromptManifest> {
    let root = wsl_prompt_sync_root_dir(app, distro, cli_key)?;
    let files_dir = wsl_prompt_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create WSL prompt files dir: {e}"))?;

    let existing_bytes =
        read_wsl_file_with_max_len(distro, target_path, WSL_PROMPT_TARGET_MAX_BYTES)?;
    let backup_rel = if let Some(bytes) = existing_bytes {
        let backup_name = std::path::Path::new(target_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("prompt.md")
            .to_string();
        let backup_path = files_dir.join(&backup_name);
        write_file_atomic(&backup_path, &bytes)?;
        Some(backup_name)
    } else {
        None
    };

    let now = crate::shared::time::now_unix_seconds();
    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);
    Ok(WslPromptManifest {
        schema_version: WSL_PROMPT_MANIFEST_SCHEMA_VERSION,
        managed_by: WSL_PROMPT_MANAGED_BY.to_string(),
        distro: distro.to_string(),
        cli_key: cli_key.to_string(),
        enabled: false,
        created_at,
        updated_at: now,
        file: WslPromptSyncFileEntry {
            path: target_path.to_string(),
            existed: backup_rel.is_some(),
            backup_rel,
        },
    })
}

fn restore_wsl_prompt_from_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    manifest: &WslPromptManifest,
) -> AppResult<()> {
    let target_path = manifest.file.path.as_str();
    if manifest.file.existed {
        let Some(backup_rel) = manifest.file.backup_rel.as_ref() else {
            return Err("WSL prompt restore backup missing".into());
        };
        let backup_root = wsl_prompt_files_dir(&wsl_prompt_sync_root_dir(
            app,
            &manifest.distro,
            &manifest.cli_key,
        )?);
        let backup_path = backup_root.join(backup_rel);
        let bytes = read_file_with_max_len(&backup_path, WSL_PROMPT_TARGET_MAX_BYTES)?;
        return write_wsl_file(distro, target_path, &bytes);
    }

    remove_wsl_file(distro, target_path)
}

pub(super) fn sync_wsl_prompt_for_cli(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    content: Option<&str>,
) -> AppResult<()> {
    if !matches!(cli_key, "claude" | "codex" | "gemini") {
        return Err(format!("unknown cli_key: {cli_key}").into());
    }

    // Resolve to an absolute path inside WSL (e.g. /home/user/.codex/AGENTS.md)
    // Must NOT pass $HOME as a literal string -- bash_single_quote would prevent expansion.
    let target_path = resolve_wsl_prompt_path(distro, cli_key)?;
    let trimmed = content.map(str::trim).filter(|value| !value.is_empty());
    let existing = read_wsl_prompt_manifest(app, distro, cli_key)?;

    match trimmed {
        Some(content) => {
            let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);
            let mut manifest = if should_backup {
                backup_wsl_prompt_for_enable(app, distro, cli_key, &target_path, existing.clone())?
            } else {
                existing.ok_or_else(|| "WSL prompt manifest missing while enabled".to_string())?
            };

            if should_backup {
                write_wsl_prompt_manifest(app, distro, cli_key, &manifest)?;
            }

            let bytes = prompt_sync::prompt_content_to_bytes(content);
            ensure_wsl_prompt_sync_bytes_within_limit(
                &bytes,
                WSL_PROMPT_TARGET_MAX_BYTES,
                "WSL prompt target",
            )?;
            write_wsl_file(distro, &target_path, &bytes)?;

            manifest.enabled = true;
            manifest.updated_at = crate::shared::time::now_unix_seconds();
            manifest.file.path = target_path;
            write_wsl_prompt_manifest(app, distro, cli_key, &manifest)
        }
        None => {
            let Some(mut manifest) = existing else {
                return Ok(());
            };
            if !manifest.enabled {
                return Ok(());
            }

            restore_wsl_prompt_from_manifest(app, distro, &manifest)?;
            manifest.enabled = false;
            manifest.updated_at = crate::shared::time::now_unix_seconds();
            write_wsl_prompt_manifest(app, distro, cli_key, &manifest)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_prompt_sync_bytes_limit_rejects_oversized_content() {
        let bytes = vec![b'x'; WSL_PROMPT_TARGET_MAX_BYTES + 1];

        let err = ensure_wsl_prompt_sync_bytes_within_limit(
            &bytes,
            WSL_PROMPT_TARGET_MAX_BYTES,
            "WSL prompt target",
        )
        .expect_err("oversized WSL prompt content should fail");

        assert!(err.to_string().contains("too large"));
    }
}
