//! Usage: Usage statistics related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, cli_sessions, usage_stats};

fn usage_folder_lookup(
    app: &tauri::AppHandle,
    items: &[usage_stats::UsageSessionLookupKey],
) -> Vec<usage_stats::UsageResolvedFolder> {
    let lookup_items: Vec<cli_sessions::CliSessionsFolderLookupKey> = items
        .iter()
        .filter_map(|item| {
            let source = item
                .cli_key
                .parse::<cli_sessions::CliSessionsSource>()
                .ok()?;
            Some(cli_sessions::CliSessionsFolderLookupKey {
                source,
                session_id: item.session_id.clone(),
            })
        })
        .collect();

    cli_sessions::folder_lookup_by_ids(app, &lookup_items, None)
        .unwrap_or_default()
        .into_iter()
        .map(|item| usage_stats::UsageResolvedFolder {
            cli_key: item.source.as_str().to_string(),
            session_id: item.session_id,
            folder_name: item.folder_name,
            folder_path: item.folder_path,
        })
        .collect()
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_summary(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    range: String,
    cli_key: Option<String>,
) -> Result<usage_stats::UsageSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("usage_summary", move || {
        usage_stats::summary(&db, &range, cli_key.as_deref())
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_summary_v2(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: usage_stats::UsageQueryParams,
) -> Result<usage_stats::UsageSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("usage_summary_v2", move || {
        usage_stats::summary_v2(&db, &params, |items| usage_folder_lookup(&app, items))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_leaderboard_provider(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    range: String,
    cli_key: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<usage_stats::UsageProviderRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(10).clamp(1, 50) as usize;
    blocking::run("usage_leaderboard_provider", move || {
        usage_stats::leaderboard_provider(&db, &range, cli_key.as_deref(), limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_leaderboard_day(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    range: String,
    cli_key: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<usage_stats::UsageDayRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(10).clamp(1, 50) as usize;
    blocking::run("usage_leaderboard_day", move || {
        usage_stats::leaderboard_day(&db, &range, cli_key.as_deref(), limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_leaderboard_v2(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    scope: String,
    params: usage_stats::UsageQueryParams,
    limit: Option<u32>,
) -> Result<Vec<usage_stats::UsageLeaderboardRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let limit = limit.map(|value| value.clamp(1, 200) as usize);
    blocking::run("usage_leaderboard_v2", move || {
        usage_stats::leaderboard_v2(&db, &scope, &params, limit, |items| {
            usage_folder_lookup(&app, items)
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_hourly_series(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    days: u32,
) -> Result<Vec<usage_stats::UsageHourlyRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let days = days.clamp(1, 60);
    blocking::run("usage_hourly_series", move || {
        usage_stats::hourly_series(&db, days)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_day_detail_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: usage_stats::UsageDayDetailParams,
) -> Result<usage_stats::UsageDayDetailV1, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("usage_day_detail_v1", move || {
        usage_stats::day_detail_v1(&db, &params, |items| usage_folder_lookup(&app, items))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_folder_options_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: usage_stats::UsageQueryParams,
) -> Result<Vec<usage_stats::UsageFolderOptionV1>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("usage_folder_options_v1", move || {
        usage_stats::folder_options_v1(&db, &params, |items| usage_folder_lookup(&app, items))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn usage_provider_cache_rate_trend_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    params: usage_stats::UsageQueryParams,
    limit: Option<u32>,
) -> Result<Vec<usage_stats::UsageProviderCacheRateTrendRowV1>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.map(|v| v as usize);

    blocking::run("usage_provider_cache_rate_trend_v1", move || {
        usage_stats::provider_cache_rate_trend_v1(&db, &params, limit)
    })
    .await
    .map_err(Into::into)
}
