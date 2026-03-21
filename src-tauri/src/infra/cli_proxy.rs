//! Usage: Manage local CLI proxy configuration files (infra adapter).

use crate::app_paths;
use crate::codex_paths;
use crate::shared::fs::{read_optional_file, write_file_atomic, write_file_atomic_if_changed};
use crate::shared::time::now_unix_seconds;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const PLACEHOLDER_KEY: &str = "aio-coding-hub";
const CODEX_PROVIDER_KEY: &str = "aio";

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProxyStatus {
    pub cli_key: String,
    pub enabled: bool,
    pub base_origin: Option<String>,
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
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}").into())
}

fn claude_settings_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(home_dir(app)?.join(".claude").join("settings.json"))
}

fn codex_config_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    codex_paths::codex_config_toml_path(app)
}

fn codex_auth_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    codex_paths::codex_auth_json_path(app)
}

fn gemini_env_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(home_dir(app)?.join(".gemini").join(".env"))
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

fn target_files<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<TargetFile>> {
    validate_cli_key(cli_key)?;

    match cli_key {
        "claude" => Ok(vec![TargetFile {
            kind: "claude_settings_json",
            path: claude_settings_path(app)?,
            backup_name: "settings.json",
        }]),
        "codex" => Ok(vec![
            TargetFile {
                kind: "codex_config_toml",
                path: codex_config_path(app)?,
                backup_name: "config.toml",
            },
            TargetFile {
                kind: "codex_auth_json",
                path: codex_auth_path(app)?,
                backup_name: "auth.json",
            },
        ]),
        "gemini" => Ok(vec![TargetFile {
            kind: "gemini_env",
            path: gemini_env_path(app)?,
            backup_name: ".env",
        }]),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
    }
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
        let existed = t.path.exists();
        let backup_rel = if existed {
            let bytes = std::fs::read(&t.path)
                .map_err(|e| format!("failed to read {}: {e}", t.path.display()))?;
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

/// Merge-restore Claude `settings.json`: only revert the two proxy-managed env
/// keys (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`) while preserving every
/// other change the user may have made while the proxy was enabled.
fn merge_restore_claude_settings_json(
    target_path: &Path,
    backup_path: &Path,
) -> crate::shared::error::AppResult<()> {
    const PROXY_ENV_KEYS: &[&str] = &["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"];

    let current_bytes = read_optional_file(target_path)?;
    let backup_bytes = std::fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup {} for claude_settings_json: {e}",
            backup_path.display()
        )
    })?;

    let mut current: serde_json::Value = match current_bytes {
        Some(b) if !b.is_empty() => {
            serde_json::from_slice(&b).unwrap_or_else(|_| serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };

    let backup: serde_json::Value =
        serde_json::from_slice(&backup_bytes).unwrap_or_else(|_| serde_json::json!({}));

    let backup_env = backup.get("env").and_then(|v| v.as_object());

    if let Some(obj) = current.as_object_mut() {
        if let Some(env) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
            for key in PROXY_ENV_KEYS {
                if let Some(original) = backup_env.and_then(|e| e.get(*key)) {
                    env.insert(key.to_string(), original.clone());
                } else {
                    env.remove(*key);
                }
            }
            if env.is_empty() {
                obj.remove("env");
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&current)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    bytes.push(b'\n');
    write_file_atomic(target_path, &bytes)?;
    Ok(())
}

/// Merge-restore Codex `auth.json`: only revert the proxy-managed keys
/// (`OPENAI_API_KEY`, `auth_mode`) and restore `tokens` / `last_refresh` from
/// the backup if they existed, while preserving any other user changes.
fn merge_restore_codex_auth_json(
    target_path: &Path,
    backup_path: &Path,
) -> crate::shared::error::AppResult<()> {
    const PROXY_INSERTED_KEYS: &[&str] = &["OPENAI_API_KEY", "auth_mode"];
    const PROXY_REMOVED_KEYS: &[&str] = &["tokens", "last_refresh"];

    let current_bytes = read_optional_file(target_path)?;
    let backup_bytes = std::fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup {} for codex_auth_json: {e}",
            backup_path.display()
        )
    })?;

    let mut current: serde_json::Value = match current_bytes {
        Some(b) if !b.is_empty() => {
            serde_json::from_slice(&b).unwrap_or_else(|_| serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };

    let backup: serde_json::Value =
        serde_json::from_slice(&backup_bytes).unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = current.as_object_mut() {
        let backup_obj = backup.as_object();

        // Revert inserted keys
        for key in PROXY_INSERTED_KEYS {
            if let Some(original) = backup_obj.and_then(|b| b.get(*key)) {
                obj.insert(key.to_string(), original.clone());
            } else {
                obj.remove(*key);
            }
        }

        // Restore keys that the proxy removed
        for key in PROXY_REMOVED_KEYS {
            if let Some(original) = backup_obj.and_then(|b| b.get(*key)) {
                obj.insert(key.to_string(), original.clone());
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&current)
        .map_err(|e| format!("failed to serialize auth.json: {e}"))?;
    bytes.push(b'\n');
    write_file_atomic(target_path, &bytes)?;
    Ok(())
}

/// Merge-restore Codex `config.toml`: revert the proxy-managed root keys
/// (`model_provider`, `preferred_auth_method`) and the `[model_providers.aio]`
/// section / `[windows] sandbox` while preserving user changes.
fn merge_restore_codex_config_toml(
    target_path: &Path,
    backup_path: &Path,
) -> crate::shared::error::AppResult<()> {
    let current_bytes = read_optional_file(target_path)?;
    let backup_bytes = std::fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup {} for codex_config_toml: {e}",
            backup_path.display()
        )
    })?;

    let current_str = current_bytes
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();
    let backup_str = String::from_utf8_lossy(&backup_bytes).to_string();

    let mut lines: Vec<String> = if current_str.is_empty() {
        Vec::new()
    } else {
        current_str.lines().map(|l| l.to_string()).collect()
    };

    let backup_lines: Vec<String> = if backup_str.is_empty() {
        Vec::new()
    } else {
        backup_str.lines().map(|l| l.to_string()).collect()
    };

    // --- Revert root `model_provider` ---
    let backup_model_provider = find_root_key_value(&backup_lines, "model_provider");
    revert_root_key(
        &mut lines,
        "model_provider",
        backup_model_provider.as_deref(),
    );

    // --- Revert root `preferred_auth_method` ---
    let backup_auth_method = find_root_key_value(&backup_lines, "preferred_auth_method");
    revert_root_key(
        &mut lines,
        "preferred_auth_method",
        backup_auth_method.as_deref(),
    );

    // --- Remove the proxy-injected `[model_providers.aio]` section ---
    // If the backup had this section, we leave it; otherwise remove it.
    let backup_had_aio =
        !find_model_provider_base_table_indices(&backup_lines, CODEX_PROVIDER_KEY).is_empty();
    if !backup_had_aio {
        remove_model_provider_section(&mut lines, CODEX_PROVIDER_KEY);
    }

    // --- Revert `[windows] sandbox` ---
    // If the backup did not have `[windows]` sandbox, remove the one the proxy added.
    let backup_had_windows_sandbox = has_windows_sandbox(&backup_lines);
    if !backup_had_windows_sandbox {
        remove_windows_sandbox(&mut lines);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    write_file_atomic(target_path, out.as_bytes())?;
    Ok(())
}

/// Merge-restore Gemini `.env`: only revert the two proxy-managed env vars
/// (`GOOGLE_GEMINI_BASE_URL`, `GEMINI_API_KEY`) while preserving other entries.
fn merge_restore_gemini_env(
    target_path: &Path,
    backup_path: &Path,
) -> crate::shared::error::AppResult<()> {
    const PROXY_ENV_KEYS: &[&str] = &["GOOGLE_GEMINI_BASE_URL", "GEMINI_API_KEY"];

    let current_bytes = read_optional_file(target_path)?;
    let backup_bytes = std::fs::read(backup_path).map_err(|e| {
        format!(
            "failed to read backup {} for gemini_env: {e}",
            backup_path.display()
        )
    })?;

    let current_str = current_bytes
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();
    let backup_str = String::from_utf8_lossy(&backup_bytes).to_string();

    let mut lines: Vec<String> = current_str.lines().map(|l| l.to_string()).collect();

    for key in PROXY_ENV_KEYS {
        let backup_val = env_var_value(&backup_str, key);
        revert_env_var_line(&mut lines, key, backup_val.as_deref());
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    write_file_atomic(target_path, out.as_bytes())?;
    Ok(())
}

// ── TOML helpers for merge-restore ──────────────────────────────────────────

/// Find the value of a root-level `key = "value"` line (before any `[table]` header).
fn find_root_key_value(lines: &[String], key: &str) -> Option<String> {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());
    for line in &lines[..first_table] {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key) {
            if let Some((_, v)) = trimmed.split_once('=') {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Revert a root-level key to its backup value, or remove it if backup didn't have it.
fn revert_root_key(lines: &mut Vec<String>, key: &str, backup_value: Option<&str>) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    let pos = lines[..first_table]
        .iter()
        .position(|l| l.trim_start().starts_with(key));

    match (pos, backup_value) {
        (Some(idx), Some(val)) => {
            lines[idx] = format!("{key} = {val}");
        }
        (Some(idx), None) => {
            lines.remove(idx);
        }
        (None, Some(val)) => {
            // Backup had it but current doesn't — shouldn't happen, but restore it
            lines.insert(0, format!("{key} = {val}"));
        }
        (None, None) => {} // Neither has it, nothing to do
    }
}

/// Remove `[model_providers.<provider_key>]` section and its nested tables.
fn remove_model_provider_section(lines: &mut Vec<String>, provider_key: &str) {
    // Remove base tables
    loop {
        let indices = find_model_provider_base_table_indices(lines, provider_key);
        if indices.is_empty() {
            break;
        }
        let start = indices[0];
        let end = find_next_table_header(lines, start.saturating_add(1));
        lines.drain(start..end);
    }

    // Remove nested tables
    loop {
        let Some(start) = find_model_provider_nested_table_index(lines, provider_key) else {
            break;
        };
        let end = find_next_table_header(lines, start.saturating_add(1));
        lines.drain(start..end);
    }
}

/// Check if backup lines contain a `[windows]` section with `sandbox` key.
fn has_windows_sandbox(lines: &[String]) -> bool {
    let Some(start) = lines.iter().position(|l| l.trim() == "[windows]") else {
        return false;
    };
    let end = find_next_table_header(lines, start.saturating_add(1));
    lines[start + 1..end]
        .iter()
        .any(|l| l.trim_start().starts_with("sandbox"))
}

/// Remove the `sandbox` key from the `[windows]` section; remove the section if empty.
fn remove_windows_sandbox(lines: &mut Vec<String>) {
    let Some(start) = lines.iter().position(|l| l.trim() == "[windows]") else {
        return;
    };
    let end = find_next_table_header(lines, start.saturating_add(1));

    // Remove sandbox line
    let mut i = start + 1;
    while i < end && i < lines.len() {
        if lines[i].trim_start().starts_with("sandbox") {
            lines.remove(i);
            break;
        }
        i += 1;
    }

    // If only the header remains (with optional blank lines), remove the whole section
    let new_end = find_next_table_header(lines, start.saturating_add(1));
    let body_empty = lines[start + 1..new_end]
        .iter()
        .all(|l| l.trim().is_empty());
    if body_empty {
        lines.drain(start..new_end);
    }
}

// ── .env helpers for merge-restore ──────────────────────────────────────────

/// Revert an env var line to its backup value, or remove it if backup didn't have it.
fn revert_env_var_line(lines: &mut Vec<String>, key: &str, backup_value: Option<&str>) {
    let prefix_plain = format!("{key}=");
    let prefix_export = format!("export {key}=");

    let pos = lines.iter().position(|l| {
        let trimmed = l.trim_start();
        trimmed.starts_with(&prefix_plain) || trimmed.starts_with(&prefix_export)
    });

    match (pos, backup_value) {
        (Some(idx), Some(val)) => {
            lines[idx] = format!("{key}={val}");
        }
        (Some(idx), None) => {
            lines.remove(idx);
        }
        (None, Some(val)) => {
            lines.push(format!("{key}={val}"));
        }
        (None, None) => {}
    }
}

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
                    merge_restore_claude_settings_json(&target_path, &backup_path)?;
                    continue;
                }
                "codex_auth_json" => {
                    merge_restore_codex_auth_json(&target_path, &backup_path)?;
                    continue;
                }
                "codex_config_toml" => {
                    merge_restore_codex_config_toml(&target_path, &backup_path)?;
                    continue;
                }
                "gemini_env" => {
                    merge_restore_gemini_env(&target_path, &backup_path)?;
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

fn patch_json_set_env_base_url(
    mut root: serde_json::Value,
    base_url: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let obj = root.as_object_mut().ok_or_else(|| {
        crate::shared::error::AppError::from(
            "CLI_PROXY_INVALID_SETTINGS_JSON: root must be a JSON object",
        )
    })?;

    let env = obj
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| {
            crate::shared::error::AppError::from(
                "CLI_PROXY_INVALID_SETTINGS_JSON: env must be an object",
            )
        })?;

    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(base_url.to_string()),
    );
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(PLACEHOLDER_KEY.to_string()),
    );

    Ok(root)
}

