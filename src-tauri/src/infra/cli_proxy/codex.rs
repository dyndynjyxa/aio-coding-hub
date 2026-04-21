//! Codex-specific CLI proxy configuration helpers.

use crate::shared::error::AppResult;
use crate::shared::fs::{read_optional_file, write_file_atomic};
use std::path::{Path, PathBuf};

use super::{
    apply_proxy_config, build_manifest_from_captured, build_manifest_with_current_target_paths,
    capture_current_target_state, restore_file_snapshots, snapshot_backup_files,
    snapshot_target_files, write_captured_backups, write_manifest, CliProxyResult, PLACEHOLDER_KEY,
};

pub(super) const CODEX_PROVIDER_KEY: &str = "aio";

pub(super) fn codex_config_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<PathBuf> {
    crate::codex_paths::codex_config_toml_path(app)
}

pub(super) fn codex_auth_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    crate::codex_paths::codex_auth_json_path(app)
}

pub(super) fn is_codex_proxy_target_state<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let config_path = match codex_config_path(app) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let auth_path = match codex_auth_path(app) {
        Ok(path) => path,
        Err(_) => return false,
    };

    let config = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    let auth_bytes = match std::fs::read(&auth_path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let auth = match serde_json::from_slice::<serde_json::Value>(&auth_bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };

    // Check for either normal mode ("aio") or remote_compaction mode ("OpenAI")
    let has_proxy_provider = check_provider_config_basic(&config, CODEX_PROVIDER_KEY)
        || check_provider_config_basic(&config, "OpenAI");
    let has_proxy_auth = auth.get("OPENAI_API_KEY").and_then(|value| value.as_str())
        == Some(PLACEHOLDER_KEY)
        && auth.get("auth_mode").and_then(|value| value.as_str()) == Some("apikey");

    has_proxy_provider && has_proxy_auth
}

/// Basic check for model_provider and model_providers table (without base_url check).
fn check_provider_config_basic(config: &str, provider_key: &str) -> bool {
    let expected_provider = format!("model_provider = \"{provider_key}\"");
    let expected_table_unquoted = format!("[model_providers.{provider_key}]");
    let expected_table_double = format!("[model_providers.\"{provider_key}\"]");
    let expected_table_single = format!("[model_providers.'{provider_key}']");

    config.contains(&expected_provider)
        && (config.contains(&expected_table_unquoted)
            || config.contains(&expected_table_double)
            || config.contains(&expected_table_single))
}

pub(super) fn rebind_codex_manifest_after_home_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mut manifest: super::CliProxyManifest,
    base_origin: &str,
    apply_live: bool,
    trace_id: String,
) -> AppResult<CliProxyResult> {
    let captured = capture_current_target_state(app, "codex")?;
    let previous_manifest = manifest.clone();
    let target_already_proxy_managed = is_proxy_config_applied(app, base_origin)
        || previous_manifest
            .base_origin
            .as_deref()
            .is_some_and(|origin| is_proxy_config_applied(app, origin))
        || is_codex_proxy_target_state(app);

    let origin = Some(base_origin.to_string());
    let rebind_msg = |live: bool| {
        if live {
            "已重绑 Codex 目录并写入当前网关配置".to_string()
        } else {
            "已重绑 Codex 目录基线，待网关启动后接管".to_string()
        }
    };

    if target_already_proxy_managed {
        let target_snapshots = snapshot_target_files(&captured)?;
        manifest = build_manifest_with_current_target_paths(app, &manifest, base_origin)?;

        if let Err(err) = write_manifest(app, "codex", &manifest) {
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_MANIFEST_WRITE_FAILED",
                err.to_string(),
                origin,
            ));
        }

        if let Err(err) = super::restore_backups_exactly_from_manifest(app, &manifest) {
            let _ = write_manifest(app, "codex", &previous_manifest);
            let _ = restore_file_snapshots(&target_snapshots);
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_RESTORE_FAILED",
                err.to_string(),
                origin,
            ));
        }

        if apply_live {
            if let Err(err) = apply_proxy_config(app, "codex", base_origin) {
                let _ = write_manifest(app, "codex", &previous_manifest);
                let _ = restore_file_snapshots(&target_snapshots);
                return Ok(CliProxyResult::failure(
                    trace_id,
                    "codex",
                    true,
                    "CLI_PROXY_REBIND_APPLY_FAILED",
                    err.to_string(),
                    origin,
                ));
            }
        }

        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            true,
            rebind_msg(apply_live),
            origin,
        ));
    }

    let backup_snapshots = snapshot_backup_files(app, "codex", &captured)?;
    let target_snapshots = snapshot_target_files(&captured)?;

    write_captured_backups(app, "codex", &captured)?;
    manifest = build_manifest_from_captured(&manifest, base_origin, captured);

    if let Err(err) = write_manifest(app, "codex", &manifest) {
        let _ = restore_file_snapshots(&backup_snapshots);
        return Ok(CliProxyResult::failure(
            trace_id,
            "codex",
            true,
            "CLI_PROXY_REBIND_MANIFEST_WRITE_FAILED",
            err.to_string(),
            origin,
        ));
    }

    if apply_live {
        if let Err(err) = apply_proxy_config(app, "codex", base_origin) {
            let _ = write_manifest(app, "codex", &previous_manifest);
            let _ = restore_file_snapshots(&backup_snapshots);
            let _ = restore_file_snapshots(&target_snapshots);
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_APPLY_FAILED",
                err.to_string(),
                origin,
            ));
        }
    }

    Ok(CliProxyResult::success(
        trace_id,
        "codex",
        true,
        rebind_msg(apply_live),
        origin,
    ))
}

