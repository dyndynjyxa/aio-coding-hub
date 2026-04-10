//! Usage: Manage local CLI proxy configuration files (infra adapter).

mod claude;
mod codex;
mod gemini;

use crate::app_paths;
use crate::shared::fs::{read_optional_file, write_file_atomic, write_file_atomic_if_changed};
use crate::shared::time::now_unix_seconds;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const PLACEHOLDER_KEY: &str = "aio-coding-hub";

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

// -- Public types -----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProxyStatus {
    pub cli_key: String,
    pub enabled: bool,
    pub base_origin: Option<String>,
    pub current_gateway_origin: Option<String>,
    pub applied_to_current_gateway: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProxyResult {
    pub trace_id: String,
    pub cli_key: String,
    pub enabled: bool,
    pub ok: bool,
    pub error_code: Option<String>,
    pub message: String,
    pub base_origin: Option<String>,
}

impl CliProxyResult {
    fn success(
        trace_id: String,
        cli_key: &str,
        enabled: bool,
        message: String,
        base_origin: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            cli_key: cli_key.to_string(),
            enabled,
            ok: true,
            error_code: None,
            message,
            base_origin,
        }
    }

    fn failure(
        trace_id: String,
        cli_key: &str,
        enabled: bool,
        error_code: &str,
        message: String,
        base_origin: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            cli_key: cli_key.to_string(),
            enabled,
            ok: false,
            error_code: Some(error_code.to_string()),
            message,
            base_origin,
        }
    }
}

// -- Internal types ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupFileEntry {
    kind: String,
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliProxyManifest {
    schema_version: u32,
    managed_by: String,
    cli_key: String,
    enabled: bool,
    base_origin: Option<String>,
    created_at: i64,
    updated_at: i64,
    files: Vec<BackupFileEntry>,
}

#[derive(Debug, Clone)]
struct TargetFile {
    kind: &'static str,
    path: PathBuf,
    backup_name: &'static str,
}

#[derive(Debug, Clone)]
struct PendingBackupEntry {
    kind: String,
    path: PathBuf,
    backup_name: &'static str,
    existed: bool,
    backup_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    existed: bool,
    bytes: Option<Vec<u8>>,
}

// -- Shared helpers ---------------------------------------------------------

fn new_trace_id(prefix: &str) -> String {
    let ts = now_unix_seconds();
    let seq = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{ts}-{seq}")
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

fn cli_proxy_root_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?
        .join("cli-proxy")
        .join(cli_key))
}

fn cli_proxy_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

fn cli_proxy_safety_dir(root: &Path) -> PathBuf {
    root.join("restore-safety")
}

fn cli_proxy_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

fn read_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<CliProxyManifest>> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    let path = cli_proxy_manifest_path(&root);
    let Some(content) = read_optional_file(&path)? else {
        return Ok(None);
    };

    let manifest: CliProxyManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse manifest.json: {e}"))?;

    if manifest.managed_by != MANAGED_BY {
        return Err(format!(
            "manifest managed_by mismatch: expected {MANAGED_BY}, got {}",
            manifest.managed_by
        )
        .into());
    }

    Ok(Some(manifest))
}

fn write_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    manifest: &CliProxyManifest,
) -> crate::shared::error::AppResult<()> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = cli_proxy_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

// -- Dispatch: target_files -------------------------------------------------

fn target_files<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<TargetFile>> {
    validate_cli_key(cli_key)?;

    match cli_key {
        "claude" => Ok(vec![TargetFile {
            kind: "claude_settings_json",
            path: claude::claude_settings_path(app)?,
            backup_name: "settings.json",
        }]),
        "codex" => Ok(vec![
            TargetFile {
                kind: "codex_config_toml",
                path: codex::codex_config_path(app)?,
                backup_name: "config.toml",
            },
            TargetFile {
                kind: "codex_auth_json",
                path: codex::codex_auth_path(app)?,
                backup_name: "auth.json",
            },
        ]),
        "gemini" => Ok(vec![TargetFile {
            kind: "gemini_env",
            path: gemini::gemini_env_path(app)?,
            backup_name: ".env",
        }]),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
    }
}

// -- Dispatch: is_proxy_config_applied --------------------------------------

fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    base_origin: &str,
) -> bool {
    match cli_key {
        "claude" => claude::is_proxy_config_applied(app, base_origin),
        "codex" => codex::is_proxy_config_applied(app, base_origin),
        "gemini" => gemini::is_proxy_config_applied(app, base_origin),
        _ => false,
    }
}

