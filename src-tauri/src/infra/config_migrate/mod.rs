//! Usage: Config export/import for machine migration.

mod export;
mod import;
mod rollback;
pub(crate) mod skill_fs;

#[cfg(test)]
mod tests;

use crate::resident;
use crate::shared::error::{db_err, AppResult};
use crate::{db, settings};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;

pub const CONFIG_BUNDLE_SCHEMA_VERSION: u32 = 2;
pub const CONFIG_BUNDLE_SCHEMA_VERSION_V1: u32 = 1;
const SKILL_MANAGED_MARKER_FILE: &str = ".aio-coding-hub.managed";
const SKILL_SOURCE_MARKER_FILE: &str = ".aio-coding-hub.source.json";

fn default_empty_json_object() -> String {
    "{}".to_string()
}

fn default_oauth_refresh_lead_seconds() -> i64 {
    3600
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct ConfigBundle {
    pub schema_version: u32,
    pub exported_at: String,
    pub app_version: String,
    pub settings: String,
    pub providers: Vec<ProviderExport>,
    pub sort_modes: Vec<SortModeExport>,
    pub sort_mode_active: HashMap<String, String>,
    pub workspaces: Vec<WorkspaceExport>,
    pub mcp_servers: Vec<McpServerExport>,
    pub skill_repos: Vec<SkillRepoExport>,
    #[serde(default)]
    pub installed_skills: Option<Vec<InstalledSkillExport>>,
    #[serde(default)]
    pub local_skills: Option<Vec<LocalSkillExport>>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct ProviderExport {
    pub id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: String,
    pub api_key_plaintext: String,
    pub auth_mode: String,
    pub oauth_provider_type: Option<String>,
    pub oauth_access_token: Option<String>,
    pub oauth_refresh_token: Option<String>,
    #[serde(default)]
    pub oauth_id_token: Option<String>,
    pub oauth_token_expiry: Option<i64>,
    pub oauth_scopes: Option<String>,
    pub oauth_token_uri: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub oauth_email: Option<String>,
    #[serde(default = "default_oauth_refresh_lead_seconds")]
    pub oauth_refresh_lead_seconds: i64,
    #[serde(default)]
    pub oauth_last_refreshed_at: Option<i64>,
    #[serde(default)]
    pub oauth_last_error: Option<String>,
    pub claude_models_json: String,
    #[serde(default = "default_empty_json_object")]
    pub supported_models_json: String,
    #[serde(default = "default_empty_json_object")]
    pub model_mapping_json: String,
    pub enabled: bool,
    pub priority: i64,
    pub cost_multiplier: f64,
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub daily_reset_mode: String,
    pub daily_reset_time: String,
    pub tags_json: String,
    pub note: String,
    pub source_provider_id: Option<i64>,
    pub source_provider_cli_key: Option<String>,
    pub bridge_type: Option<String>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct SortModeExport {
    pub name: String,
    pub is_default: bool,
    pub providers: Vec<SortModeProviderExport>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct SortModeProviderExport {
    pub cli_key: String,
    pub provider_cli_key: String,
    pub sort_order: i64,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct WorkspaceExport {
    pub cli_key: String,
    pub name: String,
    pub is_active: bool,
    #[serde(default)]
    pub prompts: Vec<PromptExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PromptExport>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct PromptExport {
    pub name: String,
    pub content: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct McpServerExport {
    pub server_key: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: String,
    pub env_json: String,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers_json: Option<String>,
    pub enabled_in_workspaces: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct SkillRepoExport {
    pub git_url: String,
    pub branch: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct InstalledSkillExport {
    pub skill_key: String,
    pub name: String,
    pub description: String,
    pub source_git_url: String,
    pub source_branch: String,
    pub source_subdir: String,
    pub enabled_in_workspaces: Vec<(String, String)>,
    pub files: Vec<SkillFileExport>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct LocalSkillExport {
    pub cli_key: String,
    pub dir_name: String,
    pub name: String,
    pub description: String,
    pub source_git_url: Option<String>,
    pub source_branch: Option<String>,
    pub source_subdir: Option<String>,
    pub files: Vec<SkillFileExport>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct SkillFileExport {
    pub relative_path: String,
    pub content_base64: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct ConfigImportResult {
    pub providers_imported: u32,
    pub sort_modes_imported: u32,
    pub workspaces_imported: u32,
    pub prompts_imported: u32,
    pub mcp_servers_imported: u32,
    pub skill_repos_imported: u32,
    pub installed_skills_imported: u32,
    pub local_skills_imported: u32,
}

// --- Shared helpers used by multiple submodules ---

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn normalize_oauth_refresh_lead_seconds(value: i64) -> i64 {
    if value > 0 {
        value
    } else {
        default_oauth_refresh_lead_seconds()
    }
}

fn prompts_for_import(
    prompts: Vec<PromptExport>,
    prompt: Option<PromptExport>,
) -> Vec<PromptExport> {
    if prompts.is_empty() {
        prompt.into_iter().collect()
    } else {
        prompts
    }
}

fn validate_bundle_schema_version(schema_version: u32) -> AppResult<()> {
    if schema_version != CONFIG_BUNDLE_SCHEMA_VERSION
        && schema_version != CONFIG_BUNDLE_SCHEMA_VERSION_V1
    {
        return Err(format!(
            "SEC_INVALID_INPUT: unsupported config bundle schema_version={}, expected one of [{}, {}]",
            schema_version, CONFIG_BUNDLE_SCHEMA_VERSION_V1, CONFIG_BUNDLE_SCHEMA_VERSION
        )
        .into());
    }
    Ok(())
}

// --- Public entry points ---

pub fn config_export<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
) -> AppResult<ConfigBundle> {
    let app_settings = settings::read(app)?;
    let settings_string = serde_json::to_string(&app_settings)
        .map_err(|e| format!("SYSTEM_ERROR: failed to serialize settings: {e}"))?;

    let conn = db.open_connection()?;
    let provider_cli_key_by_id = export::load_provider_cli_key_by_id(&conn)?;

    Ok(ConfigBundle {
        schema_version: CONFIG_BUNDLE_SCHEMA_VERSION,
        exported_at: export::query_exported_at(&conn)?,
        app_version: app.package_info().version.to_string(),
        settings: settings_string,
        providers: export::export_providers(&conn, &provider_cli_key_by_id)?,
        sort_modes: export::export_sort_modes(&conn)?,
        sort_mode_active: export::export_sort_mode_active(&conn)?,
        workspaces: export::export_workspaces(&conn)?,
        mcp_servers: export::export_mcp_servers(&conn)?,
        skill_repos: export::export_skill_repos(&conn)?,
        installed_skills: Some(export::export_installed_skills(app, &conn)?),
        local_skills: Some(export::export_local_skills(app)?),
    })
}

pub fn config_import<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    bundle: ConfigBundle,
) -> AppResult<ConfigImportResult> {
    let bundle_schema_version = bundle.schema_version;
    validate_bundle_schema_version(bundle_schema_version)?;
    let imports_full_skill_payload = bundle_schema_version >= CONFIG_BUNDLE_SCHEMA_VERSION;

    let ConfigBundle {
        schema_version: _,
        exported_at: _,
        app_version: _,
        settings,
        providers,
        sort_modes,
        sort_mode_active,
        workspaces,
        mcp_servers,
        skill_repos,
        installed_skills,
        local_skills,
    } = bundle;

    let (installed_skills, local_skills) = import::resolve_skill_payloads_for_import(
        bundle_schema_version,
        installed_skills,
        local_skills,
    )?;
    import::validate_local_skills_for_import(&local_skills)?;

    let previous_settings = settings::read(app)?;
    let mut settings_to_write: settings::AppSettings = serde_json::from_str(&settings)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid settings bundle: {e}"))?;
    settings_to_write.schema_version = settings::SCHEMA_VERSION;
    let runtime_backups = rollback::capture_cli_runtime_backups(app)?;

    let mut conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let legacy_skill_state = if imports_full_skill_payload {
        None
    } else {
        Some(import::capture_legacy_skill_state(&tx)?)
    };

    import::clear_existing_config_data(&tx, imports_full_skill_payload)?;

    let result = import::import_into_transaction(
        &tx,
        now,
        providers,
        sort_modes,
        sort_mode_active,
        workspaces,
        mcp_servers,
        skill_repos,
        imports_full_skill_payload,
        &installed_skills,
        &local_skills,
        legacy_skill_state.as_ref(),
    )?;

    let mut skill_fs_guard = if imports_full_skill_payload {
        Some(rollback::apply_skill_fs_import(
            app,
            &installed_skills,
            &local_skills,
        )?)
    } else {
        None
    };

    settings_to_write.auto_start = crate::app::autostart::reconcile_auto_start(
        app,
        previous_settings.auto_start,
        settings_to_write.auto_start,
        true,
    );

    if let Err(err) = settings::write(app, &settings_to_write) {
        rollback::rollback_after_failed_import(
            app,
            db,
            &previous_settings,
            runtime_backups,
            skill_fs_guard.as_mut(),
        );
        return Err(err);
    }

    if let Err(err) = rollback::sync_all_cli_runtime(app, &tx) {
        drop(tx);
        rollback::rollback_after_failed_import(
            app,
            db,
            &previous_settings,
            runtime_backups,
            skill_fs_guard.as_mut(),
        );
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        rollback::rollback_after_failed_import(
            app,
            db,
            &previous_settings,
            runtime_backups,
            skill_fs_guard.as_mut(),
        );
        return Err(db_err!("failed to commit transaction: {err}"));
    }

    if let Some(guard) = skill_fs_guard.take() {
        guard.finish();
    }
    app.state::<resident::ResidentState>()
        .set_tray_enabled(settings_to_write.tray_enabled);

    Ok(result)
}
