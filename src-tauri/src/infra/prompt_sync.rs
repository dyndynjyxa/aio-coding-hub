//! Usage: Sync/backup/restore prompt instruction files for supported CLIs (infra adapter).

use crate::app_paths;
use crate::codex_paths;
use crate::shared::fs::{
    copy_dir_recursive_if_missing, copy_file_if_missing, read_optional_file, write_file_atomic,
    write_file_atomic_if_changed,
};
use crate::shared::time::now_unix_seconds;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const LEGACY_APP_DOTDIR_NAMES: &[&str] = &[".aio-gateway", ".aio_gateway"];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptSyncFileEntry {
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptSyncManifest {
    schema_version: u32,
    managed_by: String,
    cli_key: String,
    enabled: bool,
    applied_prompt_id: Option<i64>,
    created_at: i64,
    updated_at: i64,
    file: PromptSyncFileEntry,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

fn prompt_target_path(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;

    match cli_key {
        "claude" => Ok(home.join(".claude").join("CLAUDE.md")),
        "codex" => codex_paths::codex_agents_md_path(app),
        "gemini" => Ok(home.join(".gemini").join("GEMINI.md")),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn prompt_sync_root_dir(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?
        .join("prompt-sync")
        .join(cli_key))
}

fn prompt_sync_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

fn prompt_sync_safety_dir(root: &Path) -> PathBuf {
    root.join("restore-safety")
}

fn prompt_sync_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

fn legacy_prompt_sync_roots(app: &tauri::AppHandle, cli_key: &str) -> Result<Vec<PathBuf>, String> {
    let home = home_dir(app)?;
    Ok(LEGACY_APP_DOTDIR_NAMES
        .iter()
        .map(|dir_name| home.join(dir_name).join("prompt-sync").join(cli_key))
        .collect())
}

fn try_migrate_legacy_prompt_sync_dir(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<bool, String> {
    let new_root = prompt_sync_root_dir(app, cli_key)?;
    let new_manifest_path = prompt_sync_manifest_path(&new_root);
    if new_manifest_path.exists() {
        return Ok(false);
    }

    for legacy_root in legacy_prompt_sync_roots(app, cli_key)? {
        let legacy_manifest_path = prompt_sync_manifest_path(&legacy_root);
        if !legacy_manifest_path.exists() {
            continue;
        }

        std::fs::create_dir_all(&new_root)
            .map_err(|e| format!("failed to create {}: {e}", new_root.display()))?;

        let _ = copy_file_if_missing(&legacy_manifest_path, &new_manifest_path)?;

        let legacy_files_dir = prompt_sync_files_dir(&legacy_root);
        if legacy_files_dir.exists() {
            let new_files_dir = prompt_sync_files_dir(&new_root);
            copy_dir_recursive_if_missing(&legacy_files_dir, &new_files_dir)?;
        }

        return Ok(true);
    }

    Ok(false)
}

pub fn read_target_bytes(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<Vec<u8>>, String> {
    let path = prompt_target_path(app, cli_key)?;
    read_optional_file(&path)
}

pub fn restore_target_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let path = prompt_target_path(app, cli_key)?;
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

pub fn read_manifest_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<Option<Vec<u8>>, String> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);
    read_optional_file(&path)
}

pub fn restore_manifest_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);
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

fn read_manifest(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<Option<PromptSyncManifest>, String> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);

    if !path.exists() {
        if let Err(err) = try_migrate_legacy_prompt_sync_dir(app, cli_key) {
            tracing::warn!("提示词同步迁移失败: {}", err);
        }
    }

    let Some(content) = read_optional_file(&path)? else {
        return Ok(None);
    };

    let manifest: PromptSyncManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse prompt manifest.json: {e}"))?;

    if manifest.managed_by != MANAGED_BY {
        return Err(format!(
            "prompt manifest managed_by mismatch: expected {MANAGED_BY}, got {}",
            manifest.managed_by
        ));
    }

    Ok(Some(manifest))
}

fn write_manifest(
    app: &tauri::AppHandle,
    cli_key: &str,
    manifest: &PromptSyncManifest,
) -> Result<(), String> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = prompt_sync_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize prompt manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

fn backup_for_enable(
    app: &tauri::AppHandle,
    cli_key: &str,
    existing: Option<PromptSyncManifest>,
) -> Result<PromptSyncManifest, String> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let files_dir = prompt_sync_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let target_path = prompt_target_path(app, cli_key)?;
    let now = now_unix_seconds();

    let existed = target_path.exists();
    let backup_rel = if existed {
        let bytes = std::fs::read(&target_path)
            .map_err(|e| format!("failed to read {}: {e}", target_path.display()))?;
        let backup_name = target_path
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

    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);

    Ok(PromptSyncManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: cli_key.to_string(),
        enabled: true,
        applied_prompt_id: None,
        created_at,
        updated_at: now,
        file: PromptSyncFileEntry {
            path: target_path.to_string_lossy().to_string(),
            existed,
            backup_rel,
        },
    })
}

fn prompt_content_to_bytes(content: &str) -> Vec<u8> {
    let trimmed = content.trim_matches('\u{feff}').trim_end();
    let mut out = trimmed.as_bytes().to_vec();
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    out
}