// -- Dispatch: apply_proxy_config -------------------------------------------

fn apply_proxy_config<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    base_origin: &str,
) -> crate::shared::error::AppResult<()> {
    validate_cli_key(cli_key)?;

    let targets = target_files(app, cli_key)?;

    for t in targets {
        let current = read_optional_file(&t.path)?;
        let bytes = match cli_key {
            "claude" => {
                claude::build_claude_settings_json(current, &format!("{base_origin}/claude"))?
            }
            "codex" => {
                if t.kind == "codex_config_toml" {
                    codex::build_codex_config_toml(
                        current,
                        &format!("{base_origin}/v1"),
                        codex::CodexConfigPlatform::current(),
                    )?
                } else {
                    codex::build_codex_auth_json(current)?
                }
            }
            "gemini" => gemini::build_gemini_env(current, &format!("{base_origin}/gemini"))?,
            _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
        };

        let _ = write_file_atomic_if_changed(&t.path, &bytes)?;
    }

    Ok(())
}

// -- Dispatch: restore_from_manifest ----------------------------------------

fn restore_from_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &CliProxyManifest,
) -> crate::shared::error::AppResult<()> {
    let cli_key = manifest.cli_key.as_str();
    validate_cli_key(cli_key)?;

    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);
    let safety_dir = cli_proxy_safety_dir(&root);
    std::fs::create_dir_all(&safety_dir)
        .map_err(|e| format!("failed to create {}: {e}", safety_dir.display()))?;

    let ts = now_unix_seconds();

    for entry in &manifest.files {
        let target_path = PathBuf::from(&entry.path);
        if entry.existed {
            let Some(rel) = entry.backup_rel.as_ref() else {
                return Err(format!("missing backup_rel for {}", entry.kind).into());
            };
            let backup_path = files_dir.join(rel);

            // Use merge-restore for known file kinds to preserve user changes
            // made while the proxy was enabled.
            match entry.kind.as_str() {
                "claude_settings_json" => {
                    claude::merge_restore_claude_settings_json(&target_path, &backup_path)?;
                    continue;
                }
                "codex_auth_json" => {
                    codex::merge_restore_codex_auth_json(&target_path, &backup_path)?;
                    continue;
                }
                "codex_config_toml" => {
                    codex::merge_restore_codex_config_toml(&target_path, &backup_path)?;
                    continue;
                }
                "gemini_env" => {
                    gemini::merge_restore_gemini_env(&target_path, &backup_path)?;
                    continue;
                }
                _ => {}
            }

            // Fallback: full restore for unknown file kinds
            let bytes = std::fs::read(&backup_path).map_err(|e| {
                format!(
                    "failed to read backup {} for {}: {e}",
                    backup_path.display(),
                    entry.kind
                )
            })?;
            write_file_atomic(&target_path, &bytes)?;
            continue;
        }

        if !target_path.exists() {
            continue;
        }

        // If the file did not exist before enabling proxy, restore to "absent".
        // Safety copy current content before removal.
        if let Ok(bytes) = std::fs::read(&target_path) {
            let safe_name = format!("{ts}_{}_before_remove", entry.kind);
            let safe_path = safety_dir.join(safe_name);
            let _ = write_file_atomic(&safe_path, &bytes);
        }

        std::fs::remove_file(&target_path)
            .map_err(|e| format!("failed to remove {}: {e}", target_path.display()))?;
    }

    Ok(())
}

// -- Shared backup / snapshot helpers ---------------------------------------

pub fn backup_file_path_for_enabled_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    kind: &str,
    backup_name: &str,
) -> crate::shared::error::AppResult<Option<PathBuf>> {
    validate_cli_key(cli_key)?;

    let Some(mut manifest) = read_manifest(app, cli_key)? else {
        return Ok(None);
    };
    if !manifest.enabled {
        return Ok(None);
    }

    let target = target_files(app, cli_key)?
        .into_iter()
        .find(|t| t.kind == kind)
        .ok_or_else(|| {
            format!("SEC_INVALID_INPUT: unknown cli backup kind={kind} for cli_key={cli_key}")
        })?;

    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let mut changed = false;
    let target_path = target.path.to_string_lossy().to_string();

    let backup_rel = if let Some(entry) = manifest.files.iter_mut().find(|entry| entry.kind == kind)
    {
        if entry.path != target_path {
            entry.path = target_path.clone();
            changed = true;
        }
        if !entry.existed {
            entry.existed = true;
            changed = true;
        }
        if entry.backup_rel.is_none() {
            entry.backup_rel = Some(backup_name.to_string());
            changed = true;
        }
        entry.backup_rel.clone()
    } else {
        let backup_rel = Some(backup_name.to_string());
        manifest.files.push(BackupFileEntry {
            kind: kind.to_string(),
            path: target_path,
            existed: true,
            backup_rel: backup_rel.clone(),
        });
        changed = true;
        backup_rel
    };

    if changed {
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, cli_key, &manifest)?;
    }

    Ok(backup_rel.map(|rel| files_dir.join(rel)))
}

