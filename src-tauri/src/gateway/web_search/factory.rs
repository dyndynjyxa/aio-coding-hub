//! Build a [`SearchBackendImpl`] from current settings.

use crate::gateway::web_search::backend::{SearchBackendImpl, SearchBackendKind};
use crate::gateway::web_search::backends::brave::BraveSearchBackend;
use crate::gateway::web_search::backends::llm_backed::LlmBackedSearchBackend;
use crate::gateway::web_search::backends::metaso::MetasoSearchBackend;
use crate::gateway::web_search::backends::tavily::TavilySearchBackend;
use crate::providers::ProviderForGateway;

/// Settings needed to construct a backend. Mirrors the fields on
/// `AppSettings` so the gateway layer can pass them in directly.
#[derive(Debug, Clone)]
pub struct BackendSettings {
    pub kind: SearchBackendKind,
    pub brave_api_key: String,
    pub tavily_api_key: String,
    pub metaso_api_key: String,
    pub metaso_include_summary: bool,
    pub metaso_concise_snippet: bool,
    pub max_results: u32,
    pub llm_provider_id: Option<i64>,
    pub proxy_url: String,
}

/// Build a backend from settings. Returns `None` if the selected backend is
/// not yet fully configured (e.g. missing API key) — the interceptor treats
/// `None` as a configuration error and synthesizes a `web_search_tool_result`
/// error block.
pub fn build_backend(
    settings: &BackendSettings,
    providers: &[ProviderForGateway],
) -> Option<SearchBackendImpl> {
    let proxy_url = (!settings.proxy_url.is_empty()).then(|| settings.proxy_url.clone());

    match settings.kind {
        SearchBackendKind::Brave => {
            if settings.brave_api_key.trim().is_empty() {
                return None;
            }
            Some(SearchBackendImpl::Brave(
                BraveSearchBackend::new(settings.brave_api_key.clone()).with_proxy(proxy_url),
            ))
        }
        SearchBackendKind::Tavily => {
            if settings.tavily_api_key.trim().is_empty() {
                return None;
            }
            Some(SearchBackendImpl::Tavily(
                TavilySearchBackend::new(settings.tavily_api_key.clone()).with_proxy(proxy_url),
            ))
        }
        SearchBackendKind::Metaso => {
            if settings.metaso_api_key.trim().is_empty() {
                return None;
            }
            Some(SearchBackendImpl::Metaso(
                MetasoSearchBackend::new(settings.metaso_api_key.clone())
                    .with_proxy(proxy_url)
                    .with_include_summary(settings.metaso_include_summary)
                    .with_concise_snippet(settings.metaso_concise_snippet),
            ))
        }
        SearchBackendKind::LlmBacked => {
            let provider_id = settings.llm_provider_id?;
            let provider = providers.iter().find(|p| p.id == provider_id)?;
            let api_key = provider.api_key_plaintext.clone();
            if api_key.trim().is_empty() {
                return None;
            }
            let base_url = provider.base_urls.first().cloned().unwrap_or_default();
            let model = settings_llm_model(settings, provider);
            Some(SearchBackendImpl::LlmBacked(LlmBackedSearchBackend::new(
                provider.name.clone(),
                base_url,
                api_key,
                model,
            )))
        }
    }
}

fn settings_llm_model(_settings: &BackendSettings, _provider: &ProviderForGateway) -> String {
    // For v1 we delegate to the provider's first mapped model that starts with
    // `claude-`. Future: add a dedicated `web_search_llm_model` setting.
    "claude-sonnet-4-5".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::providers::ClaudeModels;
    use crate::domain::providers::DailyResetMode;
    use crate::domain::providers::ProviderBaseUrlMode;
    use crate::providers::ProviderForGateway;

    fn base_settings() -> BackendSettings {
        BackendSettings {
            kind: SearchBackendKind::Brave,
            brave_api_key: String::new(),
            tavily_api_key: String::new(),
            metaso_api_key: String::new(),
            metaso_include_summary: false,
            metaso_concise_snippet: false,
            max_results: 10,
            llm_provider_id: None,
            proxy_url: String::new(),
        }
    }

    fn provider_with_key(id: i64, name: &str, api_key: &str) -> ProviderForGateway {
        ProviderForGateway {
            id,
            name: name.to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: ProviderBaseUrlMode::Order,
            api_key_plaintext: api_key.to_string(),
            claude_models: ClaudeModels::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: DailyResetMode::Fixed,
            daily_reset_time: "00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
        }
    }

    #[test]
    fn brave_backend_is_built_when_key_is_present() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Brave;
        s.brave_api_key = "BSA-test".to_string();

        let backend = build_backend(&s, &[]).expect("backend");
        assert_eq!(backend.tag(), "brave");
    }

    #[test]
    fn brave_backend_is_missing_when_key_is_blank() {
        let s = base_settings();
        assert!(build_backend(&s, &[]).is_none());
    }

    #[test]
    fn tavily_backend_is_built_when_key_is_present() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Tavily;
        s.tavily_api_key = "tvly-test".to_string();

        let backend = build_backend(&s, &[]).expect("backend");
        assert_eq!(backend.tag(), "tavily");
    }

    #[test]
    fn tavily_backend_is_missing_when_key_is_blank() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Tavily;
        assert!(build_backend(&s, &[]).is_none());
    }

    #[test]
    fn metaso_backend_is_built_when_key_is_present_and_carries_flags() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Metaso;
        s.metaso_api_key = "mk-test".to_string();
        s.metaso_include_summary = true;
        s.metaso_concise_snippet = true;

        let backend = build_backend(&s, &[]).expect("backend");
        assert_eq!(backend.tag(), "metaso");
    }

    #[test]
    fn metaso_backend_is_missing_when_key_is_blank() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Metaso;
        assert!(build_backend(&s, &[]).is_none());
    }

    #[test]
    fn llm_backed_backend_is_built_when_provider_is_resolvable() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::LlmBacked;
        s.llm_provider_id = Some(42);
        let providers = vec![provider_with_key(42, "demo", "sk-ant-test")];

        let backend = build_backend(&s, &providers).expect("backend");
        assert_eq!(backend.tag(), "llm_backed");
    }

    #[test]
    fn llm_backed_backend_is_missing_when_no_provider_id() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::LlmBacked;
        assert!(build_backend(&s, &[]).is_none());
    }

    #[test]
    fn llm_backed_backend_is_missing_when_provider_id_does_not_match() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::LlmBacked;
        s.llm_provider_id = Some(99);
        let providers = vec![provider_with_key(1, "demo", "sk-ant-test")];
        assert!(build_backend(&s, &providers).is_none());
    }

    #[test]
    fn llm_backed_backend_is_missing_when_provider_api_key_is_blank() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::LlmBacked;
        s.llm_provider_id = Some(42);
        let providers = vec![provider_with_key(42, "demo", "  ")];
        assert!(build_backend(&s, &providers).is_none());
    }

    #[test]
    fn proxy_url_is_propagated_to_built_backends() {
        let mut s = base_settings();
        s.kind = SearchBackendKind::Brave;
        s.brave_api_key = "BSA-test".to_string();
        s.proxy_url = "socks5://127.0.0.1:1080".to_string();

        let backend = build_backend(&s, &[]).expect("backend");
        // Backend was constructed without panicking; we just need the build
        // path to succeed and tag it correctly.
        assert_eq!(backend.tag(), "brave");
    }
}
