use super::*;
use std::collections::BTreeMap;

fn make_stdio_server(key: &str, env: BTreeMap<String, String>) -> McpServerForSync {
    McpServerForSync {
        server_key: key.to_string(),
        transport: "stdio".to_string(),
        command: Some("npx".to_string()),
        args: vec!["-y".to_string(), "exa-mcp-server@latest".to_string()],
        env,
        cwd: None,
        url: None,
        headers: BTreeMap::new(),
    }
}

fn make_sse_server(key: &str, url: &str, headers: BTreeMap<String, String>) -> McpServerForSync {
    McpServerForSync {
        server_key: key.to_string(),
        transport: "sse".to_string(),
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        cwd: None,
        url: Some(url.to_string()),
        headers,
    }
}

#[test]
fn codex_toml_writes_env_as_nested_table() {
    let mut env = BTreeMap::new();
    env.insert("EXA_API_KEY".to_string(), "test-key-123".to_string());
    let server = make_stdio_server("exa", env);

    let out = build_codex_config_toml(None, &[], &[server]).expect("build_codex_config_toml");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("[mcp_servers.exa.env]"), "{s}");
    assert!(s.contains("EXA_API_KEY = 'test-key-123'"), "{s}");
    assert!(!s.contains("env = {"), "{s}");
}

#[test]
fn codex_toml_removes_nested_env_table_for_managed_keys() {
    let input = r#"[mcp_servers.exa]
type = "stdio"
command = "npx"

[mcp_servers.exa.env]
EXA_API_KEY = 'legacy'

[other]
foo = "bar"
"#;

    let managed_keys = vec!["exa".to_string()];
    let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &managed_keys, &[])
        .expect("build_codex_config_toml");
    let s = String::from_utf8(out).expect("utf8");

    assert!(!s.contains("[mcp_servers.exa]"), "{s}");
    assert!(!s.contains("[mcp_servers.exa.env]"), "{s}");
    assert!(s.contains("[other]"), "{s}");
}

#[test]
fn codex_env_value_with_single_quote_falls_back_to_basic_string() {
    let mut env = BTreeMap::new();
    env.insert("EXA_API_KEY".to_string(), "o'brien".to_string());
    let server = make_stdio_server("exa", env);

    let out = build_codex_config_toml(None, &[], &[server]).expect("build_codex_config_toml");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("EXA_API_KEY = \"o'brien\""), "{s}");
}

#[test]
fn codex_overwrites_existing_mcp_server_even_when_managed_keys_is_empty() {
    let input = r#"[mcp_servers.exa]
type = "stdio"
command = "old"
args = ["--old"]

[mcp_servers.exa.env]
EXA_API_KEY = 'old'
"#;

    let mut env = BTreeMap::new();
    env.insert("EXA_API_KEY".to_string(), "new".to_string());
    let server = make_stdio_server("exa", env);

    let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &[], &[server]).unwrap();
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.matches("[mcp_servers.exa]").count() == 1, "{s}");
    assert!(s.contains("command = \"npx\""), "{s}");
    assert!(s.contains("[mcp_servers.exa.env]"), "{s}");
    assert!(s.contains("EXA_API_KEY = 'new'"), "{s}");
    assert!(!s.contains("command = \"old\""), "{s}");
    assert!(!s.contains("EXA_API_KEY = 'old'"), "{s}");
}

#[test]
fn codex_removes_duplicate_headers_for_same_key() {
    let input = r#"[mcp_servers.exa]
type = "stdio"
command = "a"

[mcp_servers.exa]
type = "stdio"
command = "b"

[mcp_servers.exa.env]
EXA_API_KEY = 'a'

[mcp_servers.exa.env]
EXA_API_KEY = 'b'
"#;

    let mut env = BTreeMap::new();
    env.insert("EXA_API_KEY".to_string(), "new".to_string());
    let server = make_stdio_server("exa", env);

    let out = build_codex_config_toml(Some(input.as_bytes().to_vec()), &[], &[server]).unwrap();
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.matches("[mcp_servers.exa]").count() == 1, "{s}");
    assert!(s.matches("[mcp_servers.exa.env]").count() == 1, "{s}");
    assert!(s.contains("command = \"npx\""), "{s}");
    assert!(s.contains("EXA_API_KEY = 'new'"), "{s}");
    assert!(!s.contains("command = \"a\""), "{s}");
    assert!(!s.contains("command = \"b\""), "{s}");
    assert!(!s.contains("EXA_API_KEY = 'a'"), "{s}");
    assert!(!s.contains("EXA_API_KEY = 'b'"), "{s}");
}

#[test]
fn codex_toml_writes_sse_transport() {
    let mut headers = BTreeMap::new();
    headers.insert("Authorization".to_string(), "Bearer xxx".to_string());
    let server = make_sse_server("remote", "https://mcp.example.com/sse", headers);

    let out = build_codex_config_toml(None, &[], &[server]).expect("build_codex_config_toml");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("[mcp_servers.remote]"), "{s}");
    assert!(s.contains("type = \"sse\""), "{s}");
    assert!(s.contains("url = \"https://mcp.example.com/sse\""), "{s}");
    assert!(s.contains("http_headers = "), "{s}");
    assert!(s.contains("\"Authorization\""), "{s}");
}
