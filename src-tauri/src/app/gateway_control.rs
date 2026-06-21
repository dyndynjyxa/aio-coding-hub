//! Usage: Control-side gateway runtime actions for app shell orchestration.

use crate::shared::error::AppResult;
use crate::{db, gateway, settings};

pub(crate) fn app_gateway_circuit_reset_provider(
    app: &tauri::AppHandle,
    db: &db::Db,
    provider_id: i64,
) -> AppResult<()> {
    super::gateway_state::with_app_running_gateway(app, |running| {
        gateway::control_service::GatewayControlService::circuit_reset_provider(
            running,
            db,
            provider_id,
        )
    })
}

pub(crate) fn app_gateway_circuit_reset_cli(
    app: &tauri::AppHandle,
    db: &db::Db,
    cli_key: &str,
) -> AppResult<usize> {
    super::gateway_state::with_app_running_gateway(app, |running| {
        gateway::control_service::GatewayControlService::circuit_reset_cli(running, db, cli_key)
    })
}

pub(crate) fn app_start_gateway(
    app: &tauri::AppHandle,
    db: db::Db,
    preferred_port: Option<u16>,
) -> AppResult<gateway::GatewayStatus> {
    super::gateway_state::with_app_running_gateway_slot_mut(app, |running| {
        let cfg = settings::read(app)?;
        let requested_port = preferred_port
            .filter(|port| *port > 0)
            .unwrap_or(cfg.preferred_port.max(settings::DEFAULT_GATEWAY_PORT));
        let start_result = gateway::control_service::GatewayControlService::start(
            running,
            app,
            db,
            &cfg,
            preferred_port,
        )?;

        if start_result.effective_preferred_port != requested_port
            && requested_port == cfg.preferred_port
        {
            if let Ok(mut current) = settings::read(app) {
                if current.preferred_port != start_result.effective_preferred_port {
                    current.preferred_port = start_result.effective_preferred_port;
                    let _ = settings::write(app, &current);
                }
            }
        }

        Ok(start_result.status)
    })
}

pub(crate) fn app_start_gateway_with_config(
    app: &tauri::AppHandle,
    db: db::Db,
    cfg: &settings::AppSettings,
    preferred_port: Option<u16>,
) -> AppResult<gateway::control_service::GatewayStartResult> {
    super::gateway_state::with_app_running_gateway_slot_mut(app, |running| {
        gateway::control_service::GatewayControlService::start(
            running,
            app,
            db,
            cfg,
            preferred_port,
        )
    })
}

pub(crate) fn app_ensure_gateway_running(
    app: &tauri::AppHandle,
    db: db::Db,
    preferred_port: Option<u16>,
) -> AppResult<gateway::GatewayStatus> {
    let status = super::gateway_runtime_access::app_gateway_status(app);
    if status.running {
        Ok(status)
    } else {
        app_start_gateway(app, db, preferred_port)
    }
}

pub(crate) fn app_gateway_clear_cli_route_runtime_state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::gateway::runtime::GatewayRouteRuntimeClearResult {
    super::gateway_state::with_app_running_gateway(app, |running| {
        running
            .map(|runtime| runtime.clear_cli_route_runtime_state(cli_key))
            .unwrap_or_default()
    })
}

pub(crate) fn app_refresh_gateway_plugins<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
) {
    super::gateway_state::with_app_running_gateway(app, |running| {
        gateway::control_service::GatewayControlService::refresh_plugins(running, db);
    });
}

pub(crate) fn try_app_gateway_update_circuit_config<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    failure_threshold: u32,
    open_duration_secs: i64,
) -> bool {
    super::gateway_state::try_with_app_running_gateway(app, |running| {
        if let Some(runtime) = running {
            runtime.update_circuit_config(failure_threshold, open_duration_secs);
        }
    })
    .is_some()
}

pub(crate) fn app_take_running_gateway<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<crate::gateway::runtime::GatewayRuntimeHandles> {
    super::gateway_state::take_app_running_gateway(app)
}
