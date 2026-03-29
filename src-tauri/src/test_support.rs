//! Usage: Public test helpers for integration tests.

use std::path::PathBuf;

#[cfg(test)]
use crate::shared::mutex_ext::MutexExt;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};

pub fn clear_settings_cache() {
    crate::settings::clear_cache();
}

#[cfg(test)]
pub fn test_env_lock() -> MutexGuard<'static, ()> {
    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock_or_recover()
}

fn serialize_json(
    value: impl serde::Serialize,
) -> crate::shared::error::AppResult<serde_json::Value> {
    Ok(serde_json::to_value(value)
        .map_err(|e| format!("SYSTEM_ERROR: failed to serialize json: {e}"))?)
}

#[derive(Debug, Clone)]
pub struct ProviderUpsertJsonInput {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cost_multiplier: f64,
    pub priority: Option<i64>,
    pub claude_models: Option<serde_json::Value>,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<String>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
}

fn parse_provider_base_url_mode(
    input: &str,
) -> crate::shared::error::AppResult<crate::providers::ProviderBaseUrlMode> {
    match input.trim() {
        "order" => Ok(crate::providers::ProviderBaseUrlMode::Order),
        "ping" => Ok(crate::providers::ProviderBaseUrlMode::Ping),
        _ => Err("SEC_INVALID_INPUT: base_url_mode must be 'order' or 'ping'"
            .to_string()
            .into()),
    }
}

fn parse_daily_reset_mode(
    input: Option<String>,
) -> crate::shared::error::AppResult<Option<crate::providers::DailyResetMode>> {
    match input
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => Ok(None),
        Some("fixed") => Ok(Some(crate::providers::DailyResetMode::Fixed)),
        Some("rolling") => Ok(Some(crate::providers::DailyResetMode::Rolling)),
        Some(_) => Err(
            "SEC_INVALID_INPUT: daily_reset_mode must be 'fixed' or 'rolling'"
                .to_string()
                .into(),
        ),
    }
}

pub fn app_data_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::infra::app_paths::app_data_dir(app)
}

pub fn db_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::infra::db::db_path(app)
}

pub fn init_db<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<()> {
    crate::infra::db::init(app).map(|_| ())
}

pub fn app_data_reset<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<bool> {
    let state = crate::app::app_state::DbInitState::default();
    let app_handle = app.clone();

    tauri::async_runtime::block_on(async move {
        let _ = crate::app::app_state::ensure_db_ready(app_handle.clone(), &state).await?;
        let _db_reset_guard = crate::app::app_state::prepare_db_reset(&state).await;
        crate::infra::data_management::app_data_reset(&app_handle)
    })
}

pub fn mcp_read_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    crate::infra::mcp_sync::read_target_bytes(app, cli_key).map_err(Into::into)
}

pub fn mcp_restore_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    crate::infra::mcp_sync::restore_target_bytes(app, cli_key, bytes).map_err(Into::into)
}

pub fn mcp_swap_local_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    managed_server_keys: Vec<String>,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    let set: std::collections::HashSet<String> = managed_server_keys.into_iter().collect();
    crate::domain::mcp::swap_local_mcp_servers_for_workspace_switch(
        app,
        cli_key,
        &set,
        from_workspace_id,
        to_workspace_id,
    )?;
    Ok(())
}

pub fn mcp_import_servers_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    servers: serde_json::Value,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let servers: Vec<crate::domain::mcp::McpImportServer> = serde_json::from_value(servers)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid mcp import servers json: {e}"))?;
    let report = crate::domain::mcp::import_servers(app, &db, workspace_id, servers)?;
    serialize_json(report)
}

pub fn mcp_import_from_workspace_cli_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let report = crate::domain::mcp::import_servers_from_workspace_cli(app, &db, workspace_id)?;
    serialize_json(report)
}

pub fn mcp_servers_list_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let rows = crate::domain::mcp::list_for_workspace(&db, workspace_id)?;
    serialize_json(rows)
}

pub fn workspace_active_id_by_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<i64> {
    let db = crate::infra::db::init(app)?;
    let result = crate::workspaces::list_by_cli(&db, cli_key)?;
    result.active_id.ok_or_else(|| {
        format!("DB_NOT_FOUND: active workspace not found for cli_key={cli_key}").into()
    })
}

pub fn codex_config_toml_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::infra::codex_paths::codex_config_toml_path(app)
}

pub fn codex_home_dir_follow_env_or_default<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::infra::codex_paths::codex_home_dir_follow_env_or_default(app)
}

pub fn codex_home_dir_user_default<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::infra::codex_paths::codex_home_dir_user_default(app)
}

pub fn codex_config_toml_raw_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    toml: String,
) -> crate::shared::error::AppResult<()> {
    crate::infra::codex_config::codex_config_toml_set_raw(app, toml).map(|_| ())
}

