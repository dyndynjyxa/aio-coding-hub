//! Usage: Manage local CLI proxy configuration files (infra adapter).

use crate::app_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
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

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn new_trace_id(prefix: &str) -> String {
    let ts = now_unix_seconds();
    let seq = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{ts}-{seq}")
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("unsupported cli_key: {cli_key}")),
    }
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

fn claude_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".claude").join("settings.json"))
}

fn codex_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".codex").join("config.toml"))
}

fn codex_auth_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".codex").join("auth.json"))
}

fn gemini_env_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".gemini").join(".env"))
}

fn cli_proxy_root_dir(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
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

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read(path)
        .map(Some)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create dir {}: {e}", parent.display()))?;
    }

    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("file");
    let tmp_path = path.with_file_name(format!("{file_name}.aio-tmp"));

    std::fs::write(&tmp_path, bytes)
        .map_err(|e| format!("failed to write temp file {}: {e}", tmp_path.display()))?;

    if path.exists() {
        let _ = std::fs::remove_file(path);
    }

    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("failed to finalize file {}: {e}", path.display()))?;

    Ok(())
}

fn write_file_atomic_if_changed(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if let Ok(existing) = std::fs::read(path) {
        if existing == bytes {
            return Ok(false);
        }
    }

    write_file_atomic(path, bytes)?;
    Ok(true)
}

fn read_manifest(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<Option<CliProxyManifest>, String> {
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
        ));
    }

    Ok(Some(manifest))
}

fn write_manifest(
    app: &tauri::AppHandle,
    cli_key: &str,
    manifest: &CliProxyManifest,
) -> Result<(), String> {
    let root = cli_proxy_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = cli_proxy_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

fn target_files(app: &tauri::AppHandle, cli_key: &str) -> Result<Vec<TargetFile>, String> {
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
        _ => Err(format!("unsupported cli_key: {cli_key}")),
    }
}

fn backup_for_enable(
    app: &tauri::AppHandle,
    cli_key: &str,
    base_origin: &str,
    existing: Option<CliProxyManifest>,
) -> Result<CliProxyManifest, String> {
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

fn restore_from_manifest(
    app: &tauri::AppHandle,
    manifest: &CliProxyManifest,
) -> Result<(), String> {
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
                return Err(format!("missing backup_rel for {}", entry.kind));
            };
            let backup_path = files_dir.join(rel);
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
) -> Result<serde_json::Value, String> {
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "settings.json root must be a JSON object".to_string())?;

    let env = obj
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| "settings.json env must be an object".to_string())?;

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

fn build_claude_settings_json(current: Option<Vec<u8>>, base_url: &str) -> Result<Vec<u8>, String> {
    let root = match current {
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    };

    let patched = patch_json_set_env_base_url(root, base_url)?;
    let mut out = serde_json::to_vec_pretty(&patched)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn remove_toml_table_block(lines: &mut Vec<String>, table_header: &str) {
    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == table_header {
            start = Some(idx);
            break;
        }
    }

    let Some(start) = start else { return };

    let end = lines[start.saturating_add(1)..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());

    lines.drain(start..end);
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

fn build_codex_config_toml(current: Option<Vec<u8>>, base_url: &str) -> Result<Vec<u8>, String> {
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
    remove_toml_table_block(
        &mut lines,
        &format!("[model_providers.{CODEX_PROVIDER_KEY}]"),
    );

    if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
        lines.push(String::new());
    }

    lines.push(format!("[model_providers.{CODEX_PROVIDER_KEY}]"));
    lines.push(format!("name = \"{CODEX_PROVIDER_KEY}\""));
    lines.push(format!("base_url = \"{base_url}\""));
    lines.push("wire_api = \"responses\"".to_string());
    lines.push("requires_openai_auth = true".to_string());

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

fn build_codex_auth_json(_current: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let value = serde_json::json!({
        "OPENAI_API_KEY": PLACEHOLDER_KEY,
    });
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

fn build_gemini_env(current: Option<Vec<u8>>, base_url: &str) -> Result<Vec<u8>, String> {
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

fn is_proxy_config_applied(app: &tauri::AppHandle, cli_key: &str, base_origin: &str) -> bool {
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
            let expected_table = format!("[model_providers.{CODEX_PROVIDER_KEY}]");

            if !config.contains(&expected_provider)
                || !config.contains(&expected_table)
                || !config.contains(&expected_base)
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

fn apply_proxy_config(
    app: &tauri::AppHandle,
    cli_key: &str,
    base_origin: &str,
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let targets = target_files(app, cli_key)?;

    for t in targets {
        let current = read_optional_file(&t.path)?;
        let bytes = match cli_key {
            "claude" => build_claude_settings_json(current, &format!("{base_origin}/claude"))?,
            "codex" => {
                if t.kind == "codex_config_toml" {
                    build_codex_config_toml(current, &format!("{base_origin}/v1"))?
                } else {
                    build_codex_auth_json(current)?
                }
            }
            "gemini" => build_gemini_env(current, &format!("{base_origin}/gemini"))?,
            _ => return Err(format!("unsupported cli_key: {cli_key}")),
        };

        let _ = write_file_atomic_if_changed(&t.path, &bytes)?;
    }

    Ok(())
}

pub fn status_all(app: &tauri::AppHandle) -> Result<Vec<CliProxyStatus>, String> {
    let mut out = Vec::new();
    for cli_key in ["claude", "codex", "gemini"] {
        let manifest = read_manifest(app, cli_key)?;
        out.push(CliProxyStatus {
            cli_key: cli_key.to_string(),
            enabled: manifest.as_ref().map(|m| m.enabled).unwrap_or(false),
            base_origin: manifest.and_then(|m| m.base_origin),
        });
    }
    Ok(out)
}

pub fn is_enabled(app: &tauri::AppHandle, cli_key: &str) -> Result<bool, String> {
    validate_cli_key(cli_key)?;
    let Some(manifest) = read_manifest(app, cli_key)? else {
        return Ok(false);
    };
    Ok(manifest.enabled)
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    cli_key: &str,
    enabled: bool,
    base_origin: &str,
) -> Result<CliProxyResult, String> {
    validate_cli_key(cli_key)?;
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("base_origin must start with http:// or https://".to_string());
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
                    message: err,
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
                    message: err,
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
                        message: err,
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
                    message: err,
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
            message: err,
            base_origin: manifest.base_origin.clone(),
        }),
    }
}

pub fn sync_enabled(
    app: &tauri::AppHandle,
    base_origin: &str,
) -> Result<Vec<CliProxyResult>, String> {
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("base_origin must start with http:// or https://".to_string());
    }

    let mut out = Vec::new();
    for cli_key in ["claude", "codex", "gemini"] {
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
                    message: err,
                    base_origin: Some(base_origin.to_string()),
                });
            }
        }
    }
    Ok(out)
}
