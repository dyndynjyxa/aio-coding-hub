use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, providers};
use base64::Engine as _;
use serde::Deserialize;

const CODEX_DEVICE_AUTH_USERCODE_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
const CODEX_DEVICE_AUTH_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const CODEX_DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";
const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const CODEX_DEVICE_CODE_DEFAULT_EXPIRES_IN: u64 = 900;
const CODEX_DEVICE_POLLING_SAFETY_MARGIN_SECS: u64 = 3;

/// RFC 8628 device-code grant (used by xAI Grok OAuth).
const OAUTH_DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const GROK_DEVICE_CODE_DEFAULT_EXPIRES_IN: u64 = 900;
const GROK_DEVICE_CODE_DEFAULT_INTERVAL_SECS: u64 = 5;

#[derive(Debug, Clone, Deserialize)]
struct CodexDeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct CodexDevicePollSuccess {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CodexDeviceTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Standard OAuth2 device authorization response (RFC 8628 / xAI Grok).
#[derive(Debug, Clone, Deserialize)]
struct StandardDeviceCodeResponse {
    device_code: String,
    user_code: String,
    #[serde(default)]
    verification_uri: Option<String>,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct StandardDeviceTokenResponse {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct CodexIdTokenClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default, rename = "https://api.openai.com/auth")]
    openai_auth: Option<CodexOpenAiAuthClaim>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct JwtEmailClaims {
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct CodexOpenAiAuthClaim {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthDeviceCodeStartResult {
    pub provider_id: i64,
    pub provider_type: String,
    pub flow_id: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderOAuthDeviceCodePollInput {
    pub provider_id: i64,
    pub flow_id: String,
    pub device_code: String,
    pub user_code: String,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthDeviceCodePollResult {
    pub completed: bool,
    pub provider_id: i64,
    pub provider_type: String,
    pub expires_at: Option<i64>,
}

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
pub(crate) struct ProviderOAuthDeviceCodeCancelResult {
    pub cancelled: bool,
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

fn parse_codex_device_interval(value: Option<&serde_json::Value>) -> u64 {
    let parsed = match value {
        Some(serde_json::Value::Number(number)) => number.as_u64(),
        Some(serde_json::Value::String(text)) => text.trim().parse::<u64>().ok(),
        _ => None,
    };
    parsed.unwrap_or(5) + CODEX_DEVICE_POLLING_SAFETY_MARGIN_SECS
}

fn compute_codex_expires_at(expires_in: Option<i64>) -> Option<i64> {
    let seconds = expires_in?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some(now + seconds)
}

fn decode_codex_id_token_claims(id_token: &str) -> Option<CodexIdTokenClaims> {
    let mut segments = id_token.split('.');
    let _header = segments.next()?;
    let claims = segments.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(claims)
        .ok()?;
    serde_json::from_slice::<CodexIdTokenClaims>(&decoded).ok()
}

pub(super) fn extract_codex_identity(id_token: Option<&str>) -> (Option<String>, Option<String>) {
    let claims = id_token.and_then(decode_codex_id_token_claims);
    let account_id = claims.as_ref().and_then(|value| {
        value.chatgpt_account_id.clone().or_else(|| {
            value
                .openai_auth
                .as_ref()
                .and_then(|auth| auth.chatgpt_account_id.clone())
        })
    });
    let email = claims.and_then(|value| value.email);
    (account_id, email)
}

fn decode_jwt_email_claim(id_token: &str) -> Option<String> {
    let mut segments = id_token.split('.');
    let _header = segments.next()?;
    let claims = segments.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(claims)
        .ok()?;
    let claims = serde_json::from_slice::<JwtEmailClaims>(&decoded).ok()?;
    claims
        .email
        .map(|email| email.trim().to_string())
        .filter(|email| !email.is_empty())
}

/// Best-effort identity extraction across OAuth adapters.
fn extract_oauth_email(cli_key: &str, id_token: Option<&str>) -> Option<String> {
    if cli_key == "codex" {
        return extract_codex_identity(id_token).1;
    }
    id_token.and_then(decode_jwt_email_claim)
}

fn supports_device_code_login(cli_key: &str) -> bool {
    matches!(cli_key, "codex" | "grok")
}

fn compute_expires_at_from_secs(expires_in: Option<i64>) -> Option<i64> {
    let seconds = expires_in?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some(now + seconds)
}

fn ensure_current_oauth_flow(flow_id: &str) -> Result<(), String> {
    if crate::gateway::oauth::is_current_flow(flow_id) {
        Ok(())
    } else {
        Err("OAuth flow cancelled: login attempt is no longer current".to_string())
    }
}

async fn codex_exchange_device_code_for_tokens(
    client: &reqwest::Client,
    client_id: &str,
    authorization_code: &str,
    code_verifier: &str,
) -> Result<CodexDeviceTokenResponse, String> {
    let response = client
        .post("https://auth.openai.com/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
            ("redirect_uri", CODEX_DEVICE_REDIRECT_URI),
            ("client_id", client_id),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| format!("device token exchange request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("device token exchange failed: {status} - {text}"));
    }

    response
        .json::<CodexDeviceTokenResponse>()
        .await
        .map_err(|e| format!("device token exchange parse failed: {e}"))
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
    let flow_lifecycle = crate::gateway::oauth::begin_flow_lifecycle();
    let flow_id = flow_lifecycle.flow_id;
    let mut abort_rx = flow_lifecycle.abort_rx;

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

    ensure_current_oauth_flow(&flow_id)?;

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
    let email = extract_oauth_email(&provider_cli_key, id_token.as_deref());

    // 10. Save to provider
    let app_handle = app.clone();
    blocking::run("provider_oauth_start_flow_save", move || {
        crate::gateway::oauth::complete_current_flow(&flow_id, || {
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
                email.as_deref(),
            )?;
            crate::domain::provider_oauth_limits::clear_snapshot(&db, provider_id)?;
            Ok(())
        })
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
pub(crate) async fn provider_oauth_start_device_flow(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<ProviderOAuthDeviceCodeStartResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let provider_cli_key =
        blocking::run("provider_oauth_start_device_flow_load_provider_cli_key", {
            let db = db.clone();
            move || {
                providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                    crate::shared::error::AppError::from(
                        "DB_NOT_FOUND: provider not found".to_string(),
                    )
                })
            }
        })
        .await
        .map_err(Into::<String>::into)?;

    if !supports_device_code_login(&provider_cli_key) {
        return Err(format!(
            "SEC_INVALID_INPUT: device code login is only supported for codex/grok providers (provider_id={provider_id}, cli_key={provider_cli_key})"
        ));
    }

    let adapter = crate::gateway::oauth::registry::global_registry()
        .get_by_cli_key(&provider_cli_key)
        .ok_or_else(|| format!("no OAuth adapter for cli_key={provider_cli_key}"))?;
    let endpoints = adapter.endpoints();
    let client = crate::gateway::oauth::build_default_oauth_http_client()?;
    let flow_id = crate::gateway::oauth::begin_flow_lifecycle().flow_id;

    if provider_cli_key == "grok" {
        return start_grok_device_flow(provider_id, adapter, endpoints, &client, flow_id).await;
    }

    // Codex proprietary device-auth API
    let response = client
        .post(CODEX_DEVICE_AUTH_USERCODE_URL)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "client_id": endpoints.client_id }))
        .send()
        .await
        .map_err(|e| format!("device code request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("device code request failed: {status} - {text}"));
    }

    let payload = response
        .json::<CodexDeviceCodeResponse>()
        .await
        .map_err(|e| format!("device code response parse failed: {e}"))?;

    let expires_in = payload
        .expires_in
        .unwrap_or(CODEX_DEVICE_CODE_DEFAULT_EXPIRES_IN);
    let interval = parse_codex_device_interval(payload.interval.as_ref());

    Ok(ProviderOAuthDeviceCodeStartResult {
        provider_id,
        provider_type: adapter.provider_type().to_string(),
        flow_id,
        device_code: payload.device_auth_id,
        user_code: payload.user_code,
        verification_uri: CODEX_DEVICE_VERIFICATION_URL.to_string(),
        expires_in,
        interval,
    })
}

async fn start_grok_device_flow(
    provider_id: i64,
    adapter: &dyn crate::gateway::oauth::provider_trait::OAuthProvider,
    endpoints: &crate::gateway::oauth::provider_trait::OAuthEndpoints,
    client: &reqwest::Client,
    flow_id: String,
) -> Result<ProviderOAuthDeviceCodeStartResult, String> {
    use crate::gateway::oauth::adapters::grok::{
        grok_client_version, GROK_CLIENT_SURFACE_UI, GROK_DEVICE_AUTHORIZATION_URL,
        GROK_OAUTH_REFERRER,
    };

    let scope = endpoints.scopes.join(" ");
    let client_version = grok_client_version();
    // Match grok-build device-code request: referrer + x-grok-client-* headers.
    let response = client
        .post(GROK_DEVICE_AUTHORIZATION_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .header("x-grok-client-version", client_version.as_str())
        .header("x-grok-client-surface", GROK_CLIENT_SURFACE_UI)
        .form(&[
            ("client_id", endpoints.client_id.as_str()),
            ("scope", scope.as_str()),
            ("referrer", GROK_OAUTH_REFERRER),
        ])
        .send()
        .await
        .map_err(|e| format!("grok device code request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "grok device code request failed: {status} - {text}"
        ));
    }

    let payload = response
        .json::<StandardDeviceCodeResponse>()
        .await
        .map_err(|e| format!("grok device code response parse failed: {e}"))?;

    let verification_uri = payload
        .verification_uri
        .as_deref()
        .or(payload.verification_uri_complete.as_deref())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "grok device code response missing verification_uri".to_string())?
        .to_string();

    let expires_in = payload
        .expires_in
        .unwrap_or(GROK_DEVICE_CODE_DEFAULT_EXPIRES_IN);
    let interval = payload
        .interval
        .unwrap_or(GROK_DEVICE_CODE_DEFAULT_INTERVAL_SECS)
        .max(1)
        + CODEX_DEVICE_POLLING_SAFETY_MARGIN_SECS;

    Ok(ProviderOAuthDeviceCodeStartResult {
        provider_id,
        provider_type: adapter.provider_type().to_string(),
        flow_id,
        device_code: payload.device_code,
        user_code: payload.user_code,
        verification_uri,
        expires_in,
        interval,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_poll_device_flow(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: ProviderOAuthDeviceCodePollInput,
) -> Result<ProviderOAuthDeviceCodePollResult, String> {
    ensure_current_oauth_flow(&input.flow_id)?;
    let provider_id = input.provider_id;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let provider_cli_key =
        blocking::run("provider_oauth_poll_device_flow_load_provider_cli_key", {
            let db = db.clone();
            move || {
                providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                    crate::shared::error::AppError::from(
                        "DB_NOT_FOUND: provider not found".to_string(),
                    )
                })
            }
        })
        .await
        .map_err(Into::<String>::into)?;

    if !supports_device_code_login(&provider_cli_key) {
        return Err(format!(
            "SEC_INVALID_INPUT: device code login is only supported for codex/grok providers (provider_id={provider_id}, cli_key={provider_cli_key})"
        ));
    }

    let adapter = crate::gateway::oauth::registry::global_registry()
        .get_by_cli_key(&provider_cli_key)
        .ok_or_else(|| format!("no OAuth adapter for cli_key={provider_cli_key}"))?;
    let endpoints = adapter.endpoints();
    let client = crate::gateway::oauth::build_default_oauth_http_client()?;

    let oauth_token_set = if provider_cli_key == "grok" {
        match poll_grok_device_token(
            &client,
            endpoints.token_url,
            &endpoints.client_id,
            &input.device_code,
            &input.flow_id,
        )
        .await?
        {
            None => {
                return Ok(ProviderOAuthDeviceCodePollResult {
                    completed: false,
                    provider_id,
                    provider_type: adapter.provider_type().to_string(),
                    expires_at: None,
                });
            }
            Some(token_set) => token_set,
        }
    } else {
        let poll_response = client
            .post(CODEX_DEVICE_AUTH_TOKEN_URL)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "device_auth_id": input.device_code,
                "user_code": input.user_code,
            }))
            .send()
            .await
            .map_err(|e| format!("device code poll failed: {e}"))?;

        ensure_current_oauth_flow(&input.flow_id)?;

        let status = poll_response.status();
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
            return Ok(ProviderOAuthDeviceCodePollResult {
                completed: false,
                provider_id,
                provider_type: adapter.provider_type().to_string(),
                expires_at: None,
            });
        }
        if status == reqwest::StatusCode::GONE {
            crate::gateway::oauth::cancel_flow(&input.flow_id);
            return Err("Device code 已过期，请重新开始登录。".to_string());
        }
        if !status.is_success() {
            let text = poll_response.text().await.unwrap_or_default();
            crate::gateway::oauth::cancel_flow(&input.flow_id);
            return Err(format!("device code poll failed: {status} - {text}"));
        }

        let success = poll_response
            .json::<CodexDevicePollSuccess>()
            .await
            .map_err(|e| format!("device code poll parse failed: {e}"))?;

        ensure_current_oauth_flow(&input.flow_id)?;

        let token_set = codex_exchange_device_code_for_tokens(
            &client,
            &endpoints.client_id,
            &success.authorization_code,
            &success.code_verifier,
        )
        .await?;

        crate::gateway::oauth::provider_trait::OAuthTokenSet {
            access_token: token_set.access_token,
            refresh_token: token_set.refresh_token,
            expires_at: compute_codex_expires_at(token_set.expires_in),
            id_token: token_set.id_token,
        }
    };

    let (effective_token, id_token) =
        adapter.resolve_effective_token(&oauth_token_set, oauth_token_set.id_token.as_deref());
    let token_expires_at = oauth_token_set.expires_at;
    let provider_type = adapter.provider_type();
    let email = extract_oauth_email(&provider_cli_key, id_token.as_deref());

    blocking::run("provider_oauth_poll_device_flow_save", move || {
        crate::gateway::oauth::complete_current_flow(&input.flow_id, || {
            crate::providers::update_oauth_tokens(
                &db,
                provider_id,
                "oauth",
                provider_type,
                &effective_token,
                oauth_token_set.refresh_token.as_deref(),
                id_token.as_deref(),
                endpoints.token_url,
                &endpoints.client_id,
                endpoints.client_secret.as_deref(),
                token_expires_at,
                email.as_deref(),
            )?;
            crate::domain::provider_oauth_limits::clear_snapshot(&db, provider_id)?;
            Ok(())
        })
    })
    .await
    .map_err(Into::<String>::into)?;

    crate::gateway::events::emit_gateway_log(
        &app,
        "info",
        "OAUTH_DEVICE_LOGIN_OK",
        format!("OAuth 设备码登录成功：provider_id={provider_id} type={provider_type}"),
    );

    Ok(ProviderOAuthDeviceCodePollResult {
        completed: true,
        provider_id,
        provider_type: provider_type.to_string(),
        expires_at: token_expires_at,
    })
}

/// Poll xAI token endpoint for RFC 8628 device authorization.
/// Returns `Ok(None)` while the user has not yet approved (`authorization_pending` / `slow_down`).
async fn poll_grok_device_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    device_code: &str,
    flow_id: &str,
) -> Result<Option<crate::gateway::oauth::provider_trait::OAuthTokenSet>, String> {
    use crate::gateway::oauth::adapters::grok::{grok_client_version, GROK_CLIENT_SURFACE_UI};

    let client_version = grok_client_version();
    // Match grok-build device-token poll identity headers.
    let response = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .header("x-grok-client-version", client_version.as_str())
        .header("x-grok-client-surface", GROK_CLIENT_SURFACE_UI)
        .form(&[
            ("grant_type", OAUTH_DEVICE_CODE_GRANT_TYPE),
            ("device_code", device_code),
            ("client_id", client_id),
        ])
        .send()
        .await
        .map_err(|e| format!("grok device token poll failed: {e}"))?;

    ensure_current_oauth_flow(flow_id)?;

    let status = response.status();
    let payload = response
        .json::<StandardDeviceTokenResponse>()
        .await
        .map_err(|e| format!("grok device token poll parse failed: {e}"))?;

    if let Some(error) = payload
        .error
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        match error {
            "authorization_pending" | "slow_down" => return Ok(None),
            "expired_token" => {
                crate::gateway::oauth::cancel_flow(flow_id);
                return Err("Device code 已过期，请重新开始登录。".to_string());
            }
            "access_denied" => {
                crate::gateway::oauth::cancel_flow(flow_id);
                return Err("设备码授权被拒绝。".to_string());
            }
            other => {
                crate::gateway::oauth::cancel_flow(flow_id);
                let desc = payload
                    .error_description
                    .as_deref()
                    .map(str::trim)
                    .filter(|d| !d.is_empty())
                    .unwrap_or(other);
                return Err(format!("grok device token error: {desc}"));
            }
        }
    }

    if !status.is_success() {
        crate::gateway::oauth::cancel_flow(flow_id);
        return Err(format!(
            "grok device token poll failed: {status} (no access_token)"
        ));
    }

    let access_token = payload
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            crate::gateway::oauth::cancel_flow(flow_id);
            "grok device token response missing access_token".to_string()
        })?
        .to_string();

    Ok(Some(crate::gateway::oauth::provider_trait::OAuthTokenSet {
        access_token,
        refresh_token: payload.refresh_token,
        expires_at: compute_expires_at_from_secs(payload.expires_in),
        id_token: payload.id_token,
    }))
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_cancel_device_flow(
    flow_id: String,
) -> Result<ProviderOAuthDeviceCodeCancelResult, String> {
    if flow_id.trim().is_empty() {
        return Ok(ProviderOAuthDeviceCodeCancelResult { cancelled: false });
    }

    Ok(ProviderOAuthDeviceCodeCancelResult {
        cancelled: crate::gateway::oauth::cancel_flow(flow_id.trim()),
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
        crate::providers::clear_oauth(&db, provider_id)?;
        crate::domain::provider_oauth_limits::clear_snapshot(&db, provider_id)?;
        Ok::<(), crate::shared::error::AppError>(())
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
