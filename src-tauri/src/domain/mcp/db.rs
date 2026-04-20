//! Usage: MCP server persistence (SQLite) and sync integration hooks.

use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::workspaces;
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};

use super::backups::{CliBackupSnapshots, SingleCliBackup};
use super::sync::{sync_all_cli, sync_one_cli};
use super::types::{McpImportServer, McpServerSummary};
use super::validate::{suggest_key, validate_cli_key, validate_server_key, validate_transport};
use crate::shared::text::normalize_name;

fn server_key_exists(conn: &Connection, server_key: &str) -> crate::shared::error::AppResult<bool> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM mcp_servers WHERE server_key = ?1",
            params![server_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query mcp server_key: {e}"))?;
    Ok(exists.is_some())
}

fn generate_unique_server_key(
    conn: &Connection,
    name: &str,
) -> crate::shared::error::AppResult<String> {
    let base = suggest_key(name);
    generate_unique_server_key_from_base(conn, &base)
}

fn generate_unique_server_key_from_base(
    conn: &Connection,
    base: &str,
) -> crate::shared::error::AppResult<String> {
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

fn args_to_json(args: &[String]) -> crate::shared::error::AppResult<String> {
    serde_json::to_string(args)
        .map_err(|e| format!("SEC_INVALID_INPUT: failed to serialize args: {e}").into())
}

fn map_to_json(
    map: &BTreeMap<String, String>,
    hint: &str,
) -> crate::shared::error::AppResult<String> {
    serde_json::to_string(map)
        .map_err(|e| format!("SEC_INVALID_INPUT: failed to serialize {hint}: {e}").into())
}

fn normalize_patch_preserve_keys(keys: Vec<String>) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for key in keys {
        let normalized = key.trim();
        if normalized.is_empty() {
            continue;
        }
        deduped.insert(normalized.to_string());
    }
    deduped.into_iter().collect()
}

fn normalize_patch_replace(
    replace: BTreeMap<String, String>,
    hint: &str,
) -> crate::shared::error::AppResult<BTreeMap<String, String>> {
    let mut normalized = BTreeMap::new();
    for (raw_key, value) in replace {
        let key = raw_key.trim();
        if key.is_empty() {
            return Err(format!("SEC_INVALID_INPUT: {hint} key is required").into());
        }
        if value.trim().is_empty() {
            return Err(
                format!("SEC_INVALID_INPUT: {hint} value is required for key '{key}'").into(),
            );
        }
        normalized.insert(key.to_string(), value);
    }
    Ok(normalized)
}

