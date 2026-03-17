//! Usage: Provider configuration related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::shared::mutex_ext::MutexExt;
use crate::{base_url_probe, blocking, providers};
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tauri::Manager;

const ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";
const ENV_DISABLE_ERROR_REPORTING: &str = "DISABLE_ERROR_REPORTING";
const ENV_DISABLE_TELEMETRY: &str = "DISABLE_TELEMETRY";
const ENV_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
const ENV_ANTHROPIC_BASE_URL: &str = "ANTHROPIC_BASE_URL";
const ENV_ANTHROPIC_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderUpsertInput {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: providers::ProviderBaseUrlMode,
    pub auth_mode: Option<providers::ProviderAuthMode>,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cost_multiplier: f64,
    pub priority: Option<i64>,
    pub claude_models: Option<providers::ClaudeModels>,
    #[serde(rename = "limit5hUsd", alias = "limit5HUsd")]
    #[specta(rename = "limit5hUsd")]
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<providers::DailyResetMode>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProviderRuntimeResetDecision {
    clear_session_bindings: bool,
}

fn submitted_api_key_changed(
    previous_api_key: Option<&str>,
    submitted_api_key: Option<&str>,
) -> bool {
    let Some(submitted) = submitted_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
    else {
        return false;
    };

    previous_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        != Some(submitted)
}

fn provider_runtime_reset_decision(
    previous: Option<&providers::ProviderSummary>,
    previous_api_key: Option<&str>,
    next: &providers::ProviderSummary,
    submitted_api_key: Option<&str>,
) -> ProviderRuntimeResetDecision {
    let Some(previous) = previous else {
        return ProviderRuntimeResetDecision::default();
    };

    let sensitive_config_changed = previous.base_urls != next.base_urls
        || previous.base_url_mode != next.base_url_mode
        || previous.auth_mode != next.auth_mode
        || submitted_api_key_changed(previous_api_key, submitted_api_key)
        || previous.source_provider_id != next.source_provider_id;

    ProviderRuntimeResetDecision {
        clear_session_bindings: sensitive_config_changed,
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("providers_list", move || {
        providers::list_by_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    gateway_state: tauri::State<'_, GatewayState>,
    input: ProviderUpsertInput,
) -> Result<providers::ProviderSummary, String> {
    let ProviderUpsertInput {
        provider_id,
        cli_key,
        name,
        base_urls,
        base_url_mode,
        auth_mode,
        api_key,
        enabled,
        cost_multiplier,
        priority,
        claude_models,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        tags,
        note,
        source_provider_id,
        bridge_type,
    } = input;

    let is_create = provider_id.is_none();
    let name_for_log = name.clone();
    let cli_key_for_log = cli_key.clone();
    let submitted_api_key = api_key.clone();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_upsert", move || {
        let previous = match provider_id {
            Some(id) => {
                let conn = db.open_connection()?;
                Some(providers::get_by_id(&conn, id)?)
            }
            None => None,
        };
        let previous_api_key = match provider_id {
            Some(id) => Some(providers::get_api_key_plaintext(&db, id)?),
            None => None,
        };

        let saved = providers::upsert(
            &db,
            providers::ProviderUpsertParams {
                provider_id,
                cli_key,
                name,
                base_urls,
                base_url_mode,
                auth_mode,
                api_key,
                enabled,
                cost_multiplier,
                priority,
                claude_models,
                limit_5h_usd,
                limit_daily_usd,
                daily_reset_mode,
                daily_reset_time,
                limit_weekly_usd,
                limit_monthly_usd,
                limit_total_usd,
                tags,
                note,
                source_provider_id,
                bridge_type,
            },
        )?;

        let decision = provider_runtime_reset_decision(
            previous.as_ref(),
            previous_api_key.as_deref(),
            &saved,
            submitted_api_key.as_deref(),
        );

        Ok::<_, crate::shared::error::AppError>((saved, decision))
    })
    .await
    .map_err(Into::into);

    if let Ok((ref provider, decision)) = result {
        if is_create {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider created"
            );
        } else {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider updated"
            );
        }

        if decision.clear_session_bindings {
            let cleared_sessions = {
                let manager = gateway_state.0.lock_or_recover();
                manager.clear_cli_session_bindings(&provider.cli_key)
            };
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions,
                "provider runtime bindings cleared after sensitive update"
            );
        }
    }

    result.map(|(provider, _)| provider)
}

