//! Usage: Shared Tauri builder setup (managed state + plugin wiring).

use super::{
    app_state::DbInitState, gateway_state::GatewayState, resident, startup_state::StartupState,
};

pub(crate) fn create_builder() -> tauri::Builder<tauri::Wry> {
    let builder = tauri::Builder::default()
        .manage(DbInitState::default())
        .manage(GatewayState::default())
        .manage(resident::ResidentState::default())
        .manage(StartupState::default())
        .manage(crate::app::heartbeat_watchdog::HeartbeatWatchdogState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            resident::show_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build());

    builder
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_builder_keeps_single_instance_registration() {
        let source = std::fs::read_to_string(file!()).expect("read plugin registry source");
        let needle = ["tauri_plugin_", "single_", "instance::", "init"].concat();

        assert!(
            source.contains("#[cfg(desktop)]") && source.contains(&needle),
            "startup request-log reconciliation relies on desktop single-instance ownership"
        );
    }
}
