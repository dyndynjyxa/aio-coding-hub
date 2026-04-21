//! Usage: Windows-only WSL bootstrap follow-up tasks.

pub(crate) async fn finalize(
    app_handle: &tauri::AppHandle,
    db: crate::db::Db,
    gateway_port: Option<u16>,
    settings: crate::settings::AppSettings,
) {
    repair_manifests(app_handle).await;
    auto_configure(app_handle, db, gateway_port, settings).await;
}

#[cfg_attr(not(windows), allow(unused_variables))]
async fn repair_manifests(app_handle: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let repair_app = app_handle.clone();
        if let Err(err) = crate::blocking::run("startup_wsl_manifest_repair", move || {
            crate::infra::wsl::startup_repair_wsl_manifests(&repair_app)
        })
        .await
        {
            tracing::warn!("WSL manifest startup repair failed: {}", err);
        }
    }
}

#[cfg_attr(not(windows), allow(unused_variables))]
async fn auto_configure(
    app_handle: &tauri::AppHandle,
    db: crate::db::Db,
    gateway_port: Option<u16>,
    settings: crate::settings::AppSettings,
) {
    #[cfg(windows)]
    if settings.wsl_auto_config {
        let auto_cfg_app = app_handle.clone();
        let auto_cfg_db = db.clone();
        let gateway_listen_mode = settings.gateway_listen_mode;

        tauri::async_runtime::spawn(async move {
            if let Err(err) = crate::commands::wsl::wsl_auto_configure_on_startup(
                &auto_cfg_app,
                auto_cfg_db,
                gateway_listen_mode,
                gateway_port,
            )
            .await
            {
                tracing::warn!("WSL startup auto-configure failed: {}", err);
            }
        });
    }
}