fn build_claude_settings_json(
    current: Option<Vec<u8>>,
    base_url: &str,
) -> crate::shared::error::AppResult<Vec<u8>> {
    let root = match current {
        Some(bytes) if bytes.is_empty() => serde_json::json!({}),
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|e| format!("CLI_PROXY_INVALID_SETTINGS_JSON: failed to parse JSON: {e}"))?,
        None => serde_json::json!({}),
    };

    let patched = patch_json_set_env_base_url(root, base_url)?;
    let mut out = serde_json::to_vec_pretty(&patched)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn find_next_table_header(lines: &[String], from: usize) -> usize {
    lines[from..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| from + offset)
        .unwrap_or(lines.len())
}

fn insert_model_provider_section(
    lines: &mut Vec<String>,
    insert_at: usize,
    provider_key: &str,
    base_url: &str,
) {
    let header = format!("[model_providers.{provider_key}]");
    let section = [
        header,
        format!("name = \"{provider_key}\""),
        format!("base_url = \"{base_url}\""),
        "wire_api = \"responses\"".to_string(),
        "requires_openai_auth = true".to_string(),
    ];

    lines.splice(insert_at..insert_at, section);
}

fn is_model_provider_base_header_line(trimmed: &str, provider_key: &str) -> bool {
    trimmed == format!("[model_providers.{provider_key}]")
        || trimmed == format!("[model_providers.\"{provider_key}\"]")
        || trimmed == format!("[model_providers.'{provider_key}']")
}

