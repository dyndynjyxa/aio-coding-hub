//! Usage: Desktop resident mode (tray icon + window lifecycle hooks).

use std::sync::atomic::{AtomicBool, Ordering};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "main-tray";
const TRAY_MENU_TOGGLE_ID: &str = "tray.toggle";
const TRAY_MENU_QUIT_ID: &str = "tray.quit";

pub struct ResidentState {
    tray_enabled: AtomicBool,
}

impl Default for ResidentState {
    fn default() -> Self {
        Self {
            tray_enabled: AtomicBool::new(true),
        }
    }
}

impl ResidentState {
    pub fn set_tray_enabled(&self, enabled: bool) {
        self.tray_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn tray_enabled(&self) -> bool {
        self.tray_enabled.load(Ordering::Relaxed)
    }
}

#[cfg(not(desktop))]
pub fn setup_tray(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(not(desktop))]
pub fn show_main_window(_app: &tauri::AppHandle) {}

#[cfg(not(desktop))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(desktop)]
use tauri::Manager;

#[cfg(desktop)]
pub fn setup_tray(app: &tauri::AppHandle) -> Result<(), String> {
    let toggle_item = MenuItem::with_id(app, TRAY_MENU_TOGGLE_ID, "显示/隐藏", true, None::<&str>)
        .map_err(|e| format!("failed to create tray toggle menu item: {e}"))?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT_ID, "退出", true, None::<&str>)
        .map_err(|e| format!("failed to create tray quit menu item: {e}"))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|e| format!("failed to create tray menu separator: {e}"))?;

    let menu = Menu::with_items(app, &[&toggle_item, &separator, &quit_item])
        .map_err(|e| format!("failed to create tray menu: {e}"))?;

    let toggle_id = toggle_item.id().clone();
    let quit_id = quit_item.id().clone();

    #[cfg(target_os = "macos")]
    let icon_bytes = include_bytes!("../../icons/trayTemplate.png");
    #[cfg(not(target_os = "macos"))]
    let icon_bytes = include_bytes!("../../icons/32x32.png");

    let icon = tauri::image::Image::from_bytes(icon_bytes)
        .map_err(|e| format!("failed to load tray icon: {e}"))?;

    let tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("AIO Coding Hub")
        .menu(&menu);

    #[cfg(target_os = "macos")]
    let tray_builder = tray_builder.icon_as_template(true);

    tray_builder
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if event.id == quit_id {
                app.exit(0);
                return;
            }
            if event.id == toggle_id {
                toggle_main_window(app);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    show_main_window(tray.app_handle());
                }
            }
        })
        .build(app)
        .map_err(|e| format!("failed to build tray icon: {e}"))?;

    Ok(())
}

#[cfg(desktop)]
pub fn show_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

#[cfg(desktop)]
fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if !is_visible || is_minimized {
        show_main_window(app);
        return;
    }

    let _ = window.hide();
}

#[cfg(desktop)]
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    api.prevent_close();

    let resident = window.state::<ResidentState>();
    if resident.tray_enabled() {
        let _ = window.hide();
    } else {
        let _ = window.minimize();
    }
}
