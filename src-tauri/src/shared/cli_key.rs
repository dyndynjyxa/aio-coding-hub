//! Usage: Shared CLI key constants, validation, and typed enum (single source of truth).

use crate::shared::error::{AppError, AppResult};
use std::fmt;

pub(crate) const SUPPORTED_CLI_KEYS: [&str; 3] = ["claude", "codex", "gemini"];

pub(crate) fn is_supported_cli_key(cli_key: &str) -> bool {
    SUPPORTED_CLI_KEYS.contains(&cli_key)
}

pub(crate) fn validate_cli_key(cli_key: &str) -> AppResult<()> {
    if is_supported_cli_key(cli_key) {
        Ok(())
    } else {
        Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into())
    }
}

// ---------------------------------------------------------------------------
// Typed CliKey enum
// ---------------------------------------------------------------------------

/// Type-safe CLI key identifier. Prefer this over raw string comparisons
/// to get compile-time exhaustiveness checking.
///
/// `allow(dead_code)`: introduced ahead of incremental migration; callers
/// will adopt it in subsequent phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum CliKey {
    Claude,
    Codex,
    Gemini,
}

#[allow(dead_code)]
impl CliKey {
    /// Parse a string into a `CliKey`, returning `SEC_INVALID_INPUT` on failure.
    pub(crate) fn parse(s: &str) -> AppResult<Self> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            _ => Err(AppError::new(
                "SEC_INVALID_INPUT",
                format!("unknown cli_key={s}"),
            )),
        }
    }

    /// Return the canonical lowercase string representation.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }
}

impl fmt::Display for CliKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[allow(dead_code)]
impl AsRef<str> for CliKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// Convenience comparisons to ease gradual migration from raw strings.

impl PartialEq<str> for CliKey {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for CliKey {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<CliKey> for str {
    fn eq(&self, other: &CliKey) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<CliKey> for &str {
    fn eq(&self, other: &CliKey) -> bool {
        *self == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- existing tests (string-based helpers) ----

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
            validate_cli_key("opencode").unwrap_err().to_string(),
            "SEC_INVALID_INPUT: unknown cli_key=opencode"
        );
    }

    // ---- CliKey enum tests ----

    #[test]
    fn parse_accepts_all_valid_keys() {
        assert_eq!(CliKey::parse("claude").unwrap(), CliKey::Claude);
        assert_eq!(CliKey::parse("codex").unwrap(), CliKey::Codex);
        assert_eq!(CliKey::parse("gemini").unwrap(), CliKey::Gemini);
    }

    #[test]
    fn parse_rejects_unknown_key() {
        let err = CliKey::parse("opencode").unwrap_err();
        assert_eq!(
            err.to_string(),
            "SEC_INVALID_INPUT: unknown cli_key=opencode"
        );
    }

    #[test]
    fn parse_rejects_empty_string() {
        let err = CliKey::parse("").unwrap_err();
        assert_eq!(err.to_string(), "SEC_INVALID_INPUT: unknown cli_key=");
    }

    #[test]
    fn parse_rejects_wrong_case() {
        assert!(CliKey::parse("Claude").is_err());
        assert!(CliKey::parse("CODEX").is_err());
    }

    #[test]
    fn as_str_roundtrip() {
        for key in [CliKey::Claude, CliKey::Codex, CliKey::Gemini] {
            assert_eq!(CliKey::parse(key.as_str()).unwrap(), key);
        }
    }

    #[test]
    fn display_matches_as_str() {
        for key in [CliKey::Claude, CliKey::Codex, CliKey::Gemini] {
            assert_eq!(format!("{key}"), key.as_str());
        }
    }

    #[test]
    fn as_ref_str() {
        let key = CliKey::Claude;
        let s: &str = key.as_ref();
        assert_eq!(s, "claude");
    }

    #[test]
    fn serde_serialize_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&CliKey::Claude).unwrap(),
            "\"claude\""
        );
        assert_eq!(serde_json::to_string(&CliKey::Codex).unwrap(), "\"codex\"");
        assert_eq!(
            serde_json::to_string(&CliKey::Gemini).unwrap(),
            "\"gemini\""
        );
    }

    #[test]
    fn serde_deserialize_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<CliKey>("\"claude\"").unwrap(),
            CliKey::Claude
        );
        assert_eq!(
            serde_json::from_str::<CliKey>("\"codex\"").unwrap(),
            CliKey::Codex
        );
        assert_eq!(
            serde_json::from_str::<CliKey>("\"gemini\"").unwrap(),
            CliKey::Gemini
        );
    }

    #[test]
    fn serde_deserialize_rejects_unknown() {
        assert!(serde_json::from_str::<CliKey>("\"opencode\"").is_err());
    }

    #[test]
    fn partial_eq_str_comparisons() {
        let key = CliKey::Claude;
        // CliKey == str
        assert!(key == *"claude");
        assert!(key != *"codex");
        // CliKey == &str
        assert!(key == "claude");
        assert!(key != "codex");
        // str == CliKey
        assert!("claude" == key);
        assert!("codex" != key);
    }
}
