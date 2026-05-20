//! Usage: MCP server import/export parsing and DB import.

use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::workspaces;
use rusqlite::params;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::backups::CliBackupSnapshots;
use super::db::{list_for_workspace, upsert_by_name};
use super::sync::sync_all_cli;
use super::types::{McpImportReport, McpImportServer, McpImportSkip, McpParseResult};
use super::validate::{suggest_key, validate_server_key};
use crate::shared::text::normalize_name;

const MCP_CONFIG_IMPORT_MAX_BYTES: usize = 1024 * 1024;
const MCP_PARSE_MAX_SERVERS: usize = 512;
const MCP_PARSE_MAX_STRING_ARRAY_ITEMS: usize = 256;
const MCP_PARSE_MAX_STRING_MAP_ENTRIES: usize = 256;

fn ensure_config_text_within_limit(text: &str, hint: &str) -> Result<(), String> {
    if text.len() > MCP_CONFIG_IMPORT_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {hint} config too large (max {MCP_CONFIG_IMPORT_MAX_BYTES} bytes)"
        ));
    }
    Ok(())
}

fn push_import_server(
    out: &mut Vec<McpImportServer>,
    server: McpImportServer,
) -> Result<(), String> {
    if out.len() >= MCP_PARSE_MAX_SERVERS {
        return Err(format!(
            "SEC_INVALID_INPUT: import supports at most {MCP_PARSE_MAX_SERVERS} MCP servers"
        ));
    }
    out.push(server);
    Ok(())
}

fn is_code_switch_r_shape(root: &serde_json::Value) -> bool {
    root.get("claude").is_some() || root.get("codex").is_some() || root.get("gemini").is_some()
}

fn ensure_unique_key(base: &str, used: &mut HashSet<String>) -> String {
    if !used.contains(base) {
        used.insert(base.to_string());
        return base.to_string();
    }

    for idx in 2..1000 {
        let suffix = format!("-{idx}");
        let mut candidate = base.to_string();
        if candidate.len() + suffix.len() > 64 {
            candidate.truncate(64 - suffix.len());
        }
        candidate.push_str(&suffix);
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
    }

    let fallback = format!("mcp-{}", now_unix_seconds());
    used.insert(fallback.clone());
    fallback
}

fn preserve_import_key_or_suggest(raw_key: &str, fallback_name: &str) -> String {
    let trimmed = raw_key.trim();
    if trimmed.is_empty() {
        return suggest_key(fallback_name);
    }

    if validate_server_key(trimmed).is_ok() {
        return trimmed.to_string();
    }

    suggest_key(trimmed)
}

fn extract_string_array(value: Option<&serde_json::Value>) -> Result<Vec<String>, String> {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    if arr.len() > MCP_PARSE_MAX_STRING_ARRAY_ITEMS {
        return Err(format!(
            "SEC_INVALID_INPUT: import string arrays must contain at most {MCP_PARSE_MAX_STRING_ARRAY_ITEMS} entries"
        ));
    }
    Ok(arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect())
}

fn extract_string_map(
    value: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let Some(obj) = value.and_then(|v| v.as_object()) else {
        return Ok(out);
    };
    if obj.len() > MCP_PARSE_MAX_STRING_MAP_ENTRIES {
        return Err(format!(
            "SEC_INVALID_INPUT: import string maps must contain at most {MCP_PARSE_MAX_STRING_MAP_ENTRIES} entries"
        ));
    }
    for (k, v) in obj {
        if let Some(s) = v.as_str() {
            out.insert(k.to_string(), s.to_string());
        }
    }
    Ok(out)
}

fn normalize_transport_from_json(spec: &serde_json::Value) -> Option<String> {
    let raw = spec
        .get("type")
        .and_then(|v| v.as_str())
        .or_else(|| spec.get("transport").and_then(|v| v.as_str()))
        .or_else(|| spec.get("transport_type").and_then(|v| v.as_str()));
    let raw = raw?;
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "stdio" => Some("stdio".to_string()),
        "http" => Some("http".to_string()),
        "sse" => Some("sse".to_string()),
        _ => None,
    }
}

fn infer_transport_from_json_spec(spec: &serde_json::Value) -> String {
    if let Some(transport) = normalize_transport_from_json(spec) {
        return transport;
    }

    if spec.get("url").and_then(|v| v.as_str()).is_some()
        || spec.get("httpUrl").and_then(|v| v.as_str()).is_some()
        || spec.get("headers").is_some()
        || spec.get("http_headers").is_some()
        || spec.get("httpHeaders").is_some()
    {
        return "http".to_string();
    }

    "stdio".to_string()
}

