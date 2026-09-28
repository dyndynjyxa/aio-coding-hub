//! Usage: Apply AIO-managed MCP server fields to Grok's format-preserving TOML document.

use super::McpServerForSync;
use std::collections::{BTreeSet, HashSet};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, TableLike, Value};

fn validate_server(server: &McpServerForSync) -> Result<(), String> {
    match server.transport.as_str() {
        "stdio" => {
            if server
                .command
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            {
                return Err("SEC_INVALID_INPUT: stdio command is required".to_string());
            }
        }
        "http" | "sse" => {
            if server
                .url
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            {
                return Err(format!(
                    "SEC_INVALID_INPUT: {} url is required",
                    server.transport
                ));
            }
        }
        other => {
            return Err(format!("SEC_INVALID_INPUT: unsupported transport={other}"));
        }
    }
    Ok(())
}

fn ensure_mcp_servers(document: &mut DocumentMut) -> Result<&mut dyn TableLike, String> {
    if !document.contains_key("mcp_servers") {
        let mut table = Table::new();
        table.set_implicit(true);
        document.insert("mcp_servers", Item::Table(table));
    }
    document
        .get_mut("mcp_servers")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| "GROK_CONFIG_INVALID_SCHEMA: mcp_servers must be a table".to_string())
}

fn ensure_server_table<'a>(
    servers: &'a mut dyn TableLike,
    server_key: &str,
    inline_parent: bool,
) -> Result<&'a mut dyn TableLike, String> {
    if !servers.contains_key(server_key) {
        let item = if inline_parent {
            Item::Value(Value::InlineTable(InlineTable::new()))
        } else {
            Item::Table(Table::new())
        };
        servers.insert(server_key, item);
    }
    servers
        .get_mut(server_key)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            format!("GROK_CONFIG_INVALID_SCHEMA: mcp_servers.{server_key} must be a table")
        })
}

fn replace_value(table: &mut dyn TableLike, key: &str, value: Value) {
    if let Some(item) = table.get_mut(key) {
        let decor = item.as_value().map(|existing| existing.decor().clone());
        *item = Item::Value(value);
        if let (Some(decor), Some(next)) = (decor, item.as_value_mut()) {
            *next.decor_mut() = decor;
        }
    } else {
        table.insert(key, Item::Value(value));
    }
}

fn string_map(values: &std::collections::BTreeMap<String, String>) -> InlineTable {
    let mut table = InlineTable::new();
    for (key, value) in values {
        table.insert(key, Value::from(value.as_str()));
    }
    table
}

fn apply_server(table: &mut dyn TableLike, server: &McpServerForSync) {
    match server.transport.as_str() {
        "stdio" => {
            table.remove("url");
            table.remove("headers");
            let command = server.command.as_deref().unwrap_or_default().trim();
            replace_value(table, "command", Value::from(command));
            if server.args.is_empty() {
                table.remove("args");
            } else {
                let mut args = Array::new();
                for value in &server.args {
                    args.push(value.as_str());
                }
                replace_value(table, "args", Value::Array(args));
            }
            if server.env.is_empty() {
                table.remove("env");
            } else {
                replace_value(table, "env", Value::InlineTable(string_map(&server.env)));
            }
        }
        "http" | "sse" => {
            table.remove("command");
            table.remove("args");
            table.remove("env");
            let url = server.url.as_deref().unwrap_or_default().trim();
            replace_value(table, "url", Value::from(url));
            if server.headers.is_empty() {
                table.remove("headers");
            } else {
                replace_value(
                    table,
                    "headers",
                    Value::InlineTable(string_map(&server.headers)),
                );
            }
        }
        _ => unreachable!("servers are validated before mutation"),
    }
    replace_value(table, "enabled", Value::from(true));
}

