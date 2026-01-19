//! Usage: Resolve per-user app data directory and related path helpers.

use std::path::PathBuf;
use tauri::Manager;

pub const APP_DOTDIR_NAME: &str = ".aio-coding-hub";
const APP_DOTDIR_NAME_ENV: &str = "AIO_CODING_HUB_DOTDIR_NAME";

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

pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let home_dir = app
        .path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))?;

    let dotdir_name = std::env::var(APP_DOTDIR_NAME_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| is_safe_dotdir_name(v))
        .unwrap_or_else(|| APP_DOTDIR_NAME.to_string());

    let dir = home_dir.join(dotdir_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create app dir: {e}"))?;

    Ok(dir)
}
