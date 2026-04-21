//! Usage: Thin IPC wrappers for settings commands.

use crate::app::settings_service;
use crate::app_state::DbInitState;

pub(crate) use crate::app::settings_service::{
    CircuitBreakerNoticeUpdate, CodexSessionIdCompletionUpdate, GatewayRectifierSettingsUpdate,
    SensitiveStringUpdate, SettingsMutationResult, SettingsMutationRuntime, SettingsUpdate,
    SettingsView,
};

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_get(app: tauri::AppHandle) -> Result<SettingsView, String> {
    settings_service::settings_get(app).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    update: SettingsUpdate,
) -> Result<SettingsMutationResult, String> {
    settings_service::settings_set_impl(app, db_state.inner(), update).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_gateway_rectifier_set(
    app: tauri::AppHandle,
    update: GatewayRectifierSettingsUpdate,
) -> Result<SettingsView, String> {
    settings_service::settings_gateway_rectifier_set(app, update).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_circuit_breaker_notice_set(
    app: tauri::AppHandle,
    update: CircuitBreakerNoticeUpdate,
) -> Result<SettingsView, String> {
    settings_service::settings_circuit_breaker_notice_set(app, update).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_codex_session_id_completion_set(
    app: tauri::AppHandle,
    update: CodexSessionIdCompletionUpdate,
) -> Result<SettingsView, String> {
    settings_service::settings_codex_session_id_completion_set(app, update).await
}