fn find_model_provider_base_table_indices(lines: &[String], provider_key: &str) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            is_model_provider_base_header_line(line.trim(), provider_key).then_some(idx)
        })
        .collect()
}

fn find_model_provider_nested_table_index(lines: &[String], provider_key: &str) -> Option<usize> {
    let prefix_unquoted = format!("[model_providers.{provider_key}.");
    let prefix_double = format!("[model_providers.\"{provider_key}\".");
    let prefix_single = format!("[model_providers.'{provider_key}'.");

    lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.starts_with(&prefix_unquoted)
            || trimmed.starts_with(&prefix_double)
            || trimmed.starts_with(&prefix_single)
    })
}

fn patch_model_provider_base_table(
    lines: &mut Vec<String>,
    start: usize,
    provider_key: &str,
    base_url: &str,
) {
    let end = find_next_table_header(lines, start.saturating_add(1));

    let mut body: Vec<String> = Vec::new();
    for line in lines[start.saturating_add(1)..end].iter() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            body.push(line.clone());
            continue;
        }

        let Some((k, _)) = trimmed.split_once('=') else {
            body.push(line.clone());
            continue;
        };

        match k.trim() {
            "name" | "base_url" | "wire_api" | "requires_openai_auth" => {}
            _ => body.push(line.clone()),
        }
    }

    let managed = [
        format!("name = \"{provider_key}\""),
        format!("base_url = \"{base_url}\""),
        "wire_api = \"responses\"".to_string(),
        "requires_openai_auth = true".to_string(),
    ];

    let mut patched: Vec<String> = Vec::with_capacity(managed.len() + body.len());
    patched.extend(managed);
    if !body.is_empty()
        && !body.first().is_some_and(|l| l.trim().is_empty())
        && !patched.last().is_some_and(|l| l.trim().is_empty())
    {
        patched.push(String::new());
    }
    patched.extend(body);

    lines.splice(start.saturating_add(1)..end, patched);
}

