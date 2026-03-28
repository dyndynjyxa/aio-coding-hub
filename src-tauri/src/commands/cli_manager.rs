//! Usage: CLI environment / integration related Tauri commands.

use crate::{blocking, claude_settings, cli_manager, codex_config, gemini_config};

#[tauri::command]
pub(crate) async fn cli_manager_claude_info_get(
    app: tauri::AppHandle,
) -> Result<cli_manager::ClaudeCliInfo, String> {
    blocking::run("cli_manager_claude_info_get", move || {
        cli_manager::claude_info_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_info_get(
    app: tauri::AppHandle,
) -> Result<cli_manager::SimpleCliInfo, String> {
    blocking::run("cli_manager_codex_info_get", move || {
        cli_manager::codex_info_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_config_get(
    app: tauri::AppHandle,
) -> Result<codex_config::CodexConfigState, String> {
    blocking::run("cli_manager_codex_config_get", move || {
        codex_config::codex_config_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_config_set(
    app: tauri::AppHandle,
    patch: codex_config::CodexConfigPatch,
) -> Result<codex_config::CodexConfigState, String> {
    blocking::run("cli_manager_codex_config_set", move || {
        codex_config::codex_config_set(&app, patch)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_config_toml_get(
    app: tauri::AppHandle,
) -> Result<codex_config::CodexConfigTomlState, String> {
    blocking::run("cli_manager_codex_config_toml_get", move || {
        codex_config::codex_config_toml_get_raw(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_config_toml_validate(
    toml: String,
) -> Result<codex_config::CodexConfigTomlValidationResult, String> {
    blocking::run("cli_manager_codex_config_toml_validate", move || {
        codex_config::codex_config_toml_validate_raw(toml)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_codex_config_toml_set(
    app: tauri::AppHandle,
    toml: String,
) -> Result<codex_config::CodexConfigState, String> {
    blocking::run("cli_manager_codex_config_toml_set", move || {
        codex_config::codex_config_toml_set_raw(&app, toml)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_gemini_info_get(
    app: tauri::AppHandle,
) -> Result<cli_manager::SimpleCliInfo, String> {
    blocking::run("cli_manager_gemini_info_get", move || {
        cli_manager::gemini_info_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_gemini_config_get(
    app: tauri::AppHandle,
) -> Result<gemini_config::GeminiConfigState, String> {
    blocking::run("cli_manager_gemini_config_get", move || {
        gemini_config::gemini_config_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_gemini_config_set(
    app: tauri::AppHandle,
    patch: gemini_config::GeminiConfigPatch,
) -> Result<gemini_config::GeminiConfigState, String> {
    blocking::run("cli_manager_gemini_config_set", move || {
        gemini_config::gemini_config_set(&app, patch)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_claude_env_set(
    app: tauri::AppHandle,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> Result<cli_manager::ClaudeEnvState, String> {
    blocking::run("cli_manager_claude_env_set", move || {
        cli_manager::claude_env_set(&app, mcp_timeout_ms, disable_error_reporting)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_claude_settings_get(
    app: tauri::AppHandle,
) -> Result<claude_settings::ClaudeSettingsState, String> {
    blocking::run("cli_manager_claude_settings_get", move || {
        claude_settings::claude_settings_get(&app)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn cli_manager_claude_settings_set(
    app: tauri::AppHandle,
    patch: claude_settings::ClaudeSettingsPatch,
) -> Result<claude_settings::ClaudeSettingsState, String> {
    blocking::run("cli_manager_claude_settings_set", move || {
        claude_settings::claude_settings_set(&app, patch)
    })
    .await
    .map_err(Into::into)
}
