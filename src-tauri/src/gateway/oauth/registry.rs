//! Usage: Global singleton registry mapping cli_key → OAuthProvider adapter.

use super::adapters;
use super::provider_trait::OAuthProvider;
use std::collections::HashMap;
use std::sync::OnceLock;

pub(crate) struct OAuthProviderRegistry {
    by_cli_key: HashMap<&'static str, Box<dyn OAuthProvider>>,
    by_provider_type: HashMap<&'static str, &'static str>,
}

impl OAuthProviderRegistry {
    fn new() -> Self {
        let mut by_cli_key: HashMap<&'static str, Box<dyn OAuthProvider>> = HashMap::new();
        let mut by_provider_type: HashMap<&'static str, &'static str> = HashMap::new();

        let claude = adapters::claude::ClaudeOAuthProvider::new();
        by_provider_type.insert(claude.provider_type(), claude.cli_key());
        by_cli_key.insert(claude.cli_key(), Box::new(claude));

        let codex = adapters::codex::CodexOAuthProvider::new();
        by_provider_type.insert(codex.provider_type(), codex.cli_key());
        by_cli_key.insert(codex.cli_key(), Box::new(codex));

        let gemini = adapters::gemini::GeminiOAuthProvider::new();
        by_provider_type.insert(gemini.provider_type(), gemini.cli_key());
        by_cli_key.insert(gemini.cli_key(), Box::new(gemini));

        let grok = adapters::grok::GrokOAuthProvider::new();
        by_provider_type.insert(grok.provider_type(), grok.cli_key());
        by_cli_key.insert(grok.cli_key(), Box::new(grok));

        Self {
            by_cli_key,
            by_provider_type,
        }
    }

    pub(crate) fn get_by_cli_key(&self, cli_key: &str) -> Option<&dyn OAuthProvider> {
        self.by_cli_key.get(cli_key).map(|v| v.as_ref())
    }

    pub(crate) fn get_by_provider_type(&self, provider_type: &str) -> Option<&dyn OAuthProvider> {
        let cli_key = self.by_provider_type.get(provider_type)?;
        self.get_by_cli_key(cli_key)
    }
}

static REGISTRY: OnceLock<OAuthProviderRegistry> = OnceLock::new();

pub(crate) fn global_registry() -> &'static OAuthProviderRegistry {
    REGISTRY.get_or_init(OAuthProviderRegistry::new)
}

/// Resolve an OAuth adapter by provider_type (preferred) or cli_key (fallback),
/// then verify the resolved adapter's cli_key matches the expected one.
///
/// This is the single canonical entry point — all call-sites should use this
/// instead of duplicating the lookup-and-verify logic.
pub(crate) fn resolve_oauth_adapter(
    cli_key: &str,
    provider_id: i64,
    oauth_provider_type: Option<&str>,
) -> Result<&'static dyn super::provider_trait::OAuthProvider, String> {
    let registry = global_registry();
    let provider_type = oauth_provider_type.map(str::trim).unwrap_or_default();
    let adapter = if provider_type.is_empty() {
        registry
            .get_by_cli_key(cli_key)
            .ok_or_else(|| format!("SEC_INVALID_INPUT: no OAuth adapter for cli_key={cli_key}"))?
    } else {
        registry
            .get_by_provider_type(provider_type)
            .ok_or_else(|| {
                format!("SEC_INVALID_INPUT: no OAuth adapter for provider_type={provider_type}")
            })?
    };

    if adapter.cli_key() != cli_key {
        return Err(format!(
            "SEC_INVALID_STATE: oauth adapter mismatch for provider_id={provider_id} \
             (cli_key={cli_key}, provider_type={}, resolved_cli_key={})",
            if provider_type.is_empty() {
                "<empty>"
            } else {
                provider_type
            },
            adapter.cli_key()
        ));
    }

    Ok(adapter)
}

/// Convenience wrapper: resolve adapter from a `ProviderOAuthDetails` struct.
pub(crate) fn resolve_oauth_adapter_for_details(
    details: &crate::providers::ProviderOAuthDetails,
) -> Result<&'static dyn super::provider_trait::OAuthProvider, String> {
    resolve_oauth_adapter(
        &details.cli_key,
        details.id,
        Some(details.oauth_provider_type.as_str()),
    )
}