fn backup_for_enable<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    base_origin: &str,
    existing: Option<CliProxyManifest>,
) -> crate::shared::error::AppResult<CliProxyManifest> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let now = now_unix_seconds();
    let targets = target_files(app, cli_key)?;

    let mut entries = Vec::with_capacity(targets.len());
    for t in targets {
        let read_bytes = read_optional_file(&t.path)?;
        let existed = read_bytes.is_some();
        let backup_rel = if let Some(bytes) = read_bytes {
            let backup_path = files_dir.join(t.backup_name);
            write_file_atomic(&backup_path, &bytes)?;
            Some(t.backup_name.to_string())
        } else {
            None
        };

        entries.push(BackupFileEntry {
            kind: t.kind.to_string(),
            path: t.path.to_string_lossy().to_string(),
            existed,
            backup_rel,
        });
    }

    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);

    Ok(CliProxyManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: cli_key.to_string(),
        enabled: true,
        base_origin: Some(base_origin.to_string()),
        created_at,
        updated_at: now,
        files: entries,
    })
}

fn capture_current_target_state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<PendingBackupEntry>> {
    let targets = target_files(app, cli_key)?;
    let mut captured = Vec::with_capacity(targets.len());

    for target in targets {
        let backup_bytes = read_optional_file(&target.path)?;

        captured.push(PendingBackupEntry {
            kind: target.kind.to_string(),
            path: target.path,
            backup_name: target.backup_name,
            existed: backup_bytes.is_some(),
            backup_bytes,
        });
    }

    Ok(captured)
}

fn manifest_target_paths_changed<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &CliProxyManifest,
) -> crate::shared::error::AppResult<bool> {
    let targets = target_files(app, manifest.cli_key.as_str())?;
    for target in targets {
        let Some(entry) = manifest
            .files
            .iter()
            .find(|entry| entry.kind == target.kind)
        else {
            continue;
        };
        if PathBuf::from(&entry.path) != target.path {
            return Ok(true);
        }
    }

    Ok(false)
}

fn write_captured_backups<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    captured: &[PendingBackupEntry],
) -> crate::shared::error::AppResult<()> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    for entry in captured {
        if let Some(bytes) = entry.backup_bytes.as_ref() {
            let backup_path = files_dir.join(entry.backup_name);
            write_file_atomic(&backup_path, bytes)?;
        }
    }

    Ok(())
}

fn snapshot_file(path: &Path) -> crate::shared::error::AppResult<FileSnapshot> {
    let bytes = read_optional_file(path)?;

    Ok(FileSnapshot {
        path: path.to_path_buf(),
        existed: bytes.is_some(),
        bytes,
    })
}

fn restore_file_snapshots(snapshots: &[FileSnapshot]) -> crate::shared::error::AppResult<()> {
    for snapshot in snapshots {
        if let Some(bytes) = snapshot.bytes.as_ref() {
            if let Some(parent) = snapshot.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
            }
            write_file_atomic(&snapshot.path, bytes)?;
            continue;
        }

        if snapshot.existed {
            return Err(format!(
                "snapshot for {} marked existed but no bytes captured",
                snapshot.path.display()
            )
            .into());
        }

        if snapshot.path.exists() {
            std::fs::remove_file(&snapshot.path)
                .map_err(|e| format!("failed to remove {}: {e}", snapshot.path.display()))?;
        }
    }

    Ok(())
}

