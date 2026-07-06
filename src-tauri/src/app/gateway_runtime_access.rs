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

/// Manually pin a sort_mode to a `(cli_key, session_id)` binding.
/// `sort_mode_id == None` pins to Default. Returns `false` when the gateway is
/// not running or the input is rejected.
pub(crate) fn app_gateway_pin_session_sort_mode<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    session_id: &str,
    sort_mode_id: Option<i64>,
    now_unix: i64,
) -> bool {
    super::gateway_state::with_app_running_gateway(app, |running| {
        running
            .map(|runtime| {
                runtime.pin_session_sort_mode(cli_key, session_id, sort_mode_id, now_unix)
            })
            .unwrap_or(false)
    })
}

/// Clear a session's manual sort_mode pin, reverting to the auto-inherited mode.
/// Returns `false` when the gateway is not running or there is no live binding.
pub(crate) fn app_gateway_unpin_session_sort_mode<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    session_id: &str,
    now_unix: i64,
) -> bool {
    super::gateway_state::with_app_running_gateway(app, |running| {
        running
            .map(|runtime| runtime.unpin_session_sort_mode(cli_key, session_id, now_unix))
            .unwrap_or(false)
    })
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
