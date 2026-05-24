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

const MCP_NAME_MAX_LEN: usize = 256;
const MCP_OPTIONAL_TEXT_MAX_LEN: usize = 4096;
const MCP_ARGS_MAX_COUNT: usize = 256;
const MCP_ARG_MAX_LEN: usize = 4096;
const MCP_SECRET_MAX_ENTRIES: usize = 256;
const MCP_SECRET_KEY_MAX_LEN: usize = 256;
const MCP_SECRET_VALUE_MAX_LEN: usize = 8192;

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

fn validate_text_len(
    hint: &str,
    value: &str,
    max_len: usize,
) -> crate::shared::error::AppResult<()> {
    if value.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {hint} too long (max {max_len})").into());
    }
    Ok(())
}

fn normalize_required_text<'a>(
    raw: &'a str,
    hint: &str,
    max_len: usize,
) -> crate::shared::error::AppResult<&'a str> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("SEC_INVALID_INPUT: {hint} is required").into());
    }
    validate_text_len(hint, value, max_len)?;
    Ok(value)
}

fn normalize_optional_text<'a>(
    raw: Option<&'a str>,
    hint: &str,
    max_len: usize,
) -> crate::shared::error::AppResult<Option<&'a str>> {
    let Some(value) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    validate_text_len(hint, value, max_len)?;
    Ok(Some(value))
}

fn normalize_args(args: Vec<String>) -> crate::shared::error::AppResult<Vec<String>> {
    let mut normalized = Vec::new();
    for arg in args {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            continue;
        }
        if normalized.len() >= MCP_ARGS_MAX_COUNT {
            return Err(format!(
                "SEC_INVALID_INPUT: args must contain at most {MCP_ARGS_MAX_COUNT} entries"
            )
            .into());
        }
        validate_text_len("args entry", trimmed, MCP_ARG_MAX_LEN)?;
        normalized.push(trimmed.to_string());
    }
    Ok(normalized)
}

