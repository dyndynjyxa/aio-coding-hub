//! Usage: Gateway startup and follow-up sync for bootstrap.

use super::{gateway_control::app_start_gateway, gateway_service};
use crate::blocking;

pub(crate) async fn start(
    app_handle: &tauri::AppHandle,
    db: crate::db::Db,
    settings: &crate::settings::AppSettings,
) -> Result<crate::gateway::GatewayStatus, String> {
    let preferred_port = settings.preferred_port;
    let enable_cli_proxy_startup_recovery = settings.enable_cli_proxy_startup_recovery;

    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let status = match blocking::run("startup_gateway_autostart", {
        let app_handle = app_handle.clone();
        let db = db.clone();
        move || app_start_gateway(&app_handle, db, Some(preferred_port))
    })
    .await
    {
        Ok(status) => status,
        Err(err) => {
            tracing::error!("gateway auto-start failed: {}", err);
            if enable_cli_proxy_startup_recovery {
                crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
                    app_handle,
                    "startup_cli_proxy_restore_keep_state",
                    "startup_recovery_gateway_failed",
                    true,
                )
                .await;
            }
            return Err(format!("网关启动失败：{err}"));
        }
    };

    crate::app::heartbeat_watchdog::gated_emit(
        app_handle,
        crate::gateway::events::GATEWAY_STATUS_EVENT_NAME,
        status.clone(),
    );

    Ok(status)
}

pub(crate) async fn sync_cli_proxy_after_autostart(
    app_handle: &tauri::AppHandle,
    _status: &crate::gateway::GatewayStatus,
) {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let status = crate::app::gateway_runtime_access::app_gateway_status(app_handle);
    gateway_service::sync_cli_proxy_to_gateway(
        app_handle,
        &status,
        "cli_proxy_sync_enabled_after_autostart",
    )
    .await;
}
