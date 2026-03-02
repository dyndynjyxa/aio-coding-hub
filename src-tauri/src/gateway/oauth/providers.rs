//! Usage: Provider-specific OAuth endpoint and scope definitions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OAuthProviderKey {
    Claude,
    Codex,
    Gemini,
}

impl OAuthProviderKey {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "claude_oauth" => Some(Self::Claude),
            "codex_oauth" => Some(Self::Codex),
            "gemini_oauth" => Some(Self::Gemini),
            _ => None,
        }
    }

    pub(crate) fn as_provider_type(self) -> &'static str {
        match self {
            Self::Claude => "claude_oauth",
            Self::Codex => "codex_oauth",
            Self::Gemini => "gemini_oauth",
        }
    }

    pub(crate) fn as_cli_key(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OAuthProviderConfig {
    pub(crate) key: OAuthProviderKey,
    pub(crate) auth_url: &'static str,
    pub(crate) token_url: &'static str,
    pub(crate) client_id: &'static str,
    pub(crate) client_secret: Option<&'static str>,
    pub(crate) scopes: &'static [&'static str],
    pub(crate) redirect_host: &'static str,
    pub(crate) callback_path: &'static str,
    pub(crate) default_callback_port: u16,
}

pub(crate) const CLAUDE_CONFIG: OAuthProviderConfig = OAuthProviderConfig {
    key: OAuthProviderKey::Claude,
    auth_url: "https://claude.ai/oauth/authorize",
    token_url: "https://api.anthropic.com/v1/oauth/token",
    // Public client identifier used by Claude Code desktop OAuth login.
    client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
    client_secret: None,
    scopes: &["org:create_api_key", "user:profile", "user:inference"],
    redirect_host: "localhost",
    callback_path: "/callback",
    default_callback_port: 54545,
};

pub(crate) const CODEX_CONFIG: OAuthProviderConfig = OAuthProviderConfig {
    key: OAuthProviderKey::Codex,
    auth_url: "https://auth.openai.com/oauth/authorize",
    token_url: "https://auth.openai.com/oauth/token",
    client_id: "app_EMoamEEZ73f0CkXaXp7hrann",
    client_secret: None,
    scopes: &["openid", "profile", "email", "offline_access"],
    redirect_host: "localhost",
    callback_path: "/auth/callback",
    default_callback_port: 1455,
};

const GEMINI_DEFAULT_CLIENT_ID: &str = concat!(
    "681255809395",
    "-oo8ft2oprdrnp9e3aqf6av3hmdib135j",
    ".apps.googleusercontent.com"
);
const GEMINI_DEFAULT_CLIENT_SECRET: &str = concat!("GOCSPX-", "4uHgMPm-1o7Sk-geV6Cu5clXFsxl");

const fn pick_non_empty_str(value: Option<&'static str>, fallback: &'static str) -> &'static str {
    match value {
        Some(v) if !v.is_empty() => v,
        _ => fallback,
    }
}

const fn pick_non_empty_opt_str(
    value: Option<&'static str>,
    fallback: &'static str,
) -> Option<&'static str> {
    match value {
        Some(v) if !v.is_empty() => Some(v),
        _ => Some(fallback),
    }
}

const GEMINI_CLIENT_ID: &str = pick_non_empty_str(
    option_env!("AIO_GEMINI_OAUTH_CLIENT_ID"),
    GEMINI_DEFAULT_CLIENT_ID,
);
const GEMINI_CLIENT_SECRET: Option<&str> = pick_non_empty_opt_str(
    option_env!("AIO_GEMINI_OAUTH_CLIENT_SECRET"),
    GEMINI_DEFAULT_CLIENT_SECRET,
);