pub fn codex_config_get_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let state = crate::infra::codex_config::codex_config_get(app)?;
    serialize_json(state)
}

pub fn skills_swap_local_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    let _ = crate::domain::skills::swap_local_skills_for_workspace_switch(
        app,
        cli_key,
        from_workspace_id,
        to_workspace_id,
    )?;
    Ok(())
}

pub fn plugins_swap_local_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    let _ = crate::domain::claude_plugins::swap_local_plugins_for_workspace_switch(
        app,
        cli_key,
        from_workspace_id,
        to_workspace_id,
    )?;
    Ok(())
}

pub fn providers_list_by_cli_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let providers = crate::providers::list_by_cli(&db, cli_key)?;
    serialize_json(providers)
}

#[allow(clippy::too_many_arguments)]
pub fn provider_upsert_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    input: ProviderUpsertJsonInput,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let ProviderUpsertJsonInput {
        provider_id,
        cli_key,
        name,
        base_urls,
        base_url_mode,
        api_key,
        enabled,
        cost_multiplier,
        priority,
        claude_models,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
    } = input;
    let claude_models = match claude_models {
        None => None,
        Some(value) => Some(
            serde_json::from_value::<crate::providers::ClaudeModels>(value)
                .map_err(|e| format!("SEC_INVALID_INPUT: invalid claude_models json: {e}"))?,
        ),
    };

    let provider = crate::providers::upsert(
        &db,
        crate::providers::ProviderUpsertParams {
            provider_id,
            cli_key,
            name,
            base_urls,
            base_url_mode: parse_provider_base_url_mode(&base_url_mode)?,
            auth_mode: None,
            api_key,
            enabled,
            cost_multiplier,
            priority,
            claude_models,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode: parse_daily_reset_mode(daily_reset_mode)?,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            tags: None,
            note: None,
            source_provider_id: None,
            bridge_type: None,
        },
    )?;
    serialize_json(provider)
}

pub fn provider_set_enabled_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let provider = crate::providers::set_enabled(&db, provider_id, enabled)?;
    serialize_json(provider)
}

pub fn provider_delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::providers::delete(&db, provider_id)?;
    Ok(true)
}

pub fn providers_reorder_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    ordered_provider_ids: Vec<i64>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let providers = crate::providers::reorder(&db, cli_key, ordered_provider_ids)?;
    serialize_json(providers)
}

pub fn cli_proxy_set_enabled_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    enabled: bool,
    base_origin: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let result = crate::infra::cli_proxy::set_enabled(app, cli_key, enabled, base_origin)?;
    serialize_json(result)
}

pub fn cli_proxy_set_enabled_via_command_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    enabled: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    if enabled {
        return Err(
            "SYSTEM_ERROR: cli_proxy_set_enabled_via_command_json only supports disable path tests"
                .into(),
        );
    }
    let result =
        tauri::async_runtime::block_on(crate::commands::cli_proxy::cli_proxy_set_disabled_impl(
            app.clone(),
            None,
            cli_key.to_string(),
        ))?;
    serialize_json(result)
}

pub fn cli_proxy_startup_repair_incomplete_enable_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let results = crate::infra::cli_proxy::startup_repair_incomplete_enable(app)?;
    serialize_json(results)
}

pub fn cli_proxy_restore_enabled_keep_state_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let results = crate::infra::cli_proxy::restore_enabled_keep_state(app)?;
    serialize_json(results)
}

pub fn gateway_check_port_available_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    port: u16,
) -> crate::shared::error::AppResult<bool> {
    tauri::async_runtime::block_on(crate::commands::gateway::gateway_check_port_available_impl(
        app.clone(),
        port,
    ))
}

pub fn cli_manager_codex_config_set_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: serde_json::Value,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let patch: crate::infra::codex_config::CodexConfigPatch = serde_json::from_value(patch)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid codex config patch: {e}"))?;
    let state = crate::infra::codex_config::codex_config_set(app, patch)?;
    serialize_json(state)
}

pub fn cli_manager_claude_settings_set_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: serde_json::Value,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let patch: crate::infra::claude_settings::ClaudeSettingsPatch =
        serde_json::from_value(patch)
            .map_err(|e| format!("SEC_INVALID_INPUT: invalid claude settings patch: {e}"))?;
    let state = crate::infra::claude_settings::claude_settings_set(app, patch)?;
    serialize_json(state)
}

pub fn cli_manager_claude_env_set_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let state =
        crate::infra::cli_manager::claude_env_set(app, mcp_timeout_ms, disable_error_reporting)?;
    serialize_json(state)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Read application settings and return as JSON Value.
