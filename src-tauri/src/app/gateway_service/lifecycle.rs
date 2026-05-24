//! Usage: Gateway lifecycle orchestration and shell-side follow-up actions.

use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::gateway_control::app_start_gateway;
use crate::gateway_runtime_access::app_gateway_status;
use crate::shared::error::AppResult;
use crate::{blocking, cli_proxy, db, gateway};

fn emit_gateway_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    status: &gateway::GatewayStatus,
) {
    crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_STATUS_EVENT_NAME, status.clone());
}

pub(crate) async fn sync_cli_proxy_to_gateway<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    status: &gateway::GatewayStatus,
    task_label: &'static str,
) {
    let Some(base_origin) = status.base_url.as_deref() else {
        return;
    };

    let app_for_sync = app.clone();
    let base_origin = base_origin.to_string();
    let _ = blocking::run(task_label, move || {
        cli_proxy::sync_enabled(&app_for_sync, &base_origin, true)
    })
    .await;
}

pub(crate) async fn start_and_sync(
    app: tauri::AppHandle,
    db: db::Db,
    preferred_port: Option<u16>,
) -> AppResult<gateway::GatewayStatus> {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let status = blocking::run("gateway_start", {
        let app = app.clone();
        let db = db.clone();
        move || app_start_gateway(&app, db, preferred_port)
    })
    .await?;

    emit_gateway_status(&app, &status);
    sync_cli_proxy_to_gateway(&app, &status, "cli_proxy_sync_enabled_after_gateway_start").await;

    Ok(status)
}

pub(crate) async fn stop_and_restore(app: tauri::AppHandle) -> AppResult<gateway::GatewayStatus> {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    crate::app::cleanup::stop_gateway_best_effort_unlocked(&app).await;

    let status = app_gateway_status(&app);
    emit_gateway_status(&app, &status);

    crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
        &app,
        "gateway_stop_cli_proxy_restore_keep_state",
        "网关停止后",
        false,
    )
    .await;

    Ok(status)
}
