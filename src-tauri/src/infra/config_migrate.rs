//! Usage: Config export/import for machine migration.

use crate::shared::error::{db_err, AppResult};
use crate::{db, settings};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CONFIG_BUNDLE_SCHEMA_VERSION: u32 = 1;

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
    pub oauth_token_expiry: Option<i64>,
    pub oauth_scopes: Option<String>,
    pub oauth_token_uri: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub oauth_email: Option<String>,
    pub claude_models_json: String,
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
pub struct ConfigImportResult {
    pub providers_imported: u32,
    pub sort_modes_imported: u32,
    pub workspaces_imported: u32,
    pub mcp_servers_imported: u32,
    pub skill_repos_imported: u32,
}

pub fn config_export(app: &tauri::AppHandle, db: &db::Db) -> AppResult<ConfigBundle> {
    let app_settings = settings::read(app)?;
    let settings_string = serde_json::to_string(&app_settings)
        .map_err(|e| format!("SYSTEM_ERROR: failed to serialize settings: {e}"))?;

    let conn = db.open_connection()?;
    let provider_cli_key_by_id = load_provider_cli_key_by_id(&conn)?;

    Ok(ConfigBundle {
        schema_version: CONFIG_BUNDLE_SCHEMA_VERSION,
        exported_at: query_exported_at(&conn)?,
        app_version: app.package_info().version.to_string(),
        settings: settings_string,
        providers: export_providers(&conn, &provider_cli_key_by_id)?,
        sort_modes: export_sort_modes(&conn)?,
        sort_mode_active: export_sort_mode_active(&conn)?,
        workspaces: export_workspaces(&conn)?,
        mcp_servers: export_mcp_servers(&conn)?,
        skill_repos: export_skill_repos(&conn)?,
    })
}