fn upsert_model_provider_base_table(lines: &mut Vec<String>, provider_key: &str, base_url: &str) {
    let mut bases = find_model_provider_base_table_indices(lines, provider_key);
    bases.sort();

    // Ensure there is exactly one base table, and keep nested tables intact.
    if let Some(&keep_start) = bases.first() {
        let nested_start = find_model_provider_nested_table_index(lines, provider_key);

        // Remove duplicates first (from bottom) to keep indices stable.
        for start in bases.into_iter().rev() {
            if start == keep_start {
                continue;
            }
            let end = find_next_table_header(lines, start.saturating_add(1));
            lines.drain(start..end);
        }

        patch_model_provider_base_table(lines, keep_start, provider_key, base_url);

        // TOML requires parent tables appear before nested child tables. If the base table
        // is currently after a nested table, move it before the first nested occurrence.
        if let Some(nested_start) = nested_start {
            if keep_start > nested_start {
                let end = find_next_table_header(lines, keep_start.saturating_add(1));
                let block: Vec<String> = lines.drain(keep_start..end).collect();
                lines.splice(nested_start..nested_start, block);
            }
        }
        return;
    }

    // No base table found: insert before the first nested table if it exists, otherwise append.
    let mut insert_at =
        find_model_provider_nested_table_index(lines, provider_key).unwrap_or(lines.len());
    if insert_at > 0 && !lines[insert_at.saturating_sub(1)].trim().is_empty() {
        lines.insert(insert_at, String::new());
        insert_at += 1;
    }

    insert_model_provider_section(lines, insert_at, provider_key, base_url);
}

