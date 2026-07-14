//! Usage: Supported CLI keys for MCP sync flows.

pub(super) static MCP_CLI_KEYS: std::sync::LazyLock<Vec<&'static str>> =
    std::sync::LazyLock::new(|| {
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Mcp).collect()
    });

pub(super) fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn mcp_cli_keys_match_mcp_capability() {
        assert_eq!(
            *MCP_CLI_KEYS,
            crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Mcp)
                .collect::<Vec<_>>()
        );
        assert!(MCP_CLI_KEYS.contains(&"grok"));
    }

    #[test]
    fn mcp_cli_keys_are_unique() {
        let mut keys = HashSet::new();
        for key in MCP_CLI_KEYS.iter().copied() {
            assert!(keys.insert(key));
        }
    }
}
