//! Usage: Application layer (Tauri-managed state, tray/window lifecycle, startup wiring).

pub(crate) mod app_state;
pub(crate) mod autostart;
pub(crate) mod bootstrap;
pub(crate) mod cleanup;
pub(crate) mod cli_proxy_service;
pub(crate) mod gateway_control;
pub(crate) mod gateway_lifecycle_lock;
pub(crate) mod gateway_runtime_access;
pub(crate) mod gateway_service;
pub(crate) mod gateway_state;
pub(crate) mod heartbeat_watchdog;
pub(crate) mod lifecycle;
pub(crate) mod linux_webkit_compat;
pub(crate) mod logging;
pub(crate) mod notice;
pub(crate) mod notification_sound;
pub(crate) mod plugin_registry;
pub(crate) mod provider_service;
pub(crate) mod resident;
pub(crate) mod settings_service;
pub(crate) mod startup_gateway;
pub(crate) mod startup_settings;
pub(crate) mod startup_state;
pub(crate) mod startup_tasks;
pub(crate) mod startup_wsl;
pub(crate) mod window_chrome;