#[tauri::command]
pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_set_enabled", move || {
        providers::set_enabled(&db, provider_id, enabled)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref provider) = result {
        tracing::info!(
            provider_id = provider.id,
            enabled = provider.enabled,
            "provider enabled state changed"
        );
    }

    result
}

#[tauri::command]
pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run(
        "provider_delete",
        move || -> crate::shared::error::AppResult<bool> {
            providers::delete(&db, provider_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into);

    if let Ok(true) = result {
        tracing::info!(provider_id = provider_id, "provider deleted");
    }

    result
}

#[tauri::command]
pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    gateway_state: tauri::State<'_, GatewayState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("providers_reorder", move || {
        providers::reorder(&db, &cli_key, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref providers) = result {
        // Provider order changes must invalidate session-bound provider_order (default TTL=300s).
        let cleared = {
            let manager = gateway_state.0.lock_or_recover();
            manager.clear_cli_session_bindings(&cli_key_for_log)
        };
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = providers.len(),
            cleared_sessions = cleared,
            "providers reordered"
        );
    }

    result
}

#[tauri::command]
pub(crate) async fn provider_claude_terminal_launch_command(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<String, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let gateway_base_origin = blocking::run("provider_claude_terminal_launch_gateway_origin", {
        let app = app.clone();
        let db = db.clone();
        move || ensure_gateway_base_origin(&app, &db)
    })
    .await?;

    blocking::run("provider_claude_terminal_launch_command", move || {
        let launch = providers::claude_terminal_launch_context(&db, provider_id)?;
        let claude_base_url = build_claude_gateway_base_url(&gateway_base_origin, provider_id);
        create_claude_terminal_launch_command(
            provider_id,
            &claude_base_url,
            &launch.api_key_plaintext,
        )
    })
    .await
    .map_err(Into::into)
}

fn ensure_gateway_base_origin(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
) -> crate::shared::error::AppResult<String> {
    let state = app.state::<GatewayState>();
    let mut manager = state.0.lock_or_recover();

    let mut status = manager.status();
    if !status.running {
        status = manager.start(app, db.clone(), None)?;
    }

    drop(manager);

    let _ = app.emit("gateway:status", status.clone());

    status
        .base_url
        .ok_or_else(|| "SYSTEM_ERROR: gateway base_url missing".to_string().into())
}

fn build_claude_gateway_base_url(gateway_base_origin: &str, provider_id: i64) -> String {
    format!(
        "{}/claude/_aio/provider/{provider_id}",
        gateway_base_origin.trim_end_matches('/')
    )
}

fn create_claude_terminal_launch_command(
    provider_id: i64,
    base_url: &str,
    api_key_plaintext: &str,
) -> crate::shared::error::AppResult<String> {
    let temp_dir = std::env::temp_dir();
    let now = crate::shared::time::now_unix_seconds();
    let pid = std::process::id();

    let config_path = temp_dir.join(format!("claude_{provider_id}_{pid}_{now}.json"));

    let settings_json = build_claude_settings_json(base_url, api_key_plaintext)?;
    std::fs::write(&config_path, settings_json)
        .map_err(|e| format!("SYSTEM_ERROR: write claude settings failed: {e}"))?;

    let (script_path, script_content, launch_command) =
        build_claude_launch_assets(provider_id, pid, now, &temp_dir, &config_path);
    if let Err(err) = std::fs::write(&script_path, script_content) {
        let _ = std::fs::remove_file(&config_path);
        return Err(format!("SYSTEM_ERROR: write launch script failed: {err}").into());
    }

    Ok(launch_command)
}

fn build_claude_launch_assets(
    provider_id: i64,
    pid: u32,
    now: i64,
    temp_dir: &Path,
    config_path: &Path,
) -> (PathBuf, String, String) {
    if cfg!(target_os = "windows") {
        let script_path =
            temp_dir.join(format!("aio_claude_launcher_{provider_id}_{pid}_{now}.ps1"));
        let script_content = build_claude_launcher_powershell_script(config_path, &script_path);
        let launch_command = build_powershell_launch_command(&script_path);
        (script_path, script_content, launch_command)
    } else {
        let script_path =
            temp_dir.join(format!("aio_claude_launcher_{provider_id}_{pid}_{now}.sh"));
        let script_content = build_claude_launcher_bash_script(config_path, &script_path);
        let launch_command = build_bash_launch_command(&script_path);
        (script_path, script_content, launch_command)
    }
}

