//! Usage: Model pricing related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, model_price_aliases, model_prices, model_prices_sync};

#[tauri::command]
pub(crate) async fn model_prices_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<model_prices::ModelPriceSummary>, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("model_prices_list", move || {
        model_prices::list_by_cli(&app, &cli_key)
    })
    .await
}

#[tauri::command]
pub(crate) async fn model_price_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    model: String,
    price_json: String,
) -> Result<model_prices::ModelPriceSummary, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("model_price_upsert", move || {
        model_prices::upsert(&app, &cli_key, &model, &price_json)
    })
    .await
}

#[tauri::command]
pub(crate) async fn model_prices_sync_basellm(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    force: Option<bool>,
) -> Result<model_prices_sync::ModelPricesSyncReport, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    model_prices_sync::sync_basellm(&app, force.unwrap_or(false)).await
}

#[tauri::command]
pub(crate) async fn model_price_aliases_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<model_price_aliases::ModelPriceAliasesV1, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("model_price_aliases_get", move || {
        Ok(model_price_aliases::read_fail_open(&app))
    })
    .await
}

#[tauri::command]
pub(crate) async fn model_price_aliases_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    aliases: model_price_aliases::ModelPriceAliasesV1,
) -> Result<model_price_aliases::ModelPriceAliasesV1, String> {
    ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("model_price_aliases_set", move || {
        model_price_aliases::write(&app, aliases)
    })
    .await
}