fn restore_from_manifest(
    app: &tauri::AppHandle,
    manifest: &PromptSyncManifest,
) -> Result<(), String> {
    let cli_key = manifest.cli_key.as_str();
    validate_cli_key(cli_key)?;

    let root = prompt_sync_root_dir(app, cli_key)?;
    let files_dir = prompt_sync_files_dir(&root);
    let safety_dir = prompt_sync_safety_dir(&root);
    std::fs::create_dir_all(&safety_dir)
        .map_err(|e| format!("failed to create {}: {e}", safety_dir.display()))?;

    let target_path = PathBuf::from(&manifest.file.path);
    let ts = now_unix_seconds();

    if manifest.file.existed {
        let mut candidates: Vec<String> = Vec::new();
        if let Some(rel) = manifest.file.backup_rel.as_ref() {
            candidates.push(rel.clone());
        }

        if let Some(file_name) = target_path.file_name().and_then(|v| v.to_str()) {
            let file_name = file_name.to_string();
            if !candidates.contains(&file_name) {
                candidates.push(file_name);
            }
        }

        for rel in candidates {
            let backup_path = files_dir.join(&rel);
            if !backup_path.exists() {
                continue;
            }
            let bytes = std::fs::read(&backup_path)
                .map_err(|e| format!("failed to read backup {}: {e}", backup_path.display()))?;
            write_file_atomic(&target_path, &bytes)?;
            return Ok(());
        }

        // No backup available. Keep current file content as-is (best-effort),
        // but store a safety snapshot to help users recover manually.
        if target_path.exists() {
            if let Ok(bytes) = std::fs::read(&target_path) {
                let safe_name = format!("{ts}_prompt_keep_current_no_backup");
                let safe_path = safety_dir.join(safe_name);
                let _ = write_file_atomic(&safe_path, &bytes);
            }
        }

        tracing::warn!(cli_key = %cli_key, "提示词同步: 未找到备份");
        return Ok(());
    }

    if !target_path.exists() {
        return Ok(());
    }

    // If the file did not exist before enabling prompt sync, restore to "absent".
    // Safety copy current content before removal.
    if let Ok(bytes) = std::fs::read(&target_path) {
        let safe_name = format!("{ts}_prompt_before_remove");
        let safe_path = safety_dir.join(safe_name);
        let _ = write_file_atomic(&safe_path, &bytes);
    }

    std::fs::remove_file(&target_path)
        .map_err(|e| format!("failed to remove {}: {e}", target_path.display()))?;

    Ok(())
}

pub fn apply_enabled_prompt(
    app: &tauri::AppHandle,
    cli_key: &str,
    prompt_id: i64,
    content: &str,
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);

    let mut manifest = match if should_backup {
        backup_for_enable(app, cli_key, existing.clone())
    } else {
        Ok(existing.unwrap())
    } {
        Ok(m) => m,
        Err(err) => return Err(format!("PROMPT_SYNC_BACKUP_FAILED: {err}")),
    };

    if should_backup {
        // Persist snapshot before applying changes so we can restore on failure.
        manifest.enabled = false;
        manifest.applied_prompt_id = None;
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, cli_key, &manifest)?;
    }

    let target_path = prompt_target_path(app, cli_key)?;
    manifest.file.path = target_path.to_string_lossy().to_string();

    let bytes = prompt_content_to_bytes(content);
    write_file_atomic_if_changed(&target_path, &bytes)?;

    manifest.enabled = true;
    manifest.applied_prompt_id = Some(prompt_id);
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    Ok(())
}

pub fn restore_disabled_prompt(app: &tauri::AppHandle, cli_key: &str) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let Some(mut manifest) = read_manifest(app, cli_key)? else {
        let root = prompt_sync_root_dir(app, cli_key)?;
        let files_dir = prompt_sync_files_dir(&root);
        let safety_dir = prompt_sync_safety_dir(&root);
        std::fs::create_dir_all(&safety_dir)
            .map_err(|e| format!("failed to create {}: {e}", safety_dir.display()))?;

        let target_path = prompt_target_path(app, cli_key)?;
        let ts = now_unix_seconds();

        let backup_rel = target_path
            .file_name()
            .and_then(|v| v.to_str())
            .and_then(|file_name| {
                let name = file_name.to_string();
                let backup_path = files_dir.join(&name);
                if !backup_path.exists() {
                    return None;
                }

                let bytes = std::fs::read(&backup_path).ok()?;
                write_file_atomic(&target_path, &bytes).ok()?;
                Some(name)
            });

        if backup_rel.is_none() && target_path.exists() {
            if let Ok(bytes) = std::fs::read(&target_path) {
                let safe_name = format!("{ts}_prompt_keep_current_no_manifest");
                let safe_path = safety_dir.join(safe_name);
                let _ = write_file_atomic(&safe_path, &bytes);
            }
            tracing::warn!(cli_key = %cli_key, "提示词同步: manifest 缺失，保留当前文件");
        }

        let now = now_unix_seconds();
        let manifest = PromptSyncManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            managed_by: MANAGED_BY.to_string(),
            cli_key: cli_key.to_string(),
            enabled: false,
            applied_prompt_id: None,
            created_at: now,
            updated_at: now,
            file: PromptSyncFileEntry {
                path: target_path.to_string_lossy().to_string(),
                existed: target_path.exists(),
                backup_rel,
            },
        };
        write_manifest(app, cli_key, &manifest)?;
        return Ok(());
    };

    restore_from_manifest(app, &manifest)?;

    manifest.enabled = false;
    manifest.applied_prompt_id = None;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    Ok(())
}