fn restore_backups_exactly_from_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &CliProxyManifest,
) -> crate::shared::error::AppResult<()> {
    let cli_key = manifest.cli_key.as_str();
    validate_cli_key(cli_key)?;

    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);

    for entry in &manifest.files {
        let target_path = PathBuf::from(&entry.path);
        if entry.existed {
            let Some(rel) = entry.backup_rel.as_ref() else {
                return Err(format!("missing backup_rel for {}", entry.kind).into());
            };
            let backup_path = files_dir.join(rel);
            let bytes = std::fs::read(&backup_path).map_err(|e| {
                format!(
                    "failed to read backup {} for {}: {e}",
                    backup_path.display(),
                    entry.kind
                )
            })?;
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
            }
            write_file_atomic(&target_path, &bytes)?;
            continue;
        }

        if target_path.exists() {
            std::fs::remove_file(&target_path)
                .map_err(|e| format!("failed to remove {}: {e}", target_path.display()))?;
        }
    }

    Ok(())
}

fn snapshot_backup_files<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    captured: &[PendingBackupEntry],
) -> crate::shared::error::AppResult<Vec<FileSnapshot>> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    let files_dir = cli_proxy_files_dir(&root);
    captured
        .iter()
        .map(|entry| snapshot_file(&files_dir.join(entry.backup_name)))
        .collect()
}

fn snapshot_target_files(
    captured: &[PendingBackupEntry],
) -> crate::shared::error::AppResult<Vec<FileSnapshot>> {
    captured
        .iter()
        .map(|entry| {
            Ok(FileSnapshot {
                path: entry.path.clone(),
                existed: entry.existed,
                bytes: entry.backup_bytes.clone(),
            })
        })
        .collect()
}

fn build_manifest_from_captured(
    existing: &CliProxyManifest,
    base_origin: &str,
    captured: Vec<PendingBackupEntry>,
) -> CliProxyManifest {
    let now = now_unix_seconds();
    let files = captured
        .into_iter()
        .map(|entry| BackupFileEntry {
            kind: entry.kind,
            path: entry.path.to_string_lossy().to_string(),
            existed: entry.existed,
            backup_rel: entry.existed.then(|| entry.backup_name.to_string()),
        })
        .collect();

    CliProxyManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: existing.cli_key.clone(),
        enabled: existing.enabled,
        base_origin: Some(base_origin.to_string()),
        created_at: existing.created_at,
        updated_at: now,
        files,
    }
}

fn build_manifest_with_current_target_paths<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    existing: &CliProxyManifest,
    base_origin: &str,
) -> crate::shared::error::AppResult<CliProxyManifest> {
    let now = now_unix_seconds();
    let files = target_files(app, existing.cli_key.as_str())?
        .into_iter()
        .map(|target| {
            let existing_entry = existing
                .files
                .iter()
                .find(|entry| entry.kind == target.kind)
                .ok_or_else(|| format!("missing manifest entry for {}", target.kind))?;

            Ok(BackupFileEntry {
                kind: existing_entry.kind.clone(),
                path: target.path.to_string_lossy().to_string(),
                existed: existing_entry.existed,
                backup_rel: existing_entry.backup_rel.clone(),
            })
        })
        .collect::<crate::shared::error::AppResult<Vec<_>>>()?;

    Ok(CliProxyManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: existing.cli_key.clone(),
        enabled: existing.enabled,
        base_origin: Some(base_origin.to_string()),
        created_at: existing.created_at,
        updated_at: now,
        files,
    })
}

// -- Public API -------------------------------------------------------------

pub fn status_all<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    current_base_origin: Option<&str>,
) -> crate::shared::error::AppResult<Vec<CliProxyStatus>> {
    let mut out = Vec::new();
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let manifest = read_manifest(app, cli_key)?;
        let enabled = manifest.as_ref().map(|m| m.enabled).unwrap_or(false);
        let manifest_base_origin = manifest.as_ref().and_then(|m| m.base_origin.clone());
        let applied_to_current_gateway = if enabled {
            current_base_origin
                .map(|base_origin| is_proxy_config_applied(app, cli_key, base_origin))
        } else {
            None
        };
        out.push(CliProxyStatus {
            cli_key: cli_key.to_string(),
            enabled,
            base_origin: manifest_base_origin,
            current_gateway_origin: current_base_origin.map(str::to_string),
            applied_to_current_gateway,
        });
    }
    Ok(out)
}

pub fn is_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<bool> {
    validate_cli_key(cli_key)?;
    let Some(manifest) = read_manifest(app, cli_key)? else {
        return Ok(false);
    };
    Ok(manifest.enabled)
}