fn upsert_root_model_provider(lines: &mut Vec<String>, value: &str) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    if let Some(line) = lines
        .iter_mut()
        .take(first_table)
        .find(|line| line.trim_start().starts_with("model_provider"))
    {
        *line = format!("model_provider = \"{value}\"");
        return;
    }

    let mut insert_at = 0;
    while insert_at < first_table {
        let trimmed = lines[insert_at].trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            insert_at += 1;
            continue;
        }
        break;
    }

    lines.insert(insert_at, format!("model_provider = \"{value}\""));
    if insert_at + 1 < lines.len() && !lines[insert_at + 1].trim().is_empty() {
        lines.insert(insert_at + 1, String::new());
    }
}

fn upsert_root_preferred_auth_method(lines: &mut Vec<String>, value: &str) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    if let Some(line) = lines
        .iter_mut()
        .take(first_table)
        .find(|line| line.trim_start().starts_with("preferred_auth_method"))
    {
        *line = format!("preferred_auth_method = \"{value}\"");
        return;
    }

    let mut insert_at = 0;
    while insert_at < first_table {
        let trimmed = lines[insert_at].trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            insert_at += 1;
            continue;
        }
        break;
    }

    lines.insert(insert_at, format!("preferred_auth_method = \"{value}\""));
}

