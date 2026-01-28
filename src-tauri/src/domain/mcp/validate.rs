//! Usage: Validation + normalization helpers for MCP server management.

pub(super) fn enabled_to_int(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

pub(super) fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

pub(super) fn validate_transport(transport: &str) -> Result<(), String> {
    match transport {
        "stdio" | "http" => Ok(()),
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}")),
    }
}

pub(super) fn validate_server_key(server_key: &str) -> Result<(), String> {
    let key = server_key.trim();
    if key.is_empty() {
        return Err("SEC_INVALID_INPUT: server_key is required".to_string());
    }
    if key.len() > 64 {
        return Err("SEC_INVALID_INPUT: server_key too long (max 64)".to_string());
    }

    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err("SEC_INVALID_INPUT: server_key is required".to_string());
    };
    if !first.is_ascii_alphanumeric() {
        return Err("SEC_INVALID_INPUT: server_key must start with [A-Za-z0-9]".to_string());
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err("SEC_INVALID_INPUT: server_key allows only [A-Za-z0-9_-]".to_string());
        }
    }

    Ok(())
}

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn suggest_key(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
            continue;
        }

        if lower == '_' || lower == '-' {
            if !out.is_empty() && !prev_dash {
                out.push(lower);
                prev_dash = true;
            }
            continue;
        }

        if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    let out = out.trim_matches('-').trim_matches('_').to_string();
    let mut key = if out.is_empty() {
        "mcp-server".to_string()
    } else {
        out
    };
    if !key.chars().next().unwrap_or('a').is_ascii_alphanumeric() {
        key = format!("mcp-{key}");
    }
    if key.len() > 64 {
        key.truncate(64);
    }
    key
}
