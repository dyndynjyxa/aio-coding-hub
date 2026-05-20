//! Usage: Read / patch Codex user-level `config.toml` ($CODEX_HOME/config.toml).

mod parsing;
mod patching;
mod types;

pub use types::{
    CodexConfigPatch, CodexConfigState, CodexConfigTomlState, CodexConfigTomlValidationError,
    CodexConfigTomlValidationResult,
};

use crate::codex_paths;
use crate::shared::fs::{
    is_symlink, read_optional_file_with_max_len, write_file_atomic_if_changed,
};
use parsing::{make_state_from_bytes, validate_codex_config_toml_raw};
use patching::patch_config_toml;
use std::path::Path;
use types::CodexConfigStateMeta;

const CODEX_CONFIG_MAX_BYTES: usize = 1024 * 1024;

fn ensure_codex_config_len(bytes: &[u8], label: &str) -> crate::shared::error::AppResult<()> {
    if bytes.len() > CODEX_CONFIG_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {label} too large (max {CODEX_CONFIG_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

fn read_optional_codex_config_file(
    path: &Path,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    read_optional_file_with_max_len(path, CODEX_CONFIG_MAX_BYTES)
}

fn sync_codex_cli_proxy_backup_if_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    next_bytes: &[u8],
) -> crate::shared::error::AppResult<()> {
    ensure_codex_config_len(next_bytes, "codex config backup")?;
    let Some(backup_path) = super::cli_proxy::backup_file_path_for_enabled_manifest(
        app,
        "codex",
        "codex_config_toml",
        "config.toml",
    )?
    else {
        return Ok(());
    };

    let _ = write_file_atomic_if_changed(&backup_path, next_bytes)?;
    Ok(())
}

#[cfg(windows)]
fn normalize_path_for_prefix_match(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(windows)]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    let dir_s = normalize_path_for_prefix_match(dir);
    let root_s = normalize_path_for_prefix_match(allowed_root);
    dir_s == root_s || dir_s.starts_with(&(root_s + "/"))
}

#[cfg(not(windows))]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    dir.starts_with(allowed_root)
}

pub fn codex_config_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let path = codex_paths::codex_config_toml_path(app)?;
    let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
    let user_default_path = codex_paths::codex_home_dir_user_default(app)?.join("config.toml");
    let user_default_dir = user_default_path
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    let follow_path = codex_paths::codex_home_dir_follow_env_or_default(app)?.join("config.toml");
    let follow_dir = follow_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let bytes = read_optional_codex_config_file(&path)?;

    let can_open_config_dir = crate::app_paths::home_dir(app)
        .ok()
        .map(|home| {
            let allowed_root = home.join(".codex");
            path_is_under_allowed_root(&dir, &allowed_root)
                || follow_dir == dir
                || codex_paths::configured_codex_home_dir(app)
                    .as_ref()
                    .is_some_and(|configured_dir| configured_dir == &dir)
        })
        .unwrap_or(false);

    make_state_from_bytes(
        CodexConfigStateMeta {
            config_dir: dir.to_string_lossy().to_string(),
            config_path: path.to_string_lossy().to_string(),
            user_home_default_dir: user_default_dir.to_string_lossy().to_string(),
            user_home_default_path: user_default_path.to_string_lossy().to_string(),
            follow_codex_home_dir: follow_dir.to_string_lossy().to_string(),
            follow_codex_home_path: follow_path.to_string_lossy().to_string(),
            can_open_config_dir,
        },
        bytes,
    )
}

pub fn codex_config_toml_get_raw<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigTomlState> {
    let path = codex_paths::codex_config_toml_path(app)?;
    let bytes = read_optional_codex_config_file(&path)?;
    let exists = bytes.is_some();

    let toml = match bytes {
        Some(bytes) => String::from_utf8(bytes)
            .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?,
        None => String::new(),
    };

    Ok(CodexConfigTomlState {
        config_path: path.to_string_lossy().to_string(),
        exists,
        toml,
    })
}

pub fn codex_config_toml_validate_raw(
    toml: String,
) -> crate::shared::error::AppResult<CodexConfigTomlValidationResult> {
    Ok(validate_codex_config_toml_raw(&toml))
}

pub fn codex_config_toml_set_raw<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mut toml: String,
) -> crate::shared::error::AppResult<CodexConfigState> {
    ensure_codex_config_len(toml.as_bytes(), "codex config.toml")?;
    let validation = validate_codex_config_toml_raw(&toml);
    if !validation.ok {
        let err = validation.error.unwrap_or(CodexConfigTomlValidationError {
            message: "invalid TOML".to_string(),
            line: None,
            column: None,
        });

        let mut msg = format!("SEC_INVALID_INPUT: invalid config.toml: {}", err.message);
        match (err.line, err.column) {
            (Some(line), Some(column)) => msg.push_str(&format!(" (line {line}, column {column})")),
            (Some(line), None) => msg.push_str(&format!(" (line {line})")),
            _ => {}
        }
        return Err(msg.into());
    }

    if !toml.ends_with('\n') {
        toml.push('\n');
    }
    ensure_codex_config_len(toml.as_bytes(), "codex config.toml")?;

    let path = codex_paths::codex_config_toml_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let _ = write_file_atomic_if_changed(&path, toml.as_bytes())?;
    sync_codex_cli_proxy_backup_if_enabled(app, toml.as_bytes())?;
    codex_config_get(app)
}

pub fn codex_config_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: CodexConfigPatch,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let path = codex_paths::codex_config_toml_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let current = read_optional_codex_config_file(&path)?;
    let next = patch_config_toml(current, patch)?;
    ensure_codex_config_len(&next, "codex config.toml")?;
    let _ = write_file_atomic_if_changed(&path, &next)?;
    sync_codex_cli_proxy_backup_if_enabled(app, &next)?;
    codex_config_get(app)
}

#[cfg(test)]
mod tests;
