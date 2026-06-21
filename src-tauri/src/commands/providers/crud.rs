//! Usage: Thin IPC wrappers for provider CRUD commands.

use crate::app::provider_service;
use crate::app_state::DbInitState;

pub(crate) use crate::app::provider_service::ProviderUpsertInput;

#[tauri::command]
#[specta::specta]
pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<crate::providers::ProviderSummary>, String> {
    provider_service::providers_list(app, db_state, cli_key).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: ProviderUpsertInput,
) -> Result<crate::providers::ProviderSummary, String> {
    provider_service::provider_upsert(app, db_state, input).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_duplicate(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<crate::providers::ProviderSummary, String> {
    provider_service::provider_duplicate(app, db_state, provider_id).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<crate::providers::ProviderSummary, String> {
    provider_service::provider_set_enabled(app, db_state, provider_id, enabled).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    clear_usage_stats: bool,
) -> Result<bool, String> {
    provider_service::provider_delete(app, db_state, provider_id, clear_usage_stats).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<crate::providers::ProviderSummary>, String> {
    provider_service::providers_reorder(app, db_state, cli_key, ordered_provider_ids).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn default_route_providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<crate::providers::ProviderRouteRow>, String> {
    provider_service::default_route_providers_list(app, db_state, cli_key).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn default_route_providers_set_order(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<crate::providers::ProviderRouteRow>, String> {
    provider_service::default_route_providers_set_order(
        app,
        db_state,
        cli_key,
        ordered_provider_ids,
    )
    .await
}
