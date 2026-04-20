//! Usage: CLI proxy configuration related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::shared::mutex_ext::MutexExt;
use crate::{blocking, cli_proxy, mcp, settings};
use tauri::Manager;

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_proxy_status_all(
    app: tauri::AppHandle,
) -> Result<Vec<cli_proxy::CliProxyStatus>, String> {
    let current_base_origin = {
        let state = app.state::<GatewayState>();
        let manager = state.0.lock_or_recover();
        let status = manager.status();
        if status.running {
            Some(status.base_url.unwrap_or_else(|| {
                format!(
                    "http://127.0.0.1:{}",
                    status.port.unwrap_or(settings::DEFAULT_GATEWAY_PORT)
                )
            }))
        } else {
            None
        }
    };

    blocking::run("cli_proxy_status_all", move || {
        cli_proxy::status_all(&app, current_base_origin.as_deref())
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_proxy_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    enabled: bool,
) -> Result<cli_proxy::CliProxyResult, String> {
    if enabled {
        cli_proxy_set_enabled_impl(app, db_state.inner(), cli_key, true).await
    } else {
        cli_proxy_set_disabled_impl(app, Some(db_state.inner()), cli_key).await
    }
}

pub(crate) async fn cli_proxy_set_enabled_impl(
    app: tauri::AppHandle,
    db_state: &DbInitState,
    cli_key: String,
    enabled: bool,
) -> Result<cli_proxy::CliProxyResult, String> {
    tracing::info!(cli_key = %cli_key, enabled = enabled, "cli proxy enabled state changing");

    let base_origin = if enabled {
        let db = ensure_db_ready(app.clone(), db_state).await?;

        blocking::run("cli_proxy_set_enabled_ensure_gateway", {
            let app = app.clone();
            let db = db.clone();
            move || -> crate::shared::error::AppResult<String> {
                let state = app.state::<GatewayState>();
                let mut manager = state.0.lock_or_recover();
                let status = if manager.status().running {
                    manager.status()
                } else {
                    let settings = settings::read(&app)?;
                    let status = manager.start(&app, db, Some(settings.preferred_port))?;
                    crate::app::heartbeat_watchdog::gated_emit(
                        &app,
                        GATEWAY_STATUS_EVENT_NAME,
                        status.clone(),
                    );
                    status
                };

                Ok(status.base_url.unwrap_or_else(|| {
                    format!(
                        "http://127.0.0.1:{}",
                        status.port.unwrap_or(settings::DEFAULT_GATEWAY_PORT)
                    )
                }))
            }
        })
        .await?
    } else {
        format!("http://127.0.0.1:{}", settings::DEFAULT_GATEWAY_PORT)
    };

    let result = blocking::run("cli_proxy_set_enabled_apply", {
        let app = app.clone();
        let cli_key = cli_key.clone();
        move || cli_proxy::set_enabled(&app, &cli_key, enabled, &base_origin)
    })
    .await
    .map_err(Into::into);

    // After successful proxy toggle, re-sync MCP servers to CLI config file.
    // cli_proxy and mcp_sync both write to the same config file (e.g. ~/.codex/config.toml).
    // Without this re-sync, MCP entries get lost during the backup/restore cycle.
    if let Ok(ref r) = result {
        if r.ok {
            match ensure_db_ready(app.clone(), db_state).await {
                Ok(db) => {
                    let sync_app = app.clone();
                    let sync_cli_key = cli_key.clone();
                    if let Err(err) = blocking::run("cli_proxy_mcp_resync", move || {
                        let conn = db.open_connection()?;
                        mcp::sync_one_cli(&sync_app, &conn, &sync_cli_key)
                    })
                    .await
                    {
                        tracing::warn!(cli_key = %cli_key, "mcp re-sync after proxy toggle failed: {err}");
                    }
                }
                Err(err) => {
                    tracing::warn!(cli_key = %cli_key, "mcp re-sync skipped, db unavailable: {err}");
                }
            }
        }
    }

    match &result {
        Ok(r) if !r.ok => {
            tracing::warn!(
                cli_key = %r.cli_key,
                error_code = %r.error_code.as_deref().unwrap_or(""),
                "cli proxy set_enabled failed: {}",
                r.message
            );
        }
        Err(err) => {
            tracing::warn!("cli proxy set_enabled error: {}", err);
        }
        _ => {}
    }

    result
}

pub(crate) async fn cli_proxy_set_disabled_impl<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_state: Option<&DbInitState>,
    cli_key: String,
) -> Result<cli_proxy::CliProxyResult, String> {
    tracing::info!(cli_key = %cli_key, enabled = false, "cli proxy enabled state changing");

    let base_origin = format!("http://127.0.0.1:{}", settings::DEFAULT_GATEWAY_PORT);
    let result = blocking::run("cli_proxy_set_enabled_apply", {
        let app = app.clone();
        let cli_key = cli_key.clone();
        move || cli_proxy::set_enabled(&app, &cli_key, false, &base_origin)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref r) = result {
        if r.ok {
            let db_result = match db_state {
                Some(db_state) => ensure_db_ready(app.clone(), db_state).await,
                None => crate::infra::db::init(&app),
            };
            match db_result {
                Ok(db) => {
                    let sync_app = app.clone();
                    let sync_cli_key = cli_key.clone();
                    if let Err(err) = blocking::run("cli_proxy_mcp_resync", move || {
                        let conn = db.open_connection()?;
                        mcp::sync_one_cli(&sync_app, &conn, &sync_cli_key)
                    })
                    .await
                    {
                        tracing::warn!(cli_key = %cli_key, "mcp re-sync after proxy toggle failed: {err}");
                    }
                }
                Err(err) => {
                    tracing::warn!(cli_key = %cli_key, "mcp re-sync skipped, db unavailable: {err}");
                }
            }
        }
    }

    match &result {
        Ok(r) if !r.ok => {
            tracing::warn!(
                cli_key = %r.cli_key,
                error_code = %r.error_code.as_deref().unwrap_or(""),
                "cli proxy set_enabled failed: {}",
                r.message
            );
        }
        Err(err) => {
            tracing::warn!("cli proxy set_enabled error: {}", err);
        }
        _ => {}
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_proxy_sync_enabled(
    app: tauri::AppHandle,
    base_origin: String,
    apply_live: Option<bool>,
) -> Result<Vec<cli_proxy::CliProxyResult>, String> {
    blocking::run("cli_proxy_sync_enabled", move || {
        cli_proxy::sync_enabled(&app, &base_origin, apply_live.unwrap_or(true))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_proxy_rebind_codex_home(
    app: tauri::AppHandle,
) -> Result<cli_proxy::CliProxyResult, String> {
    let (gateway_running, base_origin) = {
        let state = app.state::<GatewayState>();
        let manager = state.0.lock_or_recover();
        let status = manager.status();
        if status.running {
            (
                true,
                status.base_url.unwrap_or_else(|| {
                    format!(
                        "http://127.0.0.1:{}",
                        status.port.unwrap_or(settings::DEFAULT_GATEWAY_PORT)
                    )
                }),
            )
        } else {
            let settings = settings::read(&app)?;
            (
                false,
                format!("http://127.0.0.1:{}", settings.preferred_port),
            )
        }
    };

    blocking::run("cli_proxy_rebind_codex_home", move || {
        cli_proxy::rebind_codex_home_after_change(&app, &base_origin, gateway_running)
    })
    .await
    .map_err(Into::into)
}
