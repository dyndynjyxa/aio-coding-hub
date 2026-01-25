//! Usage: MCP server persistence (SQLite) and sync integration hooks.

use crate::shared::time::now_unix_seconds;
use crate::{db, mcp_sync};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::collections::BTreeMap;

use super::sync::{sync_all_cli, sync_one_cli};
use super::types::{McpImportServer, McpServerSummary};
use super::validate::{
    enabled_to_int, normalize_name, suggest_key, validate_cli_key, validate_server_key,
    validate_transport,
};

/// CLI file backup snapshots for rollback on error.
struct CliBackupSnapshots {
    claude: (Option<Vec<u8>>, Option<Vec<u8>>),
    codex: (Option<Vec<u8>>, Option<Vec<u8>>),
    gemini: (Option<Vec<u8>>, Option<Vec<u8>>),
}

impl CliBackupSnapshots {
    fn capture_all(app: &tauri::AppHandle) -> Result<Self, String> {
        Ok(Self {
            claude: (
                mcp_sync::read_target_bytes(app, "claude")?,
                mcp_sync::read_manifest_bytes(app, "claude")?,
            ),
            codex: (
                mcp_sync::read_target_bytes(app, "codex")?,
                mcp_sync::read_manifest_bytes(app, "codex")?,
            ),
            gemini: (
                mcp_sync::read_target_bytes(app, "gemini")?,
                mcp_sync::read_manifest_bytes(app, "gemini")?,
            ),
        })
    }

    fn restore_all(self, app: &tauri::AppHandle) {
        let _ = mcp_sync::restore_target_bytes(app, "claude", self.claude.0);
        let _ = mcp_sync::restore_manifest_bytes(app, "claude", self.claude.1);
        let _ = mcp_sync::restore_target_bytes(app, "codex", self.codex.0);
        let _ = mcp_sync::restore_manifest_bytes(app, "codex", self.codex.1);
        let _ = mcp_sync::restore_target_bytes(app, "gemini", self.gemini.0);
        let _ = mcp_sync::restore_manifest_bytes(app, "gemini", self.gemini.1);
    }
}

/// Single CLI file backup for rollback on error.
struct SingleCliBackup {
    target: Option<Vec<u8>>,
    manifest: Option<Vec<u8>>,
}

impl SingleCliBackup {
    fn capture(app: &tauri::AppHandle, cli_key: &str) -> Result<Self, String> {
        Ok(Self {
            target: mcp_sync::read_target_bytes(app, cli_key)?,
            manifest: mcp_sync::read_manifest_bytes(app, cli_key)?,
        })
    }

    fn restore(self, app: &tauri::AppHandle, cli_key: &str) {
        let _ = mcp_sync::restore_target_bytes(app, cli_key, self.target);
        let _ = mcp_sync::restore_manifest_bytes(app, cli_key, self.manifest);
    }
}

fn server_key_exists(conn: &Connection, server_key: &str) -> Result<bool, String> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM mcp_servers WHERE server_key = ?1",
            params![server_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query mcp server_key: {e}"))?;
    Ok(exists.is_some())
}

fn generate_unique_server_key(conn: &Connection, name: &str) -> Result<String, String> {
    let base = suggest_key(name);
    let base = base.trim();
    let base = if base.is_empty() { "mcp-server" } else { base };

    // Fast path.
    if !server_key_exists(conn, base)? {
        validate_server_key(base)?;
        return Ok(base.to_string());
    }

    for idx in 2..1000 {
        let suffix = format!("-{idx}");
        let mut candidate = base.to_string();
        if candidate.len() + suffix.len() > 64 {
            candidate.truncate(64 - suffix.len());
        }
        candidate.push_str(&suffix);
        if !server_key_exists(conn, &candidate)? {
            validate_server_key(&candidate)?;
            return Ok(candidate);
        }
    }

    let fallback = format!("mcp-{}", now_unix_seconds());
    validate_server_key(&fallback)?;
    Ok(fallback)
}

fn args_to_json(args: &[String]) -> Result<String, String> {
    serde_json::to_string(args)
        .map_err(|e| format!("SEC_INVALID_INPUT: failed to serialize args: {e}"))
}

