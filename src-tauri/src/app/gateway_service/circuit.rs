//! Usage: Gateway circuit orchestration for app shell and IPC layers.

use crate::gateway_control::{app_gateway_circuit_reset_cli, app_gateway_circuit_reset_provider};
use crate::gateway_runtime_access::app_gateway_circuit_status;
use crate::shared::error::AppResult;
use crate::{blocking, db, gateway};

pub(crate) async fn circuit_status(
    app: tauri::AppHandle,
    db: db::Db,
    cli_key: String,
) -> AppResult<Vec<gateway::GatewayProviderCircuitStatus>> {
    blocking::run("gateway_circuit_status", move || {
        app_gateway_circuit_status(&app, &db, &cli_key)
    })
    .await
}

pub(crate) async fn circuit_reset_provider(
    app: tauri::AppHandle,
    db: db::Db,
    provider_id: i64,
) -> AppResult<bool> {
    blocking::run(
        "gateway_circuit_reset_provider",
        move || -> AppResult<bool> {
            app_gateway_circuit_reset_provider(&app, &db, provider_id)?;
            Ok(true)
        },
    )
    .await
}

pub(crate) async fn circuit_reset_cli(
    app: tauri::AppHandle,
    db: db::Db,
    cli_key: String,
) -> AppResult<usize> {
    blocking::run("gateway_circuit_reset_cli", move || {
        app_gateway_circuit_reset_cli(&app, &db, &cli_key)
    })
    .await
}
