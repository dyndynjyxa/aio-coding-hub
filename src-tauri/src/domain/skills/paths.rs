use crate::app_paths;
use crate::codex_paths;
use crate::domain::skills::types::SkillsPaths;
use std::path::PathBuf;
use tauri::Manager;

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

pub(super) fn ssot_skills_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join("skills"))
}

pub(super) fn repos_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join("skill-repos"))
}

pub(super) fn cli_skills_root(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;
    match cli_key {
        "claude" => Ok(home.join(".claude").join("skills")),
        "codex" => codex_paths::codex_skills_dir(app),
        "gemini" => Ok(home.join(".gemini").join("skills")),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

pub(super) fn ensure_skills_roots(app: &tauri::AppHandle) -> Result<(), String> {
    std::fs::create_dir_all(ssot_skills_root(app)?)
        .map_err(|e| format!("failed to create ssot skills dir: {e}"))?;
    std::fs::create_dir_all(repos_root(app)?)
        .map_err(|e| format!("failed to create repos dir: {e}"))?;
    Ok(())
}

pub fn paths_get(app: &tauri::AppHandle, cli_key: &str) -> Result<SkillsPaths, String> {
    validate_cli_key(cli_key)?;
    let ssot = ssot_skills_root(app)?;
    let repos = repos_root(app)?;
    let cli = cli_skills_root(app, cli_key)?;

    Ok(SkillsPaths {
        ssot_dir: ssot.to_string_lossy().to_string(),
        repos_dir: repos.to_string_lossy().to_string(),
        cli_dir: cli.to_string_lossy().to_string(),
    })
}
