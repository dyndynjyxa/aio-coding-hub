//! Usage: Usage statistics related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, cli_sessions, usage_stats};

const USAGE_LEADERBOARD_CSV_EXPORT_MAX_BYTES: usize = 1024 * 1024;

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

fn normalize_usage_leaderboard_csv_export_input(
    file_path: String,
    csv: String,
) -> Result<(String, String), String> {
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("SEC_INVALID_INPUT: file_path is required".to_string());
    }

    if csv.trim_matches('\u{feff}').trim().is_empty() {
        return Err("SEC_INVALID_INPUT: csv is required".to_string());
    }

    let byte_len = csv.len();
    if byte_len > USAGE_LEADERBOARD_CSV_EXPORT_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: csv is too large (max {} bytes)",
            USAGE_LEADERBOARD_CSV_EXPORT_MAX_BYTES
        ));
    }

    Ok((file_path, csv))
}

fn write_usage_leaderboard_csv_export(file_path: String, csv: String) -> Result<bool, String> {
    let (file_path, csv) = normalize_usage_leaderboard_csv_export_input(file_path, csv)?;
    std::fs::write(&file_path, csv).map_err(|err| {
        format!("SYSTEM_ERROR: failed to write usage leaderboard csv file: {err}")
    })?;
    Ok(true)
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
pub(crate) async fn usage_leaderboard_csv_export(
    file_path: String,
    csv: String,
) -> Result<bool, String> {
    blocking::run("usage_leaderboard_csv_export", move || {
        write_usage_leaderboard_csv_export(file_path, csv)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_leaderboard_csv_export_writes_valid_csv() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("usage.csv");
        let content = "\u{feff}排名,供应商\r\n1,OpenAI\r\n".to_string();

        let ok =
            write_usage_leaderboard_csv_export(path.to_string_lossy().to_string(), content.clone())
                .expect("csv export should succeed");

        assert!(ok);
        assert_eq!(std::fs::read_to_string(path).expect("read csv"), content);
    }

    #[test]
    fn usage_leaderboard_csv_export_rejects_empty_path() {
        let err = write_usage_leaderboard_csv_export("   ".to_string(), "排名\r\n".to_string())
            .expect_err("empty path should fail");

        assert!(err.contains("SEC_INVALID_INPUT: file_path is required"));
    }

    #[test]
    fn usage_leaderboard_csv_export_rejects_empty_csv() {
        let err = write_usage_leaderboard_csv_export(
            "/tmp/usage.csv".to_string(),
            "\u{feff}  ".to_string(),
        )
        .expect_err("empty csv should fail");

        assert!(err.contains("SEC_INVALID_INPUT: csv is required"));
    }

    #[test]
    fn usage_leaderboard_csv_export_rejects_oversized_csv() {
        let err = write_usage_leaderboard_csv_export(
            "/tmp/usage.csv".to_string(),
            "x".repeat(USAGE_LEADERBOARD_CSV_EXPORT_MAX_BYTES + 1),
        )
        .expect_err("oversized csv should fail");

        assert!(err.contains("SEC_INVALID_INPUT: csv is too large"));
    }
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