fn merge_secret_patch(
    existing: Option<&BTreeMap<String, String>>,
    preserve_keys: Vec<String>,
    replace: BTreeMap<String, String>,
    hint: &str,
) -> crate::shared::error::AppResult<BTreeMap<String, String>> {
    let normalized_preserve_keys = normalize_patch_preserve_keys(preserve_keys);
    let normalized_replace = normalize_patch_replace(replace, hint)?;
    let mut merged = BTreeMap::new();

    if let Some(existing_map) = existing {
        for key in normalized_preserve_keys {
            if let Some(value) = existing_map.get(&key) {
                merged.insert(key, value.clone());
            }
        }
    }

    for (key, value) in normalized_replace {
        merged.insert(key, value);
    }

    Ok(merged)
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
        enabled: row.get::<_, i64>("enabled")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn get_by_id(
    conn: &Connection,
    server_id: i64,
) -> crate::shared::error::AppResult<McpServerSummary> {
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
  0 AS enabled,
  created_at,
  updated_at
FROM mcp_servers
WHERE id = ?1
"#,
        params![server_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| db_err!("failed to query mcp server: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: mcp server not found"))
}

fn get_by_id_for_workspace(
    conn: &Connection,
    workspace_id: i64,
    server_id: i64,
) -> crate::shared::error::AppResult<McpServerSummary> {
    conn.query_row(
        r#"
SELECT
  s.id,
  s.server_key,
  s.name,
  s.transport,
  s.command,
  s.args_json,
  s.env_json,
  s.cwd,
  s.url,
  s.headers_json,
  CASE WHEN e.server_id IS NULL THEN 0 ELSE 1 END AS enabled,
  s.created_at,
  s.updated_at
FROM mcp_servers s
LEFT JOIN workspace_mcp_enabled e
  ON e.workspace_id = ?1 AND e.server_id = s.id
WHERE s.id = ?2
"#,
        params![workspace_id, server_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| db_err!("failed to query mcp server: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: mcp server not found"))
}

pub fn list_for_workspace(
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<McpServerSummary>> {
    let conn = db.open_connection()?;
    let _ = workspaces::get_cli_key_by_id(&conn, workspace_id)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      s.id,
      s.server_key,
      s.name,
      s.transport,
      s.command,
      s.args_json,
      s.env_json,
      s.cwd,
      s.url,
      s.headers_json,
      CASE WHEN e.server_id IS NULL THEN 0 ELSE 1 END AS enabled,
      s.created_at,
      s.updated_at
    FROM mcp_servers s
    LEFT JOIN workspace_mcp_enabled e
      ON e.workspace_id = ?1 AND e.server_id = s.id
    ORDER BY s.updated_at DESC, s.id DESC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], row_to_summary)
        .map_err(|e| db_err!("failed to list mcp servers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read mcp row: {e}"))?);
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
    env_preserve_keys: Vec<String>,
    env_replace: BTreeMap<String, String>,
    cwd: Option<&str>,
    url: Option<&str>,
    header_preserve_keys: Vec<String>,
    header_replace: BTreeMap<String, String>,
) -> crate::shared::error::AppResult<McpServerSummary> {
    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: name is required".to_string().into());
    }

    let provided_key = server_key.trim();

    let transport = transport.trim().to_lowercase();
    validate_transport(&transport)?;

    let command = command.map(str::trim).filter(|v| !v.is_empty());
    let url = url.map(str::trim).filter(|v| !v.is_empty());
    let cwd = cwd.map(str::trim).filter(|v| !v.is_empty());

    if transport == "stdio" && command.is_none() {
        return Err("SEC_INVALID_INPUT: stdio command is required"
            .to_string()
            .into());
    }
    if transport == "http" && url.is_none() {
        return Err("SEC_INVALID_INPUT: http url is required".to_string().into());
    }

    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let existing_server = match server_id {
        Some(id) => Some(get_by_id(&tx, id)?),
        None => None,
    };

    let resolved_key = match existing_server.as_ref() {
        None => {
            if provided_key.is_empty() {
                generate_unique_server_key(&tx, name)?
            } else {
                validate_server_key(provided_key)?;
                provided_key.to_string()
            }
        }
        Some(existing_server) => {
            if !provided_key.is_empty() && existing_server.server_key != provided_key {
                return Err(
                    "SEC_INVALID_INPUT: server_key cannot be changed for existing server"
                        .to_string()
                        .into(),
                );
            }

            existing_server.server_key.clone()
        }
    };

    let env = if transport == "stdio" {
        merge_secret_patch(
            existing_server.as_ref().map(|server| &server.env),
            env_preserve_keys,
            env_replace,
            "env",
        )?
    } else {
        BTreeMap::new()
    };

    let headers = if transport == "stdio" {
        BTreeMap::new()
    } else {
        merge_secret_patch(
            existing_server.as_ref().map(|server| &server.headers),
            header_preserve_keys,
            header_replace,
            "headers",
        )?
    };

    let normalized_name = normalize_name(name);
    let snapshots = CliBackupSnapshots::capture_all(app)?;
    let args_json = args_to_json(&args)?;
    let env_json = map_to_json(&env, "env")?;
    let headers_json = map_to_json(&headers, "headers")?;

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
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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
                    now,
                    now
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == ErrorCode::ConstraintViolation =>
                {
                    crate::shared::error::AppError::new(
                        "DB_CONSTRAINT",
                        format!("mcp server_key already exists: {resolved_key}"),
                    )
                }
                other => db_err!("failed to insert mcp server: {other}"),
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
  updated_at = ?10
