//! Config export: read DB tables and skill files into a ConfigBundle.

use crate::shared::error::{db_err, AppResult};
use rusqlite::{params, Connection};
use std::collections::HashMap;

use super::normalize_oauth_refresh_lead_seconds;
use super::skill_fs::{
    cli_skills_root, export_skill_dir_files, local_skill_dirs, parse_skill_md_metadata,
    read_local_skill_source_metadata, ssot_skills_root,
};
use super::{
    InstalledSkillExport, LocalSkillExport, McpServerExport, PromptExport, ProviderExport,
    SkillRepoExport, SortModeExport, SortModeProviderExport, WorkspaceExport,
};

pub(super) fn query_exported_at(conn: &Connection) -> AppResult<String> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
    .map_err(|e| db_err!("failed to query export timestamp: {e}"))
}

pub(super) fn load_provider_cli_key_by_id(conn: &Connection) -> AppResult<HashMap<i64, String>> {
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

pub(super) fn export_providers(
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
  oauth_id_token,
  oauth_expires_at,
  oauth_token_uri,
  oauth_client_id,
  oauth_client_secret,
  oauth_email,
  oauth_refresh_lead_s,
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
                oauth_id_token: row.get("oauth_id_token")?,
                oauth_token_expiry: row.get("oauth_expires_at")?,
                oauth_scopes: None,
                oauth_token_uri: row.get("oauth_token_uri")?,
                oauth_client_id: row.get("oauth_client_id")?,
                oauth_client_secret: row.get("oauth_client_secret")?,
                oauth_email: row.get("oauth_email")?,
                oauth_refresh_lead_seconds: normalize_oauth_refresh_lead_seconds(
                    row.get::<_, i64>("oauth_refresh_lead_s")?,
                ),
                oauth_last_refreshed_at: row.get("oauth_last_refreshed_at")?,
                oauth_last_error: row.get("oauth_last_error")?,
                claude_models_json: row.get("claude_models_json")?,
                supported_models_json: row.get("supported_models_json")?,
                model_mapping_json: row.get("model_mapping_json")?,
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

pub(super) fn export_sort_modes(conn: &Connection) -> AppResult<Vec<SortModeExport>> {
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

pub(super) fn export_sort_mode_active(conn: &Connection) -> AppResult<HashMap<String, String>> {
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

pub(super) fn export_workspaces(conn: &Connection) -> AppResult<Vec<WorkspaceExport>> {
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
            prompts: export_workspace_prompts(conn, workspace_id)?,
            prompt: None,
        });
    }
    Ok(items)
}

pub(super) fn load_active_workspace_ids(conn: &Connection) -> AppResult<HashMap<String, i64>> {
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

fn export_workspace_prompts(conn: &Connection, workspace_id: i64) -> AppResult<Vec<PromptExport>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT name, content, enabled
FROM prompts
WHERE workspace_id = ?1
ORDER BY id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare workspace prompts export query: {e}"))?;
    let rows = stmt
        .query_map(params![workspace_id], |row| {
            Ok(PromptExport {
                name: row.get(0)?,
                content: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
            })
        })
        .map_err(|e| db_err!("failed to query workspace prompts for export: {e}"))?;

    let mut prompts = Vec::new();
    for row in rows {
        prompts.push(row.map_err(|e| db_err!("failed to read workspace prompt row: {e}"))?);
    }
    Ok(prompts)
}

pub(super) fn export_mcp_servers(conn: &Connection) -> AppResult<Vec<McpServerExport>> {
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

pub(super) fn export_skill_repos(conn: &Connection) -> AppResult<Vec<SkillRepoExport>> {
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

pub(super) fn export_installed_skills<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
) -> AppResult<Vec<InstalledSkillExport>> {
    let ssot_root = ssot_skills_root(app)?;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT id, skill_key, name, description, source_git_url, source_branch, source_subdir
FROM skills
ORDER BY id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare skills export query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| db_err!("failed to query skills for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (skill_id, skill_key, name, description, source_git_url, source_branch, source_subdir) =
            row.map_err(|e| db_err!("failed to read skill export row: {e}"))?;
        let skill_dir = ssot_root.join(&skill_key);
        if !skill_dir.is_dir() {
            return Err(format!(
                "SKILL_EXPORT_MISSING_SSOT_DIR: missing installed skill dir {}",
                skill_dir.display()
            )
            .into());
        }
        items.push(InstalledSkillExport {
            skill_key,
            name,
            description,
            source_git_url,
            source_branch,
            source_subdir,
            enabled_in_workspaces: export_enabled_skill_workspaces(conn, skill_id)?,
            files: export_skill_dir_files(&skill_dir, true)?,
        });
    }
    Ok(items)
}

fn export_enabled_skill_workspaces(
    conn: &Connection,
    skill_id: i64,
) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT w.cli_key, w.name
FROM workspace_skill_enabled e
JOIN workspaces w ON w.id = e.workspace_id
WHERE e.skill_id = ?1
ORDER BY w.cli_key ASC, w.name ASC, w.id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare workspace_skill_enabled export query: {e}"))?;
    let rows = stmt
        .query_map(params![skill_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query workspace_skill_enabled for export: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read enabled skill workspace row: {e}"))?);
    }
    Ok(items)
}

pub(super) fn export_local_skills<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<Vec<LocalSkillExport>> {
    let mut items = Vec::new();

    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        let root = cli_skills_root(app, cli_key)?;
        if !root.exists() {
            continue;
        }

        for path in local_skill_dirs(&root)? {
            let dir_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    format!(
                        "SKILL_EXPORT_INVALID_DIR_NAME: local skill dir name invalid: {}",
                        path.display()
                    )
                })?;
            let (name, description) = match parse_skill_md_metadata(&path.join("SKILL.md")) {
                Ok(value) => value,
                Err(_) => (dir_name.clone(), String::new()),
            };
            let source = read_local_skill_source_metadata(&path)?;

            items.push(LocalSkillExport {
                cli_key: cli_key.to_string(),
                dir_name,
                name,
                description,
                source_git_url: source.as_ref().map(|value| value.source_git_url.clone()),
                source_branch: source.as_ref().map(|value| value.source_branch.clone()),
                source_subdir: source.as_ref().map(|value| value.source_subdir.clone()),
                files: export_skill_dir_files(&path, true)?,
            });
        }
    }

    items.sort_by(|a, b| a.cli_key.cmp(&b.cli_key).then(a.dir_name.cmp(&b.dir_name)));
    Ok(items)
}
