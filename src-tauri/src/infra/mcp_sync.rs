//! Usage: Sync/backup/restore MCP configuration files across supported CLIs (infra adapter).

use crate::app_paths;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const LEGACY_APP_DOTDIR_NAMES: &[&str] = &[".aio-gateway", ".aio_gateway"];

#[derive(Debug, Clone)]
pub(crate) struct McpServerForSync {
    pub server_key: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpSyncFileEntry {
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpSyncManifest {
    schema_version: u32,
    managed_by: String,
    cli_key: String,
    enabled: bool,
    created_at: i64,
    updated_at: i64,
    file: McpSyncFileEntry,
    managed_keys: Vec<String>,
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

fn mcp_target_path(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;

    match cli_key {
        // cc-switch: Claude MCP uses ~/.claude.json
        "claude" => Ok(home.join(".claude.json")),
        // cc-switch: Codex MCP uses ~/.codex/config.toml
        "codex" => Ok(home.join(".codex").join("config.toml")),
        // cc-switch: Gemini MCP uses ~/.gemini/settings.json
        "gemini" => Ok(home.join(".gemini").join("settings.json")),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn backup_file_name(cli_key: &str) -> &'static str {
    match cli_key {
        "claude" => "claude.json",
        "codex" => "config.toml",
        "gemini" => "settings.json",
        _ => "config",
    }
}

fn mcp_sync_root_dir(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join("mcp-sync").join(cli_key))
}

fn mcp_sync_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

fn mcp_sync_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

fn legacy_mcp_sync_roots(app: &tauri::AppHandle, cli_key: &str) -> Result<Vec<PathBuf>, String> {
    let home = home_dir(app)?;
    Ok(LEGACY_APP_DOTDIR_NAMES
        .iter()
        .map(|dir_name| home.join(dir_name).join("mcp-sync").join(cli_key))
        .collect())
}

fn copy_dir_recursive_if_missing(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;

    let entries =
        std::fs::read_dir(src).map_err(|e| format!("failed to read dir {}: {e}", src.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", src.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive_if_missing(&path, &dst_path)?;
            continue;
        }

        if dst_path.exists() {
            continue;
        }

        std::fs::copy(&path, &dst_path).map_err(|e| {
            format!(
                "failed to copy {} -> {}: {e}",
                path.display(),
                dst_path.display()
            )
        })?;
    }

    Ok(())
}

fn copy_file_if_missing(src: &Path, dst: &Path) -> Result<bool, String> {
    if dst.exists() {
        return Ok(false);
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    std::fs::copy(src, dst)
        .map_err(|e| format!("failed to copy {} -> {}: {e}", src.display(), dst.display()))?;
    Ok(true)
}

fn try_migrate_legacy_mcp_sync_dir(app: &tauri::AppHandle, cli_key: &str) -> Result<bool, String> {
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

    // Windows rename requires target not to exist.
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

pub fn read_target_bytes(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<Vec<u8>>, String> {
    let path = mcp_target_path(app, cli_key)?;
    read_optional_file(&path)
}

pub fn restore_target_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let path = mcp_target_path(app, cli_key)?;
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
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);
    read_optional_file(&path)
}

pub fn restore_manifest_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);
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

fn read_manifest(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<McpSyncManifest>, String> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let path = mcp_sync_manifest_path(&root);

    if !path.exists() {
        if let Err(err) = try_migrate_legacy_mcp_sync_dir(app, cli_key) {
            eprintln!("mcp sync migrate error: {err}");
        }
    }

    let Some(content) = read_optional_file(&path)? else {
        return Ok(None);
    };

    let manifest: McpSyncManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse mcp manifest.json: {e}"))?;

    if manifest.managed_by != MANAGED_BY {
        return Err(format!(
            "mcp manifest managed_by mismatch: expected {MANAGED_BY}, got {}",
            manifest.managed_by
        ));
    }

    Ok(Some(manifest))
}

fn write_manifest(
    app: &tauri::AppHandle,
    cli_key: &str,
    manifest: &McpSyncManifest,
) -> Result<(), String> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = mcp_sync_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize mcp manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

fn backup_for_enable(
    app: &tauri::AppHandle,
    cli_key: &str,
    existing: Option<McpSyncManifest>,
) -> Result<McpSyncManifest, String> {
    let root = mcp_sync_root_dir(app, cli_key)?;
    let files_dir = mcp_sync_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let target_path = mcp_target_path(app, cli_key)?;
    let now = now_unix_seconds();

    let existed = target_path.exists();
    let backup_rel = if existed {
        let bytes = std::fs::read(&target_path)
            .map_err(|e| format!("failed to read {}: {e}", target_path.display()))?;
        let backup_name = backup_file_name(cli_key);
        let backup_path = files_dir.join(backup_name);
        write_file_atomic(&backup_path, &bytes)?;
        Some(backup_name.to_string())
    } else {
        None
    };

    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);

    Ok(McpSyncManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
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

fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> serde_json::Value {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

fn json_to_bytes(value: &serde_json::Value, hint: &str) -> Result<Vec<u8>, String> {
    let mut out =
        serde_json::to_vec_pretty(value).map_err(|e| format!("failed to serialize {hint}: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn build_claude_mcp_spec(server: &McpServerForSync) -> Result<serde_json::Value, String> {
    let transport = server.transport.as_str();
    match transport {
        "stdio" => {
            let command = server
                .command
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: stdio command is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("stdio".to_string()),
            );
            obj.insert(
                "command".to_string(),
                serde_json::Value::String(command.to_string()),
            );
            if !server.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    serde_json::Value::Array(
                        server
                            .args
                            .iter()
                            .map(|v| serde_json::Value::String(v.to_string()))
                            .collect(),
                    ),
                );
            }
            if !server.env.is_empty() {
                let mut env = serde_json::Map::new();
                for (k, v) in &server.env {
                    env.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert("env".to_string(), serde_json::Value::Object(env));
            }
            if let Some(cwd) = server
                .cwd
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                obj.insert(
                    "cwd".to_string(),
                    serde_json::Value::String(cwd.to_string()),
                );
            }
            Ok(serde_json::Value::Object(obj))
        }
        "http" => {
            let url = server
                .url
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: http url is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("http".to_string()),
            );
            obj.insert(
                "url".to_string(),
                serde_json::Value::String(url.to_string()),
            );
            if !server.headers.is_empty() {
                let mut headers = serde_json::Map::new();
                for (k, v) in &server.headers {
                    headers.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert("headers".to_string(), serde_json::Value::Object(headers));
            }
            Ok(serde_json::Value::Object(obj))
        }
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
    }
}

fn build_gemini_mcp_spec(server: &McpServerForSync) -> Result<serde_json::Value, String> {
    let transport = server.transport.as_str();
    match transport {
        "stdio" => {
            let command = server
                .command
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: stdio command is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "command".to_string(),
                serde_json::Value::String(command.to_string()),
            );
            if !server.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    serde_json::Value::Array(
                        server
                            .args
                            .iter()
                            .map(|v| serde_json::Value::String(v.to_string()))
                            .collect(),
                    ),
                );
            }
            if !server.env.is_empty() {
                let mut env = serde_json::Map::new();
                for (k, v) in &server.env {
                    env.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert("env".to_string(), serde_json::Value::Object(env));
            }
            if let Some(cwd) = server
                .cwd
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                obj.insert(
                    "cwd".to_string(),
                    serde_json::Value::String(cwd.to_string()),
                );
            }
            Ok(serde_json::Value::Object(obj))
        }
        "http" => {
            let url = server
                .url
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: http url is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "httpUrl".to_string(),
                serde_json::Value::String(url.to_string()),
            );
            if !server.headers.is_empty() {
                let mut headers = serde_json::Map::new();
                for (k, v) in &server.headers {
                    headers.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert(
                    "httpHeaders".to_string(),
                    serde_json::Value::Object(headers),
                );
            }
            Ok(serde_json::Value::Object(obj))
        }
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
    }
}

fn patch_json_mcp_servers(
    mut root: serde_json::Value,
    managed_keys: &[String],
    next: &[(String, serde_json::Value)],
) -> serde_json::Value {
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().expect("root must be object");

    let servers_value = root_obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !servers_value.is_object() {
        *servers_value = serde_json::Value::Object(serde_json::Map::new());
    }

    let servers_obj = servers_value
        .as_object_mut()
        .expect("mcpServers must be object");
    for k in managed_keys {
        servers_obj.remove(k);
    }
    for (k, v) in next {
        servers_obj.insert(k.to_string(), v.clone());
    }

    root
}

fn remove_toml_table_block(lines: &mut Vec<String>, table_header: &str) -> bool {
    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == table_header {
            start = Some(idx);
            break;
        }
    }

