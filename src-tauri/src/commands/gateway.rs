//! Usage: Gateway lifecycle / status / session / circuit commands.

use crate::app::gateway_service::{self, GatewayActiveSessionSummary};
use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_runtime_access::app_gateway_status;
use crate::shared::cli_key::CliKey;
use crate::{gateway, settings};

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayUpstreamProxyInput {
    proxy_url: String,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
}

fn resolve_gateway_upstream_proxy_context(
    app: &tauri::AppHandle,
    input: &GatewayUpstreamProxyInput,
) -> Result<
    (
        Option<String>,
        crate::gateway::http_client::GatewaySelfCheckContext,
    ),
    String,
> {
    let cfg = settings::read(app).map_err(|err| err.to_string())?;
    let proxy_url = gateway::http_client::build_effective_proxy_url(
        Some(input.proxy_url.as_str()),
        input.proxy_username.as_deref(),
        input.proxy_password.as_deref(),
    )?;
    let context = gateway::http_client::self_check_context_from_settings(&cfg)
        .map_err(|err| err.to_string())?;
    Ok((proxy_url, context))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn gateway_status(app: tauri::AppHandle) -> gateway::GatewayStatus {
    app_gateway_status(&app)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_check_port_available(
    app: tauri::AppHandle,
    port: u16,
) -> Result<bool, String> {
    gateway_service::check_port_available(app, port)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_sessions_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    limit: Option<u32>,
) -> Result<Vec<GatewayActiveSessionSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    gateway_service::list_active_sessions(app, db, limit)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_circuit_status(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<gateway::GatewayProviderCircuitStatus>, String> {
    let cli_key = normalize_gateway_cli_key(&cli_key)?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    gateway_service::circuit_status(app, db, cli_key)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_circuit_reset_provider(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    gateway_service::circuit_reset_provider(app, db, provider_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_circuit_reset_cli(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<usize, String> {
    let cli_key = normalize_gateway_cli_key(&cli_key)?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    gateway_service::circuit_reset_cli(app, db, cli_key)
        .await
        .map_err(Into::into)
}

fn normalize_gateway_cli_key(cli_key: &str) -> Result<String, String> {
    Ok(CliKey::parse(cli_key.trim())
        .map_err(String::from)?
        .to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_start(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    preferred_port: Option<u16>,
) -> Result<gateway::GatewayStatus, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    gateway_service::start_and_sync(app, db, preferred_port)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_stop(app: tauri::AppHandle) -> Result<gateway::GatewayStatus, String> {
    gateway_service::stop_and_restore(app)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn gateway_upstream_proxy_validate(
    app: tauri::AppHandle,
    input: GatewayUpstreamProxyInput,
) -> Result<(), String> {
    let (proxy_url, context) = resolve_gateway_upstream_proxy_context(&app, &input)?;
    gateway::http_client::validate_proxy_with_context(proxy_url.as_deref(), &context)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_upstream_proxy_test(
    app: tauri::AppHandle,
    input: GatewayUpstreamProxyInput,
) -> Result<(), String> {
    let (proxy_url, context) = resolve_gateway_upstream_proxy_context(&app, &input)?;
    gateway::http_client::test_proxy_with_context(proxy_url.as_deref(), &context).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_upstream_proxy_detect_ip(
    app: tauri::AppHandle,
    input: GatewayUpstreamProxyInput,
) -> Result<String, String> {
    let (proxy_url, context) = resolve_gateway_upstream_proxy_context(&app, &input)?;
    gateway::http_client::detect_proxy_exit_ip_with_context(proxy_url.as_deref(), &context).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_gateway_cli_key_trims_supported_keys() {
        assert_eq!(
            normalize_gateway_cli_key(" codex ").expect("valid cli key"),
            "codex"
        );
    }

    #[test]
    fn normalize_gateway_cli_key_rejects_invalid_keys() {
        let err = normalize_gateway_cli_key(" opencode ").expect_err("invalid cli key");
        assert_eq!(err, "SEC_INVALID_INPUT: unknown cli_key=opencode");
    }
}
