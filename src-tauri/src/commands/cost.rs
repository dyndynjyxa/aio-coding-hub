//! Usage: Cost analytics related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, cost_stats};

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_summary_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
) -> Result<cost_stats::CostSummaryV1, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("cost_summary_v1", move || {
        cost_stats::summary_v1(&db, &params)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_trend_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
) -> Result<Vec<cost_stats::CostTrendRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("cost_trend_v1", move || cost_stats::trend_v1(&db, &params))
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_breakdown_provider_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
    limit: Option<u32>,
) -> Result<Vec<cost_stats::CostProviderBreakdownRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    blocking::run("cost_breakdown_provider_v1", move || {
        cost_stats::breakdown_provider_v1(&db, &params, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_breakdown_model_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
    limit: Option<u32>,
) -> Result<Vec<cost_stats::CostModelBreakdownRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    blocking::run("cost_breakdown_model_v1", move || {
        cost_stats::breakdown_model_v1(&db, &params, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_scatter_cli_provider_model_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
    limit: Option<u32>,
) -> Result<Vec<cost_stats::CostScatterCliProviderModelRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(500).clamp(1, 5000) as usize;
    blocking::run("cost_scatter_cli_provider_model_v1", move || {
        cost_stats::scatter_cli_provider_model_v1(&db, &params, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_top_requests_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
    limit: Option<u32>,
) -> Result<Vec<cost_stats::CostTopRequestRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    blocking::run("cost_top_requests_v1", move || {
        cost_stats::top_requests_v1(&db, &params, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cost_backfill_missing_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: cost_stats::CostQueryParams,
    max_rows: Option<u32>,
) -> Result<cost_stats::CostBackfillReportV1, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let max_rows = max_rows.unwrap_or(5000).clamp(1, 10_000) as usize;
    blocking::run("cost_backfill_missing_v1", move || {
        cost_stats::backfill_missing_v1(&db, &params, max_rows)
    })
    .await
    .map_err(Into::into)
}
