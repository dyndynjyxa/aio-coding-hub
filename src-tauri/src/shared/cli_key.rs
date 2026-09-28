//! Usage: Shared CLI key constants, validation, and typed enum (single source of truth).

use crate::shared::error::{AppError, AppResult};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum CliCapability {
    Gateway = 1 << 0,
    Provider = 1 << 1,
    Logs = 1 << 2,
    Usage = 1 << 3,
    Pricing = 1 << 4,
    CliProxy = 1 << 5,
    CliManager = 1 << 6,
    Mcp = 1 << 7,
    Skills = 1 << 8,
    Prompts = 1 << 9,
    Workspaces = 1 << 10,
    Wsl = 1 << 11,
    ManagedUpdate = 1 << 12,
    ProviderPluginTarget = 1 << 13,
}

const GROK_CAPABILITIES: u32 = CliCapability::Gateway as u32
    | CliCapability::Provider as u32
    | CliCapability::Logs as u32
    | CliCapability::Usage as u32
    | CliCapability::Pricing as u32
    | CliCapability::CliProxy as u32
    | CliCapability::CliManager as u32
    | CliCapability::Mcp as u32
    | CliCapability::Skills as u32
    | CliCapability::Prompts as u32
    | CliCapability::Workspaces as u32;
const LEGACY_CLI_CAPABILITIES: u32 = GROK_CAPABILITIES
    | CliCapability::Wsl as u32
    | CliCapability::ManagedUpdate as u32
    | CliCapability::ProviderPluginTarget as u32;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CliSpec {
    pub(crate) key: CliKey,
    capabilities: u32,
}

impl CliSpec {
    pub(crate) fn supports(self, capability: CliCapability) -> bool {
        self.capabilities & capability as u32 != 0
    }
}

pub(crate) const CLI_REGISTRY: [CliSpec; 4] = [
    CliSpec {
        key: CliKey::Claude,
        capabilities: LEGACY_CLI_CAPABILITIES,
    },
    CliSpec {
        key: CliKey::Codex,
        capabilities: LEGACY_CLI_CAPABILITIES,
    },
    CliSpec {
        key: CliKey::Gemini,
        capabilities: LEGACY_CLI_CAPABILITIES,
    },
    CliSpec {
        key: CliKey::Grok,
        capabilities: GROK_CAPABILITIES,
    },
];

pub(crate) const SUPPORTED_CLI_KEYS: [&str; CLI_REGISTRY.len()] = [
    CLI_REGISTRY[0].key.as_str(),
    CLI_REGISTRY[1].key.as_str(),
    CLI_REGISTRY[2].key.as_str(),
    CLI_REGISTRY[3].key.as_str(),
];

pub(crate) fn cli_keys_with(capability: CliCapability) -> impl Iterator<Item = &'static str> {
    CLI_REGISTRY
        .iter()
        .copied()
        .filter(move |spec| spec.supports(capability))
        .map(|spec| spec.key.as_str())
}

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
    Grok,
}

#[allow(dead_code)]
impl CliKey {
    /// Parse a string into a `CliKey`, returning `SEC_INVALID_INPUT` on failure.
    pub(crate) fn parse(s: &str) -> AppResult<Self> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "grok" => Ok(Self::Grok),
            _ => Err(AppError::new(
                "SEC_INVALID_INPUT",
                format!("unknown cli_key={s}"),
            )),
        }
    }

    /// Return the canonical lowercase string representation.
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Grok => "grok",
        }
    }

    pub(crate) fn supports(self, capability: CliCapability) -> bool {
        CLI_REGISTRY
            .iter()
            .find(|spec| spec.key == self)
            .is_some_and(|spec| spec.supports(capability))
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

    const EVERY_CAPABILITY: [CliCapability; 14] = [
        CliCapability::Gateway,
        CliCapability::Provider,
        CliCapability::Logs,
        CliCapability::Usage,
        CliCapability::Pricing,
        CliCapability::CliProxy,
        CliCapability::CliManager,
        CliCapability::Mcp,
        CliCapability::Skills,
        CliCapability::Prompts,
        CliCapability::Workspaces,
        CliCapability::Wsl,
        CliCapability::ManagedUpdate,
        CliCapability::ProviderPluginTarget,
    ];

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
        assert_eq!(CliKey::parse("grok").unwrap(), CliKey::Grok);
    }

    #[test]
    fn grok_capabilities_match_first_release_scope() {
        let supported = [
            CliCapability::Gateway,
            CliCapability::Provider,
            CliCapability::Logs,
            CliCapability::Usage,
            CliCapability::Pricing,
            CliCapability::CliProxy,
            CliCapability::CliManager,
            CliCapability::Mcp,
            CliCapability::Skills,
            CliCapability::Prompts,
            CliCapability::Workspaces,
        ];
        for capability in supported {
            assert!(CliKey::Grok.supports(capability), "missing {capability:?}");
        }

        assert!(!CliKey::Grok.supports(CliCapability::Wsl));
        assert!(!CliKey::Grok.supports(CliCapability::ManagedUpdate));
        assert!(!CliKey::Grok.supports(CliCapability::ProviderPluginTarget));
    }

    #[test]
    fn registry_capability_matrix_is_exact() {
        let expected_without_grok_exclusions = EVERY_CAPABILITY.to_vec();
        for cli_key in [CliKey::Claude, CliKey::Codex, CliKey::Gemini] {
            let actual = EVERY_CAPABILITY
                .into_iter()
                .filter(|capability| cli_key.supports(*capability))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected_without_grok_exclusions, "{cli_key}");
        }

        let actual_grok = EVERY_CAPABILITY
            .into_iter()
            .filter(|capability| CliKey::Grok.supports(*capability))
            .collect::<Vec<_>>();
        assert_eq!(actual_grok, EVERY_CAPABILITY[..11]);
    }

    #[test]
    fn capability_keys_are_derived_from_registry() {
        assert_eq!(
            cli_keys_with(CliCapability::Mcp).collect::<Vec<_>>(),
            vec!["claude", "codex", "gemini", "grok"]
        );
        assert_eq!(
            cli_keys_with(CliCapability::Wsl).collect::<Vec<_>>(),
            vec!["claude", "codex", "gemini"]
        );
        assert_eq!(
            cli_keys_with(CliCapability::ManagedUpdate).collect::<Vec<_>>(),
            vec!["claude", "codex", "gemini"]
        );
        assert_eq!(
            cli_keys_with(CliCapability::ProviderPluginTarget).collect::<Vec<_>>(),
            vec!["claude", "codex", "gemini"]
        );
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
