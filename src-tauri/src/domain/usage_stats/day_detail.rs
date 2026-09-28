use crate::db;
use crate::shared::error::db_err;
use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection};
use std::collections::HashMap;

use super::folders::{
    filter_rows_by_folder_keys, folder_identity_for_row, resolved_folder_map, session_lookup_keys,
    usage_event_rows, UsageEventAgg,
};
use super::input::{normalize_day, normalize_day_start_hour, normalize_provider_id_filter};
use super::{
    normalize_cli_filter, normalize_folder_keys, ProviderAgg, UsageDayDetailParams,
    UsageDayDetailV1, UsageDayFolderRow, UsageDayHourRow,
};

pub use super::folders::{
    UsageResolvedFolder as UsageDayResolvedFolder,
    UsageSessionLookupKey as UsageDaySessionLookupKey,
};

fn local_start_ts(conn: &Connection, day: &str, day_start_hour: i64) -> Result<i64, String> {
    let time = format!("{day_start_hour:02}:00:00");
    let ts = conn
        .query_row(
            "SELECT CAST(strftime('%s', ?1 || ' ' || ?2, 'utc') AS INTEGER)",
            params![day, time],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(|e| db_err!("failed to compute local day start ts: {e}"))?
        .ok_or_else(|| format!("SEC_INVALID_INPUT: invalid day={day}"))?;
    Ok(ts)
}

fn local_day_bounds(
    conn: &Connection,
    day: &str,
    day_start_hour: i64,
) -> Result<(String, i64, i64), String> {
    let normalized = normalize_day(day)?;
    let date = NaiveDate::parse_from_str(&normalized, "%Y-%m-%d")
        .map_err(|_| format!("SEC_INVALID_INPUT: invalid day={normalized}"))?;
    let next_day = date
        .checked_add_signed(Duration::days(1))
        .ok_or_else(|| "SEC_INVALID_INPUT: day out of range".to_string())?
        .format("%Y-%m-%d")
        .to_string();
    let start_ts = local_start_ts(conn, &normalized, day_start_hour)?;
    let end_ts = local_start_ts(conn, &next_day, day_start_hour)?;
    if start_ts >= end_ts {
        return Err("SEC_INVALID_INPUT: invalid local day bounds".to_string());
    }
    Ok((normalized, start_ts, end_ts))
}

fn folder_rows(
    rows: &[UsageEventAgg],
    resolved: &HashMap<String, UsageDayResolvedFolder>,
    folder_limit: Option<usize>,
) -> Vec<UsageDayFolderRow> {
    let mut by_folder: HashMap<String, (String, String, Option<String>, ProviderAgg)> =
        HashMap::new();
    for row in rows {
        let identity = folder_identity_for_row(row, resolved);
        let entry = by_folder.entry(identity.key.clone()).or_insert_with(|| {
            (
                identity.key.clone(),
                identity.name.clone(),
                identity.folder_path.clone(),
                ProviderAgg::default(),
            )
        });
        entry.3.merge(row.agg.clone());
    }

    let mut out: Vec<UsageDayFolderRow> = by_folder
        .into_values()
        .map(|(key, name, folder_path, agg)| {
            let base = agg.into_leaderboard_row(key, name);
            UsageDayFolderRow {
                key: base.key,
                name: base.name,
                folder_path,
                requests_total: base.requests_total,
                requests_success: base.requests_success,
                requests_failed: base.requests_failed,
                total_tokens: base.total_tokens,
                io_total_tokens: base.io_total_tokens,
                input_tokens: base.input_tokens,
                output_tokens: base.output_tokens,
                cache_creation_input_tokens: base.cache_creation_input_tokens,
                cache_read_input_tokens: base.cache_read_input_tokens,
                avg_duration_ms: base.avg_duration_ms,
                avg_ttfb_ms: base.avg_ttfb_ms,
                avg_output_tokens_per_second: base.avg_output_tokens_per_second,
                cost_usd: base.cost_usd,
            }
        })
        .collect();

    out.sort_by(|a, b| {
        b.total_tokens
            .cmp(&a.total_tokens)
            .then_with(|| b.requests_total.cmp(&a.requests_total))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.key.cmp(&b.key))
    });

    if let Some(limit) = folder_limit {
        out.truncate(limit.clamp(1, 50));
    }
    out
}

fn hour_rows(rows: &[UsageEventAgg]) -> Vec<UsageDayHourRow> {
    let mut hours: Vec<UsageDayHourRow> = (0..24)
        .map(|hour| UsageDayHourRow {
            hour,
            requests_total: 0,
            total_tokens: 0,
            io_total_tokens: 0,
        })
        .collect();

    for row in rows {
        let Some(hour) = row.hour else {
            continue;
        };
        if !(0..24).contains(&hour) {
            continue;
        }
        let bucket = &mut hours[hour as usize];
        bucket.requests_total = bucket.requests_total.saturating_add(row.agg.requests_total);
        bucket.total_tokens = bucket.total_tokens.saturating_add(row.agg.total_tokens);
        bucket.io_total_tokens = bucket
            .io_total_tokens
            .saturating_add(row.agg.input_tokens.saturating_add(row.agg.output_tokens));
    }

    hours
}

pub(super) fn day_detail_v1_with_conn<F>(
    conn: &Connection,
    params: &UsageDayDetailParams,
    folder_lookup: F,
) -> Result<UsageDayDetailV1, String>
where
    F: FnOnce(&[UsageDaySessionLookupKey]) -> Vec<UsageDayResolvedFolder>,
{
    let day_start_hour = normalize_day_start_hour(params.day_start_hour)?;
    let (day, start_ts, end_ts) = local_day_bounds(conn, &params.day, day_start_hour)?;
    let cli_key = normalize_cli_filter(params.cli_key.as_deref())?;
    let provider_id = normalize_provider_id_filter(params.provider_id)?;
    let folder_limit = params.folder_limit.map(|value| value.clamp(1, 50) as usize);
    let folder_keys = normalize_folder_keys(params.folder_keys.as_deref())?;
    let exclude_cx2cc_gateway_bridge = params.exclude_cx2cc_gateway_bridge.unwrap_or(false);

    let rows = usage_event_rows(
        conn,
        Some(start_ts),
        Some(end_ts),
        cli_key,
        provider_id,
        None,
        true,
        exclude_cx2cc_gateway_bridge,
    )?;
    let lookup_keys = session_lookup_keys(&rows);
    let resolved = resolved_folder_map(folder_lookup(&lookup_keys));
    let rows = filter_rows_by_folder_keys(rows, &resolved, folder_keys.as_deref());
    let folders = folder_rows(&rows, &resolved, folder_limit);
    let hours = hour_rows(&rows);

    Ok(UsageDayDetailV1 {
        day,
        folders,
        hours,
    })
}

pub fn day_detail_v1<F>(
    db: &db::Db,
    params: &UsageDayDetailParams,
    folder_lookup: F,
) -> crate::shared::error::AppResult<UsageDayDetailV1>
where
    F: FnOnce(&[UsageDaySessionLookupKey]) -> Vec<UsageDayResolvedFolder>,
{
    let conn = db.open_connection()?;
    Ok(day_detail_v1_with_conn(&conn, params, folder_lookup)?)
}