fn build_claude_settings_json(
    base_url: &str,
    api_key_plaintext: &str,
) -> crate::shared::error::AppResult<String> {
    let value = json!({
        "env": {
            ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
            ENV_DISABLE_ERROR_REPORTING: "1",
            ENV_DISABLE_TELEMETRY: "1",
            ENV_MCP_TIMEOUT: "60000",
            ENV_ANTHROPIC_BASE_URL: base_url,
            ENV_ANTHROPIC_AUTH_TOKEN: api_key_plaintext,
        }
    });

    serde_json::to_string_pretty(&value)
        .map_err(|e| format!("SYSTEM_ERROR: serialize claude settings failed: {e}").into())
}

fn build_claude_launcher_bash_script(config_path: &Path, script_path: &Path) -> String {
    let config_var = bash_single_quote(&config_path.to_string_lossy());
    let script_var = bash_single_quote(&script_path.to_string_lossy());

    format!(
        "#!/bin/bash\n\
config_path={config_var}\n\
script_path={script_var}\n\
trap 'rm -f \"$config_path\" \"$script_path\"' EXIT\n\
echo \"Using provider-specific claude config:\"\n\
echo \"$config_path\"\n\
claude --settings \"$config_path\"\n\
exec bash --norc --noprofile\n"
    )
}

fn build_claude_launcher_powershell_script(config_path: &Path, script_path: &Path) -> String {
    let config_var = powershell_single_quote(&config_path.to_string_lossy());
    let script_var = powershell_single_quote(&script_path.to_string_lossy());

    format!(
        "$configPath = {config_var}\n\
$scriptPath = {script_var}\n\
try {{\n\
  Write-Output \"Using provider-specific claude config:\"\n\
  Write-Output $configPath\n\
  claude --settings $configPath\n\
}} finally {{\n\
  Remove-Item -LiteralPath $configPath -ErrorAction SilentlyContinue\n\
  Remove-Item -LiteralPath $scriptPath -ErrorAction SilentlyContinue\n\
}}\n"
    )
}

fn build_bash_launch_command(script_path: &Path) -> String {
    format!("bash {}", bash_single_quote(&script_path.to_string_lossy()))
}

fn build_powershell_launch_command(script_path: &Path) -> String {
    format!(
        "powershell -NoLogo -NoExit -ExecutionPolicy Bypass -File {}",
        windows_double_quote(&script_path.to_string_lossy())
    )
}

fn bash_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn powershell_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

fn windows_double_quote(value: &str) -> String {
    format!("\"{value}\"")
}