/// Merge-restore Codex `auth.json`: only revert the proxy-managed keys
/// (`OPENAI_API_KEY`, `auth_mode`) and restore `tokens` / `last_refresh` from
/// the backup if they existed, while preserving any other user changes.
pub(super) fn merge_restore_codex_auth_json(
    target_path: &Path,
    backup_path: &Path,
) -> AppResult<()> {
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
pub(super) fn merge_restore_codex_config_toml(
    target_path: &Path,
    backup_path: &Path,
) -> AppResult<()> {
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

// -- TOML helpers for merge-restore -----------------------------------------

/// Find the value of a root-level `key = "value"` line (before any `[table]` header).
pub(super) fn find_root_key_value(lines: &[String], key: &str) -> Option<String> {
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
pub(super) fn revert_root_key(lines: &mut Vec<String>, key: &str, backup_value: Option<&str>) {
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
            // Backup had it but current doesn't -- shouldn't happen, but restore it
            lines.insert(0, format!("{key} = {val}"));
        }
        (None, None) => {} // Neither has it, nothing to do
    }
}

/// Remove `[model_providers.<provider_key>]` section and its nested tables.
pub(super) fn remove_model_provider_section(lines: &mut Vec<String>, provider_key: &str) {
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
pub(super) fn has_windows_sandbox(lines: &[String]) -> bool {
    let Some(start) = lines.iter().position(|l| l.trim() == "[windows]") else {
        return false;
    };
    let end = find_next_table_header(lines, start.saturating_add(1));
    lines[start + 1..end]
        .iter()
        .any(|l| l.trim_start().starts_with("sandbox"))
}

/// Remove the `sandbox` key from the `[windows]` section; remove the section if empty.
pub(super) fn remove_windows_sandbox(lines: &mut Vec<String>) {
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

pub(super) fn find_next_table_header(lines: &[String], from: usize) -> usize {
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

pub(super) fn is_model_provider_base_header_line(trimmed: &str, provider_key: &str) -> bool {
    trimmed == format!("[model_providers.{provider_key}]")
        || trimmed == format!("[model_providers.\"{provider_key}\"]")
        || trimmed == format!("[model_providers.'{provider_key}']")
}

pub(super) fn find_model_provider_base_table_indices(
    lines: &[String],
    provider_key: &str,
) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            is_model_provider_base_header_line(line.trim(), provider_key).then_some(idx)
        })
        .collect()
}

pub(super) fn find_model_provider_nested_table_index(
    lines: &[String],
    provider_key: &str,
) -> Option<usize> {
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

pub(super) fn upsert_model_provider_base_table(
    lines: &mut Vec<String>,
    provider_key: &str,
    base_url: &str,
) {
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

/// Upsert a root-level `key = "value"` line before any `[table]` header.
/// If `trailing_blank` is true and the inserted line is followed by a non-blank
/// line, an empty separator line is added after it.
fn upsert_root_toml_key(lines: &mut Vec<String>, key: &str, value: &str, trailing_blank: bool) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    if let Some(line) = lines
        .iter_mut()
        .take(first_table)
        .find(|line| line.trim_start().starts_with(key))
    {
        *line = format!("{key} = \"{value}\"");
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

    lines.insert(insert_at, format!("{key} = \"{value}\""));
    if trailing_blank && insert_at + 1 < lines.len() && !lines[insert_at + 1].trim().is_empty() {
        lines.insert(insert_at + 1, String::new());
    }
}

pub(super) fn upsert_root_model_provider(lines: &mut Vec<String>, value: &str) {
    upsert_root_toml_key(lines, "model_provider", value, true);
}

pub(super) fn upsert_root_preferred_auth_method(lines: &mut Vec<String>, value: &str) {
    upsert_root_toml_key(lines, "preferred_auth_method", value, false);
}

pub(super) fn upsert_windows_sandbox(lines: &mut Vec<String>) {
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
pub(super) enum CodexConfigPlatform {
    Windows,
    Other,
}

impl CodexConfigPlatform {
    pub(super) fn current() -> Self {
        if std::env::consts::OS == "windows" {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

pub(super) fn build_codex_config_toml(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
) -> AppResult<Vec<u8>> {
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

pub(super) fn build_codex_auth_json(current: Option<Vec<u8>>) -> AppResult<Vec<u8>> {
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

/// Provider key used when remote_compaction is enabled (Codex requires "OpenAI" for Remote Compact).
const CODEX_REMOTE_COMPACTION_PROVIDER_KEY: &str = "OpenAI";

/// Check whether Codex proxy config is currently applied.
/// Supports both normal mode (provider key = "aio") and remote_compaction mode (provider key = "OpenAI").
pub(super) fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> bool {
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

    // Check base_url first - this must always be present
    if !config.contains(&expected_base) {
        return false;
    }

    // Check for either normal mode ("aio") or remote_compaction mode ("OpenAI")
    let has_normal_provider = check_provider_config(&config, CODEX_PROVIDER_KEY);
    let has_remote_compaction_provider =
        check_provider_config(&config, CODEX_REMOTE_COMPACTION_PROVIDER_KEY);

    if !has_normal_provider && !has_remote_compaction_provider {
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

/// Check if the config contains the expected model_provider and model_providers table for a given key.
fn check_provider_config(config: &str, provider_key: &str) -> bool {
    let expected_provider = format!("model_provider = \"{provider_key}\"");
    let expected_table_unquoted = format!("[model_providers.{provider_key}]");
    let expected_table_double = format!("[model_providers.\"{provider_key}\"]");
    let expected_table_single = format!("[model_providers.'{provider_key}']");

    if !config.contains(&expected_provider) {
        return false;
    }

    config.contains(&expected_table_unquoted)
        || config.contains(&expected_table_double)
        || config.contains(&expected_table_single)
}

/// Public entry point called from `sync_enabled` and `rebind_codex_home_after_change`.
pub(super) fn rebind_codex_home_after_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    apply_live: bool,
) -> AppResult<CliProxyResult> {
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("SEC_INVALID_INPUT: base_origin must start with http:// or https://".into());
    }

    let trace_id = super::new_trace_id("cli-proxy-codex-home-rebind");
    let origin = Some(base_origin.to_string());
    let Some(manifest) = super::read_manifest(app, "codex")? else {
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            false,
            "Codex 代理未启用，无需重绑".to_string(),
            origin,
        ));
    };

    if !manifest.enabled {
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            false,
            "Codex 代理未启用，无需重绑".to_string(),
            origin,
        ));
    }

    if !super::manifest_target_paths_changed(app, &manifest)? {
        let msg = if apply_live {
            "Codex 目录未变化，无需重绑"
        } else {
            "Codex 目录未变化，待网关启动后按现有配置接管"
        };
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            true,
            msg.to_string(),
            origin,
        ));
    }

    rebind_codex_manifest_after_home_change(app, manifest, base_origin, apply_live, trace_id)
}
