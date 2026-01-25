//! Usage: Build Codex MCP config TOML bytes.

use std::collections::BTreeMap;

use super::McpServerForSync;

fn remove_toml_table_block(lines: &mut Vec<String>, table_header: &str) -> bool {
    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == table_header {
            start = Some(idx);
            break;
        }
    }

    let Some(start) = start else { return false };

    let end = lines[start.saturating_add(1)..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());

    lines.drain(start..end);
    true
}

fn toml_escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn is_toml_bare_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

fn toml_key(key: &str) -> String {
    if is_toml_bare_key(key) {
        key.to_string()
    } else {
        format!("\"{}\"", toml_escape_string(key))
    }
}

fn toml_escape_basic_string_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if c.is_control() => {
                let code = c as u32;
                if code <= 0xFFFF {
                    out.push_str(&format!("\\u{:04X}", code));
                } else {
                    out.push_str(&format!("\\U{:08X}", code));
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn toml_string_value_prefer_single_quotes(value: &str) -> String {
    if !value.chars().any(|c| c == '\'' || c.is_control()) {
        return format!("'{}'", value);
    }
    format!("\"{}\"", toml_escape_basic_string_value(value))
}

fn toml_inline_table(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return "{}".to_string();
    }

    let items: Vec<String> = map
        .iter()
        .map(|(k, v)| {
            format!(
                "\"{}\" = \"{}\"",
                toml_escape_string(k),
                toml_escape_string(v)
            )
        })
        .collect();
    format!("{{ {} }}", items.join(", "))
}

fn toml_array(values: &[String]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|v| format!("\"{}\"", toml_escape_string(v)))
        .collect();
    format!("[{}]", items.join(", "))
}

pub(super) fn build_codex_config_toml(
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    let mut keys_to_purge: Vec<&str> = Vec::with_capacity(managed_keys.len() + servers.len());
    for key in managed_keys {
        keys_to_purge.push(key.as_str());
    }
    for server in servers {
        keys_to_purge.push(server.server_key.as_str());
    }
    keys_to_purge.sort();
    keys_to_purge.dedup();

    for key in keys_to_purge {
        while remove_toml_table_block(&mut lines, &format!("[mcp_servers.{key}.env]")) {}
        while remove_toml_table_block(&mut lines, &format!("[mcp_servers.{key}]")) {}
    }

    if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
        lines.push(String::new());
    }

    for server in servers {
        let key = server.server_key.as_str();
        lines.push(format!("[mcp_servers.{key}]"));
        let transport = server.transport.as_str();
        match transport {
            "stdio" => {
                let command = server
                    .command
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "SEC_INVALID_INPUT: stdio command is required".to_string())?;
                lines.push("type = \"stdio\"".to_string());
                lines.push(format!("command = \"{}\"", toml_escape_string(command)));
                if !server.args.is_empty() {
                    lines.push(format!("args = {}", toml_array(&server.args)));
                }
                if let Some(cwd) = server
                    .cwd
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!("cwd = \"{}\"", toml_escape_string(cwd)));
                }
            }
            "http" => {
                let url = server
                    .url
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "SEC_INVALID_INPUT: http url is required".to_string())?;
                lines.push("type = \"http\"".to_string());
                lines.push(format!("url = \"{}\"", toml_escape_string(url)));
                if !server.headers.is_empty() {
                    lines.push(format!(
                        "http_headers = {}",
                        toml_inline_table(&server.headers)
                    ));
                }
            }
            other => return Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
        }

        if !server.env.is_empty() {
            lines.push(String::new());
            lines.push(format!("[mcp_servers.{key}.env]"));
            for (env_key, env_value) in &server.env {
                lines.push(format!(
                    "{} = {}",
                    toml_key(env_key),
                    toml_string_value_prefer_single_quotes(env_value)
                ));
            }
        }

        lines.push(String::new());
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
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
}
