//! Usage: Windows-only WSL bootstrap follow-up tasks.

pub(crate) async fn finalize(
    app_handle: &tauri::AppHandle,
    _db: crate::db::Db,
    _gateway_port: Option<u16>,
    _settings: crate::settings::AppSettings,
) {
    repair_manifests(app_handle).await;
    auto_configure(app_handle).await;
}

#[cfg_attr(not(windows), allow(unused_variables))]
async fn repair_manifests(app_handle: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let sync_guard = crate::commands::wsl::lock_wsl_sync().await;
        let repair_app = app_handle.clone();
        if let Err(err) = crate::blocking::run("startup_wsl_manifest_repair", move || {
            let _sync_guard = sync_guard;
            crate::infra::wsl::startup_repair_wsl_manifests(&repair_app)
        })
        .await
        {
            tracing::warn!("WSL manifest startup repair failed: {}", err);
        }
    }
}

#[cfg_attr(not(windows), allow(unused_variables))]
async fn auto_configure(app_handle: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let auto_cfg_app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) =
                crate::commands::wsl::wsl_auto_configure_on_startup(&auto_cfg_app).await
            {
                tracing::warn!("WSL startup auto-configure failed: {}", err);
            }
        });
    }
}