    let Some(start) = start else { return false };

    let end = lines[start.saturating_add(1)..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());

    lines.drain(start..end);
    true
}

fn toml_escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn is_toml_bare_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

fn toml_key(key: &str) -> String {
    if is_toml_bare_key(key) {
        key.to_string()
    } else {
        format!("\"{}\"", toml_escape_string(key))
    }
}

fn toml_escape_basic_string_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if c.is_control() => {
                let code = c as u32;
                if code <= 0xFFFF {
                    out.push_str(&format!("\\u{:04X}", code));
                } else {
                    out.push_str(&format!("\\U{:08X}", code));
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn toml_string_value_prefer_single_quotes(value: &str) -> String {
    if !value.chars().any(|c| c == '\'' || c.is_control()) {
        return format!("'{}'", value);
    }
    format!("\"{}\"", toml_escape_basic_string_value(value))
}

fn toml_inline_table(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return "{}".to_string();
    }

    let items: Vec<String> = map
        .iter()
        .map(|(k, v)| {
            format!(
                "\"{}\" = \"{}\"",
                toml_escape_string(k),
                toml_escape_string(v)
            )
        })
        .collect();
    format!("{{ {} }}", items.join(", "))
}

fn toml_array(values: &[String]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|v| format!("\"{}\"", toml_escape_string(v)))
        .collect();
    format!("[{}]", items.join(", "))
}

