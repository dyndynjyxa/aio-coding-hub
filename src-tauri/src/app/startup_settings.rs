//! Usage: Startup settings loading and initial window-state application.

use super::resident;
use crate::{blocking, cli_proxy, settings};
use tauri::Manager;

pub(crate) async fn read(
    app_handle: &tauri::AppHandle,
) -> Result<crate::settings::AppSettings, String> {
    let settings = match blocking::run("startup_read_settings", {
        let app_handle = app_handle.clone();
        move || settings::read(&app_handle)
    })
    .await
    {
        Ok(cfg) => cfg,
        Err(err) => {
            tracing::error!(
                "startup settings read failed; skipping settings-dependent startup tasks: {}",
                err
            );
            crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
                app_handle,
                "startup_cli_proxy_restore_on_settings_read_failed",
                "startup_settings_read_failed",
                false,
            )
            .await;
            resident::show_main_window(app_handle);
            return Err(format!("设置读取失败：{err}"));
        }
    };

    if settings.enable_cli_proxy_startup_recovery {
        repair_cli_proxy_enable_state(app_handle).await;
    }

    Ok(settings)
}

pub(crate) fn apply_window_state(
    app_handle: &tauri::AppHandle,
    settings: &crate::settings::AppSettings,
) {
    app_handle
        .state::<resident::ResidentState>()
        .set_tray_enabled(settings.tray_enabled);

    if settings.start_minimized {
        resident::hide_main_window_on_startup(app_handle);
    } else {
        resident::show_main_window(app_handle);
    }
}

async fn repair_cli_proxy_enable_state(app_handle: &tauri::AppHandle) {
    match blocking::run("startup_cli_proxy_repair_incomplete_enable", {
        let app_handle = app_handle.clone();
        move || cli_proxy::startup_repair_incomplete_enable(&app_handle)
    })
    .await
    {
        Ok(results) => {
            let mut repaired = Vec::new();
            for result in results {
                if result.ok {
                    repaired.push(result.cli_key);
                    continue;
                }

                tracing::warn!(
                    cli_key = %result.cli_key,
                    trace_id = %result.trace_id,
                    error_code = %result.error_code.unwrap_or_default(),
                    "startup recovery: cli_proxy enable state repair failed: {}",
                    result.message
                );
            }

            if !repaired.is_empty() {
                tracing::info!(
                    repaired = repaired.len(),
                    cli_keys = ?repaired,
                    "startup recovery: repaired cli_proxy enable state inconsistencies"
                );
            }
        }
        Err(err) => {
            tracing::warn!(
                "startup recovery: cli_proxy enable state repair task failed: {}",
                err
            );
        }
    }
}
