//! Usage: Read-side gateway runtime accessors for app shell and IPC layers.

use crate::shared::error::AppResult;
use crate::{db, gateway};

pub(crate) fn app_gateway_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> gateway::GatewayStatus {
    super::gateway_state::with_app_running_gateway(app, |running| {
        running.map(|runtime| runtime.status()).unwrap_or_default()
    })
}

pub(crate) fn try_app_gateway_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<gateway::GatewayStatus> {
    super::gateway_state::try_with_app_running_gateway(app, |running| {
        running.map(|runtime| runtime.status()).unwrap_or_default()
    })
}

pub(crate) fn app_gateway_active_sessions<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    now_unix: i64,
    limit: usize,
) -> Vec<crate::session_manager::ActiveSessionSnapshot> {
    super::gateway_state::with_app_running_gateway(app, |running| {
        running
            .map(|runtime| runtime.active_sessions(now_unix, limit))
            .unwrap_or_default()
    })
}

pub(crate) fn app_gateway_active_requests_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Vec<gateway::active_requests::ActiveRequestSnapshotItem> {
    super::gateway_state::try_with_app_running_gateway(app, |running| {
        running
            .map(|runtime| runtime.active_requests_snapshot())
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

pub(crate) fn app_gateway_circuit_status(
    app: &tauri::AppHandle,
    db: &db::Db,
    cli_key: &str,
) -> AppResult<Vec<gateway::GatewayProviderCircuitStatus>> {
    super::gateway_state::with_app_running_gateway(app, |running| {
        gateway::control_service::GatewayControlService::circuit_status(running, app, db, cli_key)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_requests_snapshot_returns_empty_without_running_gateway() {
        let app = tauri::test::mock_app();

        assert!(app_gateway_active_requests_snapshot(app.handle()).is_empty());
    }
}