fn build_codex_config_toml(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    let mut keys_to_purge: Vec<&str> = Vec::with_capacity(managed_keys.len() + servers.len());
    for key in managed_keys {
        keys_to_purge.push(key.as_str());
    }
    for server in servers {
        keys_to_purge.push(server.server_key.as_str());
    }
    keys_to_purge.sort();
    keys_to_purge.dedup();

    for key in keys_to_purge {
        while remove_toml_table_block(&mut lines, &format!("[mcp_servers.{key}.env]")) {}
        while remove_toml_table_block(&mut lines, &format!("[mcp_servers.{key}]")) {}
    }

    if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
        lines.push(String::new());
    }

    for server in servers {
        let key = server.server_key.as_str();
        lines.push(format!("[mcp_servers.{key}]"));
        let transport = server.transport.as_str();
        match transport {
            "stdio" => {
                let command = server
                    .command
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "SEC_INVALID_INPUT: stdio command is required".to_string())?;
                lines.push("type = \"stdio\"".to_string());
                lines.push(format!("command = \"{}\"", toml_escape_string(command)));
                if !server.args.is_empty() {
                    lines.push(format!("args = {}", toml_array(&server.args)));
                }
                if let Some(cwd) = server
                    .cwd
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!("cwd = \"{}\"", toml_escape_string(cwd)));
                }
            }
            "http" => {
                let url = server
                    .url
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "SEC_INVALID_INPUT: http url is required".to_string())?;
                lines.push("type = \"http\"".to_string());
                lines.push(format!("url = \"{}\"", toml_escape_string(url)));
                if !server.headers.is_empty() {
                    lines.push(format!(
                        "http_headers = {}",
                        toml_inline_table(&server.headers)
                    ));
                }
            }
            other => return Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
        }

        if !server.env.is_empty() {
            lines.push(String::new());
            lines.push(format!("[mcp_servers.{key}.env]"));
            for (env_key, env_value) in &server.env {
                lines.push(format!(
                    "{} = {}",
                    toml_key(env_key),
                    toml_string_value_prefer_single_quotes(env_value)
                ));
            }
        }

        lines.push(String::new());
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

