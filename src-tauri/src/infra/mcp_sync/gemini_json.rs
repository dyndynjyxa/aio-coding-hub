//! Usage: Write Antigravity CLI MCP config and release legacy Gemini CLI entries.

use super::json_patch::json_to_bytes;
use super::McpServerForSync;
use serde_json::{json, Value};

pub(crate) fn parse_mcp_root(bytes: Option<&[u8]>) -> Result<Value, String> {
    let Some(bytes) = bytes else {
        return Ok(json!({}));
    };
    let root: Value = serde_json::from_slice(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
        .map_err(|e| format!("ANTIGRAVITY_MCP_INVALID_CONFIG: {e}"))?;
    if !root.is_object() || root.get("mcpServers").is_some_and(|v| !v.is_object()) {
        return Err("ANTIGRAVITY_MCP_INVALID_CONFIG: config and mcpServers must be objects".into());
    }
    Ok(root)
}

fn server_spec(server: &McpServerForSync) -> Result<Value, String> {
    match server.transport.as_str() {
        "stdio" => {
            let command = server.command.as_deref().map(str::trim).filter(|s| !s.is_empty())
                .ok_or("SEC_INVALID_INPUT: stdio command is required")?;
            let mut spec = json!({"command": command});
            if !server.args.is_empty() { spec["args"] = json!(server.args); }
            if !server.env.is_empty() { spec["env"] = json!(server.env); }
            if let Some(cwd) = server.cwd.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                spec["cwd"] = json!(cwd);
            }
            Ok(spec)
        }
        "http" => {
            let url = server.url.as_deref().map(str::trim).filter(|s| !s.is_empty())
                .ok_or("SEC_INVALID_INPUT: http url is required")?;
            let mut spec = json!({"serverUrl": url});
            if !server.headers.is_empty() { spec["headers"] = json!(server.headers); }
            Ok(spec)
        }
        "sse" => Err(format!("SEC_INVALID_INPUT: Antigravity CLI 不支持旧版 SSE，请停用 {} 或改用 Streamable HTTP 端点", server.server_key)),
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
    }
}

pub(crate) struct AntigravityMcpMerge {
    pub(crate) bytes: Vec<u8>,
    pub(crate) managed_keys: Vec<String>,
}

pub(crate) fn merge_antigravity_mcp(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<AntigravityMcpMerge, String> {
    let mut accepted = Vec::new();
    let mut root = parse_mcp_root(current.as_deref())?;
    let original = root.clone();
    let entries = root
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap();

    for server in servers {
        let key = &server.server_key;
        let desired = server_spec(server)?;
        accepted.push(key.clone());
        let Some(entry) = entries.get_mut(key) else {
            entries.insert(key.clone(), desired);
            continue;
        };
        // Like the other CLIs, AIO takes over a same-name entry. Keep agy-native
        // options (oauth, enabledTools, ...) but replace the connection, including
        // agy's "url" alias and a stale disabled flag.
        let mut native = entry.as_object().cloned().unwrap_or_default();
        for field in [
            "command",
            "args",
            "env",
            "cwd",
            "serverUrl",
            "url",
            "headers",
            "disabled",
        ] {
            native.remove(field);
        }
        native.extend(desired.as_object().unwrap().clone());
        *entry = Value::Object(native);
    }
    for key in managed_keys {
        if !servers.iter().any(|s| &s.server_key == key) {
            entries.remove(key);
        }
    }
    let bytes = match current {
        Some(bytes) if root == original => bytes,
        _ => json_to_bytes(&root, "Antigravity mcp_config.json")?,
    };
    if bytes.len() > super::MCP_SYNC_TARGET_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: Antigravity MCP config exceeds {} bytes",
            super::MCP_SYNC_TARGET_MAX_BYTES
        ));
    }
    accepted.sort();
    accepted.dedup();
    Ok(AntigravityMcpMerge {
        bytes,
        managed_keys: accepted,
    })
}

/// Restore only keys owned by the old manifest. An existing key may have been
/// overwritten when Gemini CLI was first enabled; its original is in the backup.
pub(crate) fn release_legacy_entries(
    current: &[u8],
    backup: Option<&[u8]>,
    managed_keys: &[String],
) -> Result<Option<Vec<u8>>, String> {
    if managed_keys.is_empty() {
        return Ok(None);
    }
    use jsonc_parser::cst::{CstNode, CstRootNode};

    fn parse(bytes: &[u8]) -> Result<CstRootNode, String> {
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
        let text = std::str::from_utf8(bytes).map_err(|_| "旧 Gemini MCP 配置不是 UTF-8")?;
        let doc = CstRootNode::parse(
            text,
            &jsonc_parser::ParseOptions {
                allow_comments: true,
                allow_trailing_commas: true,
                allow_loose_object_property_names: false,
                allow_missing_commas: false,
                allow_single_quoted_strings: false,
                allow_hexadecimal_numbers: false,
                allow_unary_plus_numbers: false,
                allow_bare_decimal_point_numbers: false,
                allow_non_finite_numbers: false,
                allow_extended_string_escapes: false,
            },
        )
        .map_err(|_| "旧 Gemini MCP 配置不是有效的 JSONC")?;
        let root = doc.object_value().ok_or("旧 Gemini MCP 配置不是对象")?;
        if root.get("mcpServers").is_some() && root.object_value("mcpServers").is_none() {
            return Err("旧 Gemini mcpServers 不是对象".into());
        }
        Ok(doc)
    }

    // The CST exposes sibling indices but not byte ranges. Reconstruct the
    // range after removals, so a restored value keeps its exact backup text.
    fn start_offset(mut node: CstNode) -> usize {
        let mut offset = 0;
        while let Some(parent) = node.parent() {
            offset += parent
                .children()
                .iter()
                .take(node.child_index())
                .map(|sibling| sibling.to_string().len())
                .sum::<usize>();
            node = parent.into();
        }
        offset
    }

    let doc = parse(current)?;
    let Some(entries) = doc
        .object_value()
        .and_then(|r| r.object_value("mcpServers"))
    else {
        return Ok(None);
    };
    let original = backup
        .map(parse)
        .transpose()?
        .and_then(|doc| doc.object_value()?.object_value("mcpServers"));
    let mut changed = false;
    let mut replacements = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for key in managed_keys {
        if !seen.insert(key.as_str()) {
            continue;
        }
        // If the user already removed it from the retired client, leave it absent.
        let Some(entry) = entries.get(key) else {
            continue;
        };
        if let Some(initial) = original.as_ref().and_then(|o| o.get(key)) {
            let original_text = initial
                .value()
                .ok_or("缺少旧 Gemini MCP 备份值")?
                .to_string();
            let current = entry.value().ok_or("缺少旧 Gemini MCP 配置值")?;
            let current_text = current.to_string();
            if current_text == original_text {
                continue;
            }
            replacements.push((current, current_text.len(), original_text));
        } else {
            entry.remove();
        }
        changed = true;
    }
    Ok(changed.then(|| {
        let mut bytes = current
            .strip_prefix(b"\xef\xbb\xbf")
            .map_or_else(Vec::new, |_| b"\xef\xbb\xbf".to_vec());
        let mut doc_text = doc.to_string();
        let mut patches: Vec<_> = replacements
            .into_iter()
            .map(|(node, len, raw)| (start_offset(node), len, raw))
            .collect();
        patches.sort_unstable_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        for (start, len, raw) in patches {
            doc_text.replace_range(start..start + len, &raw);
        }
        bytes.extend_from_slice(doc_text.as_bytes());
        bytes
    }))
}
