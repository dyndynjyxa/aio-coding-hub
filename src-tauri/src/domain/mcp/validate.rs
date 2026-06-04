//! Usage: Validation + normalization helpers for MCP server management.

pub(super) fn validate_transport(transport: &str) -> crate::shared::error::AppResult<()> {
    match transport {
        "stdio" | "http" | "sse" => Ok(()),
        other => Err(format!("SEC_INVALID_INPUT: unsupported transport={other}").into()),
    }
}

pub(super) fn validate_server_key(server_key: &str) -> crate::shared::error::AppResult<()> {
    let key = server_key.trim();
    if key.is_empty() {
        return Err("SEC_INVALID_INPUT: server_key is required".into());
    }
    if key.len() > 64 {
        return Err("SEC_INVALID_INPUT: server_key too long (max 64)".into());
    }

    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err("SEC_INVALID_INPUT: server_key is required".into());
    };
    if !first.is_ascii_alphanumeric() {
        return Err("SEC_INVALID_INPUT: server_key must start with [A-Za-z0-9]".into());
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err("SEC_INVALID_INPUT: server_key allows only [A-Za-z0-9_-]".into());
        }
    }

    Ok(())
}

pub(super) fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
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

#[cfg(test)]
mod tests {
    use super::validate_transport;

    #[test]
    fn validate_transport_accepts_all_supported_mcp_transports() {
        for transport in ["stdio", "http", "sse"] {
            assert!(
                validate_transport(transport).is_ok(),
                "transport={transport} should be supported"
            );
        }
    }

    #[test]
    fn validate_transport_rejects_unknown_transports() {
        let err = validate_transport("websocket").expect_err("unknown transport should fail");
        assert!(err.to_string().contains("unsupported transport=websocket"));
    }
}
