//! Usage: Tauri run-event lifecycle hooks extracted from `lib.rs`.

use super::resident;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static EXIT_CLEANUP_SPAWNED: AtomicBool = AtomicBool::new(false);

pub(crate) fn handle_run_event(app_handle: &tauri::AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
        if *code != Some(tauri::RESTART_EXIT_CODE) {
            app_handle.state::<resident::ResidentState>().begin_exit();
            api.prevent_exit();

            if EXIT_CLEANUP_SPAWNED.swap(true, Ordering::SeqCst) {
                return;
            }

            tracing::info!("exit requested, starting cleanup...");
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                crate::app::cleanup::cleanup_before_exit(&app_handle).await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                std::process::exit(0);
            });
        }
    }

    #[cfg(target_os = "macos")]
    if let tauri::RunEvent::Reopen {
        has_visible_windows,
        ..
    } = event
    {
        if !has_visible_windows {
            resident::show_main_window(app_handle);
        }
    }
}