fn build_claude_config_json(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    let mut next_entries: Vec<(String, serde_json::Value)> = Vec::with_capacity(servers.len());
    for s in servers {
        next_entries.push((s.server_key.to_string(), build_claude_mcp_spec(s)?));
    }

    let root = json_root_from_bytes(current);
    let patched = patch_json_mcp_servers(root, managed_keys, &next_entries);
    json_to_bytes(&patched, "claude.json")
}

fn build_gemini_settings_json(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    let mut next_entries: Vec<(String, serde_json::Value)> = Vec::with_capacity(servers.len());
    for s in servers {
        next_entries.push((s.server_key.to_string(), build_gemini_mcp_spec(s)?));
    }

    let root = json_root_from_bytes(current);
    let patched = patch_json_mcp_servers(root, managed_keys, &next_entries);
    json_to_bytes(&patched, "gemini/settings.json")
}

fn normalized_keys(servers: &[McpServerForSync]) -> Vec<String> {
    let mut keys: Vec<String> = servers.iter().map(|s| s.server_key.to_string()).collect();
    keys.sort();
    keys.dedup();
    keys
}

pub fn sync_cli(
    app: &tauri::AppHandle,
    cli_key: &str,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);

    let desired_keys = normalized_keys(servers);

    // If no enabled servers, remove previously managed keys (and keep user config untouched).
    if desired_keys.is_empty() {
        if let Some(mut manifest) = existing {
            if !manifest.managed_keys.is_empty() {
                let target_path = mcp_target_path(app, cli_key)?;
                let current = read_optional_file(&target_path)?;
                if current.is_some() {
                    let managed_keys = manifest.managed_keys.clone();
                    let next_bytes = match cli_key {
                        "claude" => build_claude_config_json(current, &managed_keys, &[])?,
                        "codex" => build_codex_config_toml(current, &managed_keys, &[])?,
                        "gemini" => build_gemini_settings_json(current, &managed_keys, &[])?,
                        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
                    };
                    write_file_atomic_if_changed(&target_path, &next_bytes)?;
                }
            }

            manifest.enabled = false;
            manifest.managed_keys.clear();
            manifest.updated_at = now_unix_seconds();
            write_manifest(app, cli_key, &manifest)?;
        } else {
            let _ = try_migrate_legacy_mcp_sync_dir(app, cli_key);
        }
        return Ok(());
    }

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

    let current = read_optional_file(&target_path)?;
    let managed_keys = manifest.managed_keys.clone();

    let next_bytes = match cli_key {
        "claude" => build_claude_config_json(current, &managed_keys, servers)?,
        "codex" => build_codex_config_toml(current, &managed_keys, servers)?,
        "gemini" => build_gemini_settings_json(current, &managed_keys, servers)?,
        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    };

    write_file_atomic_if_changed(&target_path, &next_bytes)?;

    manifest.enabled = true;
    manifest.managed_keys = desired_keys;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    // Best-effort: sanity check to avoid duplicated keys in manifest.
    let set: HashSet<String> = manifest.managed_keys.iter().cloned().collect();
    if set.len() != manifest.managed_keys.len() {
        eprintln!("mcp sync warning: duplicated managed_keys for cli_key={cli_key}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_stdio_server(key: &str, env: BTreeMap<String, String>) -> McpServerForSync {
        McpServerForSync {
            server_key: key.to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "exa-mcp-server@latest".to_string()],
            env,
            cwd: None,
            url: None,
            headers: BTreeMap::new(),
        }
    }

    #[test]
    fn codex_toml_writes_env_as_nested_table() {
        let mut env = BTreeMap::new();
        env.insert("EXA_API_KEY".to_string(), "test-key-123".to_string());
        let server = make_stdio_server("exa", env);

        let out = build_codex_config_toml(None, &[], &[server]).expect("build_codex_config_toml");
        let s = String::from_utf8(out).expect("utf8");

        assert!(s.contains("[mcp_servers.exa.env]"), "{s}");
        assert!(s.contains("EXA_API_KEY = 'test-key-123'"), "{s}");
        assert!(!s.contains("env = {"), "{s}");
    }

    #[test]
    fn codex_toml_removes_nested_env_table_for_managed_keys() {
        let input = r#"[mcp_servers.exa]
type = "stdio"
command = "npx"

[mcp_servers.exa.env]
EXA_API_KEY = 'legacy'

[other]
foo = "bar"
"#;

        let managed_keys = vec!["exa".to_string()];
        let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &managed_keys, &[])
            .expect("build_codex_config_toml");
        let s = String::from_utf8(out).expect("utf8");

        assert!(!s.contains("[mcp_servers.exa]"), "{s}");
        assert!(!s.contains("[mcp_servers.exa.env]"), "{s}");
        assert!(s.contains("[other]"), "{s}");
    }

    #[test]
    fn codex_env_value_with_single_quote_falls_back_to_basic_string() {
        let mut env = BTreeMap::new();
        env.insert("EXA_API_KEY".to_string(), "o'brien".to_string());
        let server = make_stdio_server("exa", env);

        let out = build_codex_config_toml(None, &[], &[server]).expect("build_codex_config_toml");
        let s = String::from_utf8(out).expect("utf8");

        assert!(s.contains("EXA_API_KEY = \"o'brien\""), "{s}");
    }

    #[test]
    fn codex_overwrites_existing_mcp_server_even_when_managed_keys_is_empty() {
        let input = r#"[mcp_servers.exa]
type = "stdio"
command = "old"
args = ["--old"]

[mcp_servers.exa.env]
EXA_API_KEY = 'old'
"#;

        let mut env = BTreeMap::new();
        env.insert("EXA_API_KEY".to_string(), "new".to_string());
        let server = make_stdio_server("exa", env);

        let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &[], &[server]).unwrap();
        let s = String::from_utf8(out).expect("utf8");

        assert!(s.matches("[mcp_servers.exa]").count() == 1, "{s}");
        assert!(s.contains("command = \"npx\""), "{s}");
        assert!(s.contains("[mcp_servers.exa.env]"), "{s}");
        assert!(s.contains("EXA_API_KEY = 'new'"), "{s}");
        assert!(!s.contains("command = \"old\""), "{s}");
        assert!(!s.contains("EXA_API_KEY = 'old'"), "{s}");
    }

    #[test]
    fn codex_removes_duplicate_headers_for_same_key() {
        let input = r#"[mcp_servers.exa]
type = "stdio"
command = "a"

[mcp_servers.exa]
type = "stdio"
command = "b"

[mcp_servers.exa.env]
EXA_API_KEY = 'a'

[mcp_servers.exa.env]
EXA_API_KEY = 'b'
"#;

        let mut env = BTreeMap::new();
        env.insert("EXA_API_KEY".to_string(), "new".to_string());
        let server = make_stdio_server("exa", env);

        let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &[], &[server]).unwrap();
        let s = String::from_utf8(out).expect("utf8");

        assert!(s.matches("[mcp_servers.exa]").count() == 1, "{s}");
        assert!(s.matches("[mcp_servers.exa.env]").count() == 1, "{s}");
        assert!(s.contains("command = \"npx\""), "{s}");
        assert!(s.contains("EXA_API_KEY = 'new'"), "{s}");
        assert!(!s.contains("command = \"a\""), "{s}");
        assert!(!s.contains("command = \"b\""), "{s}");
        assert!(!s.contains("EXA_API_KEY = 'a'"), "{s}");
        assert!(!s.contains("EXA_API_KEY = 'b'"), "{s}");
    }
}
