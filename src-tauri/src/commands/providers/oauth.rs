use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, providers};

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthStartFlowResult {
    pub success: bool,
    pub provider_id: i64,
    pub provider_type: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthRefreshResult {
    pub success: bool,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthDisconnectResult {
    pub success: bool,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthStatusResult {
    pub connected: bool,
    pub provider_type: Option<String>,
    pub email: Option<String>,
    pub expires_at: Option<i64>,
    pub has_refresh_token: Option<bool>,
}

fn build_oauth_authorize_url(
    endpoints: &crate::gateway::oauth::provider_trait::OAuthEndpoints,
    redirect_uri: &str,
    oauth_state: &str,
    code_challenge: &str,
    extra_params: &[(&'static str, &'static str)],
) -> String {
    let scopes = endpoints.scopes.join(" ");
    let mut authorize_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        endpoints.auth_url,
        crate::gateway::util::encode_url_component(&endpoints.client_id),
        crate::gateway::util::encode_url_component(redirect_uri),
        crate::gateway::util::encode_url_component(&scopes),
        crate::gateway::util::encode_url_component(oauth_state),
        crate::gateway::util::encode_url_component(code_challenge),
    );

    for (key, value) in extra_params {
        authorize_url.push('&');
        authorize_url.push_str(&crate::gateway::util::encode_url_component(key));
        authorize_url.push('=');
        authorize_url.push_str(&crate::gateway::util::encode_url_component(value));
    }

    authorize_url
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_start_flow(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    provider_id: i64,
) -> Result<ProviderOAuthStartFlowResult, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let provider_cli_key = blocking::run("provider_oauth_start_flow_load_provider_cli_key", {
        let db = db.clone();
        move || {
            providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found".to_string())
            })
        }
    })
    .await
    .map_err(Into::<String>::into)?;

    if provider_cli_key != cli_key {
        return Err(format!(
            "SEC_INVALID_INPUT: provider cli_key mismatch for provider_id={provider_id} (expected={provider_cli_key}, got={cli_key})"
        ));
    }

    // 1. Lookup OAuth provider adapter from registry
    let adapter = crate::gateway::oauth::registry::global_registry()
        .get_by_cli_key(&provider_cli_key)
        .ok_or_else(|| format!("no OAuth adapter for cli_key={provider_cli_key}"))?;

    let endpoints = adapter.endpoints();

    // 2. Generate PKCE pair
    let pkce = crate::gateway::oauth::pkce::generate_pkce_pair();

    // 3. Generate random state
    use rand::RngCore;
    let mut state_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let oauth_state = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        state_bytes,
    );

    // 3b. Cancel any prior pending OAuth flow so its listener is dropped (frees port).
    let mut abort_rx = crate::gateway::oauth::cancel_previous_flow();

    // 4. Bind callback listener
    let listener = crate::gateway::oauth::callback_server::bind_callback_listener(
        endpoints.default_callback_port,
    )
    .await
    .map_err(|e| format!("failed to bind callback listener: {e}"))?;

    let redirect_uri =
        crate::gateway::oauth::provider_trait::make_redirect_uri(endpoints, listener.port);

    // 5. Build authorize URL
    // 对齐官方 Codex 登录 URL 形状，不再强制追加 prompt=login。
    // 这样可避免偏离上游登录流，降低浏览器端 unknown_error 风险。
    let authorize_url = build_oauth_authorize_url(
        endpoints,
        &redirect_uri,
        &oauth_state,
        &pkce.code_challenge,
        &adapter.extra_authorize_params(),
    );

    // 6. Open browser
    tauri_plugin_opener::open_url(&authorize_url, None::<&str>)
        .map_err(|e| format!("failed to open OAuth authorize URL: {e}"))?;

    // 7. Wait for callback (300s timeout), but abort if a newer flow cancels us.
    let callback = tokio::select! {
        result = listener.wait_for_callback(&oauth_state, 300) => {
            result.map_err(|e| format!("OAuth callback failed: {e}"))?
        }
        _ = abort_rx.changed() => {
            return Err("OAuth flow cancelled: a new login attempt was started".to_string());
        }
    };

    let code = callback
        .code
        .ok_or("OAuth callback missing authorization code")?;

    // 8. Exchange code for tokens
    let client = crate::gateway::oauth::build_default_oauth_http_client()?;
    let token_set = crate::gateway::oauth::token_exchange::exchange_authorization_code(
        &client,
        &crate::gateway::oauth::token_exchange::TokenExchangeRequest {
            token_uri: endpoints.token_url.to_string(),
            client_id: endpoints.client_id.clone(),
            client_secret: endpoints.client_secret.clone(),
            code,
            redirect_uri,
            code_verifier: pkce.code_verifier,
            state: Some(oauth_state),
        },
    )
    .await
    .map_err(|e| format!("token exchange failed: {e}"))?;

    // 9. Resolve effective token
    let (effective_token, id_token) = adapter.resolve_effective_token(&token_set, None);
    let token_expires_at = token_set.expires_at;
    let provider_type = adapter.provider_type();

    // 10. Save to provider
    let app_handle = app.clone();
    blocking::run("provider_oauth_start_flow_save", move || {
        crate::providers::update_oauth_tokens(
            &db,
            provider_id,
            "oauth",
            provider_type,
            &effective_token,
            token_set.refresh_token.as_deref(),
            id_token.as_deref(),
            endpoints.token_url,
            &endpoints.client_id,
            endpoints.client_secret.as_deref(),
            token_expires_at,
            None,
        )
    })
    .await
    .map_err(Into::<String>::into)?;

    crate::gateway::events::emit_gateway_log(
        &app_handle,
        "info",
        "OAUTH_LOGIN_OK",
        format!("OAuth 登录成功：provider_id={provider_id} type={provider_type}"),
    );

    Ok(ProviderOAuthStartFlowResult {
        success: true,
        provider_id,
        provider_type: provider_type.to_string(),
        expires_at: token_expires_at,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_refresh(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<ProviderOAuthRefreshResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;

    let details = blocking::run("provider_oauth_refresh_load", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;

    let token_uri = details
        .oauth_token_uri
        .as_deref()
        .ok_or("provider missing token_uri")?
        .to_string();
    let client_id = details
        .oauth_client_id
        .as_deref()
        .ok_or("provider missing client_id")?
        .to_string();
    let refresh_token = details
        .oauth_refresh_token
        .as_deref()
        .ok_or("provider missing refresh_token")?
        .to_string();

    let client = crate::gateway::oauth::build_default_oauth_http_client()?;
    let token_set = crate::gateway::oauth::refresh::refresh_provider_token_with_retry(
        &client,
        &token_uri,
        &client_id,
        details.oauth_client_secret.as_deref(),
        &refresh_token,
    )
    .await
    .map_err(|e| format!("token refresh failed: {e}"))?;

    // Resolve effective token via validated adapter.
    let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(&details)?;
    let (effective_token, id_token) =
        adapter.resolve_effective_token(&token_set, details.oauth_id_token.as_deref());

    let new_refresh_token = token_set
        .refresh_token
        .as_deref()
        .or(Some(refresh_token.as_str()))
        .map(str::to_string);
    let oauth_provider_type = if details.oauth_provider_type.trim().is_empty() {
        adapter.provider_type().to_string()
    } else {
        details.oauth_provider_type.clone()
    };
    let oauth_client_secret = details.oauth_client_secret.clone();
    let oauth_email = details.oauth_email.clone();
    let expires_at = token_set.expires_at;
    let expected_last_refreshed_at = details.oauth_last_refreshed_at;

    let persisted = blocking::run("provider_oauth_refresh_save", move || {
        crate::providers::update_oauth_tokens_if_last_refreshed_matches(
            &db,
            provider_id,
            "oauth",
            &oauth_provider_type,
            &effective_token,
            new_refresh_token.as_deref(),
            id_token.as_deref(),
            &token_uri,
            &client_id,
            oauth_client_secret.as_deref(),
            expires_at,
            oauth_email.as_deref(),
            expected_last_refreshed_at,
        )
    })
    .await
    .map_err(Into::<String>::into)?;
    if !persisted {
        return Err(format!(
            "OAUTH_REFRESH_CONFLICT: provider_id={provider_id} tokens updated concurrently; retry refresh"
        ));
    }

    Ok(ProviderOAuthRefreshResult {
        success: true,
        expires_at: token_set.expires_at,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_disconnect(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<ProviderOAuthDisconnectResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("provider_oauth_disconnect", move || {
        crate::providers::clear_oauth(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    Ok(ProviderOAuthDisconnectResult { success: true })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_status(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<ProviderOAuthStatusResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_oauth_status", move || {
        crate::providers::get_oauth_details(&db, provider_id)
    })
    .await;

    match result {
        Ok(details) => Ok(ProviderOAuthStatusResult {
            connected: true,
            provider_type: Some(details.oauth_provider_type),
            email: details.oauth_email,
            expires_at: details.oauth_expires_at,
            has_refresh_token: Some(details.oauth_refresh_token.is_some()),
        }),
        Err(e) => {
            let err_str = e.to_string();
            // DB_NOT_FOUND = provider exists but has no OAuth tokens → expected disconnected state.
            // Any other error (DB_ERROR, INTERNAL_ERROR) is a real failure that must surface.
            if err_str.starts_with("DB_NOT_FOUND") {
                Ok(ProviderOAuthStatusResult {
                    connected: false,
                    provider_type: None,
                    email: None,
                    expires_at: None,
                    has_refresh_token: None,
                })
            } else {
                tracing::warn!(
                    provider_id,
                    "provider_oauth_status unexpected error: {err_str}"
                );
                Err(format!("provider_oauth_status failed: {err_str}"))
            }
        }
    }
}

pub(super) fn oauth_details_can_refresh(details: &crate::providers::ProviderOAuthDetails) -> bool {
    details
        .oauth_refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && details
            .oauth_token_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
        && details
            .oauth_client_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
}

pub(super) fn effective_oauth_access_token(
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
) -> Result<String, String> {
    let token_set = crate::gateway::oauth::provider_trait::OAuthTokenSet {
        access_token: details.oauth_access_token.clone(),
        refresh_token: details.oauth_refresh_token.clone(),
        expires_at: details.oauth_expires_at,
        id_token: details.oauth_id_token.clone(),
    };
    let (token, _) = adapter.resolve_effective_token(&token_set, details.oauth_id_token.as_deref());
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("OAuth access token is empty".to_string());
    }
    Ok(token)
}

pub(super) async fn refresh_oauth_details_for_limits(
    db: &crate::db::Db,
    client: &reqwest::Client,
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
) -> Result<crate::providers::ProviderOAuthDetails, String> {
    let provider_id = details.id;
    let token_uri = details
        .oauth_token_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing token_uri")?
        .to_string();
    let client_id = details
        .oauth_client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing client_id")?
        .to_string();
    let refresh_token = details
        .oauth_refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing refresh_token")?
        .to_string();

    let token_set = crate::gateway::oauth::refresh::refresh_provider_token_with_retry(
        client,
        &token_uri,
        &client_id,
        details.oauth_client_secret.as_deref(),
        &refresh_token,
    )
    .await
    .map_err(|e| format!("token refresh failed: {e}"))?;

    let (effective_token, id_token) =
        adapter.resolve_effective_token(&token_set, details.oauth_id_token.as_deref());
    if effective_token.trim().is_empty() {
        return Err("token refresh failed: refreshed access_token is empty".to_string());
    }

    let oauth_provider_type = if details.oauth_provider_type.trim().is_empty() {
        adapter.provider_type().to_string()
    } else {
        details.oauth_provider_type.clone()
    };
    let oauth_client_secret = details.oauth_client_secret.clone();
    let oauth_email = details.oauth_email.clone();
    let new_refresh_token = token_set
        .refresh_token
        .as_deref()
        .or(Some(refresh_token.as_str()))
        .map(str::to_string);
    let expires_at = token_set.expires_at;
    let expected_last_refreshed_at = details.oauth_last_refreshed_at;

    let persisted = blocking::run("provider_oauth_fetch_limits_refresh_save", {
        let db = db.clone();
        let oauth_provider_type = oauth_provider_type.clone();
        let effective_token = effective_token.clone();
        let id_token = id_token.clone();
        let token_uri = token_uri.clone();
        let client_id = client_id.clone();
        let oauth_client_secret = oauth_client_secret.clone();
        let oauth_email = oauth_email.clone();
        let new_refresh_token = new_refresh_token.clone();
        move || {
            crate::providers::update_oauth_tokens_if_last_refreshed_matches(
                &db,
                provider_id,
                "oauth",
                &oauth_provider_type,
                &effective_token,
                new_refresh_token.as_deref(),
                id_token.as_deref(),
                &token_uri,
                &client_id,
                oauth_client_secret.as_deref(),
                expires_at,
                oauth_email.as_deref(),
                expected_last_refreshed_at,
            )
        }
    })
    .await
    .map_err(Into::<String>::into)?;

    if !persisted {
        tracing::info!(
            provider_id,
            "provider_oauth_fetch_limits: refresh CAS conflict, reloading latest tokens"
        );
    }

    blocking::run("provider_oauth_fetch_limits_reload", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)
}

pub(super) fn should_retry_oauth_limits_after_refresh(err: &str) -> bool {
    err.contains("401 Unauthorized") || err.contains("403 Forbidden")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_oauth_authorize_url_keeps_extra_params_without_forcing_prompt_login() {
        let endpoints = crate::gateway::oauth::provider_trait::OAuthEndpoints {
            auth_url: "https://auth.openai.com/oauth/authorize",
            token_url: "https://auth.openai.com/oauth/token",
            client_id: "client_123".to_string(),
            client_secret: None,
            scopes: vec![
                "openid",
                "profile",
                "email",
                "offline_access",
                "api.connectors.read",
                "api.connectors.invoke",
            ],
            redirect_host: "localhost",
            callback_path: "/auth/callback",
            default_callback_port: 1455,
        };

        let authorize_url = build_oauth_authorize_url(
            &endpoints,
            "http://localhost:1455/auth/callback",
            "state_abc",
            "challenge_xyz",
            &[
                ("id_token_add_organizations", "true"),
                ("codex_cli_simplified_flow", "true"),
                ("originator", "codex_cli_rs"),
            ],
        );

        assert!(authorize_url.contains("response_type=code"));
        assert!(
            authorize_url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback")
        );
        assert!(authorize_url.contains(
            "scope=openid%20profile%20email%20offline_access%20api.connectors.read%20api.connectors.invoke"
        ));
        assert!(authorize_url.contains("id_token_add_organizations=true"));
        assert!(authorize_url.contains("codex_cli_simplified_flow=true"));
        assert!(authorize_url.contains("originator=codex_cli_rs"));
        assert!(!authorize_url.contains("prompt=login"));
    }
}
