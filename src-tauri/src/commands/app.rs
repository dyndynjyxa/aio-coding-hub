//! Usage: App-level Tauri commands (about info, lifecycle, etc.).

use tauri::utils::config::BundleType;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AppAboutInfo {
    os: String,
    arch: String,
    profile: String,
    app_version: String,
    bundle_type: Option<String>,
    run_mode: String,
}

#[tauri::command]
pub(crate) fn app_about_get() -> AppAboutInfo {
    let bundle_type = tauri::utils::platform::bundle_type();
    let run_mode = match bundle_type {
        Some(BundleType::Nsis | BundleType::Msi | BundleType::Deb | BundleType::Rpm) => "installer",
        Some(BundleType::AppImage) => "portable",
        Some(BundleType::App | BundleType::Dmg) => "unknown",
        None => "unknown",
    }
    .to_string();

    AppAboutInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        bundle_type: bundle_type.map(|t| t.to_string()),
        run_mode,
    }
}

#[tauri::command]
pub(crate) fn app_exit(app: tauri::AppHandle) -> Result<bool, String> {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        app.exit(0);
    });
    Ok(true)
}

#[tauri::command]
pub(crate) fn app_restart(app: tauri::AppHandle) -> Result<bool, String> {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        tauri::async_runtime::block_on(crate::app::cleanup::cleanup_before_exit(&app));
        app.request_restart();
    });
    Ok(true)
}