pub(super) fn apply_grok_mcp_servers(
    document: &mut DocumentMut,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<(), String> {
    for server in servers {
        validate_server(server)?;
    }

    let inline_parent = document
        .get("mcp_servers")
        .is_some_and(Item::is_inline_table);
    let root = ensure_mcp_servers(document)?;
    let active_keys: BTreeSet<&str> = servers
        .iter()
        .map(|server| server.server_key.as_str())
        .collect();

    for key in managed_keys {
        if !active_keys.contains(key.as_str()) {
            root.remove(key);
        }
    }

    for server in servers {
        let table = ensure_server_table(root, &server.server_key, inline_parent)?;
        apply_server(table, server);
    }

    Ok(())
}

fn table_to_inline(table: Table) -> Result<InlineTable, String> {
    let mut inline = InlineTable::new();
    for (key, item) in table {
        inline.insert(&key, item_to_value(item)?);
    }
    Ok(inline)
}

fn item_to_value(item: Item) -> Result<Value, String> {
    match item {
        Item::Value(value) => Ok(value),
        Item::Table(table) => Ok(Value::InlineTable(table_to_inline(table)?)),
        Item::None => Err("GROK_CONFIG_INVALID_SCHEMA: empty MCP server item".to_string()),
        Item::ArrayOfTables(_) => {
            Err("GROK_CONFIG_INVALID_SCHEMA: MCP server cannot be an array of tables".to_string())
        }
    }
}

fn insert_server_item(
    root: &mut dyn TableLike,
    inline_parent: bool,
    key: &str,
    item: Item,
) -> Result<(), String> {
    let item = if inline_parent {
        Item::Value(item_to_value(item)?)
    } else {
        item
    };
    root.insert(key, item);
    Ok(())
}

fn parse_stash(bytes: &[u8]) -> Result<DocumentMut, String> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| format!("MCP_LOCAL_STASH_INVALID_UTF8: {error}"))?;
    source
        .parse::<DocumentMut>()
        .map_err(|error| format!("MCP_LOCAL_STASH_INVALID_TOML: {error}"))
}