pub fn config_import(
    app: &tauri::AppHandle,
    db: &db::Db,
    bundle: ConfigBundle,
) -> AppResult<ConfigImportResult> {
    validate_bundle_schema_version(bundle.schema_version)?;

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
    } = bundle;

    let settings_to_write: settings::AppSettings = serde_json::from_str(&settings)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid settings bundle: {e}"))?;

    let mut conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    clear_existing_config_data(&tx)?;

    let result = import_into_transaction(
        &tx,
        now,
        providers,
        sort_modes,
        sort_mode_active,
        workspaces,
        mcp_servers,
        skill_repos,
    )?;

    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;
    if let Err(err) = settings::write(app, &settings_to_write) {
        tracing::warn!(
            error = %err,
            "config import: DB committed but settings write failed; restart required to apply settings"
        );
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn import_into_transaction(
    tx: &Connection,
    now: i64,
    providers: Vec<ProviderExport>,
    sort_modes: Vec<SortModeExport>,
    sort_mode_active: HashMap<String, String>,
    workspaces: Vec<WorkspaceExport>,
    mcp_servers: Vec<McpServerExport>,
    skill_repos: Vec<SkillRepoExport>,
) -> AppResult<ConfigImportResult> {
    let mut provider_id_by_cli_and_name: HashMap<(String, String), i64> = HashMap::new();
    let mut provider_id_by_source_id: HashMap<i64, i64> = HashMap::new();
    let mut first_provider_id_by_cli_key: HashMap<String, i64> = HashMap::new();
    let mut provider_sort_order_by_cli_key: HashMap<String, i64> = HashMap::new();
    let mut pending_provider_source_links: Vec<(i64, Option<i64>, Option<String>)> = Vec::new();
    let mut providers_imported = 0_u32;

    for provider in providers {
        let sort_order = provider_sort_order_by_cli_key
            .entry(provider.cli_key.clone())
            .or_insert(0);
        let base_urls_json = serde_json::to_string(&provider.base_urls)
            .map_err(|e| format!("SYSTEM_ERROR: failed to serialize base_urls: {e}"))?;
        let base_url_primary = provider.base_urls.first().cloned().unwrap_or_default();

        tx.execute(
            r#"
INSERT INTO providers(
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  auth_mode,
  claude_models_json,
  supported_models_json,
  model_mapping_json,
  api_key_plaintext,
  enabled,
  priority,
  sort_order,
  cost_multiplier,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  tags_json,
  note,
  oauth_provider_type,
  oauth_access_token,
  oauth_refresh_token,
  oauth_token_uri,
  oauth_client_id,
  oauth_client_secret,
  oauth_expires_at,
  oauth_email,
  source_provider_id,
  bridge_type,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}', '{}', ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, NULL, ?30, ?31, ?31)
"#,
            params![
                provider.cli_key,
                provider.name,
                base_url_primary,
                base_urls_json,
                provider.base_url_mode,
                provider.auth_mode,
                provider.claude_models_json,
                provider.api_key_plaintext,
                bool_to_int(provider.enabled),
                provider.priority,
                *sort_order,
                provider.cost_multiplier,
                provider.limit_5h_usd,
                provider.limit_daily_usd,
                provider.daily_reset_mode,
                provider.daily_reset_time,
                provider.limit_weekly_usd,
                provider.limit_monthly_usd,
                provider.limit_total_usd,
                provider.tags_json,
                provider.note,
                provider.oauth_provider_type,
                provider.oauth_access_token,
                provider.oauth_refresh_token,
                provider.oauth_token_uri,
                provider.oauth_client_id,
                provider.oauth_client_secret,
                provider.oauth_token_expiry,
                provider.oauth_email,
                provider.bridge_type,
                now,
            ],
        )
        .map_err(|e| db_err!("failed to insert provider: {e}"))?;

        let provider_id = tx.last_insert_rowid();
        let inserted_cli_key: String = tx
            .query_row(
                "SELECT cli_key FROM providers WHERE id = ?1",
                params![provider_id],
                |row| row.get(0),
            )
            .map_err(|e| db_err!("failed to read inserted provider cli_key: {e}"))?;
        let inserted_name: String = tx
            .query_row(
                "SELECT name FROM providers WHERE id = ?1",
                params![provider_id],
                |row| row.get(0),
            )
            .map_err(|e| db_err!("failed to read inserted provider name: {e}"))?;

        provider_id_by_cli_and_name.insert((inserted_cli_key.clone(), inserted_name), provider_id);
        first_provider_id_by_cli_key
            .entry(inserted_cli_key)
            .or_insert(provider_id);
        // Map old exported ID → new imported ID so source links can be remapped
        if let Some(exported_id) = provider.id {
            provider_id_by_source_id.insert(exported_id, provider_id);
        }
        pending_provider_source_links.push((
            provider_id,
            provider.source_provider_id,
            provider.source_provider_cli_key,
        ));
        *sort_order += 1;
        providers_imported += 1;
    }

    for (provider_id, source_provider_id_exported, source_provider_cli_key) in
        pending_provider_source_links
    {
        let source_provider_id = if let Some(exported_id) = source_provider_id_exported {
            provider_id_by_source_id.get(&exported_id).copied()
        } else {
            None
        };

        let source_provider_id = source_provider_id.or_else(|| {
            source_provider_cli_key
                .as_ref()
                .and_then(|cli_key| first_provider_id_by_cli_key.get(cli_key).copied())
        });

        let Some(source_id) = source_provider_id else {
            continue;
        };

        tx.execute(
            "UPDATE providers SET source_provider_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![source_id, now, provider_id],
        )
        .map_err(|e| db_err!("failed to update provider source_provider_id: {e}"))?;
    }

    let (sort_modes_imported, sort_mode_id_by_name) =
        import_sort_modes(tx, now, sort_modes, &provider_id_by_cli_and_name)?;
    import_sort_mode_active(tx, now, sort_mode_active, &sort_mode_id_by_name)?;
    let (workspaces_imported, workspace_id_by_cli_and_name) =
        import_workspaces(tx, now, workspaces)?;
    let mcp_servers_imported =
        import_mcp_servers(tx, now, mcp_servers, &workspace_id_by_cli_and_name)?;
    let skill_repos_imported = import_skill_repos(tx, now, skill_repos)?;

    Ok(ConfigImportResult {
        providers_imported,
        sort_modes_imported,
        workspaces_imported,
        mcp_servers_imported,
        skill_repos_imported,
    })
}

fn import_sort_modes(
    tx: &Connection,
    now: i64,
    sort_modes: Vec<SortModeExport>,
    provider_id_by_cli_and_name: &HashMap<(String, String), i64>,
) -> AppResult<(u32, HashMap<String, i64>)> {
    let mut imported = 0_u32;
    let mut sort_mode_id_by_name = HashMap::new();

    for sort_mode in sort_modes {
        tx.execute(
            r#"
INSERT INTO sort_modes(name, created_at, updated_at)
VALUES (?1, ?2, ?2)
"#,
            params![sort_mode.name, now],
        )
        .map_err(|e| db_err!("failed to insert sort_mode: {e}"))?;
        let mode_id = tx.last_insert_rowid();
        let mode_name: String = tx
            .query_row(
                "SELECT name FROM sort_modes WHERE id = ?1",
                params![mode_id],
                |row| row.get(0),
            )
            .map_err(|e| db_err!("failed to read inserted sort_mode name: {e}"))?;
        sort_mode_id_by_name.insert(mode_name, mode_id);

        for provider in sort_mode.providers {
            let provider_id = provider_id_by_cli_and_name
                .get(&(provider.cli_key.clone(), provider.provider_cli_key.clone()))
                .copied()
                .ok_or_else(|| {
                    crate::shared::error::AppError::from(format!(
                        "DB_NOT_FOUND: provider not found for sort mode: cli_key={}, provider={}",
                        provider.cli_key, provider.provider_cli_key
                    ))
                })?;

            tx.execute(
                r#"
INSERT INTO sort_mode_providers(
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
"#,
                params![
                    mode_id,
                    provider.cli_key,
                    provider_id,
                    provider.sort_order,
                    bool_to_int(provider.enabled),
                    now,
                ],
            )
            .map_err(|e| db_err!("failed to insert sort_mode_provider: {e}"))?;
        }

        imported += 1;
    }

    Ok((imported, sort_mode_id_by_name))
}

fn import_sort_mode_active(
    tx: &Connection,
    now: i64,
    sort_mode_active: HashMap<String, String>,
    sort_mode_id_by_name: &HashMap<String, i64>,
) -> AppResult<()> {
    for (cli_key, mode_name) in sort_mode_active {
        let mode_id = sort_mode_id_by_name
            .get(&mode_name)
            .copied()
            .ok_or_else(|| {
                crate::shared::error::AppError::from(format!(
                    "DB_NOT_FOUND: active sort mode not found: {mode_name}"
                ))
            })?;
        tx.execute(
            r#"
INSERT INTO sort_mode_active(cli_key, mode_id, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  mode_id = excluded.mode_id,
  updated_at = excluded.updated_at
"#,
            params![cli_key, mode_id, now],
        )
        .map_err(|e| db_err!("failed to insert sort_mode_active: {e}"))?;
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
fn import_workspaces(
    tx: &Connection,
    now: i64,
    workspaces: Vec<WorkspaceExport>,
) -> AppResult<(u32, HashMap<(String, String), i64>)> {
    let mut imported = 0_u32;
    let mut workspace_id_by_cli_and_name = HashMap::new();

    for workspace in workspaces {
        let normalized_name = crate::shared::text::normalize_name(&workspace.name);
        tx.execute(
            r#"
INSERT INTO workspaces(cli_key, name, normalized_name, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
"#,
            params![workspace.cli_key, workspace.name, normalized_name, now],
        )
        .map_err(|e| db_err!("failed to insert workspace: {e}"))?;
        let workspace_id = tx.last_insert_rowid();
        let inserted_cli_key: String = tx
            .query_row(
                "SELECT cli_key FROM workspaces WHERE id = ?1",
                params![workspace_id],
                |row| row.get(0),
            )
            .map_err(|e| db_err!("failed to read inserted workspace cli_key: {e}"))?;
        let inserted_name: String = tx
            .query_row(
                "SELECT name FROM workspaces WHERE id = ?1",
                params![workspace_id],
                |row| row.get(0),
            )
            .map_err(|e| db_err!("failed to read inserted workspace name: {e}"))?;

        workspace_id_by_cli_and_name
            .entry((inserted_cli_key.clone(), inserted_name))
            .or_insert(workspace_id);

        if let Some(prompt) = workspace.prompt {
            tx.execute(
                r#"
INSERT INTO prompts(
  workspace_id,
  name,
  content,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
"#,
                params![
                    workspace_id,
                    prompt.name,
                    prompt.content,
                    bool_to_int(prompt.enabled),
                    now,
                ],
            )
            .map_err(|e| db_err!("failed to insert prompt: {e}"))?;
        }

        if workspace.is_active {
            tx.execute(
                r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
                params![inserted_cli_key, workspace_id, now],
            )
            .map_err(|e| db_err!("failed to insert workspace_active: {e}"))?;
        }

        imported += 1;
    }

    Ok((imported, workspace_id_by_cli_and_name))
}

fn import_mcp_servers(
    tx: &Connection,
    now: i64,
    mcp_servers: Vec<McpServerExport>,
    workspace_id_by_cli_and_name: &HashMap<(String, String), i64>,
) -> AppResult<u32> {
    let mut imported = 0_u32;

    for server in mcp_servers {
        let normalized_name = crate::shared::text::normalize_name(&server.name);
        tx.execute(
            r#"
INSERT INTO mcp_servers(
  server_key,
  name,
  normalized_name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
"#,
            params![
                server.server_key,
                server.name,
                normalized_name,
                server.transport,
                server.command,
                server.args_json,
                server.env_json,
                server.cwd,
                server.url,
                server.headers_json.unwrap_or_else(|| "{}".to_string()),
                now,
            ],
        )
        .map_err(|e| db_err!("failed to insert mcp_server: {e}"))?;
        let server_id = tx.last_insert_rowid();

        for (workspace_cli_key, workspace_name) in server.enabled_in_workspaces {
            let workspace_id = workspace_id_by_cli_and_name
                .get(&(workspace_cli_key.clone(), workspace_name.clone()))
                .copied()
                .ok_or_else(|| {
                    crate::shared::error::AppError::from(format!(
                        "DB_NOT_FOUND: workspace not found for MCP enablement: cli_key={}, workspace={}",
                        workspace_cli_key, workspace_name
                    ))
                })?;
            tx.execute(
                r#"
INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, server_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
                params![workspace_id, server_id, now],
            )
            .map_err(|e| db_err!("failed to insert workspace_mcp_enabled: {e}"))?;
        }

        imported += 1;
    }

    Ok(imported)
}

fn import_skill_repos(
    tx: &Connection,
    now: i64,
    skill_repos: Vec<SkillRepoExport>,
) -> AppResult<u32> {
    let mut imported = 0_u32;
    for repo in skill_repos {
        tx.execute(
            r#"
INSERT INTO skill_repos(git_url, branch, enabled, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
"#,
            params![repo.git_url, repo.branch, bool_to_int(repo.enabled), now],
        )
        .map_err(|e| db_err!("failed to insert skill_repo: {e}"))?;
        imported += 1;
    }
    Ok(imported)
}

fn validate_bundle_schema_version(schema_version: u32) -> AppResult<()> {
    if schema_version != CONFIG_BUNDLE_SCHEMA_VERSION {
        return Err(format!(
            "SEC_INVALID_INPUT: unsupported config bundle schema_version={}, expected={}",
            schema_version, CONFIG_BUNDLE_SCHEMA_VERSION
        )
        .into());
    }
    Ok(())
}

fn query_exported_at(conn: &Connection) -> AppResult<String> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
    .map_err(|e| db_err!("failed to query export timestamp: {e}"))
}

fn load_provider_cli_key_by_id(conn: &Connection) -> AppResult<HashMap<i64, String>> {
    let mut stmt = conn
        .prepare_cached("SELECT id, cli_key FROM providers")
        .map_err(|e| db_err!("failed to prepare provider cli_key query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query provider cli_keys: {e}"))?;

    let mut map = HashMap::new();
    for row in rows {
        let (id, cli_key) = row.map_err(|e| db_err!("failed to read provider cli_key row: {e}"))?;
        map.insert(id, cli_key);
    }
    Ok(map)
}

fn export_providers(
    conn: &Connection,
    provider_cli_key_by_id: &HashMap<i64, String>,
) -> AppResult<Vec<ProviderExport>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  auth_mode,
  oauth_provider_type,
  oauth_access_token,
  oauth_refresh_token,
  oauth_expires_at,
  oauth_token_uri,
  oauth_client_id,
  oauth_client_secret,
  oauth_email,
  claude_models_json,
  enabled,
  priority,
  cost_multiplier,
  limit_5h_usd,
  limit_daily_usd,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  daily_reset_mode,
  daily_reset_time,
  tags_json,
  note,
  source_provider_id,
  bridge_type
FROM providers
ORDER BY cli_key ASC, sort_order ASC, id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare providers export query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let base_url: String = row.get("base_url")?;
            let base_urls_json: String = row.get("base_urls_json")?;
            let mut base_urls =
                serde_json::from_str::<Vec<String>>(&base_urls_json).unwrap_or_default();
            base_urls.retain(|value| !value.trim().is_empty());
            if base_urls.is_empty() && !base_url.trim().is_empty() {
                base_urls.push(base_url);
            }

            Ok(ProviderExport {
                id: row.get("id")?,
                cli_key: row.get("cli_key")?,
                name: row.get("name")?,
                base_urls,
                base_url_mode: row.get("base_url_mode")?,
                api_key_plaintext: row.get("api_key_plaintext")?,
                auth_mode: row
                    .get::<_, Option<String>>("auth_mode")?
                    .unwrap_or_else(|| "api_key".to_string()),
                oauth_provider_type: row.get("oauth_provider_type")?,
                oauth_access_token: row.get("oauth_access_token")?,
                oauth_refresh_token: row.get("oauth_refresh_token")?,
                oauth_token_expiry: row.get("oauth_expires_at")?,
                oauth_scopes: None,
                oauth_token_uri: row.get("oauth_token_uri")?,
                oauth_client_id: row.get("oauth_client_id")?,
                oauth_client_secret: row.get("oauth_client_secret")?,
                oauth_email: row.get("oauth_email")?,
                claude_models_json: row.get("claude_models_json")?,
                enabled: row.get::<_, i64>("enabled")? != 0,
                priority: row.get("priority")?,
                cost_multiplier: row.get("cost_multiplier")?,
                limit_5h_usd: row.get("limit_5h_usd")?,
                limit_daily_usd: row.get("limit_daily_usd")?,
                limit_weekly_usd: row.get("limit_weekly_usd")?,
                limit_monthly_usd: row.get("limit_monthly_usd")?,
                limit_total_usd: row.get("limit_total_usd")?,
                daily_reset_mode: row.get("daily_reset_mode")?,
                daily_reset_time: row.get("daily_reset_time")?,
                tags_json: row.get("tags_json")?,
                note: row.get("note")?,
                source_provider_id: row.get("source_provider_id")?,
                source_provider_cli_key: row
                    .get::<_, Option<i64>>("source_provider_id")?
                    .and_then(|source_id| provider_cli_key_by_id.get(&source_id).cloned()),
                bridge_type: row.get("bridge_type")?,
            })
        })
        .map_err(|e| db_err!("failed to query providers for export: {e}"))?;

    let mut providers = Vec::new();
    for row in rows {
        providers.push(row.map_err(|e| db_err!("failed to read provider export row: {e}"))?);
    }
    Ok(providers)
}

fn export_sort_modes(conn: &Connection) -> AppResult<Vec<SortModeExport>> {
    let mut stmt = conn
        .prepare_cached("SELECT id, name FROM sort_modes ORDER BY id ASC")
        .map_err(|e| db_err!("failed to prepare sort_modes export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query sort_modes for export: {e}"))?;

    let mut modes = Vec::new();
    for row in rows {
        let (mode_id, name) = row.map_err(|e| db_err!("failed to read sort_mode row: {e}"))?;
        modes.push(SortModeExport {
            name,
            is_default: false,
            providers: export_sort_mode_providers(conn, mode_id)?,
        });
    }
    Ok(modes)
}

fn export_sort_mode_providers(
    conn: &Connection,
    mode_id: i64,
) -> AppResult<Vec<SortModeProviderExport>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  mp.cli_key,
  p.name,
  mp.sort_order,
  mp.enabled
FROM sort_mode_providers mp
JOIN providers p ON p.id = mp.provider_id
WHERE mp.mode_id = ?1
ORDER BY mp.sort_order ASC, p.id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare sort_mode_providers export query: {e}"))?;
    let rows = stmt
        .query_map(params![mode_id], |row| {
            Ok(SortModeProviderExport {
                cli_key: row.get(0)?,
                // Historical field name in bundle schema; stores provider name.
                provider_cli_key: row.get(1)?,
                sort_order: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| db_err!("failed to query sort_mode_providers for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read sort_mode_provider row: {e}"))?);
    }
    Ok(items)
}

fn export_sort_mode_active(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT a.cli_key, m.name
FROM sort_mode_active a
JOIN sort_modes m ON m.id = a.mode_id
ORDER BY a.cli_key ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare sort_mode_active export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query sort_mode_active for export: {e}"))?;

    let mut items = HashMap::new();
    for row in rows {
        let (cli_key, mode_name) =
            row.map_err(|e| db_err!("failed to read sort_mode_active row: {e}"))?;
        items.insert(cli_key, mode_name);
    }
    Ok(items)
}

fn export_workspaces(conn: &Connection) -> AppResult<Vec<WorkspaceExport>> {
    let active_by_cli = load_active_workspace_ids(conn)?;
    let mut stmt = conn
        .prepare_cached("SELECT id, cli_key, name FROM workspaces ORDER BY id ASC")
        .map_err(|e| db_err!("failed to prepare workspaces export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| db_err!("failed to query workspaces for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (workspace_id, cli_key, name) =
            row.map_err(|e| db_err!("failed to read workspace export row: {e}"))?;
        items.push(WorkspaceExport {
            cli_key: cli_key.clone(),
            name,
            is_active: active_by_cli.get(&cli_key).copied() == Some(workspace_id),
            prompt: export_workspace_prompt(conn, workspace_id)?,
        });
    }
    Ok(items)
}

fn load_active_workspace_ids(conn: &Connection) -> AppResult<HashMap<String, i64>> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT cli_key, workspace_id FROM workspace_active WHERE workspace_id IS NOT NULL",
        )
        .map_err(|e| db_err!("failed to prepare workspace_active export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| db_err!("failed to query workspace_active for export: {e}"))?;

    let mut map = HashMap::new();
    for row in rows {
        let (cli_key, workspace_id) =
            row.map_err(|e| db_err!("failed to read workspace_active row: {e}"))?;
        map.insert(cli_key, workspace_id);
    }
    Ok(map)
}

fn export_workspace_prompt(
    conn: &Connection,
    workspace_id: i64,
) -> AppResult<Option<PromptExport>> {
    conn.query_row(
        r#"
SELECT name, content, enabled
FROM prompts
WHERE workspace_id = ?1
ORDER BY enabled DESC, updated_at DESC, id DESC
LIMIT 1
"#,
        params![workspace_id],
        |row| {
            Ok(PromptExport {
                name: row.get(0)?,
                content: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
            })
        },
    )
    .optional()
    .map_err(|e| db_err!("failed to query workspace prompt for export: {e}"))
}

fn export_mcp_servers(conn: &Connection) -> AppResult<Vec<McpServerExport>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  server_key,
  name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json
FROM mcp_servers
ORDER BY id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare mcp_servers export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .map_err(|e| db_err!("failed to query mcp_servers for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (
            server_id,
            server_key,
            name,
            transport,
            command,
            args_json,
            env_json,
            cwd,
            url,
            headers_json,
        ) = row.map_err(|e| db_err!("failed to read mcp_server export row: {e}"))?;
        items.push(McpServerExport {
            server_key,
            name,
            transport,
            command,
            args_json,
            env_json,
            cwd,
            url,
            headers_json: Some(headers_json),
            enabled_in_workspaces: export_enabled_mcp_workspaces(conn, server_id)?,
        });
    }
    Ok(items)
}

fn export_enabled_mcp_workspaces(
    conn: &Connection,
    server_id: i64,
) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT w.cli_key, w.name
FROM workspace_mcp_enabled e
JOIN workspaces w ON w.id = e.workspace_id
WHERE e.server_id = ?1
ORDER BY w.cli_key ASC, w.name ASC, w.id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare workspace_mcp_enabled export query: {e}"))?;
    let rows = stmt
        .query_map(params![server_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query workspace_mcp_enabled for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read enabled MCP workspace row: {e}"))?);
    }
    Ok(items)
}

fn export_skill_repos(conn: &Connection) -> AppResult<Vec<SkillRepoExport>> {
    let mut stmt = conn
        .prepare_cached("SELECT git_url, branch, enabled FROM skill_repos ORDER BY id ASC")
        .map_err(|e| db_err!("failed to prepare skill_repos export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SkillRepoExport {
                git_url: row.get(0)?,
                branch: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
            })
        })
        .map_err(|e| db_err!("failed to query skill_repos for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read skill_repo export row: {e}"))?);
    }
    Ok(items)
}

fn clear_existing_config_data(conn: &Connection) -> AppResult<()> {
    for statement in [
        "DELETE FROM workspace_mcp_enabled",
        "DELETE FROM sort_mode_providers",
        "DELETE FROM sort_mode_active",
        "DELETE FROM prompts",
        "DELETE FROM workspaces",
        "DELETE FROM workspace_active",
        "DELETE FROM mcp_servers",
        "DELETE FROM sort_modes",
        "DELETE FROM provider_circuit_breakers",
        "DELETE FROM providers",
        "DELETE FROM skill_repos",
    ] {
        conn.execute(statement, [])
            .map_err(|e| db_err!("failed to clear table with '{statement}': {e}"))?;
    }
    Ok(())
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_bundle_schema_version_accepts_current_version() {
        assert!(validate_bundle_schema_version(CONFIG_BUNDLE_SCHEMA_VERSION).is_ok());
    }

    #[test]
    fn validate_bundle_schema_version_rejects_mismatch() {
        let err = validate_bundle_schema_version(CONFIG_BUNDLE_SCHEMA_VERSION + 1)
            .expect_err("schema version should fail");
        assert!(err
            .to_string()
            .contains("SEC_INVALID_INPUT: unsupported config bundle schema_version"));
    }
}
