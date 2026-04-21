//! Global singleton registry mapping cli_key -> CliAuthStrategy adapter.
//!
//! Pattern follows `OAuthProviderRegistry` (OnceLock + HashMap).

use super::claude::ClaudeAuthStrategy;
use super::codex::CodexAuthStrategy;
use super::gemini::GeminiAuthStrategy;
use super::strategy::CliAuthStrategy;
use std::collections::HashMap;
use std::sync::OnceLock;

pub(crate) struct CliAuthRegistry {
    by_cli_key: HashMap<&'static str, Box<dyn CliAuthStrategy>>,
}

impl CliAuthRegistry {
    fn new() -> Self {
        let mut by_cli_key: HashMap<&'static str, Box<dyn CliAuthStrategy>> = HashMap::new();

        let claude = ClaudeAuthStrategy;
        by_cli_key.insert(claude.cli_key_str(), Box::new(claude));

        let codex = CodexAuthStrategy;
        by_cli_key.insert(codex.cli_key_str(), Box::new(codex));

        let gemini = GeminiAuthStrategy;
        by_cli_key.insert(gemini.cli_key_str(), Box::new(gemini));

        Self { by_cli_key }
    }

    pub(crate) fn get(&self, cli_key: &str) -> Option<&dyn CliAuthStrategy> {
        self.by_cli_key.get(cli_key).map(|v| v.as_ref())
    }
}

static REGISTRY: OnceLock<CliAuthRegistry> = OnceLock::new();

pub(crate) fn global_cli_auth_registry() -> &'static CliAuthRegistry {
    REGISTRY.get_or_init(CliAuthRegistry::new)
}