fn upsert_windows_sandbox(lines: &mut Vec<String>) {
    let header = "[windows]";
    if let Some(start) = lines.iter().position(|l| l.trim() == header) {
        let end = find_next_table_header(lines, start.saturating_add(1));
        let has_sandbox = lines[start + 1..end]
            .iter()
            .any(|l| l.trim_start().starts_with("sandbox"));
        if !has_sandbox {
            lines.insert(start + 1, "sandbox = \"elevated\"".to_string());
        }
    } else {
        if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
            lines.push(String::new());
        }
        lines.push(header.to_string());
        lines.push("sandbox = \"elevated\"".to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexConfigPlatform {
    Windows,
    Other,
}

impl CodexConfigPlatform {
    fn current() -> Self {
        if std::env::consts::OS == "windows" {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

fn build_codex_config_toml(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
) -> crate::shared::error::AppResult<Vec<u8>> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    upsert_root_model_provider(&mut lines, CODEX_PROVIDER_KEY);
    upsert_root_preferred_auth_method(&mut lines, "apikey");
    upsert_model_provider_base_table(&mut lines, CODEX_PROVIDER_KEY, base_url);
    if platform == CodexConfigPlatform::Windows {
        upsert_windows_sandbox(&mut lines);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

fn build_codex_auth_json(current: Option<Vec<u8>>) -> crate::shared::error::AppResult<Vec<u8>> {
    let mut value = match current {
        Some(bytes) if bytes.is_empty() => serde_json::json!({}),
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|e| format!("CLI_PROXY_INVALID_AUTH_JSON: failed to parse auth.json: {e}"))?,
        None => serde_json::json!({}),
    };

    let obj = value.as_object_mut().ok_or_else(|| {
        crate::shared::error::AppError::from(
            "CLI_PROXY_INVALID_AUTH_JSON: auth.json root must be a JSON object",
        )
    })?;
    obj.insert(
        "OPENAI_API_KEY".to_string(),
        serde_json::Value::String(PLACEHOLDER_KEY.to_string()),
    );
    obj.insert(
        "auth_mode".to_string(),
        serde_json::Value::String("apikey".to_string()),
    );
    // Remove OAuth residuals that would confuse Codex CLI into chatgpt auth mode.
    obj.remove("tokens");
    obj.remove("last_refresh");

    let mut out = serde_json::to_vec_pretty(&value)
        .map_err(|e| format!("failed to serialize auth.json: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn set_env_var_lines(input: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    let mut replaced = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        let raw = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        if raw.starts_with(&format!("{key}=")) {
            *line = format!("{key}={value}");
            replaced = true;
            break;
        }
    }

    if !replaced {
        if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("{key}={value}"));
    }

    lines.join("\n")
}

fn build_gemini_env(
    current: Option<Vec<u8>>,
    base_url: &str,
) -> crate::shared::error::AppResult<Vec<u8>> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut next = set_env_var_lines(&input, "GOOGLE_GEMINI_BASE_URL", base_url);
    next = set_env_var_lines(&next, "GEMINI_API_KEY", PLACEHOLDER_KEY);
    next.push('\n');
    Ok(next.into_bytes())
}

fn env_var_value(input: &str, key: &str) -> Option<String> {
    for line in input.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let raw = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((k, v)) = raw.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        return Some(v.trim().to_string());
    }
    None
}

fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    base_origin: &str,
) -> bool {
    match cli_key {
        "claude" => {
            let path = match claude_settings_path(app) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(_) => return false,
            };
            let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let Some(env) = value.get("env").and_then(|v| v.as_object()) else {
                return false;
            };
            let Some(base) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) else {
                return false;
            };
            base == format!("{base_origin}/claude")
        }
        "codex" => {
            let config_path = match codex_config_path(app) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let auth_path = match codex_auth_path(app) {
                Ok(p) => p,
                Err(_) => return false,
            };

            let config = match std::fs::read_to_string(&config_path) {
                Ok(v) => v,
                Err(_) => return false,
            };

            let expected_base = format!("base_url = \"{base_origin}/v1\"");
            let expected_provider = format!("model_provider = \"{CODEX_PROVIDER_KEY}\"");
            let expected_table_unquoted = format!("[model_providers.{CODEX_PROVIDER_KEY}]");
            let expected_table_double = format!("[model_providers.\"{CODEX_PROVIDER_KEY}\"]");
            let expected_table_single = format!("[model_providers.'{CODEX_PROVIDER_KEY}']");

            if !config.contains(&expected_provider) || !config.contains(&expected_base) {
                return false;
            }

            if !config.contains(&expected_table_unquoted)
                && !config.contains(&expected_table_double)
                && !config.contains(&expected_table_single)
            {
                return false;
            }

            let auth_bytes = match std::fs::read(&auth_path) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let auth = match serde_json::from_slice::<serde_json::Value>(&auth_bytes) {
                Ok(v) => v,
                Err(_) => return false,
            };
            auth.get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .is_some()
        }
        "gemini" => {
            let path = match gemini_env_path(app) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let content = match std::fs::read_to_string(&path) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let Some(base) = env_var_value(&content, "GOOGLE_GEMINI_BASE_URL") else {
                return false;
            };
            base == format!("{base_origin}/gemini")
        }
        _ => false,
    }
}

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
            "claude" => build_claude_settings_json(current, &format!("{base_origin}/claude"))?,
            "codex" => {
                if t.kind == "codex_config_toml" {
                    build_codex_config_toml(
                        current,
                        &format!("{base_origin}/v1"),
                        CodexConfigPlatform::current(),
                    )?
                } else {
                    build_codex_auth_json(current)?
                }
            }
            "gemini" => build_gemini_env(current, &format!("{base_origin}/gemini"))?,
            _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
        };

        let _ = write_file_atomic_if_changed(&t.path, &bytes)?;
    }

    Ok(())
}