pub(super) fn swap_grok_local_servers(
    document: &mut DocumentMut,
    managed_keys: &HashSet<String>,
    target_stash: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let inline_parent = document
        .get("mcp_servers")
        .is_some_and(Item::is_inline_table);
    let root = ensure_mcp_servers(document)?;
    let local_items: Vec<(String, Item)> = root
        .iter()
        .filter(|(key, _)| !managed_keys.contains(*key))
        .map(|(key, item)| (key.to_string(), item.clone()))
        .collect();
    for (key, _) in &local_items {
        root.remove(key);
    }

    let mut stash = DocumentMut::new();
    if inline_parent {
        let mut stash_root = InlineTable::new();
        for (key, item) in local_items {
            stash_root.insert(&key, item_to_value(item)?);
        }
        stash.insert("mcp_servers", Item::Value(Value::InlineTable(stash_root)));
    } else {
        let mut stash_root = Table::new();
        stash_root.set_implicit(true);
        for (key, item) in local_items {
            stash_root.insert(&key, item);
        }
        stash.insert("mcp_servers", Item::Table(stash_root));
    }

    if let Some(bytes) = target_stash.filter(|bytes| !bytes.is_empty()) {
        let target = parse_stash(bytes)?;
        if let Some(target_root_item) = target.get("mcp_servers") {
            let target_root = target_root_item.as_table_like().ok_or_else(|| {
                "MCP_LOCAL_STASH_INVALID_SCHEMA: mcp_servers must be a table".to_string()
            })?;
            let target_items: Vec<(String, Item)> = target_root
                .iter()
                .filter(|(key, _)| !managed_keys.contains(*key))
                .map(|(key, item)| (key.to_string(), item.clone()))
                .collect();
            for (key, item) in target_items {
                insert_server_item(root, inline_parent, &key, item)?;
            }
        }
    }

    Ok(stash.to_string().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_sync::McpServerForSync;
    use std::collections::BTreeMap;

    fn stdio_server(key: &str) -> McpServerForSync {
        McpServerForSync {
            server_key: key.to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "@example/mcp".to_string()],
            env: BTreeMap::from([("MCP_TOKEN".to_string(), "test-token".to_string())]),
            cwd: Some("/must/not/be-managed".to_string()),
            url: None,
            headers: BTreeMap::new(),
        }
    }

    fn remote_server(key: &str, transport: &str) -> McpServerForSync {
        McpServerForSync {
            server_key: key.to_string(),
            transport: transport.to_string(),
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            url: Some("https://mcp.example.test/events".to_string()),
            headers: BTreeMap::from([("Authorization".to_string(), "Bearer test".to_string())]),
        }
    }

    fn table<'a>(document: &'a toml_edit::DocumentMut, key: &str) -> &'a dyn toml_edit::TableLike {
        document["mcp_servers"][key]
            .as_table_like()
            .expect("MCP server table")
    }

    #[test]
    fn grok_mcp_writes_native_stdio_and_remote_fields_without_touching_proxy_config() {
        let mut document = r#"# keep root comment
[models]
default = "aio"
session_summary = "aio"

[model.aio]
model = "grok-build"
base_url = "http://127.0.0.1:37123/grok/v1"
supports_backend_search = true

[mcp_servers.local]
url = "https://local.example.test"

[mcp_servers.managed]
command = "old" # keep command comment
startup_timeout_sec = 7
tool_timeout_sec = 11
custom_field = "keep" # keep field comment
"#
        .parse::<toml_edit::DocumentMut>()
        .expect("fixture TOML");

        apply_grok_mcp_servers(
            &mut document,
            &["managed".to_string()],
            &[stdio_server("managed"), remote_server("remote", "sse")],
        )
        .expect("apply Grok MCP servers");

        let output = document.to_string();
        let reparsed = output
            .parse::<toml_edit::DocumentMut>()
            .expect("valid output TOML");
        assert!(output.starts_with("# keep root comment"));
        assert!(output.contains("command = \"npx\" # keep command comment"));
        assert!(output.contains("custom_field = \"keep\" # keep field comment"));
        assert_eq!(reparsed["models"]["default"].as_str(), Some("aio"));
        assert_eq!(
            reparsed["model"]["aio"]["base_url"].as_str(),
            Some("http://127.0.0.1:37123/grok/v1")
        );
        assert_eq!(
            table(&reparsed, "local")
                .get("url")
                .and_then(toml_edit::Item::as_str),
            Some("https://local.example.test")
        );

        let managed = table(&reparsed, "managed");
        assert_eq!(
            managed.get("command").and_then(toml_edit::Item::as_str),
            Some("npx")
        );
        assert_eq!(
            managed.get("enabled").and_then(toml_edit::Item::as_bool),
            Some(true)
        );
        assert_eq!(
            managed
                .get("startup_timeout_sec")
                .and_then(toml_edit::Item::as_integer),
            Some(7)
        );
        assert_eq!(
            managed
                .get("tool_timeout_sec")
                .and_then(toml_edit::Item::as_integer),
            Some(11)
        );
        assert_eq!(
            managed
                .get("custom_field")
                .and_then(toml_edit::Item::as_str),
            Some("keep")
        );
        assert!(managed.get("cwd").is_none());
        assert!(managed.get("type").is_none());
        assert!(managed.get("url").is_none());
        assert_eq!(
            managed
                .get("env")
                .and_then(toml_edit::Item::as_inline_table)
                .and_then(|env| env.get("MCP_TOKEN"))
                .and_then(toml_edit::Value::as_str),
            Some("test-token")
        );

        let remote = table(&reparsed, "remote");
        assert_eq!(
            remote.get("url").and_then(toml_edit::Item::as_str),
            Some("https://mcp.example.test/events")
        );
        assert_eq!(
            remote.get("enabled").and_then(toml_edit::Item::as_bool),
            Some(true)
        );
        assert!(remote.get("type").is_none());
        assert!(remote.get("http_headers").is_none());
        assert_eq!(
            remote
                .get("headers")
                .and_then(toml_edit::Item::as_inline_table)
                .and_then(|headers| headers.get("Authorization"))
                .and_then(toml_edit::Value::as_str),
            Some("Bearer test")
        );
    }

    #[test]
    fn grok_mcp_removes_only_stale_managed_servers() {
        let mut document = r#"[mcp_servers.stale]
command = "remove"

[mcp_servers.stale.tool_timeouts]
search = 5

[mcp_servers.local]
command = "keep"
"#
        .parse::<toml_edit::DocumentMut>()
        .expect("fixture TOML");

        apply_grok_mcp_servers(&mut document, &["stale".to_string()], &[])
            .expect("remove stale managed server");

        assert!(document["mcp_servers"].get("stale").is_none());
        assert_eq!(
            document["mcp_servers"]["local"]["command"].as_str(),
            Some("keep")
        );
    }

    #[test]
    fn grok_mcp_supports_inline_tables_and_is_idempotent() {
        let mut document =
            r#"mcp_servers = { inline = { command = "old", startup_timeout_sec = 3 } }
"#
            .parse::<toml_edit::DocumentMut>()
            .expect("fixture TOML");
        let server = stdio_server("inline");

        apply_grok_mcp_servers(&mut document, &[], std::slice::from_ref(&server))
            .expect("first apply");
        let first = document.to_string();
        apply_grok_mcp_servers(&mut document, &["inline".to_string()], &[server])
            .expect("second apply");

        assert_eq!(document.to_string(), first);
        assert!(document["mcp_servers"].is_inline_table());
        assert_eq!(
            table(&document, "inline")
                .get("startup_timeout_sec")
                .and_then(toml_edit::Item::as_integer),
            Some(3)
        );
    }

    #[test]
    fn grok_mcp_rejects_invalid_schema_and_transport() {
        let mut invalid_schema = "mcp_servers = \"invalid\"\n"
            .parse::<toml_edit::DocumentMut>()
            .expect("fixture TOML");
        let schema_error = apply_grok_mcp_servers(&mut invalid_schema, &[], &[])
            .expect_err("invalid root schema must fail");
        assert!(schema_error.contains("GROK_CONFIG_INVALID_SCHEMA"));

        let mut document = toml_edit::DocumentMut::new();
        let mut server = remote_server("remote", "websocket");
        server.url = Some("wss://mcp.example.test".to_string());
        let transport_error = apply_grok_mcp_servers(&mut document, &[], &[server])
            .expect_err("unsupported transport must fail");
        assert!(transport_error.contains("unsupported transport"));
    }

    #[test]
    fn grok_local_mcp_swap_preserves_managed_and_proxy_fields() {
        let mut document = r#"# current config
[model.aio]
model = "grok-build"

[mcp_servers.managed]
command = "managed"

[mcp_servers.current_local]
command = "current"
custom = "keep in stash"
"#
        .parse::<toml_edit::DocumentMut>()
        .expect("current TOML");
        let target_stash = br#"# target stash
[mcp_servers.target_local]
url = "https://target.example.test"
headers = { Authorization = "Bearer target" }
"#;

        let current_stash = swap_grok_local_servers(
            &mut document,
            &std::collections::HashSet::from(["managed".to_string()]),
            Some(target_stash),
        )
        .expect("swap local Grok MCP servers");

        let output = document.to_string();
        let current_stash = String::from_utf8(current_stash).expect("UTF-8 stash");
        assert!(output.starts_with("# current config"));
        assert_eq!(
            document["model"]["aio"]["model"].as_str(),
            Some("grok-build")
        );
        assert_eq!(
            document["mcp_servers"]["managed"]["command"].as_str(),
            Some("managed")
        );
        assert!(document["mcp_servers"].get("current_local").is_none());
        assert_eq!(
            document["mcp_servers"]["target_local"]["url"].as_str(),
            Some("https://target.example.test")
        );
        assert!(current_stash.contains("[mcp_servers.current_local]"));
        assert!(current_stash.contains("custom = \"keep in stash\""));
        assert!(!current_stash.contains("mcp_servers.managed"));
    }
}