pub fn set_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    enabled: bool,
    base_origin: &str,
) -> crate::shared::error::AppResult<CliProxyResult> {
    validate_cli_key(cli_key)?;
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("SEC_INVALID_INPUT: base_origin must start with http:// or https://".into());
    }

    let trace_id = new_trace_id("cli-proxy");
    let existing = read_manifest(app, cli_key)?;

    if enabled {
        let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);
        let origin = Some(base_origin.to_string());
        let mut manifest = match if should_backup {
            backup_for_enable(app, cli_key, base_origin, existing.clone())
        } else {
            Ok(existing.unwrap())
        } {
            Ok(m) => m,
            Err(err) => {
                return Ok(CliProxyResult::failure(
                    trace_id,
                    cli_key,
                    false,
                    "CLI_PROXY_BACKUP_FAILED",
                    err.to_string(),
                    origin,
                ));
            }
        };

        // Persist snapshot before applying changes to ensure we can restore on failure.
        if should_backup {
            manifest.enabled = false;
            manifest.base_origin = Some(base_origin.to_string());
            manifest.updated_at = now_unix_seconds();
            if let Err(err) = write_manifest(app, cli_key, &manifest) {
                return Ok(CliProxyResult::failure(
                    trace_id,
                    cli_key,
                    false,
                    "CLI_PROXY_MANIFEST_WRITE_FAILED",
                    err.to_string(),
                    origin,
                ));
            }
        }

        return match apply_proxy_config(app, cli_key, base_origin) {
            Ok(()) => {
                manifest.enabled = true;
                manifest.base_origin = Some(base_origin.to_string());
                manifest.updated_at = now_unix_seconds();
                if let Err(err) = write_manifest(app, cli_key, &manifest) {
                    return Ok(CliProxyResult::failure(
                        trace_id,
                        cli_key,
                        true,
                        "CLI_PROXY_MANIFEST_WRITE_FAILED",
                        err.to_string(),
                        origin,
                    ));
                }

                Ok(CliProxyResult::success(
                    trace_id,
                    cli_key,
                    true,
                    "已开启代理：已备份直连配置并写入网关地址".to_string(),
                    origin,
                ))
            }
            Err(err) => {
                // Best-effort rollback if we just created a new snapshot.
                if should_backup {
                    let _ = restore_from_manifest(app, &manifest);
                    manifest.enabled = false;
                    manifest.updated_at = now_unix_seconds();
                    let _ = write_manifest(app, cli_key, &manifest);
                }

                Ok(CliProxyResult::failure(
                    trace_id,
                    cli_key,
                    false,
                    "CLI_PROXY_ENABLE_FAILED",
                    err.to_string(),
                    origin,
                ))
            }
        };
    }

    let Some(mut manifest) = existing else {
        return Ok(CliProxyResult::failure(
            trace_id,
            cli_key,
            false,
            "CLI_PROXY_NO_BACKUP",
            "未找到备份，无法自动恢复；请手动处理".to_string(),
            Some(base_origin.to_string()),
        ));
    };

    match restore_from_manifest(app, &manifest) {
        Ok(()) => {
            manifest.enabled = false;
            manifest.updated_at = now_unix_seconds();
            let _ = write_manifest(app, cli_key, &manifest);

            Ok(CliProxyResult::success(
                trace_id,
                cli_key,
                false,
                "已关闭代理：已恢复备份直连配置".to_string(),
                manifest.base_origin.clone(),
            ))
        }
        Err(err) => Ok(CliProxyResult::failure(
            trace_id,
            cli_key,
            manifest.enabled,
            "CLI_PROXY_DISABLE_FAILED",
            err.to_string(),
            manifest.base_origin.clone(),
        )),
    }
}

pub fn startup_repair_incomplete_enable<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Vec<CliProxyResult>> {
    let mut out = Vec::new();

    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let Some(mut manifest) = read_manifest(app, cli_key)? else {
            continue;
        };
        if manifest.enabled {
            continue;
        }

        let Some(base_origin) = manifest.base_origin.clone() else {
            continue;
        };

        if !is_proxy_config_applied(app, cli_key, &base_origin) {
            continue;
        }

        let trace_id = new_trace_id("cli-proxy-startup-repair");

        manifest.enabled = true;
        manifest.updated_at = now_unix_seconds();
        match write_manifest(app, cli_key, &manifest) {
            Ok(()) => out.push(CliProxyResult::success(
                trace_id,
                cli_key,
                true,
                "启动自愈：已修复异常中断导致的启用状态不一致".to_string(),
                Some(base_origin),
            )),
            Err(err) => out.push(CliProxyResult::failure(
                trace_id,
                cli_key,
                false,
                "CLI_PROXY_STARTUP_REPAIR_FAILED",
                err.to_string(),
                Some(base_origin),
            )),
        }
    }

    Ok(out)
}