fn normalize_patch_preserve_keys(
    keys: Vec<String>,
    hint: &str,
) -> crate::shared::error::AppResult<Vec<String>> {
    let mut deduped = BTreeSet::new();
    for key in keys {
        let normalized = key.trim();
        if normalized.is_empty() {
            continue;
        }
        if deduped.len() >= MCP_SECRET_MAX_ENTRIES && !deduped.contains(normalized) {
            return Err(format!(
                "SEC_INVALID_INPUT: {hint} preserve_keys must contain at most {MCP_SECRET_MAX_ENTRIES} entries"
            )
            .into());
        }
        validate_text_len(&format!("{hint} key"), normalized, MCP_SECRET_KEY_MAX_LEN)?;
        deduped.insert(normalized.to_string());
    }
    Ok(deduped.into_iter().collect())
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
        if normalized.len() >= MCP_SECRET_MAX_ENTRIES && !normalized.contains_key(key) {
            return Err(format!(
                "SEC_INVALID_INPUT: {hint} must contain at most {MCP_SECRET_MAX_ENTRIES} entries"
            )
            .into());
        }
        validate_text_len(&format!("{hint} key"), key, MCP_SECRET_KEY_MAX_LEN)?;
        if value.trim().is_empty() {
            return Err(
                format!("SEC_INVALID_INPUT: {hint} value is required for key '{key}'").into(),
            );
        }
        validate_text_len(
            &format!("{hint} value for key '{key}'"),
            &value,
            MCP_SECRET_VALUE_MAX_LEN,
        )?;
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
    let normalized_preserve_keys = normalize_patch_preserve_keys(preserve_keys, hint)?;
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
        if !merged.contains_key(&key) && merged.len() >= MCP_SECRET_MAX_ENTRIES {
            return Err(format!(
                "SEC_INVALID_INPUT: {hint} must contain at most {MCP_SECRET_MAX_ENTRIES} entries"
            )
            .into());
        }
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
    let name = normalize_required_text(name, "name", MCP_NAME_MAX_LEN)?;

    let provided_key = server_key.trim();

    let transport = transport.trim().to_lowercase();
    validate_transport(&transport)?;

    let command = normalize_optional_text(command, "command", MCP_OPTIONAL_TEXT_MAX_LEN)?;
    let url = normalize_optional_text(url, "url", MCP_OPTIONAL_TEXT_MAX_LEN)?;
    let cwd = normalize_optional_text(cwd, "cwd", MCP_OPTIONAL_TEXT_MAX_LEN)?;

    if transport == "stdio" && command.is_none() {
        return Err("SEC_INVALID_INPUT: stdio command is required"
            .to_string()
            .into());
    }
    if transport != "stdio" && url.is_none() {
        return Err(format!("SEC_INVALID_INPUT: {transport} url is required").into());
    }

    let args = normalize_args(args)?;

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
    let name = normalize_required_text(&input.name, "name", MCP_NAME_MAX_LEN)?;
    let transport = input.transport.trim().to_lowercase();
    validate_transport(&transport)?;

    let command = normalize_optional_text(
        input.command.as_deref(),
        "command",
        MCP_OPTIONAL_TEXT_MAX_LEN,
    )?;
    let url = normalize_optional_text(input.url.as_deref(), "url", MCP_OPTIONAL_TEXT_MAX_LEN)?;
    let cwd = normalize_optional_text(input.cwd.as_deref(), "cwd", MCP_OPTIONAL_TEXT_MAX_LEN)?;

    if transport == "stdio" && command.is_none() {
        return Err(format!(
            "SEC_INVALID_INPUT: stdio command is required for server='{}'",
            name
        )
        .into());
    }
    if transport != "stdio" && url.is_none() {
        return Err(format!(
            "SEC_INVALID_INPUT: {transport} url is required for server='{}'",
            name
        )
        .into());
    }

    let args = normalize_args(input.args.clone())?;
    let env = normalize_patch_replace(input.env.clone(), "env")?;
    let headers = normalize_patch_replace(input.headers.clone(), "headers")?;
    let args_json = args_to_json(&args)?;
    let env_json = map_to_json(&env, "env")?;
    let headers_json = map_to_json(&headers, "headers")?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_args_trims_filters_and_caps_entries() {
        let normalized =
            normalize_args(vec!["  --flag  ".to_string(), "   ".to_string()]).expect("args");
        assert_eq!(normalized, vec!["--flag"]);

        let too_many = (0..=MCP_ARGS_MAX_COUNT)
            .map(|index| format!("arg-{index}"))
            .collect();
        let err = normalize_args(too_many).expect_err("too many args should fail");
        assert!(err.to_string().contains("args must contain at most"));

        let err =
            normalize_args(vec!["x".repeat(MCP_ARG_MAX_LEN + 1)]).expect_err("long arg fails");
        assert!(err.to_string().contains("args entry too long"));
    }

    #[test]
    fn merge_secret_patch_bounds_keys_values_and_total_entries() {
        let mut existing = BTreeMap::new();
        let preserve_keys = (0..MCP_SECRET_MAX_ENTRIES)
            .map(|index| {
                let key = format!("KEY_{index}");
                existing.insert(key.clone(), "value".to_string());
                key
            })
            .collect();
        let mut replace = BTreeMap::new();
        replace.insert("EXTRA".to_string(), "value".to_string());

        let err = merge_secret_patch(Some(&existing), preserve_keys, replace, "env")
            .expect_err("merged map should stay capped");
        assert!(err.to_string().contains("env must contain at most"));

        let mut long_key = BTreeMap::new();
        long_key.insert("K".repeat(MCP_SECRET_KEY_MAX_LEN + 1), "value".to_string());
        let err = normalize_patch_replace(long_key, "headers").expect_err("long key fails");
        assert!(err.to_string().contains("headers key too long"));

        let mut long_value = BTreeMap::new();
        long_value.insert(
            "Authorization".to_string(),
            "x".repeat(MCP_SECRET_VALUE_MAX_LEN + 1),
        );
        let err = normalize_patch_replace(long_value, "headers").expect_err("long value fails");
        assert!(err.to_string().contains("headers value for key"));
    }
}
