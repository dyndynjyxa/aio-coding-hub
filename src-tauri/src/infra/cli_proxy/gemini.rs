//! Gemini-specific CLI proxy configuration helpers.

use crate::shared::error::AppResult;
use std::path::Path;

use super::{
    read_cli_proxy_file, read_optional_cli_proxy_file, write_cli_proxy_file_atomic, PLACEHOLDER_KEY,
};

pub(super) fn gemini_env_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<std::path::PathBuf> {
    Ok(super::home_dir(app)?.join(".gemini").join(".env"))
}

// -- .env helpers -----------------------------------------------------------

/// Revert an env var line to its backup value, or remove it if backup didn't have it.
pub(super) fn revert_env_var_line(lines: &mut Vec<String>, key: &str, backup_value: Option<&str>) {
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

pub(super) fn set_env_var_lines(input: &str, key: &str, value: &str) -> String {
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

pub(super) fn env_var_value(input: &str, key: &str) -> Option<String> {
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

/// Merge-restore Gemini `.env`: only revert the two proxy-managed env vars
/// (`GOOGLE_GEMINI_BASE_URL`, `GEMINI_API_KEY`) while preserving other entries.
pub(super) fn merge_restore_gemini_env(target_path: &Path, backup_path: &Path) -> AppResult<()> {
    const PROXY_ENV_KEYS: &[&str] = &["GOOGLE_GEMINI_BASE_URL", "GEMINI_API_KEY"];

    let current_bytes = read_optional_cli_proxy_file(target_path)?;
    let backup_bytes = read_cli_proxy_file(backup_path)?;

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
    write_cli_proxy_file_atomic(target_path, out.as_bytes())?;
    Ok(())
}

pub(super) fn build_gemini_env(current: Option<Vec<u8>>, base_url: &str) -> AppResult<Vec<u8>> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut next = set_env_var_lines(&input, "GOOGLE_GEMINI_BASE_URL", base_url);
    next = set_env_var_lines(&next, "GEMINI_API_KEY", PLACEHOLDER_KEY);
    next.push('\n');
    Ok(next.into_bytes())
}

/// Check whether Gemini proxy config is currently applied.
pub(super) fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> bool {
    let path = match gemini_env_path(app) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let content = match read_cli_proxy_file(&path) {
        Ok(v) => String::from_utf8_lossy(&v).to_string(),
        Err(_) => return false,
    };
    let Some(base) = env_var_value(&content, "GOOGLE_GEMINI_BASE_URL") else {
        return false;
    };
    base == format!("{base_origin}/gemini")
}
