mod support;

use serde_json::json;

#[test]
fn mcp_import_from_workspace_cli_flows_are_stable() {
    mcp_import_from_workspace_cli_reads_claude_json_and_imports();
    mcp_import_from_workspace_cli_parses_codex_toml();
    mcp_import_from_workspace_cli_preserves_sse_transport();
    mcp_import_from_workspace_cli_preserves_server_key_casing();
    mcp_import_from_workspace_cli_codex_does_not_create_suffix_key();
}

fn mcp_import_from_workspace_cli_reads_claude_json_and_imports() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let source = json!({
      "mcpServers": {
        "fetch": {
          "type": "stdio",
          "command": "uvx",
          "args": ["mcp-server-fetch"],
          "env": {"FOO": "bar"},
          "cwd": "/tmp/project"
        }
      }
    });

    let bytes = serde_json::to_vec(&source).expect("json bytes");
    aio_coding_hub_lib::test_support::mcp_restore_target_bytes(&handle, "claude", Some(bytes))
        .expect("write claude target");

    let workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "claude")
            .expect("claude active workspace");

    let report =
        aio_coding_hub_lib::test_support::mcp_import_from_workspace_cli_json(&handle, workspace_id)
            .expect("import from workspace cli");

    assert_eq!(report.get("inserted").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(report.get("updated").and_then(|v| v.as_u64()), Some(0));

    let rows = aio_coding_hub_lib::test_support::mcp_servers_list_json(&handle, workspace_id)
        .expect("list imported rows");
    let rows = rows.as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1);

    let first = &rows[0];
    assert_eq!(first.get("name").and_then(|v| v.as_str()), Some("fetch"));
    assert_eq!(
        first.get("transport").and_then(|v| v.as_str()),
        Some("stdio")
    );
    assert_eq!(first.get("command").and_then(|v| v.as_str()), Some("uvx"));
    assert_eq!(first.get("enabled").and_then(|v| v.as_bool()), Some(true));
}

fn mcp_import_from_workspace_cli_preserves_sse_transport() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let source = json!({
      "mcpServers": {
        "remote": {
          "type": "sse",
          "url": "https://example.com/mcp/sse",
          "headers": {"Authorization": "Bearer token"}
        }
      }
    });

    let bytes = serde_json::to_vec(&source).expect("json bytes");
    aio_coding_hub_lib::test_support::mcp_restore_target_bytes(&handle, "claude", Some(bytes))
        .expect("write claude target");

    let workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "claude")
            .expect("claude active workspace");

    let report =
        aio_coding_hub_lib::test_support::mcp_import_from_workspace_cli_json(&handle, workspace_id)
            .expect("import from workspace cli");

    assert_eq!(report.get("inserted").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(report.get("updated").and_then(|v| v.as_u64()), Some(0));

    let rows = aio_coding_hub_lib::test_support::mcp_servers_list_json(&handle, workspace_id)
        .expect("list imported rows");
    let rows = rows.as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1);

    let first = &rows[0];
    assert_eq!(first.get("name").and_then(|v| v.as_str()), Some("remote"));
    assert_eq!(first.get("transport").and_then(|v| v.as_str()), Some("sse"));
    assert_eq!(
        first.get("url").and_then(|v| v.as_str()),
        Some("https://example.com/mcp/sse")
    );
    assert_eq!(
        first
            .get("header_keys")
            .and_then(|v| v.as_array())
            .map(|v| {
                v.iter()
                    .filter_map(|item| item.as_str())
                    .collect::<Vec<_>>()
            }),
        Some(vec!["Authorization"])
    );

    let synced = aio_coding_hub_lib::test_support::mcp_read_target_bytes(&handle, "claude")
        .expect("read synced claude target")
        .expect("synced claude target must exist");
    let root: serde_json::Value = serde_json::from_slice(&synced).expect("parse synced json");
    let remote = &root["mcpServers"]["remote"];
    assert_eq!(remote.get("type").and_then(|v| v.as_str()), Some("sse"));
    assert_eq!(
        remote.get("url").and_then(|v| v.as_str()),
        Some("https://example.com/mcp/sse")
    );
}

fn mcp_import_from_workspace_cli_parses_codex_toml() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let codex_toml = r#"
[mcp_servers.fetch]
type = "stdio"
command = "uvx"
args = ["mcp-server-fetch"]
cwd = "/tmp/workspace"