fn resolve_enabled_from_json(entry: &serde_json::Value, spec: &serde_json::Value) -> bool {
    if let Some(enabled) = entry.get("enabled").and_then(|v| v.as_bool()) {
        return enabled;
    }

    let legacy_present = entry.get("enabled_claude").is_some()
        || entry.get("enabled_codex").is_some()
        || entry.get("enabled_gemini").is_some();

    if legacy_present {
        return entry
            .get("enabled_claude")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || entry
                .get("enabled_codex")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || entry
                .get("enabled_gemini")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
    }

    spec.get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn parse_json_server_entry(
    name_hint: Option<&str>,
    entry: &serde_json::Value,
) -> Result<McpImportServer, String> {
    let spec = entry
        .get("server")
        .or_else(|| entry.get("spec"))
        .unwrap_or(entry);

    let transport = infer_transport_from_json_spec(spec);
    let command = spec
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let url = spec
        .get("url")
        .or_else(|| spec.get("httpUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let headers = extract_string_map(
        spec.get("headers")
            .or_else(|| spec.get("http_headers"))
            .or_else(|| spec.get("httpHeaders")),
    )?;

    let raw_name = name_hint
        .or_else(|| entry.get("name").and_then(|v| v.as_str()))
        .or_else(|| spec.get("name").and_then(|v| v.as_str()))
        .unwrap_or("mcp-server")
        .trim();

    let name = if raw_name.is_empty() {
        "mcp-server".to_string()
    } else {
        raw_name.to_string()
    };

    if transport == "stdio" && command.as_deref().unwrap_or("").trim().is_empty() {
        return Err(format!(
            "SEC_INVALID_INPUT: import server '{name}' missing command"
        ));
    }
    if transport != "stdio" && url.as_deref().unwrap_or("").trim().is_empty() {
        return Err(format!(
            "SEC_INVALID_INPUT: import server '{name}' missing {transport} url"
        ));
    }

    Ok(McpImportServer {
        server_key: preserve_import_key_or_suggest(&name, &name),
        name,
        transport,
        command,
        args: extract_string_array(spec.get("args"))?,
        env: extract_string_map(spec.get("env"))?,
        cwd: spec
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        url,
        headers,
        enabled: resolve_enabled_from_json(entry, spec),
    })
}

fn parse_json_mcp_servers_map(
    servers_obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<McpImportServer>, String> {
    let mut used_keys = HashSet::new();
    let mut out = Vec::new();

    for (key, entry) in servers_obj {
        let mut server = parse_json_server_entry(Some(key), entry)?;
        let base = preserve_import_key_or_suggest(key, &server.name);
        server.server_key = ensure_unique_key(&base, &mut used_keys);
        push_import_server(&mut out, server)?;
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn extract_toml_string_array(value: Option<&toml::Value>) -> Result<Vec<String>, String> {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };

    if arr.len() > MCP_PARSE_MAX_STRING_ARRAY_ITEMS {
        return Err(format!(
            "SEC_INVALID_INPUT: import string arrays must contain at most {MCP_PARSE_MAX_STRING_ARRAY_ITEMS} entries"
        ));
    }
    Ok(arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect())
}

fn extract_toml_string_map(
    value: Option<&toml::Value>,
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let Some(table) = value.and_then(|v| v.as_table()) else {
        return Ok(out);
    };

    if table.len() > MCP_PARSE_MAX_STRING_MAP_ENTRIES {
        return Err(format!(
            "SEC_INVALID_INPUT: import string maps must contain at most {MCP_PARSE_MAX_STRING_MAP_ENTRIES} entries"
        ));
    }
    for (k, v) in table {
        if let Some(s) = v.as_str() {
            out.insert(k.to_string(), s.to_string());
        }
    }

    Ok(out)
}

fn parse_codex_toml_mcp_servers(toml_text: &str) -> Result<Vec<McpImportServer>, String> {
    ensure_config_text_within_limit(toml_text, "codex toml")?;
    let root: toml::Value = toml::from_str(toml_text)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid codex toml: {e}"))?;
    let Some(servers) = root.get("mcp_servers").and_then(|v| v.as_table()) else {
        return Ok(Vec::new());
    };

    let mut used_keys = HashSet::new();
    let mut out = Vec::new();

    for (key, value) in servers {
        let Some(spec) = value.as_table() else {
            continue;
        };

        let transport = spec
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_else(|| {
                if spec.get("url").and_then(|v| v.as_str()).is_some()
                    || spec.get("http_headers").is_some()
                {
                    "http".to_string()
                } else {
                    "stdio".to_string()
                }
            });

        let command = spec
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let url = spec
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if transport == "stdio" && command.as_deref().unwrap_or("").trim().is_empty() {
            return Err(format!(
                "SEC_INVALID_INPUT: import codex server '{key}' missing command"
            ));
        }
        if transport == "http" && url.as_deref().unwrap_or("").trim().is_empty() {
            return Err(format!(
                "SEC_INVALID_INPUT: import codex server '{key}' missing url"
            ));
        }

        let name = key.to_string();
        let base = preserve_import_key_or_suggest(key, &name);
        let server_key = ensure_unique_key(&base, &mut used_keys);

        push_import_server(
            &mut out,
            McpImportServer {
                server_key,
                name,
                transport,
                command,
                args: extract_toml_string_array(spec.get("args"))?,
                env: extract_toml_string_map(spec.get("env"))?,
                cwd: spec
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                url,
                headers: extract_toml_string_map(
                    spec.get("headers").or_else(|| spec.get("http_headers")),
                )?,
                enabled: true,
            },
        )?;
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn parse_code_switch_r(root: &serde_json::Value) -> Result<Vec<McpImportServer>, String> {
    let mut by_name: HashMap<String, McpImportServer> = HashMap::new();

    for cli_key in ["claude", "codex", "gemini"] {
        let Some(section) = root.get(cli_key) else {
            continue;
        };
        let Some(servers) = section.get("servers").and_then(|v| v.as_object()) else {
            continue;
        };

        for (name, entry) in servers {
            let enabled = entry
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let spec = entry
                .get("server")
                .or_else(|| entry.get("spec"))
                .unwrap_or(entry);

            let transport =
                normalize_transport_from_json(spec).unwrap_or_else(|| "stdio".to_string());

            let command = spec
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let url = spec
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let cwd = spec
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let args = extract_string_array(spec.get("args"))?;
            let env = extract_string_map(spec.get("env"))?;
            let headers =
                extract_string_map(spec.get("headers").or_else(|| spec.get("http_headers")))?;

            if transport == "stdio" && command.as_deref().unwrap_or("").trim().is_empty() {
                return Err(format!(
                    "SEC_INVALID_INPUT: import {cli_key} server '{name}' missing command"
                ));
            }
            if transport == "http" && url.as_deref().unwrap_or("").trim().is_empty() {
                return Err(format!(
                    "SEC_INVALID_INPUT: import {cli_key} server '{name}' missing url"
                ));
            }

            if !by_name.contains_key(name) && by_name.len() >= MCP_PARSE_MAX_SERVERS {
                return Err(format!(
                    "SEC_INVALID_INPUT: import supports at most {MCP_PARSE_MAX_SERVERS} MCP servers"
                ));
            }

            let item = by_name
                .entry(name.to_string())
                .or_insert_with(|| McpImportServer {
                    server_key: String::new(),
                    name: name.to_string(),
                    transport: transport.clone(),
                    command: command.clone(),
                    args: args.clone(),
                    env: env.clone(),
                    cwd: cwd.clone(),
                    url: url.clone(),
                    headers: headers.clone(),
                    enabled: false,
                });

            // If the same server name appears in multiple platform sections, require compatible specs.
            if item.transport != transport
                || item.command != command
                || item.url != url
                || item.args != args
            {
                return Err(format!(
                    "SEC_INVALID_INPUT: import conflict for server '{name}' across platforms"
                ));
            }

            item.enabled = item.enabled || enabled;
        }
    }

    let mut used_keys = HashSet::new();
    let mut out: Vec<McpImportServer> = by_name
        .into_values()
        .map(|mut item| {
            let base = preserve_import_key_or_suggest(&item.name, &item.name);
            let key = ensure_unique_key(&base, &mut used_keys);
            item.server_key = key;
            item
        })
        .collect();

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn parse_json(json_text: &str) -> crate::shared::error::AppResult<McpParseResult> {
    let json_text = json_text.trim();
    if json_text.is_empty() {
        return Err("SEC_INVALID_INPUT: JSON is required".to_string().into());
    }
    ensure_config_text_within_limit(json_text, "json")
        .map_err(crate::shared::error::AppError::from)?;

    let root: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid JSON: {e}"))?;

    let servers = if is_code_switch_r_shape(&root) {
        parse_code_switch_r(&root)?
    } else if let Some(servers_obj) = root.get("mcpServers").and_then(|v| v.as_object()) {
        parse_json_mcp_servers_map(servers_obj)?
    } else if let Some(arr) = root.as_array() {
        // Optional: support simplified array format used by this project.
        let mut out = Vec::new();
        for item in arr {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.trim().is_empty() {
                continue;
            }
            let base = suggest_key(&name);
            let transport = obj
                .get("transport")
                .and_then(|v| v.as_str())
                .unwrap_or("stdio")
                .trim()
                .to_lowercase();
            let command = obj
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let url = obj
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let enabled = obj
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| {
                    let legacy_present = obj.contains_key("enabled_claude")
                        || obj.contains_key("enabled_codex")
                        || obj.contains_key("enabled_gemini");
                    let legacy_any = obj
                        .get("enabled_claude")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                        || obj
                            .get("enabled_codex")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        || obj
                            .get("enabled_gemini")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    if legacy_present {
                        legacy_any
                    } else {
                        true
                    }
                });

            push_import_server(
                &mut out,
                McpImportServer {
                    server_key: base,
                    name,
                    transport,
                    command,
                    args: extract_string_array(obj.get("args"))?,
                    env: extract_string_map(obj.get("env"))?,
                    cwd: obj
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    url,
                    headers: extract_string_map(obj.get("headers"))?,
                    enabled,
                },
            )?;
        }
        out
    } else {
        return Err("SEC_INVALID_INPUT: unsupported JSON shape"
            .to_string()
            .into());
    };

    Ok(McpParseResult { servers })
}

pub fn parse_workspace_cli_target_json<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<McpParseResult> {
    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;

    let bytes = crate::mcp_sync::read_target_bytes(app, &cli_key)
        .map_err(|e| format!("SYSTEM_ERROR: failed to read {cli_key} target config: {e}"))?;

    let Some(raw) = bytes else {
        return Err(format!("SEC_INVALID_INPUT: {cli_key} target config not found").into());
    };
    if raw.len() > MCP_CONFIG_IMPORT_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {cli_key} target config too large (max {MCP_CONFIG_IMPORT_MAX_BYTES} bytes)"
        )
        .into());
    }

    let text = String::from_utf8(raw)
        .map_err(|e| format!("SEC_INVALID_INPUT: {cli_key} target config is not utf8: {e}"))?;

    let parsed = if cli_key == "codex" {
        McpParseResult {
            servers: parse_codex_toml_mcp_servers(&text)
                .map_err(crate::shared::error::AppError::from)?,
        }
    } else {
        parse_json(&text)?
    };

    if parsed.servers.is_empty() {
        return Err(
            format!("SEC_INVALID_INPUT: no importable mcp servers in {cli_key} config").into(),
        );
    }

    Ok(parsed)
}

pub fn import_servers_from_workspace_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<McpImportReport> {
    let parsed = parse_workspace_cli_target_json(app, db, workspace_id)?;
    import_servers(app, db, workspace_id, parsed.servers)
}

pub fn import_servers<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    servers: Vec<McpImportServer>,
) -> crate::shared::error::AppResult<McpImportReport> {
    if servers.is_empty() {
        return Err("SEC_INVALID_INPUT: servers is required".to_string().into());
    }

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let _cli_key = workspaces::get_cli_key_by_id(&tx, workspace_id)?;
    let snapshots = CliBackupSnapshots::capture_all(app)?;

    let mut inserted = 0u32;
    let mut updated = 0u32;
    let mut skipped: Vec<McpImportSkip> = Vec::new();

    let mut deduped: Vec<McpImportServer> = Vec::new();
    let mut index_by_name: HashMap<String, usize> = HashMap::new();
    for server in servers {
        let norm = normalize_name(&server.name);
        if norm.is_empty() {
            return Err("SEC_INVALID_INPUT: name is required".to_string().into());
        }
        if let Some(idx) = index_by_name.get(&norm).copied() {
            deduped[idx] = server;
            continue;
        }
        index_by_name.insert(norm, deduped.len());
        deduped.push(server);
    }

    let existing_servers = list_for_workspace(db, workspace_id)?;
    let mut existing_by_name: HashMap<String, super::types::McpServerSummary> = HashMap::new();
    let mut existing_by_key: HashMap<String, super::types::McpServerSummary> = HashMap::new();
    for row in existing_servers {
        existing_by_name.insert(normalize_name(&row.name), row.clone());
        existing_by_key.insert(row.server_key.clone(), row);
    }

    for server in &deduped {
        let normalized = normalize_name(&server.name);
        if let Some(existing) = existing_by_name.get(&normalized) {
            if server.enabled && !existing.enabled {
                tx.execute(
                    r#"
INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, server_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
                    params![workspace_id, existing.id, now],
                )
                .map_err(|e| db_err!("failed to merge enabled imported mcp server: {e}"))?;
                updated += 1;

                let mut merged = existing.clone();
                merged.enabled = true;
                existing_by_name.insert(normalized, merged.clone());
                existing_by_key.insert(merged.server_key.clone(), merged);
            } else {
                skipped.push(McpImportSkip {
                    name: server.name.clone(),
                    reason: "already exists; kept existing config".to_string(),
                });
            }
            continue;
        }

        if let Some(existing) = existing_by_key.get(&server.server_key) {
            if server.enabled && !existing.enabled {
                tx.execute(
                    r#"
INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, server_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
                    params![workspace_id, existing.id, now],
                )
                .map_err(|e| db_err!("failed to merge enabled imported mcp server by key: {e}"))?;
                updated += 1;

                let mut merged = existing.clone();
                merged.enabled = true;
                existing_by_name.insert(normalized, merged.clone());
                existing_by_key.insert(merged.server_key.clone(), merged);
            } else {
                skipped.push(McpImportSkip {
                    name: server.name.clone(),
                    reason: "server_key already exists; kept existing config".to_string(),
                });
            }
            continue;
        }

        let (is_insert, id) = upsert_by_name(&tx, server, now)?;
        if is_insert {
            inserted += 1;
        } else {
            updated += 1;
        }

        if server.enabled {
            tx.execute(
                r#"
INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, server_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
                params![workspace_id, id, now],
            )
            .map_err(|e| db_err!("failed to enable imported mcp server: {e}"))?;
        }

        let inserted_row = super::types::McpServerSummary {
            id,
            server_key: server.server_key.clone(),
            name: server.name.clone(),
            transport: server.transport.clone(),
            command: server.command.clone(),
            args: server.args.clone(),
            env: server.env.clone(),
            cwd: server.cwd.clone(),
            url: server.url.clone(),
            headers: server.headers.clone(),
            enabled: server.enabled,
            created_at: now,
            updated_at: now,
        };
        existing_by_name.insert(normalized, inserted_row.clone());
        existing_by_key.insert(inserted_row.server_key.clone(), inserted_row);
    }

    if let Err(err) = sync_all_cli(app, &tx) {
        snapshots.restore_all(app);
        return Err(err);
    }

    if let Err(err) = tx.commit() {
        snapshots.restore_all(app);
        return Err(db_err!("failed to commit: {err}"));
    }

    Ok(McpImportReport {
        inserted,
        updated,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_rejects_oversized_config_text() {
        let text = format!(
            "{{\"mcpServers\":{{\"fetch\":{{\"command\":\"{}\"}}}}}}",
            "x".repeat(MCP_CONFIG_IMPORT_MAX_BYTES)
        );

        let err = parse_json(&text).expect_err("oversized json should fail");

        assert!(err.to_string().contains("json config too large"));
    }

    #[test]
    fn parse_json_caps_server_count() {
        let servers = (0..=MCP_PARSE_MAX_SERVERS)
            .map(|index| format!("\"srv-{index}\":{{\"command\":\"npx\"}}"))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!("{{\"mcpServers\":{{{servers}}}}}");

        let err = parse_json(&text).expect_err("too many servers should fail");

        assert!(err.to_string().contains("at most 512 MCP servers"));
    }

    #[test]
    fn parse_json_caps_nested_string_arrays_and_maps() {
        let args = (0..=MCP_PARSE_MAX_STRING_ARRAY_ITEMS)
            .map(|index| format!("\"arg-{index}\""))
            .collect::<Vec<_>>()
            .join(",");
        let text =
            format!("{{\"mcpServers\":{{\"fetch\":{{\"command\":\"npx\",\"args\":[{args}]}}}}}}");

        let err = parse_json(&text).expect_err("too many args should fail");
        assert!(err.to_string().contains("string arrays"));

        let env = (0..=MCP_PARSE_MAX_STRING_MAP_ENTRIES)
            .map(|index| format!("\"KEY_{index}\":\"value\""))
            .collect::<Vec<_>>()
            .join(",");
        let text =
            format!("{{\"mcpServers\":{{\"fetch\":{{\"command\":\"npx\",\"env\":{{{env}}}}}}}}}");

        let err = parse_json(&text).expect_err("too many env keys should fail");
        assert!(err.to_string().contains("string maps"));
    }

    #[test]
    fn parse_codex_toml_caps_server_count() {
        let text = (0..=MCP_PARSE_MAX_SERVERS)
            .map(|index| format!("[mcp_servers.srv-{index}]\ncommand = \"npx\"\n"))
            .collect::<Vec<_>>()
            .join("\n");

        let err = parse_codex_toml_mcp_servers(&text).expect_err("too many toml servers fail");

        assert!(err.to_string().contains("at most 512 MCP servers"));
    }
}
