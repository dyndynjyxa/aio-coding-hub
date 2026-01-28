//! Usage: Shared CLI key constants and validation (single source of truth).

pub(crate) const SUPPORTED_CLI_KEYS: [&str; 3] = ["claude", "codex", "gemini"];

pub(crate) fn is_supported_cli_key(cli_key: &str) -> bool {
    SUPPORTED_CLI_KEYS.contains(&cli_key)
}

pub(crate) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    if is_supported_cli_key(cli_key) {
        Ok(())
    } else {
        Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_supported_cli_key_accepts_supported() {
        for cli_key in SUPPORTED_CLI_KEYS {
            assert!(is_supported_cli_key(cli_key));
        }
    }

    #[test]
    fn is_supported_cli_key_rejects_unknown() {
        assert!(!is_supported_cli_key("opencode"));
        assert!(!is_supported_cli_key(""));
    }

    #[test]
    fn validate_cli_key_returns_sec_invalid_input_error() {
        assert_eq!(
            validate_cli_key("opencode").unwrap_err(),
            "SEC_INVALID_INPUT: unknown cli_key=opencode"
        );
    }
}
