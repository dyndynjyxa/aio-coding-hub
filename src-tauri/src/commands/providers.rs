//! Usage: Provider configuration related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{base_url_probe, blocking, providers};

#[tauri::command]
pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderSummary>, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("providers_list", move || {
        providers::list_by_cli(&app, &cli_key)
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: Option<i64>,
    cli_key: String,
    name: String,
    base_urls: Vec<String>,
    base_url_mode: String,
    api_key: Option<String>,
    enabled: bool,
    cost_multiplier: f64,
    priority: Option<i64>,
    claude_models: Option<providers::ClaudeModels>,
) -> Result<providers::ProviderSummary, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("provider_upsert", move || {
        providers::upsert(
            &app,
            provider_id,
            &cli_key,
            &name,
            base_urls,
            &base_url_mode,
            api_key.as_deref(),
            enabled,
            cost_multiplier,
            priority,
            claude_models,
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<providers::ProviderSummary, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("provider_set_enabled", move || {
        providers::set_enabled(&app, provider_id, enabled)
    })
    .await
}

#[tauri::command]
pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("provider_delete", move || {
        providers::delete(&app, provider_id)?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderSummary>, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("providers_reorder", move || {
        providers::reorder(&app, &cli_key, ordered_provider_ids)
    })
    .await
}

#[tauri::command]
pub(crate) async fn base_url_ping_ms(base_url: String) -> Result<u64, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("aio-coding-hub-ping/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("PING_HTTP_CLIENT_INIT: {e}"))?;
    base_url_probe::probe_base_url_ms(&client, &base_url, std::time::Duration::from_secs(3)).await
}