WHERE id = ?11
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
                    now,
                    id
                ],
            )
            .map_err(|e| db_err!("failed to update mcp server: {e}"))?;
            id
        }
    };

    if let Err(err) = sync_all_cli(app, &tx) {
        snapshots.restore_all(app);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        snapshots.restore_all(app);
        return Err(db_err!("failed to commit: {err}"));
    }

    get_by_id(&conn, id)
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    db: &db::Db,
    workspace_id: i64,
    server_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<McpServerSummary> {
    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let cli_key = workspaces::get_cli_key_by_id(&tx, workspace_id)?;
    validate_cli_key(&cli_key)?;
    let should_sync = workspaces::is_active_workspace(&tx, workspace_id)?;

    let backup = if should_sync {
        Some(SingleCliBackup::capture(app, &cli_key)?)
    } else {
        None
    };

    if enabled {
        tx.execute(
            r#"
INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, server_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
            params![workspace_id, server_id, now],
        )
        .map_err(|e| db_err!("failed to enable mcp server: {e}"))?;
    } else {
        tx.execute(
            "DELETE FROM workspace_mcp_enabled WHERE workspace_id = ?1 AND server_id = ?2",
            params![workspace_id, server_id],
        )
        .map_err(|e| db_err!("failed to disable mcp server: {e}"))?;
    }

    if should_sync {
        if let Err(err) = sync_one_cli(app, &tx, &cli_key) {
            if let Some(backup) = backup {
                backup.restore(app, &cli_key);
            }
            return Err(err);
        }
    }

    if let Err(err) = tx.commit() {
        if let Some(backup) = backup {
            backup.restore(app, &cli_key);
        }
        return Err(db_err!("failed to commit: {err}"));
    }

    get_by_id_for_workspace(&conn, workspace_id, server_id)
}

pub fn delete(
    app: &tauri::AppHandle,
    db: &db::Db,
    server_id: i64,
) -> crate::shared::error::AppResult<()> {
    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let snapshots = CliBackupSnapshots::capture_all(app)?;

    let changed = tx
        .execute("DELETE FROM mcp_servers WHERE id = ?1", params![server_id])
        .map_err(|e| db_err!("failed to delete mcp server: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: mcp server not found".to_string().into());
    }

    if let Err(err) = sync_all_cli(app, &tx) {
        snapshots.restore_all(app);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        snapshots.restore_all(app);
        return Err(db_err!("failed to commit: {err}"));
    }

    Ok(())
}

pub(super) fn upsert_by_name(
    tx: &Connection,
    input: &McpImportServer,
    now: i64,
) -> crate::shared::error::AppResult<(bool, i64)> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: name is required".into());
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
        )
        .into());
    }
    if transport == "http" && url.is_none() {
        return Err(format!(
            "SEC_INVALID_INPUT: http url is required for server='{}'",
            name
        )
        .into());
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
        .map_err(|e| db_err!("failed to query mcp server by name: {e}"))?;

    match existing_id {
        None => {
            let provided_key = input.server_key.trim();
            let resolved_key = if provided_key.is_empty() {
                generate_unique_server_key(tx, name)?
            } else {
                validate_server_key(provided_key)?;
                generate_unique_server_key_from_base(tx, provided_key)?
            };
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
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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
                    now,
                    now
                ],
            )
            .map_err(|e| db_err!("failed to insert mcp server: {e}"))?;

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
  updated_at = ?10
WHERE id = ?11
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
                    now,
                    id
                ],
            )
            .map_err(|e| db_err!("failed to update mcp server: {e}"))?;

            Ok((false, id))
        }
    }
}