[mcp_servers.fetch.env]
FOO = "bar"
"#;

    aio_coding_hub_lib::test_support::mcp_restore_target_bytes(
        &handle,
        "codex",
        Some(codex_toml.as_bytes().to_vec()),
    )
    .expect("write codex target");

    let workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "codex")
            .expect("codex active workspace");

    let report =
        aio_coding_hub_lib::test_support::mcp_import_from_workspace_cli_json(&handle, workspace_id)
            .expect("import codex target");

    assert_eq!(report.get("inserted").and_then(|v| v.as_u64()), Some(1));

    let rows = aio_coding_hub_lib::test_support::mcp_servers_list_json(&handle, workspace_id)
        .expect("list imported rows");
    let rows = rows.as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("command").and_then(|v| v.as_str()), Some("uvx"));
}

fn mcp_import_from_workspace_cli_preserves_server_key_casing() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let source = json!({
      "mcpServers": {
        "AMap": {
          "type": "stdio",
          "command": "node",
          "args": ["D:/tools/mcp/amap-maps/start.mjs"],
          "env": {"AMAP_MAPS_API_KEY": "xxx"}
        }
      }
    });

    let bytes = serde_json::to_vec(&source).expect("json bytes");
    aio_coding_hub_lib::test_support::mcp_restore_target_bytes(&handle, "claude", Some(bytes))
        .expect("write claude target");

    let workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "claude")
            .expect("claude active workspace");

    let report =
        aio_coding_hub_lib::test_support::mcp_import_from_workspace_cli_json(&handle, workspace_id)
            .expect("import from workspace cli");

    assert_eq!(report.get("inserted").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(report.get("updated").and_then(|v| v.as_u64()), Some(0));

    let rows = aio_coding_hub_lib::test_support::mcp_servers_list_json(&handle, workspace_id)
        .expect("list imported rows");
    let rows = rows.as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("server_key").and_then(|v| v.as_str()),
        Some("AMap")
    );

    let synced = aio_coding_hub_lib::test_support::mcp_read_target_bytes(&handle, "claude")
        .expect("read synced claude target")
        .expect("synced claude target must exist");
    let root: serde_json::Value = serde_json::from_slice(&synced).expect("parse synced json");
    let servers = root
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .expect("mcpServers object");
    assert!(servers.contains_key("AMap"));
    assert!(!servers.contains_key("amap"));
}

fn mcp_import_from_workspace_cli_codex_does_not_create_suffix_key() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let claude_workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "claude")
            .expect("claude active workspace");
    let seed_report = aio_coding_hub_lib::test_support::mcp_import_servers_json(
        &handle,
        claude_workspace_id,
        serde_json::json!([
          {
            "server_key": "mcp-server",
            "name": "legacy-managed-server",
            "transport": "stdio",
            "command": "node",
            "args": ["legacy.js"],
            "env": {},
            "cwd": null,
            "url": null,
            "headers": {},
            "enabled": false
          }
        ]),
    )
    .expect("seed existing mcp-server key");
    assert_eq!(
        seed_report.get("inserted").and_then(|v| v.as_u64()),
        Some(1)
    );

    let codex_toml = r#"
[mcp_servers.mcp-server]
type = "stdio"
command = "npx"
args = ["-y", "--prefer-online", "fast-context-mcp"]
"#;
    aio_coding_hub_lib::test_support::mcp_restore_target_bytes(
        &handle,
        "codex",
        Some(codex_toml.as_bytes().to_vec()),
    )
    .expect("write codex target");

    let codex_workspace_id =
        aio_coding_hub_lib::test_support::workspace_active_id_by_cli(&handle, "codex")
            .expect("codex active workspace");
    let report = aio_coding_hub_lib::test_support::mcp_import_from_workspace_cli_json(
        &handle,
        codex_workspace_id,
    )
    .expect("import codex target");
    assert_eq!(report.get("inserted").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(report.get("updated").and_then(|v| v.as_u64()), Some(1));

    let synced = aio_coding_hub_lib::test_support::mcp_read_target_bytes(&handle, "codex")
        .expect("read synced codex target")
        .expect("synced codex target must exist");
    let synced_text = String::from_utf8(synced).expect("utf8 codex target");
    assert!(synced_text.contains("[mcp_servers.mcp-server]"));
    assert!(!synced_text.contains("[mcp_servers.mcp-server-2]"));
}
