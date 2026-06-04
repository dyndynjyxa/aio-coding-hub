//! Usage: Synchronous Tauri setup wiring extracted from `lib.rs`.

use super::resident;
use tauri_plugin_dialog::DialogExt;

pub(crate) fn setup(app: &mut tauri::App<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    crate::app::logging::init(app.handle());
    guard_restart_storm(app);
    crate::app::heartbeat_watchdog::install(app.handle());
    install_panic_hook();
    init_desktop_integrations(app);
    init_main_window_chrome(app);
    log_dev_diagnostics(app);
    crate::app::startup_tasks::spawn(app.handle().clone());
    Ok(())
}

fn guard_restart_storm(app: &mut tauri::App<tauri::Wry>) {
    if crate::app::heartbeat_watchdog::check_and_clear_restart_marker(app.handle()) {
        tracing::error!("startup: restart storm detected, auto-recovery disabled for this session");
        app.dialog()
            .message(
                "AIO Coding Hub 检测到 WebView 反复崩溃，已停止自动恢复。\n\n\
                 如果问题持续出现，请检查系统 WebView2 运行时是否正常。",
            )
            .title("WebView 恢复失败")
            .blocking_show();
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        tracing::error!(
            location = %location,
            "PANIC: application panicked at {location}. Check the log file for context leading up to this panic."
        );
    }));
}

fn init_desktop_integrations(app: &mut tauri::App<tauri::Wry>) {
    #[cfg(desktop)]
    {
        if let Err(err) = app
            .handle()
            .plugin(tauri_plugin_updater::Builder::new().build())
        {
            tracing::error!("updater initialization failed: {}", err);
        }

        if let Err(err) = resident::setup_tray(app.handle()) {
            tracing::error!("system tray initialization failed: {}", err);
        }
    }
}

fn init_main_window_chrome(app: &tauri::App<tauri::Wry>) {
    use tauri::Manager;

    if let Some(window) = app.get_webview_window("main") {
        crate::app::window_chrome::apply_main_window_chrome(&window);
    }
}

#[cfg(debug_assertions)]
fn log_dev_diagnostics(app: &tauri::App<tauri::Wry>) {
    let enabled = std::env::var("AIO_CODING_HUB_DEV_DIAGNOSTICS")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .is_some_and(|v| v == "1" || v == "true" || v == "yes");
    if enabled {
        let identifier = &app.config().identifier;
        let product_name = app.config().product_name.as_deref().unwrap_or("<missing>");
        tracing::info!(identifier = %identifier, "[dev] tauri identifier");
        tracing::info!(product_name = %product_name, "[dev] productName");
        if let Ok(dotdir_name) = std::env::var("AIO_CODING_HUB_DOTDIR_NAME") {
            tracing::info!(dotdir_name = %dotdir_name, "[dev] AIO_CODING_HUB_DOTDIR_NAME");
        }
        if let Ok(dir) = crate::app_paths::app_data_dir(app.handle()) {
            tracing::info!(dir = %dir.display(), "[dev] app data dir");
        }
    }
}

#[cfg(not(debug_assertions))]
fn log_dev_diagnostics(_app: &tauri::App<tauri::Wry>) {}
