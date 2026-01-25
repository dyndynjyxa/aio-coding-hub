//! Usage: Sync enabled MCP servers to supported CLI config files.

use crate::mcp_sync;
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) fn list_enabled_for_cli(
    conn: &Connection,
    cli_key: &str,
) -> Result<Vec<mcp_sync::McpServerForSync>, String> {
    let (col, _) = match cli_key {
        "claude" => ("enabled_claude", ".claude.json"),
        "codex" => ("enabled_codex", ".codex/config.toml"),
        "gemini" => ("enabled_gemini", ".gemini/settings.json"),
        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    };

    let sql = format!(
        r#"
SELECT
  server_key,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json
FROM mcp_servers
WHERE {col} = 1
ORDER BY server_key ASC
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
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
        .map_err(|e| format!("DB_ERROR: failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

pub(super) fn sync_all_cli(app: &tauri::AppHandle, conn: &Connection) -> Result<(), String> {
    let claude = list_enabled_for_cli(conn, "claude")?;
    mcp_sync::sync_cli(app, "claude", &claude)?;

    let codex = list_enabled_for_cli(conn, "codex")?;
    mcp_sync::sync_cli(app, "codex", &codex)?;

    let gemini = list_enabled_for_cli(conn, "gemini")?;
    mcp_sync::sync_cli(app, "gemini", &gemini)?;

    Ok(())
}

pub(super) fn sync_one_cli(
    app: &tauri::AppHandle,
    conn: &Connection,
    cli_key: &str,
) -> Result<(), String> {
    let servers = list_enabled_for_cli(conn, cli_key)?;
    mcp_sync::sync_cli(app, cli_key, &servers)?;
    Ok(())
}