#[tauri::command]
pub(crate) async fn provider_get_api_key(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<String, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("provider_get_api_key", move || {
        providers::get_api_key_plaintext(&db, provider_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn base_url_ping_ms(base_url: String) -> Result<u64, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("aio-coding-hub-ping/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("PING_HTTP_CLIENT_INIT: {e}"))?;
    base_url_probe::probe_base_url_ms(&client, &base_url, std::time::Duration::from_secs(3)).await
}

#[tauri::command]
pub(crate) async fn provider_oauth_start_flow(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
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
    let scopes = endpoints.scopes.join(" ");
    let mut authorize_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        endpoints.auth_url,
        crate::gateway::util::encode_url_component(&endpoints.client_id),
        crate::gateway::util::encode_url_component(&redirect_uri),
        crate::gateway::util::encode_url_component(&scopes),
        crate::gateway::util::encode_url_component(&oauth_state),
        crate::gateway::util::encode_url_component(&pkce.code_challenge),
    );

    for (key, value) in adapter.extra_authorize_params() {
        authorize_url.push('&');
        authorize_url.push_str(&crate::gateway::util::encode_url_component(key));
        authorize_url.push('=');
        authorize_url.push_str(&crate::gateway::util::encode_url_component(value));
    }

    // Force a fresh login session to avoid stale PKCE/state conflicts on retry,
    // but only if the adapter hasn't already set a `prompt` parameter (e.g. Gemini
    // uses `prompt=consent` to guarantee refresh token issuance from Google).
    let adapter_sets_prompt = adapter
        .extra_authorize_params()
        .iter()
        .any(|(k, _)| *k == "prompt");
    if !adapter_sets_prompt {
        authorize_url.push_str("&prompt=login");
    }

    // 6. Open browser
    let _ = tauri_plugin_opener::open_url(&authorize_url, None::<&str>);

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

    Ok(serde_json::json!({
        "success": true,
        "provider_id": provider_id,
        "provider_type": provider_type,
        "expires_at": token_expires_at,
    }))
}

#[tauri::command]
pub(crate) async fn provider_oauth_refresh(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
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

    Ok(serde_json::json!({
        "success": true,
        "expires_at": token_set.expires_at,
    }))
}

#[tauri::command]
pub(crate) async fn provider_oauth_disconnect(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("provider_oauth_disconnect", move || {
        crate::providers::clear_oauth(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub(crate) async fn provider_oauth_status(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_oauth_status", move || {
        crate::providers::get_oauth_details(&db, provider_id)
    })
    .await;

    match result {
        Ok(details) => Ok(serde_json::json!({
            "connected": true,
            "provider_type": details.oauth_provider_type,
            "email": details.oauth_email,
            "expires_at": details.oauth_expires_at,
            "has_refresh_token": details.oauth_refresh_token.is_some(),
        })),
        Err(e) => {
            let err_str = e.to_string();
            // DB_NOT_FOUND = provider exists but has no OAuth tokens → expected disconnected state.
            // Any other error (DB_ERROR, INTERNAL_ERROR) is a real failure that must surface.
            if err_str.starts_with("DB_NOT_FOUND") {
                Ok(serde_json::json!({ "connected": false }))
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

#[tauri::command]
pub(crate) async fn provider_oauth_fetch_limits(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let details = blocking::run("provider_oauth_fetch_limits_load", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;

    let token = details.oauth_access_token.trim().to_string();
    if token.is_empty() {
        return Err("OAuth access token is empty".to_string());
    }

    let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(&details)?;

    let client = crate::gateway::oauth::build_oauth_http_client(
        &format!("aio-coding-hub-oauth-command/{}", env!("CARGO_PKG_VERSION")),
        15,
        10,
    )?;
    let limits = adapter
        .fetch_limits(&client, &token)
        .await
        .map_err(|e| format!("fetch_limits failed: {e}"))?;

    let limit_short_label =
        normalize_oauth_short_window_label(adapter.cli_key(), limits.limit_short_label.as_deref());

    // If the adapter already parsed limit texts, use them directly.
    // Otherwise, try to parse from raw_json based on cli_key.
    let (limit_5h_text, limit_weekly_text) =
        if limits.limit_5h_text.is_some() || limits.limit_weekly_text.is_some() {
            (
                limits.limit_5h_text.clone(),
                limits.limit_weekly_text.clone(),
            )
        } else if let Some(ref raw) = limits.raw_json {
            let cli_key = adapter.cli_key();
            match cli_key {
                "codex" => parse_codex_limits(raw),
                "claude" => parse_claude_limits(raw),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

    Ok(serde_json::json!({
        "limit_short_label": limit_short_label,
        "limit_5h_text": limit_5h_text,
        "limit_weekly_text": limit_weekly_text,
        "raw_json": limits.raw_json,
    }))
}

fn default_oauth_short_window_label(cli_key: &str) -> Option<String> {
    match cli_key {
        "codex" | "claude" => Some("5h".to_string()),
        "gemini" => Some("短窗".to_string()),
        _ => None,
    }
}

fn normalize_oauth_short_window_label(
    cli_key: &str,
    adapter_label: Option<&str>,
) -> Option<String> {
    let adapter_label = adapter_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    match cli_key {
        "gemini" => Some("短窗".to_string()),
        _ => adapter_label.or_else(|| default_oauth_short_window_label(cli_key)),
    }
}

fn parse_remaining_percent_from_window(window: &serde_json::Value) -> Option<f64> {
    if !window.is_object() {
        return None;
    }
    if let Some(used) = window
        .get("used_percent")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            window
                .get("usedPercent")
                .and_then(serde_json::Value::as_f64)
        })
    {
        let remaining = (100.0 - used).clamp(0.0, 100.0);
        return Some(remaining);
    }
    let remaining = window
        .get("remaining_count")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            window
                .get("remainingCount")
                .and_then(serde_json::Value::as_f64)
        });
    let total = window
        .get("total_count")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| window.get("totalCount").and_then(serde_json::Value::as_f64));
    match (remaining, total) {
        (Some(rem), Some(t)) if t > 0.0 => Some((rem / t * 100.0).clamp(0.0, 100.0)),
        _ => None,
    }
}

fn format_percent_label(value: f64) -> String {
    format!("{:.0}%", value.clamp(0.0, 100.0))
}

fn parse_codex_limits(body: &serde_json::Value) -> (Option<String>, Option<String>) {
    let rate_limit = body.get("rate_limit").unwrap_or(body);
    let primary = rate_limit
        .get("primary_window")
        .or_else(|| rate_limit.get("primaryWindow"))
        .or_else(|| body.get("5_hour_window"))
        .or_else(|| body.get("fiveHourWindow"));
    let secondary = rate_limit
        .get("secondary_window")
        .or_else(|| rate_limit.get("secondaryWindow"))
        .or_else(|| body.get("weekly_window"))
        .or_else(|| body.get("weeklyWindow"));

    let limit_5h = primary
        .and_then(parse_remaining_percent_from_window)
        .map(format_percent_label);
    let limit_weekly = secondary
        .and_then(parse_remaining_percent_from_window)
        .map(format_percent_label);
    (limit_5h, limit_weekly)
}

fn parse_claude_limits(body: &serde_json::Value) -> (Option<String>, Option<String>) {
    fn extract_utilization(window: &serde_json::Value) -> Option<f64> {
        window
            .get("utilization")
            .and_then(serde_json::Value::as_f64)
            .or_else(|| {
                window
                    .get("utilization")
                    .and_then(serde_json::Value::as_str)?
                    .parse::<f64>()
                    .ok()
            })
    }

    let limit_5h = body
        .get("five_hour")
        .and_then(extract_utilization)
        .map(|used| format_percent_label(100.0 - used));
    let limit_weekly = body
        .get("seven_day")
        .and_then(extract_utilization)
        .map(|used| format_percent_label(100.0 - used));
    (limit_5h, limit_weekly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_single_quote_escapes_single_quote() {
        assert_eq!(bash_single_quote("a'b"), "'a'\"'\"'b'");
    }

    #[test]
    fn powershell_single_quote_escapes_single_quote() {
        assert_eq!(powershell_single_quote("a'b"), "'a''b'");
    }

    #[test]
    fn build_settings_contains_required_envs() {
        let json_text = build_claude_settings_json("https://example.com", "sk-test").unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_text).unwrap();
        let env = value
            .get("env")
            .and_then(|v| v.as_object())
            .expect("env object");

        assert_eq!(
            env.get(ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC)
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_DISABLE_ERROR_REPORTING)
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_DISABLE_TELEMETRY).and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_MCP_TIMEOUT).and_then(|v| v.as_str()),
            Some("60000")
        );
        assert_eq!(
            env.get(ENV_ANTHROPIC_BASE_URL).and_then(|v| v.as_str()),
            Some("https://example.com")
        );
        assert_eq!(
            env.get(ENV_ANTHROPIC_AUTH_TOKEN).and_then(|v| v.as_str()),
            Some("sk-test")
        );
    }

    #[test]
    fn build_claude_gateway_base_url_trims_trailing_slash() {
        let url = build_claude_gateway_base_url("http://127.0.0.1:18080/", 12);
        assert_eq!(url, "http://127.0.0.1:18080/claude/_aio/provider/12");
    }

    #[test]
    fn bash_launch_script_includes_cleanup_and_claude_settings() {
        let config_path = Path::new("/tmp/claude_x.json");
        let script_path = Path::new("/tmp/aio_launcher.sh");
        let script = build_claude_launcher_bash_script(config_path, script_path);

        assert!(script.contains("trap 'rm -f \"$config_path\" \"$script_path\"' EXIT"));
        assert!(script.contains("claude --settings \"$config_path\""));
        assert!(script.contains("exec bash --norc --noprofile"));
    }

    #[test]
    fn powershell_launch_script_includes_cleanup_and_claude_settings() {
        let config_path = Path::new(r"C:\\Temp\\claude_x.json");
        let script_path = Path::new(r"C:\\Temp\\aio_launcher.ps1");
        let script = build_claude_launcher_powershell_script(config_path, script_path);

        assert!(script.contains("Write-Output \"Using provider-specific claude config:\""));
        assert!(script.contains("claude --settings $configPath"));
        assert!(
            script.contains("Remove-Item -LiteralPath $configPath -ErrorAction SilentlyContinue")
        );
        assert!(
            script.contains("Remove-Item -LiteralPath $scriptPath -ErrorAction SilentlyContinue")
        );
    }

    #[test]
    fn powershell_launch_command_uses_expected_flags() {
        let script_path = Path::new(r"C:\\Temp\\aio_launcher.ps1");
        let command = build_powershell_launch_command(script_path);

        assert!(command.starts_with("powershell -NoLogo -NoExit -ExecutionPolicy Bypass -File"));
        assert!(command.contains("\"C:\\\\Temp\\\\aio_launcher.ps1\""));
    }
    #[test]
    fn provider_upsert_input_deserializes_runtime_camel_case_shape() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "order",
            "authMode": "api_key",
            "apiKey": "k1",
            "enabled": true,
            "costMultiplier": 1.0,
            "priority": 10,
            "claudeModels": null,
            "limit5hUsd": 5.0,
            "limitDailyUsd": 10.0,
            "dailyResetMode": "fixed",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null,
            "tags": ["x"],
            "note": "n"
        }))
        .expect("deserialize provider input");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Order);
        assert_eq!(input.auth_mode, Some(providers::ProviderAuthMode::ApiKey));
        assert_eq!(input.limit_5h_usd, Some(5.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Fixed)
        );
    }

    #[test]
    fn provider_upsert_input_accepts_legacy_generated_limit_alias() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "ping",
            "enabled": true,
            "costMultiplier": 1.0,
            "limit5HUsd": 7.0,
            "limitDailyUsd": null,
            "dailyResetMode": "rolling",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null
        }))
        .expect("deserialize provider input legacy alias");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Ping);
        assert_eq!(input.limit_5h_usd, Some(7.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Rolling)
        );
    }

    #[test]
    fn normalize_oauth_short_window_label_forces_gemini_to_short_window() {
        assert_eq!(
            normalize_oauth_short_window_label("gemini", Some("1h")).as_deref(),
            Some("短窗")
        );
        assert_eq!(
            normalize_oauth_short_window_label("gemini", None).as_deref(),
            Some("短窗")
        );
        assert_eq!(
            normalize_oauth_short_window_label("codex", Some("custom")).as_deref(),
            Some("custom")
        );
    }

    #[test]
    fn provider_runtime_reset_decision_ignores_create_and_non_sensitive_edits() {
        let next = providers::ProviderSummary {
            id: 1,
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
        };

        assert_eq!(
            provider_runtime_reset_decision(None, None, &next, None),
            ProviderRuntimeResetDecision::default()
        );

        let mut previous = next.clone();
        previous.name = "Old Name".to_string();
        previous.note = "old".to_string();
        previous.updated_at = 0;

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("   ")
            ),
            ProviderRuntimeResetDecision::default()
        );

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("sk-existing")
            ),
            ProviderRuntimeResetDecision::default()
        );
    }

    #[test]
    fn provider_runtime_reset_decision_detects_sensitive_claude_changes() {
        let previous = providers::ProviderSummary {
            id: 1,
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.old.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
        };

        let mut next = previous.clone();
        next.base_urls = vec!["https://api.new.example.com".to_string()];

        assert_eq!(
            provider_runtime_reset_decision(Some(&previous), Some("sk-old"), &next, None),
            ProviderRuntimeResetDecision {
                clear_session_bindings: true,
            }
        );

        let mut next_non_claude = previous.clone();
        next_non_claude.cli_key = "codex".to_string();

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&next_non_claude),
                Some("sk-old"),
                &next_non_claude,
                Some("sk-new")
            ),
            ProviderRuntimeResetDecision {
                clear_session_bindings: true,
            }
        );
    }
}
