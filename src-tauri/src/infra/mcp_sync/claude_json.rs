//! Usage: Build Claude MCP config JSON bytes.

use super::json_patch::{json_root_from_bytes, json_to_bytes, patch_json_mcp_servers};
use super::McpServerForSync;

fn build_claude_mcp_spec(server: &McpServerForSync) -> Result<serde_json::Value, String> {
    let transport = server.transport.as_str();
    match transport {
        "stdio" => {
            let command = server
                .command
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: stdio command is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("stdio".to_string()),
            );
            obj.insert(
                "command".to_string(),
                serde_json::Value::String(command.to_string()),
            );
            if !server.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    serde_json::Value::Array(
                        server
                            .args
                            .iter()
                            .map(|v| serde_json::Value::String(v.to_string()))
                            .collect(),
                    ),
                );
            }
            if !server.env.is_empty() {
                let mut env = serde_json::Map::new();
                for (k, v) in &server.env {
                    env.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert("env".to_string(), serde_json::Value::Object(env));
            }
            if let Some(cwd) = server
                .cwd
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                obj.insert(
                    "cwd".to_string(),
                    serde_json::Value::String(cwd.to_string()),
                );
            }
            Ok(serde_json::Value::Object(obj))
        }
        "http" => {
            let url = server
                .url
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "SEC_INVALID_INPUT: http url is required".to_string())?;

            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("http".to_string()),
            );
            obj.insert(
                "url".to_string(),
                serde_json::Value::String(url.to_string()),
            );
            if !server.headers.is_empty() {
                let mut headers = serde_json::Map::new();
                for (k, v) in &server.headers {
                    headers.insert(k.to_string(), serde_json::Value::String(v.to_string()));
                }
                obj.insert("headers".to_string(), serde_json::Value::Object(headers));
            }
            Ok(serde_json::Value::Object(obj))
        }
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
    }
}

pub(super) fn build_claude_config_json(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    let mut next_entries: Vec<(String, serde_json::Value)> = Vec::with_capacity(servers.len());
    for s in servers {
        next_entries.push((s.server_key.to_string(), build_claude_mcp_spec(s)?));
    }

    let root = json_root_from_bytes(current);
    let patched = patch_json_mcp_servers(root, managed_keys, &next_entries);
    json_to_bytes(&patched, "claude.json")
}
