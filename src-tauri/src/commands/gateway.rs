//! Usage: Gateway lifecycle / status / session / circuit commands.

use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::commands::limit::normalize_limit;
use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::shared::mutex_ext::MutexExt;
use crate::{blocking, cli_proxy, gateway, providers, request_logs, settings, wsl};
use tauri::Manager;

const GATEWAY_SESSIONS_DEFAULT_LIMIT: u32 = 50;
const GATEWAY_SESSIONS_MAX_LIMIT: u32 = 200;

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayUpstreamProxyInput {
    proxy_url: String,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
}

fn gateway_sessions_limit(limit: Option<u32>) -> usize {
    normalize_limit(
        limit,
        GATEWAY_SESSIONS_DEFAULT_LIMIT,
        1,
        GATEWAY_SESSIONS_MAX_LIMIT,
    )
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

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct GatewayActiveSessionSummary {
    cli_key: String,
    session_id: String,
    session_suffix: String,
    provider_id: i64,
    provider_name: String,
    expires_at: i64,
    request_count: Option<i64>,
    total_input_tokens: Option<i64>,
    total_output_tokens: Option<i64>,
    total_cost_usd: Option<f64>,
    total_duration_ms: Option<i64>,
}

#[tauri::command]
#[specta::specta]
pub(crate) fn gateway_status(state: tauri::State<'_, GatewayState>) -> gateway::GatewayStatus {
    let manager = state.0.lock_or_recover();
    manager.status()
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_check_port_available(
    app: tauri::AppHandle,
    port: u16,
) -> Result<bool, String> {
    gateway_check_port_available_impl(app, port)
        .await
        .map_err(Into::into)
}

pub(crate) async fn gateway_check_port_available_impl<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    port: u16,
) -> crate::shared::error::AppResult<bool> {
    if port < 1024 {
        return Ok(false);
    }

    blocking::run(
        "gateway_check_port_available",
        move || -> crate::shared::error::AppResult<bool> {
            let cfg = settings::read(&app)?;
            let host = match cfg.gateway_listen_mode {
                settings::GatewayListenMode::Localhost => "127.0.0.1".to_string(),
                settings::GatewayListenMode::Lan => "0.0.0.0".to_string(),
                settings::GatewayListenMode::WslAuto => {
                    wsl::host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
                }
                settings::GatewayListenMode::Custom => {
                    gateway::listen::parse_custom_listen_address(&cfg.gateway_custom_listen_address)
                        .map(|v| v.host)
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                }
            };

            Ok(std::net::TcpListener::bind((host.as_str(), port)).is_ok())
        },
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_sessions_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    state: tauri::State<'_, GatewayState>,
    limit: Option<u32>,
) -> Result<Vec<GatewayActiveSessionSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    let limit = gateway_sessions_limit(limit);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let sessions = {
        let manager = state.0.lock_or_recover();
        manager.active_sessions(now_unix, limit)
    };

    if sessions.is_empty() {
        return Ok(Vec::new());
    }

    let provider_ids: Vec<i64> = sessions.iter().map(|s| s.provider_id).collect();
    let session_ids: Vec<String> = sessions.iter().map(|s| s.session_id.clone()).collect();

    let db_for_names = db.clone();
    let provider_names = blocking::run("providers_names_by_id", move || {
        providers::names_by_id(&db_for_names, &provider_ids)
    })
    .await?;

    let db_for_agg = db.clone();
    let session_stats = blocking::run("request_logs_aggregate_by_session_ids", move || {
        request_logs::aggregate_by_session_ids(&db_for_agg, &session_ids)
    })
    .await?;

    Ok(sessions
        .into_iter()
        .map(|s| {
            let cli_key = s.cli_key;
            let session_id = s.session_id;
            let session_suffix = s.session_suffix;
            let provider_id = s.provider_id;
            let expires_at = s.expires_at;

            let provider_name = provider_names
                .get(&provider_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let stats = session_stats.get(&(cli_key.clone(), session_id.clone()));

            GatewayActiveSessionSummary {
                cli_key,
                session_id,
                session_suffix,
                provider_id,
                provider_name,
                expires_at,
                request_count: stats.map(|row| row.request_count).filter(|v| *v > 0),
                total_input_tokens: stats.map(|row| row.total_input_tokens).filter(|v| *v > 0),
                total_output_tokens: stats.map(|row| row.total_output_tokens).filter(|v| *v > 0),
                total_cost_usd: stats
                    .map(|row| row.total_cost_usd_femto)
                    .filter(|v| *v > 0)
                    .map(|v| v as f64 / 1_000_000_000_000_000.0),
                total_duration_ms: stats.map(|row| row.total_duration_ms).filter(|v| *v > 0),
            }
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_circuit_status(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<gateway::GatewayProviderCircuitStatus>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("gateway_circuit_status", move || {
        let state = app.state::<GatewayState>();
        let manager = state.0.lock_or_recover();
        manager.circuit_status(&app, &db, &cli_key)
    })
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
    blocking::run(
        "gateway_circuit_reset_provider",
        move || -> crate::shared::error::AppResult<bool> {
            let state = app.state::<GatewayState>();
            let manager = state.0.lock_or_recover();
            manager.circuit_reset_provider(&db, provider_id)?;
            Ok(true)
        },
    )
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
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("gateway_circuit_reset_cli", move || {
        let state = app.state::<GatewayState>();
        let manager = state.0.lock_or_recover();
        manager.circuit_reset_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_start(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    preferred_port: Option<u16>,
) -> Result<gateway::GatewayStatus, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let status = blocking::run("gateway_start", {
        let app = app.clone();
        let db = db.clone();
        move || {
            let state = app.state::<GatewayState>();
            let mut manager = state.0.lock_or_recover();
            manager.start(&app, db, preferred_port)
        }
    })
    .await?;

    crate::app::heartbeat_watchdog::gated_emit(&app, GATEWAY_STATUS_EVENT_NAME, status.clone());
    if let Some(base_origin) = status.base_url.as_deref() {
        // Best-effort: if any CLI proxy is enabled, keep its config aligned with the actual gateway port.
        let app_for_sync = app.clone();
        let base_origin = base_origin.to_string();
        let _ = blocking::run("cli_proxy_sync_enabled_after_gateway_start", move || {
            cli_proxy::sync_enabled(&app_for_sync, &base_origin, true)
        })
        .await;
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::gateway_sessions_limit;

    #[test]
    fn gateway_sessions_limit_uses_default_and_clamps() {
        assert_eq!(gateway_sessions_limit(None), 50);
        assert_eq!(gateway_sessions_limit(Some(0)), 1);
        assert_eq!(gateway_sessions_limit(Some(999)), 200);
        assert_eq!(gateway_sessions_limit(Some(88)), 88);
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn gateway_stop(
    app: tauri::AppHandle,
    state: tauri::State<'_, GatewayState>,
) -> Result<gateway::GatewayStatus, String> {
    crate::app::cleanup::stop_gateway_best_effort(&app).await;

    let status = gateway_status(state);
    crate::app::heartbeat_watchdog::gated_emit(&app, GATEWAY_STATUS_EVENT_NAME, status.clone());

    // Best-effort: if any CLI proxy is enabled, restore its live config when the gateway is stopped,
    // so CLI tools won't keep pointing at a dead localhost gateway. Keep `enabled` state for auto re-takeover.
    crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
        &app,
        "gateway_stop_cli_proxy_restore_keep_state",
        "网关停止后",
        false,
    )
    .await;

    Ok(status)
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
