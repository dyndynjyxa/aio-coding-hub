//! Usage: Startup settings loading and initial window-state application.

use super::resident;
use crate::{blocking, settings};
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
            let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
            crate::app::cleanup::restore_cli_proxy_keep_state_best_effort(
                app_handle,
                "startup_cli_proxy_restore_on_settings_read_failed",
            )
            .await;
            resident::show_main_window(app_handle);
            return Err(format!("设置读取失败：{err}"));
        }
    };

    if settings.enable_cli_proxy_startup_recovery {
        let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
        if let Err(err) = blocking::run("startup_cli_proxy_repair", {
            let app = app_handle.clone();
            move || {
                crate::cli_proxy::startup_repair_incomplete_enable(&app)
                    .and_then(super::cli_proxy_service::require_success)
            }
        })
        .await
        {
            tracing::warn!(error = %err, "startup CLI proxy repair failed");
        }
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