pub fn status_all<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Vec<CliProxyStatus>> {
    let mut out = Vec::new();
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let manifest = read_manifest(app, cli_key)?;
        let enabled = manifest.as_ref().map(|m| m.enabled).unwrap_or(false);
        let base_origin = manifest.as_ref().and_then(|m| m.base_origin.clone());
        let applied_to_current_gateway = if enabled {
            Some(
                base_origin
                    .as_deref()
                    .map(|base_origin| is_proxy_config_applied(app, cli_key, base_origin))
                    .unwrap_or(false),
            )
        } else {
            None
        };
        out.push(CliProxyStatus {
            cli_key: cli_key.to_string(),
            enabled,
            base_origin,
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
        let mut manifest = match if should_backup {
            backup_for_enable(app, cli_key, base_origin, existing.clone())
        } else {
            Ok(existing.unwrap())
        } {
            Ok(m) => m,
            Err(err) => {
                return Ok(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: false,
                    ok: false,
                    error_code: Some("CLI_PROXY_BACKUP_FAILED".to_string()),
                    message: err.to_string(),
                    base_origin: Some(base_origin.to_string()),
                });
            }
        };

        // Persist snapshot before applying changes to ensure we can restore on failure.
        if should_backup {
            manifest.enabled = false;
            manifest.base_origin = Some(base_origin.to_string());
            manifest.updated_at = now_unix_seconds();
            if let Err(err) = write_manifest(app, cli_key, &manifest) {
                return Ok(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: false,
                    ok: false,
                    error_code: Some("CLI_PROXY_MANIFEST_WRITE_FAILED".to_string()),
                    message: err.to_string(),
                    base_origin: Some(base_origin.to_string()),
                });
            }
        }

        return match apply_proxy_config(app, cli_key, base_origin) {
            Ok(()) => {
                manifest.enabled = true;
                manifest.base_origin = Some(base_origin.to_string());
                manifest.updated_at = now_unix_seconds();
                if let Err(err) = write_manifest(app, cli_key, &manifest) {
                    return Ok(CliProxyResult {
                        trace_id,
                        cli_key: cli_key.to_string(),
                        enabled: true,
                        ok: false,
                        error_code: Some("CLI_PROXY_MANIFEST_WRITE_FAILED".to_string()),
                        message: err.to_string(),
                        base_origin: Some(base_origin.to_string()),
                    });
                }

                Ok(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: true,
                    ok: true,
                    error_code: None,
                    message: "已开启代理：已备份直连配置并写入网关地址".to_string(),
                    base_origin: Some(base_origin.to_string()),
                })
            }
            Err(err) => {
                // Best-effort rollback if we just created a new snapshot.
                if should_backup {
                    let _ = restore_from_manifest(app, &manifest);
                    manifest.enabled = false;
                    manifest.updated_at = now_unix_seconds();
                    let _ = write_manifest(app, cli_key, &manifest);
                }

                Ok(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: false,
                    ok: false,
                    error_code: Some("CLI_PROXY_ENABLE_FAILED".to_string()),
                    message: err.to_string(),
                    base_origin: Some(base_origin.to_string()),
                })
            }
        };
    }

    let Some(mut manifest) = existing else {
        return Ok(CliProxyResult {
            trace_id,
            cli_key: cli_key.to_string(),
            enabled: false,
            ok: false,
            error_code: Some("CLI_PROXY_NO_BACKUP".to_string()),
            message: "未找到备份，无法自动恢复；请手动处理".to_string(),
            base_origin: Some(base_origin.to_string()),
        });
    };

    match restore_from_manifest(app, &manifest) {
        Ok(()) => {
            manifest.enabled = false;
            manifest.updated_at = now_unix_seconds();
            let _ = write_manifest(app, cli_key, &manifest);

            Ok(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: false,
                ok: true,
                error_code: None,
                message: "已关闭代理：已恢复备份直连配置".to_string(),
                base_origin: manifest.base_origin.clone(),
            })
        }
        Err(err) => Ok(CliProxyResult {
            trace_id,
            cli_key: cli_key.to_string(),
            enabled: manifest.enabled,
            ok: false,
            error_code: Some("CLI_PROXY_DISABLE_FAILED".to_string()),
            message: err.to_string(),
            base_origin: manifest.base_origin.clone(),
        }),
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
            Ok(()) => out.push(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: true,
                ok: true,
                error_code: None,
                message: "启动自愈：已修复异常中断导致的启用状态不一致".to_string(),
                base_origin: Some(base_origin),
            }),
            Err(err) => out.push(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: false,
                ok: false,
                error_code: Some("CLI_PROXY_STARTUP_REPAIR_FAILED".to_string()),
                message: err.to_string(),
                base_origin: Some(base_origin),
            }),
        }
    }

    Ok(out)
}

