//! Usage: CLI proxy configuration related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_runtime_access::app_gateway_status;
use crate::{blocking, cli_proxy, mcp, settings};

pub(crate) async fn cli_proxy_status_all(
    app: tauri::AppHandle,
) -> Result<Vec<cli_proxy::CliProxyStatus>, String> {
    let status = app_gateway_status(&app);
    let current_base_origin = if status.running {
        Some(status.base_url.unwrap_or_else(|| {
            format!(
                "http://127.0.0.1:{}",
                status.port.unwrap_or(settings::DEFAULT_GATEWAY_PORT)
            )
        }))
    } else {
        None
    };

    blocking::run("cli_proxy_status_all", move || {
        cli_proxy::status_all(&app, current_base_origin.as_deref())
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn cli_proxy_set_enabled_impl<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_state: &DbInitState,
    cli_key: String,
    enabled: bool,
) -> Result<cli_proxy::CliProxyResult, String>
where
    R::Handle: Unpin,
{
    tracing::info!(cli_key = %cli_key, enabled = enabled, "cli proxy enabled state changing");

    let db = if enabled {
        Some(ensure_db_ready(app.clone(), db_state).await?)
    } else {
        None
    };
    let gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let base_origin = if let Some(db) = db {
        let status =
            crate::app::gateway_service::ensure_running_and_sync_unlocked(&app, db, None).await?;
        status
            .base_url
            .ok_or_else(|| "SYSTEM_ERROR: gateway base_url missing".to_string())?
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

    drop(gateway_lifecycle);
    result
}

pub(crate) async fn cli_proxy_set_disabled_impl<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_state: Option<&DbInitState>,
    cli_key: String,
) -> Result<cli_proxy::CliProxyResult, String> {
    tracing::info!(cli_key = %cli_key, enabled = false, "cli proxy enabled state changing");
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;

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

pub(crate) async fn cli_proxy_sync_enabled(
    app: tauri::AppHandle,
    base_origin: String,
    apply_live: Option<bool>,
) -> Result<Vec<cli_proxy::CliProxyResult>, String> {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    blocking::run("cli_proxy_sync_enabled", move || {
        cli_proxy::sync_enabled(&app, &base_origin, apply_live.unwrap_or(true))
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn cli_proxy_rebind_codex_home(
    app: tauri::AppHandle,
) -> Result<cli_proxy::CliProxyResult, String> {
    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let status = app_gateway_status(&app);
    let (gateway_running, base_origin) = if status.running {
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
    };

    blocking::run("cli_proxy_rebind_codex_home", move || {
        cli_proxy::rebind_codex_home_after_change(&app, &base_origin, gateway_running)
    })
    .await
    .map_err(Into::into)
}

// Batch CLI operations keep processing after one failure; callers requiring a
// successful batch must also inspect the per-CLI result, not just the outer Result.
pub(crate) fn require_success(
    results: Vec<cli_proxy::CliProxyResult>,
) -> crate::shared::error::AppResult<()> {
    let errors: Vec<_> = results
        .into_iter()
        .filter(|result| !result.ok)
        .map(|result| format!("{}: {}", result.cli_key, result.message))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::shared::error::AppError::new(
            "CLI_PROXY_FAILED",
            errors.join("; "),
        ))
    }
}

#[cfg(test)]
mod tests;
