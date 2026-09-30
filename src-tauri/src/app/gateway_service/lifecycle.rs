//! Usage: Gateway lifecycle orchestration and shell-side follow-up actions.

use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::gateway_control::{app_ensure_gateway_running, app_start_gateway};
use crate::gateway_runtime_access::app_gateway_status;
use crate::shared::error::AppResult;
use crate::{blocking, db, gateway};

fn emit_gateway_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    status: &gateway::GatewayStatus,
) {
    crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_STATUS_EVENT_NAME, status.clone());
}

pub(crate) async fn sync_cli_proxy_to_gateway<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: db::Db,
    status: &gateway::GatewayStatus,
    task_label: &'static str,
) {
    let Some(base_origin) = status.base_url.as_deref().filter(|_| status.running) else {
        return;
    };

    let app_for_sync = app.clone();
    let base_origin = base_origin.to_string();
    if let Err(err) = blocking::run(task_label, move || {
        crate::cli_proxy::sync_enabled(&app_for_sync, &base_origin, true)
            .and_then(crate::app::cli_proxy_service::require_success)
    })
    .await
    {
        tracing::warn!(error = %err, "CLI proxy sync task failed");
    }

    // The sync short-circuits when the codex config is already applied; refresh the
    // capability catalog asynchronously so DB changes made while the gateway was
    // off still land without blocking startup on the 20s CLI export.
    crate::app::provider_service::spawn_claude_desktop_models_refresh(app, db.clone());
    crate::app::provider_service::spawn_codex_catalog_refresh(app, db);
    emit_gateway_status(app, status);
}

// Caller holds GatewayLifecycleLock across start, client sync, and its own mutation.
pub(crate) async fn ensure_running_and_sync_unlocked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: db::Db,
    preferred_port: Option<u16>,
) -> AppResult<gateway::GatewayStatus>
where
    R::Handle: Unpin,
{
    let status = blocking::run("cli_proxy_ensure_gateway", {
        let app = app.clone();
        let db = db.clone();
        move || app_ensure_gateway_running(&app, db, preferred_port)
    })
    .await?;
    sync_cli_proxy_to_gateway(app, db, &status, "cli_proxy_sync_after_ensure").await;
    Ok(status)
}

pub(crate) async fn start_and_sync<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    preferred_port: Option<u16>,
) -> AppResult<gateway::GatewayStatus>
where
    R::Handle: Unpin,
{
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let status = blocking::run("gateway_start", {
        let app = app.clone();
        let db = db.clone();
        move || app_start_gateway(&app, db, preferred_port)
    })
    .await?;

    sync_cli_proxy_to_gateway(&app, db, &status, "cli_proxy_sync_after_gateway_start").await;
    Ok(status)
}

pub(crate) async fn stop_and_restore(app: tauri::AppHandle) -> AppResult<gateway::GatewayStatus> {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    crate::app::cleanup::stop_gateway_best_effort_unlocked(&app).await;
    crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
        &app,
        "gateway_stop_cli_proxy_restore_keep_state",
    )
    .await;
    let status = app_gateway_status(&app);
    emit_gateway_status(&app, &status);
    Ok(status)
}

// Use after a failed settings transaction has settled on its final listener.
pub(crate) async fn reconcile_cli_proxy_unlocked<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let status = crate::app::gateway_runtime_access::app_gateway_status(app);
    let app_for_sync = app.clone();
    let status_for_sync = status.clone();
    if let Err(err) = blocking::run("cli_proxy_reconcile_gateway", move || match status_for_sync
        .base_url
        .as_deref()
        .filter(|_| status_for_sync.running)
    {
        Some(origin) => crate::cli_proxy::sync_enabled(&app_for_sync, origin, true)
            .and_then(crate::app::cli_proxy_service::require_success),
        None => crate::cli_proxy::restore_enabled_keep_state(&app_for_sync)
            .and_then(crate::app::cli_proxy_service::require_success),
    })
    .await
    {
        tracing::warn!(error = %err, "CLI proxy could not reconcile final gateway state");
    }
    crate::app::heartbeat_watchdog::gated_emit(
        app,
        crate::gateway::events::GATEWAY_STATUS_EVENT_NAME,
        status,
    );
}