pub fn sync_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    apply_live: bool,
) -> crate::shared::error::AppResult<Vec<CliProxyResult>> {
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("SEC_INVALID_INPUT: base_origin must start with http:// or https://".into());
    }

    let mut out = Vec::new();
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let Some(mut manifest) = read_manifest(app, cli_key)? else {
            continue;
        };
        if !manifest.enabled {
            continue;
        }

        let trace_id = new_trace_id("cli-proxy-sync");
        let needs_target_rebind =
            cli_key == "codex" && manifest_target_paths_changed(app, &manifest)?;

        if needs_target_rebind {
            out.push(codex::rebind_codex_manifest_after_home_change(
                app,
                manifest,
                base_origin,
                apply_live,
                trace_id,
            )?);
            continue;
        }

        if !apply_live {
            if manifest.base_origin.as_deref() != Some(base_origin) {
                manifest.base_origin = Some(base_origin.to_string());
                manifest.updated_at = now_unix_seconds();
                write_manifest(app, cli_key, &manifest)?;
            }
            out.push(CliProxyResult::success(
                trace_id,
                cli_key,
                true,
                "已更新代理目标端口，待网关启动后接管".to_string(),
                Some(base_origin.to_string()),
            ));
            continue;
        }

        if manifest.base_origin.as_deref() == Some(base_origin)
            && is_proxy_config_applied(app, cli_key, base_origin)
        {
            out.push(CliProxyResult::success(
                trace_id,
                cli_key,
                true,
                "已是最新，无需同步".to_string(),
                Some(base_origin.to_string()),
            ));
            continue;
        }

        match apply_proxy_config(app, cli_key, base_origin) {
            Ok(()) => {
                manifest.base_origin = Some(base_origin.to_string());
                manifest.updated_at = now_unix_seconds();
                write_manifest(app, cli_key, &manifest)?;
                out.push(CliProxyResult::success(
                    trace_id,
                    cli_key,
                    true,
                    "已同步代理配置到新端口".to_string(),
                    Some(base_origin.to_string()),
                ));
            }
            Err(err) => {
                out.push(CliProxyResult::failure(
                    trace_id,
                    cli_key,
                    true,
                    "CLI_PROXY_SYNC_FAILED",
                    err.to_string(),
                    Some(base_origin.to_string()),
                ));
            }
        }
    }
    Ok(out)
}

pub fn rebind_codex_home_after_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    apply_live: bool,
) -> crate::shared::error::AppResult<CliProxyResult> {
    codex::rebind_codex_home_after_change(app, base_origin, apply_live)
}

pub fn restore_enabled_keep_state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Vec<CliProxyResult>> {
    let mut out = Vec::new();
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let Some(manifest) = read_manifest(app, cli_key)? else {
            continue;
        };
        if !manifest.enabled {
            continue;
        }

        let trace_id = new_trace_id("cli-proxy-restore");

        match restore_from_manifest(app, &manifest) {
            Ok(()) => out.push(CliProxyResult::success(
                trace_id,
                cli_key,
                true,
                "已恢复备份直连配置（保留启用状态）".to_string(),
                manifest.base_origin.clone(),
            )),
            Err(err) => out.push(CliProxyResult::failure(
                trace_id,
                cli_key,
                true,
                "CLI_PROXY_RESTORE_FAILED",
                err.to_string(),
                manifest.base_origin.clone(),
            )),
        }
    }
    Ok(out)
}

// Re-export submodule items for tests (tests use `super::*`).
#[cfg(test)]
use claude::{build_claude_settings_json, merge_restore_claude_settings_json};
#[cfg(test)]
use codex::{
    build_codex_auth_json, build_codex_config_toml, codex_auth_path, codex_config_path,
    merge_restore_codex_auth_json, merge_restore_codex_config_toml, CodexConfigPlatform,
};
#[cfg(test)]
use gemini::merge_restore_gemini_env;

#[cfg(test)]
mod tests;
