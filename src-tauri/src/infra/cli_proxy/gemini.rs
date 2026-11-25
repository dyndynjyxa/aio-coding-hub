//! Native integration for Antigravity's shared Gemini gateway.
use super::legacy_gemini_env::{legacy_env_assignment, legacy_env_owned_base};
use super::{agy_gateway, CliProxyManifest};
use crate::shared::error::AppResult;
use crate::shared::fs::{regular_file_metadata, write_private_file_atomic};
use std::path::{Path, PathBuf};

pub(super) const KIND: &str = "agy_gateway_json";
pub(super) fn path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(super::home_dir(app)?.join(".gemini/config/config.json"))
}

pub(super) fn read(path: &Path) -> AppResult<Option<Vec<u8>>> {
    if regular_file_metadata(path)?.is_none() {
        return Ok(None);
    }
    super::read_optional_cli_proxy_file(path)
}

pub(super) fn ready<R: tauri::Runtime>(app: &tauri::AppHandle<R>, origin: &str) -> bool {
    path(app)
        .and_then(|p| read(&p))
        .ok()
        .flatten()
        .is_some_and(|bytes| agy_gateway::is_applied(&bytes, origin))
}

pub(super) fn save(path: &Path, before: Option<&[u8]>, after: Option<&[u8]>) -> AppResult<()> {
    if before == after {
        return Ok(());
    }
    if read(path)?.as_deref() != before {
        return Err("AGY_CONFIG_CHANGED: 配置已发生变化，请关闭 Antigravity 后重试".into());
    }
    if let Some(bytes) = after {
        super::ensure_cli_proxy_bytes_len(
            bytes,
            super::CLI_PROXY_FILE_MAX_BYTES,
            "Antigravity config",
        )?;
        write_private_file_atomic(path, bytes)
    } else {
        std::fs::remove_file(path).map_err(|e| format!("恢复 {} 失败：{e}", path.display()).into())
    }
}

pub(super) fn apply<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &mut CliProxyManifest,
    origin: &str,
) -> AppResult<()> {
    let path = path(app)?;
    let entry = manifest
        .files
        .iter_mut()
        .find(|f| f.kind == KIND && Path::new(&f.path) == path)
        .ok_or("CLI_PROXY_TARGET_CHANGED: Antigravity 配置目录已变化，请重新开启代理")?;
    let current = read(&path)?;
    // Validate the live config before refreshing a direct-mode backup.
    let next = agy_gateway::apply(current.as_deref(), origin)?;
    // With no prior same-home journal, even a manually configured loopback is the user's baseline.
    let previous_gateway = match (current.as_deref(), manifest.base_origin.as_deref()) {
        (Some(bytes), Some(old)) => agy_gateway::check_managed_endpoint(bytes, &[old, origin])?,
        _ => false,
    };
    if previous_gateway {
        // Keep recovery possible if the saved direct configuration has been removed.
        if entry.existed {
            let backup = backup_path(app, entry)?;
            if regular_file_metadata(&backup)?.is_none() {
                return Err("AGY_NO_BACKUP: 找不到原网关备份，无法安全地重新接管".into());
            }
        }
    } else {
        entry.existed = current.is_some();
        if let Some(bytes) = current.as_ref() {
            let rel = "antigravity-config.json";
            let backup =
                super::cli_proxy_files_dir(&super::cli_proxy_root_dir(app, "gemini")?).join(rel);
            write_private_file_atomic(&backup, bytes)?;
            entry.backup_rel = Some(rel.into());
        } else {
            entry.backup_rel = None;
        }
    }
    manifest.pending_apply = Some(origin.into());
    super::write_manifest(app, "gemini", manifest)?;
    if let Err(error) = save(&path, current.as_deref(), Some(&next)) {
        manifest.pending_apply = None;
        super::write_manifest(app, "gemini", manifest)?;
        return Err(error);
    }
    Ok(())
}

fn backup_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &super::BackupFileEntry,
) -> AppResult<PathBuf> {
    let rel = entry
        .backup_rel
        .as_deref()
        .ok_or("AGY_NO_BACKUP: missing original gateway config")?;
    Ok(super::cli_proxy_files_dir(&super::cli_proxy_root_dir(app, "gemini")?).join(rel))
}

