//! Usage: Sync enabled MCP servers to supported CLI config files.

use crate::mcp_sync;
use crate::shared::error::db_err;
use crate::workspaces;
use rusqlite::Connection;
use std::collections::BTreeMap;

use super::cli_specs::{validate_cli_key, MCP_CLI_KEYS};

pub(crate) fn list_enabled_for_cli(
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<mcp_sync::McpServerForSync>> {
    validate_cli_key(cli_key)?;

    let Some(workspace_id) = workspaces::active_id_by_cli(conn, cli_key)? else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      s.server_key,
      s.transport,
      s.command,
      s.args_json,
      s.env_json,
      s.cwd,
      s.url,
      s.headers_json
    FROM mcp_servers s
    JOIN workspace_mcp_enabled e
      ON e.server_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.server_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| {
            let args_json: String = row.get("args_json")?;
            let env_json: String = row.get("env_json")?;
            let headers_json: String = row.get("headers_json")?;

            let args = serde_json::from_str::<Vec<String>>(&args_json).unwrap_or_default();
            let env =
                serde_json::from_str::<BTreeMap<String, String>>(&env_json).unwrap_or_default();
            let headers =
                serde_json::from_str::<BTreeMap<String, String>>(&headers_json).unwrap_or_default();

            Ok(mcp_sync::McpServerForSync {
                server_key: row.get("server_key")?,
                transport: row.get("transport")?,
                command: row.get("command")?,
                args,
                env,
                cwd: row.get("cwd")?,
                url: row.get("url")?,
                headers,
            })
        })
        .map_err(|e| db_err!("failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

pub(crate) fn list_enabled_for_workspace(
    conn: &Connection,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<mcp_sync::McpServerForSync>> {
    let _cli_key = workspaces::get_cli_key_by_id(conn, workspace_id)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      s.server_key,
      s.transport,
      s.command,
      s.args_json,
      s.env_json,
      s.cwd,
      s.url,
      s.headers_json
    FROM mcp_servers s
    JOIN workspace_mcp_enabled e
      ON e.server_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.server_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| {
            let args_json: String = row.get("args_json")?;
            let env_json: String = row.get("env_json")?;
            let headers_json: String = row.get("headers_json")?;

            let args = serde_json::from_str::<Vec<String>>(&args_json).unwrap_or_default();
            let env =
                serde_json::from_str::<BTreeMap<String, String>>(&env_json).unwrap_or_default();
            let headers =
                serde_json::from_str::<BTreeMap<String, String>>(&headers_json).unwrap_or_default();

            Ok(mcp_sync::McpServerForSync {
                server_key: row.get("server_key")?,
                transport: row.get("transport")?,
                command: row.get("command")?,
                args,
                env,
                cwd: row.get("cwd")?,
                url: row.get("url")?,
                headers,
            })
        })
        .map_err(|e| db_err!("failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

pub(crate) fn sync_cli_for_workspace<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    let cli_key = workspaces::get_cli_key_by_id(conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    let servers = list_enabled_for_workspace(conn, workspace_id)?;
    mcp_sync::sync_cli(app, &cli_key, &servers)?;
    Ok(())
}

pub(super) fn sync_all_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
) -> crate::shared::error::AppResult<()> {
    for cli_key in MCP_CLI_KEYS.iter().copied() {
        sync_one_cli(app, conn, cli_key)?;
    }

    Ok(())
}

pub(crate) fn sync_one_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<()> {
    let servers = list_enabled_for_cli(conn, cli_key)?;
    mcp_sync::sync_cli(app, cli_key, &servers)?;
    Ok(())
}
