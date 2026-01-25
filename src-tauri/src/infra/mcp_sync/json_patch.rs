//! Usage: Patch managed MCP servers into JSON config roots.

pub(super) fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> serde_json::Value {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

pub(super) fn json_to_bytes(value: &serde_json::Value, hint: &str) -> Result<Vec<u8>, String> {
    let mut out =
        serde_json::to_vec_pretty(value).map_err(|e| format!("failed to serialize {hint}: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

pub(super) fn patch_json_mcp_servers(
    mut root: serde_json::Value,
    managed_keys: &[String],
    next: &[(String, serde_json::Value)],
) -> serde_json::Value {
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().expect("root must be object");

    let servers_value = root_obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !servers_value.is_object() {
        *servers_value = serde_json::Value::Object(serde_json::Map::new());
    }

    let servers_obj = servers_value
        .as_object_mut()
        .expect("mcpServers must be object");
    for k in managed_keys {
        servers_obj.remove(k);
    }
    for (k, v) in next {
        servers_obj.insert(k.to_string(), v.clone());
    }

    root
}