///
/// Use the real settings entrypoint so migrations, sanitization, and the in-memory
/// cache behave the same way as production code.
pub fn settings_get_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let settings = crate::settings::read(app)?;
    serialize_json(settings)
}

/// Update application settings from a JSON Value and return the persisted result.
///
/// Use the real write helper so tests observe the same sanitization and cache updates
/// as production code.
pub fn settings_set_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    update: serde_json::Value,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let settings: crate::settings::AppSettings = serde_json::from_value(update)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid settings json: {e}"))?;
    let persisted = crate::settings::write(app, &settings)?;
    serialize_json(persisted)
}

/// Update application settings through the real `settings_set` command path.
///
/// This exercises the same read-merge-write logic as the frontend settings page.
pub fn settings_set_via_command_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    update: serde_json::Value,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let update: crate::commands::settings::SettingsUpdate = serde_json::from_value(update)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid settings command payload: {e}"))?;
    let persisted = tauri::async_runtime::block_on(crate::commands::settings::settings_set_impl(
        app.clone(),
        update,
    ))?;
    serialize_json(persisted)
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

pub fn workspaces_list_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let result = crate::workspaces::list_by_cli(&db, cli_key)?;
    serialize_json(result)
}

pub fn workspace_create_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    name: &str,
    clone_from_active: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let workspace = crate::workspaces::create(&db, cli_key, name, clone_from_active)?;
    serialize_json(workspace)
}

pub fn workspace_rename_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    name: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let workspace = crate::workspaces::rename(&db, workspace_id, name)?;
    serialize_json(workspace)
}

pub fn workspace_delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::workspaces::delete(&db, workspace_id)
}

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

pub fn skills_installed_list_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let rows = crate::skills::installed_list_for_workspace(&db, workspace_id)?;
    serialize_json(rows)
}

pub fn skill_set_enabled_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    skill_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let row = crate::skills::set_enabled(app, &db, workspace_id, skill_id, enabled)?;
    serialize_json(row)
}

pub fn skill_uninstall<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    skill_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::skills::uninstall(app, &db, skill_id)?;
    Ok(true)
}

pub fn skill_return_to_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    skill_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::skills::return_to_local(app, &db, workspace_id, skill_id)?;
    Ok(true)
}

pub fn skill_local_delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    dir_name: &str,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::skills::delete_local(app, &db, workspace_id, dir_name)?;
    Ok(true)
}

pub fn skills_local_list_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let rows = crate::skills::local_list(app, &db, workspace_id)?;
    serialize_json(rows)
}

pub fn skill_import_local_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workspace_id: i64,
    dir_name: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let row = crate::skills::import_local(app, &db, workspace_id, dir_name)?;
    serialize_json(row)
}

// ---------------------------------------------------------------------------
// Sort Modes
// ---------------------------------------------------------------------------

pub fn sort_modes_list_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let modes = crate::sort_modes::list_modes(&db)?;
    serialize_json(modes)
}

pub fn sort_mode_create_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let mode = crate::sort_modes::create_mode(&db, name)?;
    serialize_json(mode)
}

pub fn sort_mode_rename_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mode_id: i64,
    name: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let mode = crate::sort_modes::rename_mode(&db, mode_id, name)?;
    serialize_json(mode)
}

pub fn sort_mode_delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mode_id: i64,
) -> crate::shared::error::AppResult<bool> {
    let db = crate::infra::db::init(app)?;
    crate::sort_modes::delete_mode(&db, mode_id)?;
    Ok(true)
}

pub fn sort_mode_active_set_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    mode_id: Option<i64>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let row = crate::sort_modes::set_active(&db, cli_key, mode_id)?;
    serialize_json(row)
}

// ---------------------------------------------------------------------------
// Data Management
// ---------------------------------------------------------------------------

pub fn db_disk_usage_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db_path = crate::infra::db::db_path(app)?;
    let wal_path = {
        let mut out = db_path.clone().into_os_string();
        out.push("-wal");
        std::path::PathBuf::from(out)
    };
    let shm_path = {
        let mut out = db_path.clone().into_os_string();
        out.push("-shm");
        std::path::PathBuf::from(out)
    };

    let file_len =
        |p: &std::path::Path| -> u64 { std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) };

    let db_bytes = file_len(&db_path);
    let wal_bytes = file_len(&wal_path);
    let shm_bytes = file_len(&shm_path);

    serialize_json(serde_json::json!({
        "db_bytes": db_bytes,
        "wal_bytes": wal_bytes,
        "shm_bytes": shm_bytes,
        "total_bytes": db_bytes + wal_bytes + shm_bytes,
    }))
}

pub fn request_logs_clear_all_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let db = crate::infra::db::init(app)?;
    let result = crate::data_management::request_logs_clear_all(&db)?;
    serialize_json(result)
}
