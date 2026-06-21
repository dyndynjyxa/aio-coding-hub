//! Usage: Resolve per-user app data directory and related path helpers.

use std::path::PathBuf;
use tauri::Manager;
pub const APP_DOTDIR_NAME: &str = ".aio-coding-hub";
const APP_DOTDIR_NAME_ENV: &str = "AIO_CODING_HUB_DOTDIR_NAME";
const TEST_HOME_DIR_ENV: &str = "AIO_CODING_HUB_TEST_HOME";
const HOME_DIR_OVERRIDE_ENV: &str = "AIO_CODING_HUB_HOME_DIR";

fn is_safe_dotdir_name(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if !name.starts_with('.') {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

pub fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    if let Some(path) = std::env::var_os(TEST_HOME_DIR_ENV)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path);
    }

    if let Some(path) = std::env::var_os(HOME_DIR_OVERRIDE_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }

    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}").into())
}

pub fn app_data_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    let home_dir = home_dir(app)?;

    let dotdir_name = std::env::var(APP_DOTDIR_NAME_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| is_safe_dotdir_name(v))
        .unwrap_or_else(|| APP_DOTDIR_NAME.to_string());

    let dir = home_dir.join(dotdir_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create app dir: {e}"))?;

    Ok(dir)
}

pub(crate) fn plugin_id_path_segment(plugin_id: &str) -> crate::shared::error::AppResult<&str> {
    let value = plugin_id.trim();
    if value.is_empty() || value == "." || value == ".." {
        return Err("SEC_INVALID_INPUT: invalid plugin id path segment".into());
    }
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err("SEC_INVALID_INPUT: invalid plugin id path segment".into());
    }
    if value.split('.').any(|segment| segment.is_empty()) {
        return Err("SEC_INVALID_INPUT: invalid plugin id path segment".into());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.')
    {
        return Err("SEC_INVALID_INPUT: invalid plugin id path segment".into());
    }
    Ok(value)
}

pub(crate) fn plugins_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join("plugins"))
}

pub(crate) fn plugins_installed_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(plugins_root(app)?.join("installed"))
}

pub(crate) fn plugins_cache_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(plugins_root(app)?.join("cache"))
}

pub(crate) fn plugins_data_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(plugins_root(app)?.join("data"))
}

pub(crate) fn plugins_logs_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(plugins_root(app)?.join("logs"))
}

pub(crate) fn plugin_data_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    plugin_id: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(plugins_data_dir(app)?.join(plugin_id_path_segment(plugin_id)?))
}

#[cfg(test)]
mod plugin_path_tests {
    #[test]
    fn plugin_id_path_segment_rejects_traversal() {
        assert!(super::plugin_id_path_segment("../evil").is_err());
        assert!(super::plugin_id_path_segment("official/evil").is_err());
        assert!(super::plugin_id_path_segment("official\\evil").is_err());
        assert!(super::plugin_id_path_segment(".").is_err());
        assert!(super::plugin_id_path_segment("community.prompt-helper").is_ok());
    }
}