pub fn sync_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
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

        if manifest.base_origin.as_deref() == Some(base_origin)
            && is_proxy_config_applied(app, cli_key, base_origin)
        {
            out.push(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: true,
                ok: true,
                error_code: None,
                message: "已是最新，无需同步".to_string(),
                base_origin: Some(base_origin.to_string()),
            });
            continue;
        }

        match apply_proxy_config(app, cli_key, base_origin) {
            Ok(()) => {
                manifest.base_origin = Some(base_origin.to_string());
                manifest.updated_at = now_unix_seconds();
                write_manifest(app, cli_key, &manifest)?;
                out.push(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: true,
                    ok: true,
                    error_code: None,
                    message: "已同步代理配置到新端口".to_string(),
                    base_origin: Some(base_origin.to_string()),
                });
            }
            Err(err) => {
                out.push(CliProxyResult {
                    trace_id,
                    cli_key: cli_key.to_string(),
                    enabled: true,
                    ok: false,
                    error_code: Some("CLI_PROXY_SYNC_FAILED".to_string()),
                    message: err.to_string(),
                    base_origin: Some(base_origin.to_string()),
                });
            }
        }
    }
    Ok(out)
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
            Ok(()) => out.push(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: true,
                ok: true,
                error_code: None,
                message: "已恢复备份直连配置（保留启用状态）".to_string(),
                base_origin: manifest.base_origin.clone(),
            }),
            Err(err) => out.push(CliProxyResult {
                trace_id,
                cli_key: cli_key.to_string(),
                enabled: true,
                ok: false,
                error_code: Some("CLI_PROXY_RESTORE_FAILED".to_string()),
                message: err.to_string(),
                base_origin: manifest.base_origin.clone(),
            }),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