pub(super) fn restore_if_owned<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &CliProxyManifest,
) -> AppResult<bool> {
    let Some(entry) = manifest.files.iter().find(|f| f.kind == KIND) else {
        return Ok(false);
    };
    let path = Path::new(&entry.path);
    let current = read(path)?;
    let origin = manifest
        .pending_apply
        .as_deref()
        .filter(|origin| {
            current
                .as_deref()
                .is_some_and(|b| agy_gateway::has_owned_endpoint(b, origin))
        })
        .or(manifest.base_origin.as_deref());
    let Some(origin) = origin else {
        return Ok(false);
    };
    if !current
        .as_deref()
        .map(|b| agy_gateway::check_managed_endpoint(b, &[origin]))
        .transpose()?
        .unwrap_or(false)
    {
        return Ok(false);
    }
    let original = if entry.existed {
        Some(super::read_cli_proxy_file(&backup_path(app, entry)?)?)
    } else {
        None
    };
    let after = agy_gateway::restore(current.as_deref(), original.as_deref(), origin)?;
    let changed = current != after;
    save(path, current.as_deref(), after.as_deref())?;
    Ok(changed)
}

pub(super) fn transaction_lock() -> AppResult<std::sync::MutexGuard<'static, ()>> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .map_err(|_| "AGY_PROXY_TRANSACTION_LOCK_POISONED".into())
}

pub(super) fn legacy(manifest: &CliProxyManifest) -> bool {
    manifest.cli_key == "gemini" && manifest.files.iter().any(|f| f.kind == "gemini_env")
}

// An old opt-in covered only the Gemini CLI .env.
pub(super) fn retire_legacy<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &mut CliProxyManifest,
) -> AppResult<()> {
    let entry = manifest
        .files
        .iter()
        .find(|f| f.kind == "gemini_env")
        .ok_or("missing Gemini .env entry")?;
    let path = Path::new(&entry.path);
    if let Some(current) = read(path)? {
        let current_str =
            std::str::from_utf8(&current).map_err(|_| "旧 Gemini .env 需要 UTF-8 编码")?;
        let base = manifest
            .base_origin
            .as_ref()
            .map(|base| format!("{base}/gemini"));
        let owned_base =
            legacy_env_owned_base(current_str, base.as_deref(), Some(super::PLACEHOLDER_KEY));
        let owned = |line: &str| -> Option<&'static str> {
            match legacy_env_assignment(line)? {
                ("GEMINI_API_KEY", super::PLACEHOLDER_KEY) => Some("GEMINI_API_KEY"),
                ("GOOGLE_GEMINI_BASE_URL", val)
                    if Some(val) == base.as_deref() || Some(val) == owned_base =>
                {
                    Some("GOOGLE_GEMINI_BASE_URL")
                }
                _ => None,
            }
        };
        if current_str.lines().any(|line| owned(line).is_some()) {
            let original = if entry.existed {
                let rel = entry
                    .backup_rel
                    .as_deref()
                    .ok_or("missing backup for legacy Gemini .env")?;
                super::read_cli_proxy_file(
                    &super::cli_proxy_files_dir(&super::cli_proxy_root_dir(app, "gemini")?)
                        .join(rel),
                )?
            } else {
                Vec::new()
            };
            let original =
                std::str::from_utf8(&original).map_err(|_| "旧 Gemini .env 备份需要 UTF-8 编码")?;
            let mut restored = String::new();
            for line in current_str.split_inclusive('\n') {
                if let Some(key) = owned(line) {
                    if key == "GEMINI_API_KEY" && owned_base.is_none() {
                        continue;
                    }
                    if let Some(old) = original
                        .lines()
                        .find(|old| legacy_env_assignment(old).is_some_and(|(name, _)| name == key))
                    {
                        restored.push_str(old.trim_end_matches(['\r', '\n']));
                        restored.push_str(&line[line.trim_end_matches(['\r', '\n']).len()..]);
                    }
                } else {
                    restored.push_str(line);
                }
            }
            if !entry.existed && restored.trim().is_empty() {
                save(path, Some(&current), None)?;
            } else {
                save(path, Some(&current), Some(restored.as_bytes()))?;
            }
        }
    }
    manifest.files.clear();
    manifest.enabled = false;
    manifest.pending_apply = None;
    manifest.base_origin = None;
    manifest.updated_at = super::now_unix_seconds();
    super::write_manifest(app, "gemini", manifest)?;
    tracing::info!("已恢复旧 Gemini CLI 代理配置，Antigravity CLI 需重新开启代理");
    Ok(())
}
