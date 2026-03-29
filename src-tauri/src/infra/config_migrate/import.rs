//! Config import: write a ConfigBundle into DB tables.

use crate::shared::error::{db_err, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

use super::export::load_active_workspace_ids;
use super::{
    bool_to_int, normalize_oauth_refresh_lead_seconds, prompts_for_import, ConfigImportResult,
    InstalledSkillExport, LocalSkillExport, McpServerExport, ProviderExport, SkillRepoExport,
    SortModeExport, WorkspaceExport, CONFIG_BUNDLE_SCHEMA_VERSION,
};

#[derive(Debug, Default)]
pub(super) struct LegacySkillState {
    pub(super) enabled_skill_keys_by_workspace: HashMap<(String, String), Vec<String>>,
    pub(super) active_workspace_skill_keys_by_cli: HashMap<String, Vec<String>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn import_into_transaction(
    tx: &Connection,
    now: i64,
    providers: Vec<ProviderExport>,
    sort_modes: Vec<SortModeExport>,
    sort_mode_active: HashMap<String, String>,
    workspaces: Vec<WorkspaceExport>,
    mcp_servers: Vec<McpServerExport>,
    skill_repos: Vec<SkillRepoExport>,
    imports_full_skill_payload: bool,
    installed_skills: &[InstalledSkillExport],
    local_skills: &[LocalSkillExport],
    legacy_skill_state: Option<&LegacySkillState>,
) -> AppResult<ConfigImportResult> {
    let mut provider_id_by_cli_and_name: HashMap<(String, String), i64> = HashMap::new();
    let mut provider_id_by_source_id: HashMap<i64, i64> = HashMap::new();
    let mut first_provider_id_by_cli_key: HashMap<String, i64> = HashMap::new();
    let mut provider_sort_order_by_cli_key: HashMap<String, i64> = HashMap::new();
    let mut pending_provider_source_links: Vec<(i64, Option<i64>, Option<String>)> = Vec::new();
    let mut providers_imported = 0_u32;

    for provider in providers {
        let ProviderExport {
            id,
            cli_key,
            name,
            base_urls,
            base_url_mode,
            api_key_plaintext,
            auth_mode,
            oauth_provider_type,
            oauth_access_token,
            oauth_refresh_token,
            oauth_id_token,
            oauth_token_expiry,
            oauth_scopes: _,
            oauth_token_uri,
            oauth_client_id,
            oauth_client_secret,
            oauth_email,
            oauth_refresh_lead_seconds,
            oauth_last_refreshed_at,
            oauth_last_error,
            claude_models_json,
            supported_models_json,
            model_mapping_json,
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
            source_provider_cli_key,
            bridge_type,
        } = provider;

        let sort_order = provider_sort_order_by_cli_key
            .entry(cli_key.clone())
            .or_insert(0);
        let base_urls_json = serde_json::to_string(&base_urls)
            .map_err(|e| format!("SYSTEM_ERROR: failed to serialize base_urls: {e}"))?;
        let base_url_primary = base_urls.first().cloned().unwrap_or_default();

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
  oauth_id_token,
  oauth_token_uri,
  oauth_client_id,
  oauth_client_secret,
  oauth_expires_at,
  oauth_email,
  oauth_refresh_lead_s,
  oauth_last_refreshed_at,
  oauth_last_error,
  source_provider_id,
  bridge_type,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, NULL, ?36, ?37, ?37)
"#,
            params![
                cli_key,
                name,
                base_url_primary,
                base_urls_json,
                base_url_mode,
                auth_mode,
                claude_models_json,
                supported_models_json,
                model_mapping_json,
                api_key_plaintext,
                bool_to_int(enabled),
                priority,
                *sort_order,
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
                oauth_id_token,
                oauth_token_uri,
                oauth_client_id,
                oauth_client_secret,
                oauth_token_expiry,
                oauth_email,
                normalize_oauth_refresh_lead_seconds(oauth_refresh_lead_seconds),
                oauth_last_refreshed_at,
                oauth_last_error,
                bridge_type,
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
        if let Some(exported_id) = id {
            provider_id_by_source_id.insert(exported_id, provider_id);
        }
        pending_provider_source_links.push((
            provider_id,
            source_provider_id,
            source_provider_cli_key,
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
    let (workspaces_imported, prompts_imported, workspace_id_by_cli_and_name) =
        import_workspaces(tx, now, workspaces)?;
    let mcp_servers_imported =
        import_mcp_servers(tx, now, mcp_servers, &workspace_id_by_cli_and_name)?;
    let skill_repos_imported = import_skill_repos(tx, now, skill_repos)?;
    let (installed_skills_imported, local_skills_imported) = if imports_full_skill_payload {
        (
            import_installed_skills(tx, now, installed_skills, &workspace_id_by_cli_and_name)?,
            local_skills.len() as u32,
        )
    } else {
        if let Some(legacy_skill_state) = legacy_skill_state {
            restore_legacy_skill_state(tx, now, legacy_skill_state, &workspace_id_by_cli_and_name)?;
        }
        (0, 0)
    };

    Ok(ConfigImportResult {
        providers_imported,
        sort_modes_imported,
        workspaces_imported,
        prompts_imported,
        mcp_servers_imported,
        skill_repos_imported,
        installed_skills_imported,
        local_skills_imported,
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
) -> AppResult<(u32, u32, HashMap<(String, String), i64>)> {
    let mut imported = 0_u32;
    let mut prompts_imported = 0_u32;
    let mut workspace_id_by_cli_and_name = HashMap::new();

    for workspace in workspaces {
        let WorkspaceExport {
            cli_key,
            name,
            is_active,
            prompts,
            prompt,
        } = workspace;
        let normalized_name = crate::shared::text::normalize_name(&name);
        tx.execute(
            r#"
INSERT INTO workspaces(cli_key, name, normalized_name, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
"#,
            params![cli_key, name, normalized_name, now],
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

        let prompts = prompts_for_import(prompts, prompt);
        for prompt in prompts {
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
            prompts_imported += 1;
        }

        if workspace_is_active(tx, &inserted_cli_key, workspace_id, is_active)? {
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

    Ok((imported, prompts_imported, workspace_id_by_cli_and_name))
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

fn import_installed_skills(
    tx: &Connection,
    now: i64,
    installed_skills: &[InstalledSkillExport],
    workspace_id_by_cli_and_name: &HashMap<(String, String), i64>,
) -> AppResult<u32> {
    let mut imported = 0_u32;
    let mut seen_skill_keys = HashSet::new();

    for skill in installed_skills {
        let skill_key = skill.skill_key.trim();
        if skill_key.is_empty() {
            return Err("SEC_INVALID_INPUT: installed skill_key is required"
                .to_string()
                .into());
        }
        if !seen_skill_keys.insert(skill_key.to_string()) {
            return Err(
                format!("SEC_INVALID_INPUT: duplicate installed skill_key={skill_key}").into(),
            );
        }

        let normalized_name = crate::shared::text::normalize_name(&skill.name);
        tx.execute(
            r#"
INSERT INTO skills(
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
"#,
            params![
                skill_key,
                &skill.name,
                normalized_name,
                &skill.description,
                &skill.source_git_url,
                &skill.source_branch,
                &skill.source_subdir,
                now,
            ],
        )
        .map_err(|e| db_err!("failed to insert installed skill: {e}"))?;
        let skill_id = tx.last_insert_rowid();

        for (workspace_cli_key, workspace_name) in &skill.enabled_in_workspaces {
            let workspace_id = workspace_id_by_cli_and_name
                .get(&(workspace_cli_key.clone(), workspace_name.clone()))
                .copied()
                .ok_or_else(|| {
                    crate::shared::error::AppError::from(format!(
                        "DB_NOT_FOUND: workspace not found for skill enablement: cli_key={}, workspace={}",
                        workspace_cli_key, workspace_name
                    ))
                })?;
            tx.execute(
                r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, skill_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
                params![workspace_id, skill_id, now],
            )
            .map_err(|e| db_err!("failed to insert workspace_skill_enabled: {e}"))?;
        }

        imported += 1;
    }

    Ok(imported)
}

pub(super) fn clear_existing_config_data(conn: &Connection, clear_skills: bool) -> AppResult<()> {
    let mut statements = vec![
        "DELETE FROM workspace_mcp_enabled",
        "DELETE FROM sort_mode_providers",
        "DELETE FROM sort_mode_active",
        "DELETE FROM prompts",
        "DELETE FROM workspace_active",
        "DELETE FROM mcp_servers",
        "DELETE FROM sort_modes",
        "DELETE FROM provider_circuit_breakers",
        "DELETE FROM providers",
        "DELETE FROM workspaces",
        "DELETE FROM skill_repos",
    ];
    if clear_skills {
        statements.insert(0, "DELETE FROM workspace_skill_enabled");
        statements.insert(1, "DELETE FROM skills");
    }

    for statement in statements {
        conn.execute(statement, [])
            .map_err(|e| db_err!("failed to clear table with '{statement}': {e}"))?;
    }
    Ok(())
}

fn workspace_is_active(
    tx: &Connection,
    cli_key: &str,
    workspace_id: i64,
    wants_active: bool,
) -> AppResult<bool> {
    if !wants_active {
        return Ok(false);
    }

    let existing: Option<i64> = tx
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = ?1",
            params![cli_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query workspace_active during import: {e}"))?;

    Ok(existing.is_none() || existing == Some(workspace_id))
}

pub(super) fn resolve_skill_payloads_for_import(
    schema_version: u32,
    installed_skills: Option<Vec<InstalledSkillExport>>,
    local_skills: Option<Vec<LocalSkillExport>>,
) -> AppResult<(Vec<InstalledSkillExport>, Vec<LocalSkillExport>)> {
    if schema_version >= CONFIG_BUNDLE_SCHEMA_VERSION {
        let installed_skills = installed_skills.ok_or_else(|| {
            crate::shared::error::AppError::from(
                "SEC_INVALID_INPUT: config bundle missing installed_skills for schema_version=2",
            )
        })?;
        let local_skills = local_skills.ok_or_else(|| {
            crate::shared::error::AppError::from(
                "SEC_INVALID_INPUT: config bundle missing local_skills for schema_version=2",
            )
        })?;
        Ok((installed_skills, local_skills))
    } else {
        Ok((
            installed_skills.unwrap_or_default(),
            local_skills.unwrap_or_default(),
        ))
    }
}

pub(super) fn validate_local_skills_for_import(local_skills: &[LocalSkillExport]) -> AppResult<()> {
    for local_skill in local_skills {
        crate::shared::cli_key::validate_cli_key(local_skill.cli_key.trim()).map_err(|_| {
            crate::shared::error::AppError::from(format!(
                "SEC_INVALID_INPUT: unknown local skill cli_key={}",
                local_skill.cli_key
            ))
        })?;
    }
    Ok(())
}

pub(super) fn capture_legacy_skill_state(conn: &Connection) -> AppResult<LegacySkillState> {
    let mut state = LegacySkillState::default();

    let mut enabled_stmt = conn
        .prepare_cached(
            r#"
SELECT w.cli_key, w.name, s.skill_key
FROM workspace_skill_enabled e
JOIN workspaces w ON w.id = e.workspace_id
JOIN skills s ON s.id = e.skill_id
ORDER BY w.cli_key ASC, w.name ASC, s.skill_key ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare legacy skill state query: {e}"))?;
    let enabled_rows = enabled_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| db_err!("failed to query legacy skill state: {e}"))?;

    for row in enabled_rows {
        let (cli_key, workspace_name, skill_key) =
            row.map_err(|e| db_err!("failed to read legacy skill state row: {e}"))?;
        state
            .enabled_skill_keys_by_workspace
            .entry((cli_key, workspace_name))
            .or_default()
            .push(skill_key);
    }

    let mut active_stmt = conn
        .prepare_cached(
            r#"
SELECT a.cli_key, s.skill_key
FROM workspace_active a
JOIN workspace_skill_enabled e ON e.workspace_id = a.workspace_id
JOIN skills s ON s.id = e.skill_id
ORDER BY a.cli_key ASC, s.skill_key ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare legacy active skill query: {e}"))?;
    let active_rows = active_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query legacy active skill state: {e}"))?;

    for row in active_rows {
        let (cli_key, skill_key) =
            row.map_err(|e| db_err!("failed to read legacy active skill row: {e}"))?;
        state
            .active_workspace_skill_keys_by_cli
            .entry(cli_key)
            .or_default()
            .push(skill_key);
    }

    Ok(state)
}

fn load_skill_id_by_key(conn: &Connection) -> AppResult<HashMap<String, i64>> {
    let mut stmt = conn
        .prepare_cached("SELECT skill_key, id FROM skills ORDER BY id ASC")
        .map_err(|e| db_err!("failed to prepare skill id query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| db_err!("failed to query skill ids: {e}"))?;

    let mut skill_id_by_key = HashMap::new();
    for row in rows {
        let (skill_key, skill_id) = row.map_err(|e| db_err!("failed to read skill id row: {e}"))?;
        skill_id_by_key.insert(skill_key, skill_id);
    }
    Ok(skill_id_by_key)
}

fn insert_skill_enablements(
    conn: &Connection,
    now: i64,
    workspace_id: i64,
    skill_keys: &[String],
    skill_id_by_key: &HashMap<String, i64>,
) -> AppResult<()> {
    for skill_key in skill_keys {
        let Some(skill_id) = skill_id_by_key.get(skill_key).copied() else {
            continue;
        };
        conn.execute(
            r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, skill_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
            params![workspace_id, skill_id, now],
        )
        .map_err(|e| db_err!("failed to restore workspace_skill_enabled: {e}"))?;
    }
    Ok(())
}

fn restore_legacy_skill_state(
    conn: &Connection,
    now: i64,
    legacy_skill_state: &LegacySkillState,
    workspace_id_by_cli_and_name: &HashMap<(String, String), i64>,
) -> AppResult<()> {
    let skill_id_by_key = load_skill_id_by_key(conn)?;

    for ((cli_key, workspace_name), skill_keys) in
        &legacy_skill_state.enabled_skill_keys_by_workspace
    {
        let Some(workspace_id) = workspace_id_by_cli_and_name
            .get(&(cli_key.clone(), workspace_name.clone()))
            .copied()
        else {
            continue;
        };
        insert_skill_enablements(conn, now, workspace_id, skill_keys, &skill_id_by_key)?;
    }

    let active_workspace_ids = load_active_workspace_ids(conn)?;
    for (cli_key, skill_keys) in &legacy_skill_state.active_workspace_skill_keys_by_cli {
        let Some(workspace_id) = active_workspace_ids.get(cli_key).copied() else {
            continue;
        };
        insert_skill_enablements(conn, now, workspace_id, skill_keys, &skill_id_by_key)?;
    }

    Ok(())
}