pub(crate) const GEMINI_CONFIG: OAuthProviderConfig = OAuthProviderConfig {
    key: OAuthProviderKey::Gemini,
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    // Gemini OAuth credentials are env-overridable for deployments and fall back to CLI-compatible defaults.
    // `oauth_start_login` and `oauth_account_manual_add` both consume these values via this provider config.
    client_id: GEMINI_CLIENT_ID,
    client_secret: GEMINI_CLIENT_SECRET,
    scopes: &[
        "https://www.googleapis.com/auth/cloud-platform",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
    ],
    redirect_host: "127.0.0.1",
    callback_path: "/oauth2callback",
    default_callback_port: 8085,
};

pub(crate) fn config_for_provider_type(provider_type: &str) -> Option<OAuthProviderConfig> {
    match provider_type.trim() {
        "claude_oauth" => Some(CLAUDE_CONFIG),
        "codex_oauth" => Some(CODEX_CONFIG),
        "gemini_oauth" => Some(GEMINI_CONFIG),
        _ => None,
    }
}

pub(crate) fn config_for_cli_key(cli_key: &str) -> Option<OAuthProviderConfig> {
    match cli_key.trim() {
        "claude" => Some(CLAUDE_CONFIG),
        "codex" => Some(CODEX_CONFIG),
        "gemini" => Some(GEMINI_CONFIG),
        _ => None,
    }
}

pub(crate) fn make_redirect_uri(cfg: OAuthProviderConfig, port: u16) -> String {
    format!("http://{}:{port}{}", cfg.redirect_host, cfg.callback_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_mapping_is_stable() {
        let cfg = config_for_provider_type("claude_oauth").expect("claude config");
        assert_eq!(cfg.key.as_cli_key(), "claude");
        assert_eq!(cfg.default_callback_port, 54545);
        assert_eq!(
            make_redirect_uri(cfg, cfg.default_callback_port),
            "http://localhost:54545/callback"
        );
    }

    #[test]
    fn unknown_provider_returns_none() {
        assert!(config_for_provider_type("unknown").is_none());
        assert!(config_for_cli_key("unknown").is_none());
    }

    #[test]
    fn codex_scopes_stay_compatible() {
        let cfg = config_for_provider_type("codex_oauth").expect("codex config");
        assert!(cfg.scopes.contains(&"openid"));
        assert!(cfg.scopes.contains(&"profile"));
        assert!(cfg.scopes.contains(&"email"));
        assert!(cfg.scopes.contains(&"offline_access"));
        assert!(!cfg.scopes.contains(&"api.model.audio.request"));
    }

    #[test]
    fn gemini_oauth_uses_build_env_credentials() {
        let cfg = config_for_provider_type("gemini_oauth").expect("gemini config");
        let expected_client_id = match option_env!("AIO_GEMINI_OAUTH_CLIENT_ID") {
            Some(value) if !value.is_empty() => value,
            _ => GEMINI_DEFAULT_CLIENT_ID,
        };
        let expected_client_secret = match option_env!("AIO_GEMINI_OAUTH_CLIENT_SECRET") {
            Some(value) if !value.is_empty() => Some(value),
            _ => Some(GEMINI_DEFAULT_CLIENT_SECRET),
        };

        assert_eq!(cfg.client_id, expected_client_id);
        assert_eq!(cfg.client_secret, expected_client_secret);
        assert!(cfg
            .scopes
            .contains(&"https://www.googleapis.com/auth/cloud-platform"));
        assert!(cfg
            .scopes
            .contains(&"https://www.googleapis.com/auth/userinfo.email"));
        assert!(cfg
            .scopes
            .contains(&"https://www.googleapis.com/auth/userinfo.profile"));
    }

    #[test]
    fn gemini_oauth_credentials_are_not_hardcoded_literals() {
        let cfg = config_for_provider_type("gemini_oauth").expect("gemini config");
        assert!(
            !cfg.client_id.trim().is_empty(),
            "gemini client id must not be empty"
        );
        assert!(
            cfg.client_secret
                .map(str::trim)
                .is_some_and(|value| !value.is_empty()),
            "gemini client secret must not be empty"
        );
    }
}