fn map_to_json(map: &BTreeMap<String, String>, hint: &str) -> Result<String, String> {
    serde_json::to_string(map)
        .map_err(|e| format!("SEC_INVALID_INPUT: failed to serialize {hint}: {e}"))
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<McpServerSummary, rusqlite::Error> {
    let args_json: String = row.get("args_json")?;
    let env_json: String = row.get("env_json")?;
    let headers_json: String = row.get("headers_json")?;

    let args = serde_json::from_str::<Vec<String>>(&args_json).unwrap_or_default();
    let env = serde_json::from_str::<BTreeMap<String, String>>(&env_json).unwrap_or_default();
    let headers =
        serde_json::from_str::<BTreeMap<String, String>>(&headers_json).unwrap_or_default();

    Ok(McpServerSummary {
        id: row.get("id")?,
        server_key: row.get("server_key")?,
        name: row.get("name")?,
        transport: row.get("transport")?,
        command: row.get("command")?,
        args,
        env,
        cwd: row.get("cwd")?,
        url: row.get("url")?,
        headers,
        enabled_claude: row.get::<_, i64>("enabled_claude")? != 0,
        enabled_codex: row.get::<_, i64>("enabled_codex")? != 0,
        enabled_gemini: row.get::<_, i64>("enabled_gemini")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn get_by_id(conn: &Connection, server_id: i64) -> Result<McpServerSummary, String> {
    conn.query_row(
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
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
FROM mcp_servers
WHERE id = ?1
"#,
        params![server_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query mcp server: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: mcp server not found".to_string())
}

pub fn list_all(db: &db::Db) -> Result<Vec<McpServerSummary>, String> {
    let conn = db.open_connection()?;

    let mut stmt = conn
        .prepare(
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
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
FROM mcp_servers
ORDER BY updated_at DESC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list mcp servers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read mcp row: {e}"))?);
    }
    Ok(items)
}

#[allow(clippy::too_many_arguments)]
pub fn upsert(
    app: &tauri::AppHandle,
    db: &db::Db,
    server_id: Option<i64>,
    server_key: &str,
    name: &str,
    transport: &str,
    command: Option<&str>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<&str>,
    url: Option<&str>,
    headers: BTreeMap<String, String>,
    enabled_claude: bool,
    enabled_codex: bool,
    enabled_gemini: bool,
) -> Result<McpServerSummary, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: name is required".to_string());
    }

    let provided_key = server_key.trim();

    let transport = transport.trim().to_lowercase();
    validate_transport(&transport)?;

    let command = command.map(str::trim).filter(|v| !v.is_empty());
    let url = url.map(str::trim).filter(|v| !v.is_empty());
    let cwd = cwd.map(str::trim).filter(|v| !v.is_empty());

    if transport == "stdio" && command.is_none() {
        return Err("SEC_INVALID_INPUT: stdio command is required".to_string());
    }
    if transport == "http" && url.is_none() {
        return Err("SEC_INVALID_INPUT: http url is required".to_string());
    }

    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let args_json = args_to_json(&args)?;
    let env_json = map_to_json(&env, "env")?;
    let headers_json = map_to_json(&headers, "headers")?;

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let resolved_key = match server_id {
        None => {
            if provided_key.is_empty() {
                generate_unique_server_key(&tx, name)?
            } else {
                validate_server_key(provided_key)?;
                provided_key.to_string()
            }
        }
        Some(id) => {
            let existing_key: Option<String> = tx
                .query_row(
                    "SELECT server_key FROM mcp_servers WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| format!("DB_ERROR: failed to query mcp server: {e}"))?;

            let Some(existing_key) = existing_key else {
                return Err("DB_NOT_FOUND: mcp server not found".to_string());
            };

            if !provided_key.is_empty() && existing_key != provided_key {
                return Err(
                    "SEC_INVALID_INPUT: server_key cannot be changed for existing server"
                        .to_string(),
                );
            }

            existing_key
        }
    };

    let normalized_name = normalize_name(name);
    let snapshots = CliBackupSnapshots::capture_all(app)?;

    let id = match server_id {
        None => {
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
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
"#,
                params![
                    resolved_key,
                    name,
                    normalized_name,
                    transport,
                    command,
                    args_json,
                    env_json,
                    cwd,
                    url,
                    headers_json,
                    enabled_to_int(enabled_claude),
                    enabled_to_int(enabled_codex),
                    enabled_to_int(enabled_gemini),
                    now,
                    now
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == ErrorCode::ConstraintViolation =>
                {
                    format!("DB_CONSTRAINT: mcp server_key already exists: {resolved_key}")
                }
                other => format!("DB_ERROR: failed to insert mcp server: {other}"),
            })?;
            tx.last_insert_rowid()
        }
        Some(id) => {
            tx.execute(
                r#"
UPDATE mcp_servers
SET
  name = ?1,
  normalized_name = ?2,
  transport = ?3,
  command = ?4,
  args_json = ?5,
  env_json = ?6,
  cwd = ?7,
  url = ?8,
  headers_json = ?9,
  enabled_claude = ?10,
  enabled_codex = ?11,
  enabled_gemini = ?12,
  updated_at = ?13
WHERE id = ?14
"#,
                params![
                    name,
                    normalized_name,
                    transport,
                    command,
                    args_json,
                    env_json,
                    cwd,
                    url,
                    headers_json,
                    enabled_to_int(enabled_claude),
                    enabled_to_int(enabled_codex),
                    enabled_to_int(enabled_gemini),
                    now,
                    id
                ],
            )
            .map_err(|e| format!("DB_ERROR: failed to update mcp server: {e}"))?;
            id
        }
    };

    if let Err(err) = sync_all_cli(app, &tx) {
        snapshots.restore_all(app);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        snapshots.restore_all(app);
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    get_by_id(&conn, id)
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    db: &db::Db,
    server_id: i64,
    cli_key: &str,
    enabled: bool,
) -> Result<McpServerSummary, String> {
    validate_cli_key(cli_key)?;

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let backup = SingleCliBackup::capture(app, cli_key)?;

    let column = match cli_key {
        "claude" => "enabled_claude",
        "codex" => "enabled_codex",
        "gemini" => "enabled_gemini",
        _ => return Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    };

    let sql = format!("UPDATE mcp_servers SET {column} = ?1, updated_at = ?2 WHERE id = ?3");
    let changed = tx
        .execute(&sql, params![enabled_to_int(enabled), now, server_id])
        .map_err(|e| format!("DB_ERROR: failed to update mcp server: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: mcp server not found".to_string());
    }

    if let Err(err) = sync_one_cli(app, &tx, cli_key) {
        backup.restore(app, cli_key);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        backup.restore(app, cli_key);
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    get_by_id(&conn, server_id)
}

pub fn delete(app: &tauri::AppHandle, db: &db::Db, server_id: i64) -> Result<(), String> {
    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let snapshots = CliBackupSnapshots::capture_all(app)?;

    let changed = tx
        .execute("DELETE FROM mcp_servers WHERE id = ?1", params![server_id])
        .map_err(|e| format!("DB_ERROR: failed to delete mcp server: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: mcp server not found".to_string());
    }

    if let Err(err) = sync_all_cli(app, &tx) {
        snapshots.restore_all(app);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        snapshots.restore_all(app);
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    Ok(())
}

pub(super) fn upsert_by_name(
    tx: &Connection,
    input: &McpImportServer,
    now: i64,
) -> Result<(bool, i64), String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: name is required".to_string());
    }
    let transport = input.transport.trim().to_lowercase();
    validate_transport(&transport)?;

    let command = input
        .command
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let url = input
        .url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let cwd = input
        .cwd
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    if transport == "stdio" && command.is_none() {
        return Err(format!(
            "SEC_INVALID_INPUT: stdio command is required for server='{}'",
            name
        ));
    }
    if transport == "http" && url.is_none() {
        return Err(format!(
            "SEC_INVALID_INPUT: http url is required for server='{}'",
            name
        ));
    }

    let args_json = args_to_json(&input.args)?;
    let env_json = map_to_json(&input.env, "env")?;
    let headers_json = map_to_json(&input.headers, "headers")?;

    let normalized_name = normalize_name(name);
    let existing_id: Option<i64> = tx
        .query_row(
            r#"
SELECT id
FROM mcp_servers
WHERE normalized_name = ?1
ORDER BY updated_at DESC, id DESC
LIMIT 1
"#,
            params![normalized_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query mcp server by name: {e}"))?;

    match existing_id {
        None => {
            let resolved_key = generate_unique_server_key(tx, name)?;
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
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
"#,
                params![
                    resolved_key,
                    name,
                    normalized_name,
                    transport,
                    command,
                    args_json,
                    env_json,
                    cwd,
                    url,
                    headers_json,
                    enabled_to_int(input.enabled_claude),
                    enabled_to_int(input.enabled_codex),
                    enabled_to_int(input.enabled_gemini),
                    now,
                    now
                ],
            )
            .map_err(|e| format!("DB_ERROR: failed to insert mcp server: {e}"))?;

            Ok((true, tx.last_insert_rowid()))
        }
        Some(id) => {
            tx.execute(
                r#"
UPDATE mcp_servers
SET
  name = ?1,
  normalized_name = ?2,
  transport = ?3,
  command = ?4,
  args_json = ?5,
  env_json = ?6,
  cwd = ?7,
  url = ?8,
  headers_json = ?9,
  enabled_claude = ?10,
  enabled_codex = ?11,
  enabled_gemini = ?12,
  updated_at = ?13
WHERE id = ?14
"#,
                params![
                    name,
                    normalized_name,
                    transport,
                    command,
                    args_json,
                    env_json,
                    cwd,
                    url,
                    headers_json,
                    enabled_to_int(input.enabled_claude),
                    enabled_to_int(input.enabled_codex),
                    enabled_to_int(input.enabled_gemini),
                    now,
                    id
                ],
            )
            .map_err(|e| format!("DB_ERROR: failed to update mcp server: {e}"))?;

            Ok((false, id))
        }
    }
}
