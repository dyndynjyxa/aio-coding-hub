use crate::db;
use crate::shared::error::db_err;
use chrono::{Duration, NaiveDate};
use rusqlite::{params_from_iter, Connection, OptionalExtension, Row};
use std::collections::{HashMap, HashSet};

use super::filters::{
    build_optional_range_cli_provider_filters, build_optional_range_filters_with_offset,
    sql_exclude_cx2cc_gateway_bridge_clause, SqlValues,
};
use super::folders::{
    filter_rows_by_folder_keys, folder_identity_for_row, folder_identity_for_session,
    resolved_folder_map, session_lookup_keys, usage_event_rows, UsageEventAgg,
};
use super::{
    effective_total_from_buckets, extract_final_provider, has_valid_provider_key, parse_scope_v2,
    resolve_query_params, sql_effective_input_tokens_expr,
    sql_effective_input_tokens_expr_with_alias, DevelopmentTimeGapThresholds, ProviderAgg,
    ProviderKey, UsageLeaderboardRow, UsageQueryParams, UsageResolvedFolder, UsageScopeV2,
    UsageSessionLookupKey,
};

const MAX_LEADERBOARD_ROWS: usize = 200;

fn effective_leaderboard_limit(limit: Option<usize>) -> usize {
    limit.map_or(MAX_LEADERBOARD_ROWS, |limit| {
        limit.clamp(1, MAX_LEADERBOARD_ROWS)
    })
}

fn aggregated_total_tokens(row: &Row<'_>) -> rusqlite::Result<i64> {
    Ok(effective_total_from_buckets(
        row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
        row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
        row.get::<_, Option<i64>>("cache_creation_input_tokens")?
            .unwrap_or(0),
        row.get::<_, Option<i64>>("cache_read_input_tokens")?
            .unwrap_or(0),
    ))
}

fn local_day_bucket_sql(timestamp_expr: &str, day_start_hour: i64) -> String {
    if day_start_hour == 0 {
        return format!("strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime')");
    }
    format!(
        "strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime', '-{day_start_hour} hours')"
    )
}

fn local_day_key_for_timestamp(
    conn: &Connection,
    timestamp: i64,
    day_start_hour: i64,
) -> Result<Option<String>, String> {
    let sql = format!("SELECT {}", local_day_bucket_sql("?1", day_start_hour));
    let mut stmt = conn
        .prepare_cached(&sql)
        .map_err(|e| db_err!("failed to prepare usage day key query: {e}"))?;
    let key = stmt
        .query_row([timestamp], |row| row.get(0))
        .map_err(|e| db_err!("failed to resolve usage day key: {e}"))?;
    Ok(key)
}

fn expected_day_keys(
    conn: &Connection,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    day_start_hour: i64,
) -> Result<Vec<String>, String> {
    let Some(start_ts) = start_ts else {
        return Ok(Vec::new());
    };
    let end_ts = match end_ts {
        Some(end_ts) => end_ts.saturating_sub(1),
        None => conn
            .query_row("SELECT CAST(strftime('%s', 'now') AS INTEGER)", [], |row| {
                row.get(0)
            })
            .map_err(|e| db_err!("failed to resolve current usage timestamp: {e}"))?,
    };
    let Some(start_key) = local_day_key_for_timestamp(conn, start_ts, day_start_hour)? else {
        return Ok(Vec::new());
    };
    let Some(end_key) = local_day_key_for_timestamp(conn, end_ts, day_start_hour)? else {
        return Ok(Vec::new());
    };
    let Ok(mut current) = NaiveDate::parse_from_str(&start_key, "%Y-%m-%d") else {
        return Ok(Vec::new());
    };
    let Ok(end) = NaiveDate::parse_from_str(&end_key, "%Y-%m-%d") else {
        return Ok(Vec::new());
    };
    if current > end {
        return Ok(Vec::new());
    }
    if let Some(latest_start) =
        end.checked_sub_signed(Duration::days((MAX_LEADERBOARD_ROWS - 1) as i64))
    {
        current = current.max(latest_start);
    }

    let mut keys = Vec::new();
    while current <= end {
        keys.push(current.format("%Y-%m-%d").to_string());
        let Some(next) = current.succ_opt() else {
            break;
        };
        current = next;
    }
    Ok(keys)
}

fn fill_missing_day_rows(
    conn: &Connection,
    rows: &mut Vec<UsageLeaderboardRow>,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    day_start_hour: i64,
) -> Result<(), String> {
    let mut existing_keys: HashSet<String> = rows.iter().map(|row| row.key.clone()).collect();
    for key in expected_day_keys(conn, start_ts, end_ts, day_start_hour)? {
        if !existing_keys.insert(key.clone()) {
            continue;
        }
        let mut row = ProviderAgg::default().into_leaderboard_row(key.clone(), key);
        row.estimated_development_time_ms = Some(0);
        rows.push(row);
    }
    Ok(())
}

const MAX_ESTIMATED_DEVELOPMENT_TIME_MS: i64 = 24 * 60 * 60 * 1000;
const HOURS_PER_DAY: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UsageActivityRow {
    cli_key: String,
    session_id: Option<String>,
    day_key: String,
    start_ms: i64,
    duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DayActivityMetrics {
    last_request_completed_at_ms: i64,
    estimated_development_time_ms: i64,
    hourly_estimated_development_time_ms: Option<Vec<i64>>,
}

fn weighted_idle_gap_ms(gap_ms: i64, thresholds: DevelopmentTimeGapThresholds) -> i64 {
    if gap_ms <= thresholds.full_idle_gap_ms {
        return gap_ms.max(0);
    }
    if gap_ms > thresholds.session_break_gap_ms {
        return 0;
    }
    let remaining_weight = thresholds.session_break_gap_ms - gap_ms;
    ((gap_ms as i128 * remaining_weight as i128)
        / (thresholds.session_break_gap_ms - thresholds.full_idle_gap_ms) as i128) as i64
}

fn merged_activity_intervals(rows: &[UsageActivityRow]) -> Vec<(i64, i64)> {
    let mut intervals: Vec<(i64, i64)> = rows
        .iter()
        .filter_map(|row| {
            let duration_ms = row.duration_ms.max(0);
            (duration_ms > 0).then(|| (row.start_ms, row.start_ms.saturating_add(duration_ms)))
        })
        .collect();
    intervals.sort_unstable_by_key(|(start_ms, end_ms)| (*start_ms, *end_ms));

    let mut merged: Vec<(i64, i64)> = Vec::with_capacity(intervals.len());
    for (start_ms, end_ms) in intervals {
        if let Some((_, current_end_ms)) = merged.last_mut() {
            if start_ms <= *current_end_ms {
                *current_end_ms = (*current_end_ms).max(end_ms);
                continue;
            }
        }
        merged.push((start_ms, end_ms));
    }

    merged
}

fn summarize_day_activity(
    rows: &[UsageActivityRow],
    thresholds: DevelopmentTimeGapThresholds,
) -> Option<DayActivityMetrics> {
    summarize_merged_day_activity(rows, &merged_activity_intervals(rows), thresholds)
}

fn summarize_merged_day_activity(
    rows: &[UsageActivityRow],
    merged: &[(i64, i64)],
    thresholds: DevelopmentTimeGapThresholds,
) -> Option<DayActivityMetrics> {
    let last_request_completed_at_ms = rows
        .iter()
        .map(|row| row.start_ms.saturating_add(row.duration_ms.max(0)))
        .max()?;
    let mut estimated_ms: i128 = 0;
    let mut previous_end_ms: Option<i64> = None;
    for &(start_ms, end_ms) in merged {
        if let Some(previous_end_ms) = previous_end_ms {
            estimated_ms +=
                weighted_idle_gap_ms(start_ms.saturating_sub(previous_end_ms), thresholds) as i128;
        }
        estimated_ms += end_ms.saturating_sub(start_ms) as i128;
        previous_end_ms = Some(end_ms);
    }

    Some(DayActivityMetrics {
        last_request_completed_at_ms,
        estimated_development_time_ms: estimated_ms
            .clamp(0, MAX_ESTIMATED_DEVELOPMENT_TIME_MS as i128)
            as i64,
        hourly_estimated_development_time_ms: None,
    })
}

fn local_hour_index(conn: &Connection, timestamp_ms: i64) -> Result<usize, String> {
    let timestamp_seconds = timestamp_ms.div_euclid(1_000);
    let mut stmt = conn
        .prepare_cached("SELECT CAST(strftime('%H', ?1, 'unixepoch', 'localtime') AS INTEGER)")
        .map_err(|e| db_err!("failed to prepare local activity hour query: {e}"))?;
    let hour = stmt
        .query_row([timestamp_seconds], |row| row.get::<_, i64>(0))
        .map_err(|e| db_err!("failed to resolve local activity hour: {e}"))?;
    Ok(hour.clamp(0, (HOURS_PER_DAY - 1) as i64) as usize)
}

fn next_local_hour_boundary_ms(conn: &Connection, timestamp_ms: i64) -> Result<i64, String> {
    let timestamp_seconds = timestamp_ms.div_euclid(1_000);
    let mut stmt = conn
        .prepare_cached(
            "SELECT CAST(strftime('%s', strftime('%Y-%m-%d %H:00:00', ?1, 'unixepoch', 'localtime'), '+1 hour', 'utc') AS INTEGER)",
        )
        .map_err(|e| db_err!("failed to prepare next local activity hour query: {e}"))?;
    let boundary_seconds = stmt
        .query_row([timestamp_seconds], |row| row.get::<_, i64>(0))
        .map_err(|e| db_err!("failed to resolve next local activity hour: {e}"))?;
    Ok(boundary_seconds.saturating_mul(1_000))
}

fn distribute_estimated_time_by_hour(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    estimated_ms: i64,
    hourly: &mut [i64],
) -> Result<(), String> {
    if estimated_ms <= 0 || end_ms <= start_ms {
        return Ok(());
    }
    let mut current_ms = start_ms;
    let mut remaining_span_ms = end_ms.saturating_sub(start_ms) as i128;
    let mut remaining_estimated_ms = estimated_ms as i128;
    while current_ms < end_ms && remaining_estimated_ms > 0 {
        let boundary_ms = next_local_hour_boundary_ms(conn, current_ms)?;
        let next_ms = if boundary_ms <= current_ms {
            end_ms
        } else {
            boundary_ms.min(end_ms)
        };
        let span_ms = next_ms.saturating_sub(current_ms) as i128;
        if span_ms <= 0 {
            break;
        }
        let estimated_part_ms = if next_ms == end_ms {
            remaining_estimated_ms
        } else {
            remaining_estimated_ms * span_ms / remaining_span_ms
        };
        let hour = local_hour_index(conn, current_ms)?;
        hourly[hour] = hourly[hour].saturating_add(estimated_part_ms as i64);
        remaining_estimated_ms = remaining_estimated_ms.saturating_sub(estimated_part_ms);
        remaining_span_ms = remaining_span_ms.saturating_sub(span_ms);
        current_ms = next_ms;
    }
    Ok(())
}

fn summarize_day_activity_with_hours(
    conn: &Connection,
    rows: &[UsageActivityRow],
    thresholds: DevelopmentTimeGapThresholds,
) -> Result<Option<DayActivityMetrics>, String> {
    let merged = merged_activity_intervals(rows);
    let Some(mut metrics) = summarize_merged_day_activity(rows, &merged, thresholds) else {
        return Ok(None);
    };
    if merged.is_empty() {
        return Ok(Some(metrics));
    }
    let mut hourly = vec![0_i64; HOURS_PER_DAY];
    let mut allocated_ms = 0_i64;
    let mut previous_end_ms: Option<i64> = None;
    for (start_ms, end_ms) in merged {
        if let Some(previous_end_ms) = previous_end_ms {
            let weighted_gap_ms =
                weighted_idle_gap_ms(start_ms.saturating_sub(previous_end_ms), thresholds);
            let allowed_gap_ms =
                weighted_gap_ms.min(MAX_ESTIMATED_DEVELOPMENT_TIME_MS.saturating_sub(allocated_ms));
            distribute_estimated_time_by_hour(
                conn,
                previous_end_ms,
                start_ms,
                allowed_gap_ms,
                &mut hourly,
            )?;
            allocated_ms = allocated_ms.saturating_add(allowed_gap_ms);
        }
        let allowed_duration_ms = end_ms
            .saturating_sub(start_ms)
            .min(MAX_ESTIMATED_DEVELOPMENT_TIME_MS.saturating_sub(allocated_ms));
        distribute_estimated_time_by_hour(
            conn,
            start_ms,
            end_ms,
            allowed_duration_ms,
            &mut hourly,
        )?;
        allocated_ms = allocated_ms.saturating_add(allowed_duration_ms);
        previous_end_ms = Some(end_ms);
    }
    metrics.hourly_estimated_development_time_ms = Some(hourly);
    Ok(Some(metrics))
}

fn summarize_activity_by_day(
    conn: &Connection,
    rows: impl IntoIterator<Item = UsageActivityRow>,
    thresholds: DevelopmentTimeGapThresholds,
) -> Result<HashMap<String, DayActivityMetrics>, String> {
    let mut by_day: HashMap<String, Vec<UsageActivityRow>> = HashMap::new();
    for row in rows {
        by_day.entry(row.day_key.clone()).or_default().push(row);
    }
    let mut metrics_by_day = HashMap::new();
    for (day_key, rows) in by_day {
        if let Some(metrics) = summarize_day_activity_with_hours(conn, &rows, thresholds)? {
            metrics_by_day.insert(day_key, metrics);
        }
    }
    Ok(metrics_by_day)
}

fn summarize_activity_by_folder(
    rows: impl IntoIterator<Item = UsageActivityRow>,
    resolved: &HashMap<String, UsageResolvedFolder>,
    thresholds: DevelopmentTimeGapThresholds,
) -> HashMap<String, i64> {
    let mut by_folder_day: HashMap<(String, String), Vec<UsageActivityRow>> = HashMap::new();
    for row in rows {
        let folder = folder_identity_for_session(&row.cli_key, row.session_id.as_deref(), resolved);
        by_folder_day
            .entry((folder.key, row.day_key.clone()))
            .or_default()
            .push(row);
    }

    let mut by_folder = HashMap::new();
    for ((folder_key, _), rows) in by_folder_day {
        let Some(metrics) = summarize_day_activity(&rows, thresholds) else {
            continue;
        };
        let total = by_folder.entry(folder_key).or_insert(0_i64);
        *total = total.saturating_add(metrics.estimated_development_time_ms);
    }
    by_folder
}

#[allow(clippy::too_many_arguments)]
fn day_activity_rows_with_conn(
    conn: &Connection,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    exclude_cx2cc_gateway_bridge: bool,
    day_start_hour: i64,
    min_day_key: Option<&str>,
) -> Result<Vec<UsageActivityRow>, String> {
    let day_bucket_sql = local_day_bucket_sql("r.created_at", day_start_hour);
    let (where_clause, mut where_params) = build_optional_range_cli_provider_filters(
        "r.created_at",
        "r.cli_key",
        "r.final_provider_id",
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    );
    // Day keys are %Y-%m-%d, so lexicographic >= matches chronological >=.
    let min_day_key_clause = match min_day_key {
        Some(min_day_key) => {
            where_params.push(min_day_key.to_string().into());
            format!("\nAND {day_bucket_sql} >= ?{}", where_params.len())
        }
        None => String::new(),
    };
    let cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), exclude_cx2cc_gateway_bridge);
    let sql = format!(
        r#"
SELECT
  r.cli_key,
  NULLIF(TRIM(COALESCE(r.session_id, '')), '') AS session_id,
  {day_bucket_sql} AS day_key,
  r.created_at_ms,
  r.created_at,
  COALESCE(r.duration_ms, 0) AS duration_ms
FROM request_logs r
WHERE r.excluded_from_stats = 0
{where_clause}
{min_day_key_clause}
{cx2cc_filter_clause}
"#,
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare day activity query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(where_params), |row| {
            let created_at_ms = row.get::<_, Option<i64>>("created_at_ms")?.unwrap_or(0);
            let created_at = row.get::<_, i64>("created_at")?;
            Ok(UsageActivityRow {
                cli_key: row.get("cli_key")?,
                session_id: row.get("session_id")?,
                day_key: row.get("day_key")?,
                start_ms: if created_at_ms > 0 {
                    created_at_ms
                } else {
                    created_at.saturating_mul(1000)
                },
                duration_ms: row.get("duration_ms")?,
            })
        })
        .map_err(|e| db_err!("failed to run day activity query: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read day activity row: {e}"))?);
    }
    Ok(out)
}

fn apply_day_activity_metrics(
    rows: &mut [UsageLeaderboardRow],
    metrics_by_day: HashMap<String, DayActivityMetrics>,
) {
    for row in rows {
        let Some(metrics) = metrics_by_day.get(&row.key) else {
            continue;
        };
        row.last_request_completed_at_ms = Some(metrics.last_request_completed_at_ms);
        row.estimated_development_time_ms = Some(metrics.estimated_development_time_ms);
        row.hourly_estimated_development_time_ms =
            metrics.hourly_estimated_development_time_ms.clone();
    }
}

fn apply_folder_activity_metrics(
    rows: &mut [UsageLeaderboardRow],
    metrics_by_folder: HashMap<String, i64>,
) {
    for row in rows {
        let Some(estimated_ms) = metrics_by_folder.get(&row.key) else {
            continue;
        };
        row.estimated_development_time_ms = Some(*estimated_ms);
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn leaderboard_v2_with_conn(
    conn: &Connection,
    scope: UsageScopeV2,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    limit: Option<usize>,
    exclude_cx2cc_gateway_bridge: bool,
) -> Result<Vec<UsageLeaderboardRow>, String> {
    leaderboard_v2_with_conn_day_start(
        conn,
        scope,
        start_ts,
        end_ts,
        cli_key,
        provider_id,
        limit,
        exclude_cx2cc_gateway_bridge,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn leaderboard_v2_with_conn_day_start(
    conn: &Connection,
    scope: UsageScopeV2,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    limit: Option<usize>,
    exclude_cx2cc_gateway_bridge: bool,
    day_start_hour: i64,
) -> Result<Vec<UsageLeaderboardRow>, String> {
    leaderboard_v2_with_conn_day_start_and_thresholds(
        conn,
        scope,
        start_ts,
        end_ts,
        cli_key,
        provider_id,
        limit,
        exclude_cx2cc_gateway_bridge,
        day_start_hour,
        DevelopmentTimeGapThresholds::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn leaderboard_v2_with_conn_day_start_and_thresholds(
    conn: &Connection,
    scope: UsageScopeV2,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    limit: Option<usize>,
    exclude_cx2cc_gateway_bridge: bool,
    day_start_hour: i64,
    development_time_gap_thresholds: DevelopmentTimeGapThresholds,
) -> Result<Vec<UsageLeaderboardRow>, String> {
    let effective_input_expr = sql_effective_input_tokens_expr();
    let day_bucket_sql = local_day_bucket_sql("created_at", day_start_hour);
    let (where_clause, where_params) = build_optional_range_cli_provider_filters(
        "created_at",
        "cli_key",
        "final_provider_id",
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    );
    let (provider_where_clause, provider_where_params) = build_optional_range_cli_provider_filters(
        "r.created_at",
        "r.cli_key",
        "r.final_provider_id",
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    );
    let (provider_fallback_where_clause, provider_fallback_range_params) =
        build_optional_range_filters_with_offset("r.created_at", start_ts, end_ts, 2);
    let cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(None, exclude_cx2cc_gateway_bridge);
    let provider_cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), exclude_cx2cc_gateway_bridge);

    let mut out: Vec<UsageLeaderboardRow> = match scope {
        UsageScopeV2::Folder => {
            return Err("folder leaderboard requires folder metadata lookup".to_string());
        }
        UsageScopeV2::Cli => {
            let sql = format!(
                r#"
SELECT
  cli_key AS key,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN 1 ELSE 0 END) AS requests_success,
  SUM(
    CASE WHEN (
      status IS NULL OR
      status < 200 OR
      status >= 300 OR
      error_code IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_failed,
		  SUM({effective_input_expr}) AS input_tokens,
	  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
	  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
	  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
	      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
	    ) THEN 1 ELSE 0 END
	  ) AS cost_covered_success,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
	      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
	    ) THEN cost_usd_femto ELSE 0 END
	  ) AS total_cost_usd_femto,
	  SUM(duration_ms) AS total_duration_ms,
	  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN duration_ms ELSE 0 END) AS success_duration_ms_sum,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN ttfb_ms ELSE 0 END
  ) AS success_ttfb_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN 1 ELSE 0 END
  ) AS success_ttfb_ms_count,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN (duration_ms - ttfb_ms) ELSE 0 END
  ) AS success_generation_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN output_tokens ELSE 0 END
  ) AS success_output_tokens_for_rate_sum
FROM request_logs
WHERE excluded_from_stats = 0
{where_clause}
{cx2cc_filter_clause}
GROUP BY cli_key
"#,
                effective_input_expr = effective_input_expr,
                where_clause = where_clause,
                cx2cc_filter_clause = cx2cc_filter_clause
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err!("failed to prepare cli leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params_from_iter(where_params.clone()), |row| {
                    let key: String = row.get("key")?;
                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
                        total_duration_ms: row
                            .get::<_, Option<i64>>("total_duration_ms")?
                            .unwrap_or(0),
                        first_request_created_at_ms: None,
                        last_request_created_at_ms: None,
                        success_duration_ms_sum: row
                            .get::<_, Option<i64>>("success_duration_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_sum: row
                            .get::<_, Option<i64>>("success_ttfb_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_count: row
                            .get::<_, Option<i64>>("success_ttfb_ms_count")?
                            .unwrap_or(0),
                        success_generation_ms_sum: row
                            .get::<_, Option<i64>>("success_generation_ms_sum")?
                            .unwrap_or(0),
                        success_output_tokens_for_rate_sum: row
                            .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
                            .unwrap_or(0),
                        total_tokens: aggregated_total_tokens(row)?,
                        input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                        output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                        cache_creation_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_input_tokens")?
                            .unwrap_or(0),
                        cache_read_input_tokens: row
                            .get::<_, Option<i64>>("cache_read_input_tokens")?
                            .unwrap_or(0),
                        cache_creation_5m_input_tokens: 0,
                        cache_creation_1h_input_tokens: 0,
                        cost_covered_success: row
                            .get::<_, Option<i64>>("cost_covered_success")?
                            .unwrap_or(0),
                        total_cost_usd_femto: row
                            .get::<_, Option<i64>>("total_cost_usd_femto")?
                            .unwrap_or(0),
                    };

                    Ok(agg.into_leaderboard_row(key.clone(), key))
                })
                .map_err(|e| db_err!("failed to run cli leaderboard query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| db_err!("failed to read cli row: {e}"))?);
            }
            items
        }
        UsageScopeV2::Model => {
            let sql = format!(
                r#"
SELECT
  COALESCE(NULLIF(requested_model, ''), 'Unknown') AS key,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN 1 ELSE 0 END) AS requests_success,
  SUM(
    CASE WHEN (
      status IS NULL OR
      status < 200 OR
      status >= 300 OR
      error_code IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_failed,
		  SUM({effective_input_expr}) AS input_tokens,
	  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
	  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
	  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
	      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
	    ) THEN 1 ELSE 0 END
	  ) AS cost_covered_success,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
	      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
	    ) THEN cost_usd_femto ELSE 0 END
	  ) AS total_cost_usd_femto,
	  SUM(duration_ms) AS total_duration_ms,
	  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN duration_ms ELSE 0 END) AS success_duration_ms_sum,
	  SUM(
	    CASE WHEN (
	      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN ttfb_ms ELSE 0 END
  ) AS success_ttfb_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN 1 ELSE 0 END
  ) AS success_ttfb_ms_count,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN (duration_ms - ttfb_ms) ELSE 0 END
  ) AS success_generation_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN output_tokens ELSE 0 END
  ) AS success_output_tokens_for_rate_sum
FROM request_logs
WHERE excluded_from_stats = 0
{where_clause}
{cx2cc_filter_clause}
GROUP BY COALESCE(NULLIF(requested_model, ''), 'Unknown')
"#,
                effective_input_expr = effective_input_expr,
                where_clause = where_clause,
                cx2cc_filter_clause = cx2cc_filter_clause
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err!("failed to prepare model leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params_from_iter(where_params.clone()), |row| {
                    let key: String = row.get("key")?;
                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
                        total_duration_ms: row
                            .get::<_, Option<i64>>("total_duration_ms")?
                            .unwrap_or(0),
                        first_request_created_at_ms: None,
                        last_request_created_at_ms: None,
                        success_duration_ms_sum: row
                            .get::<_, Option<i64>>("success_duration_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_sum: row
                            .get::<_, Option<i64>>("success_ttfb_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_count: row
                            .get::<_, Option<i64>>("success_ttfb_ms_count")?
                            .unwrap_or(0),
                        success_generation_ms_sum: row
                            .get::<_, Option<i64>>("success_generation_ms_sum")?
                            .unwrap_or(0),
                        success_output_tokens_for_rate_sum: row
                            .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
                            .unwrap_or(0),
                        total_tokens: aggregated_total_tokens(row)?,
                        input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                        output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                        cache_creation_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_input_tokens")?
                            .unwrap_or(0),
                        cache_read_input_tokens: row
                            .get::<_, Option<i64>>("cache_read_input_tokens")?
                            .unwrap_or(0),
                        cache_creation_5m_input_tokens: 0,
                        cache_creation_1h_input_tokens: 0,
                        cost_covered_success: row
                            .get::<_, Option<i64>>("cost_covered_success")?
                            .unwrap_or(0),
                        total_cost_usd_femto: row
                            .get::<_, Option<i64>>("total_cost_usd_femto")?
                            .unwrap_or(0),
                    };

                    Ok(agg.into_leaderboard_row(key.clone(), key))
                })
                .map_err(|e| db_err!("failed to run model leaderboard query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| db_err!("failed to read model row: {e}"))?);
            }
            items
        }
        UsageScopeV2::Day => {
            let sql = format!(
                r#"
SELECT
  {day_bucket_sql} AS key,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN 1 ELSE 0 END) AS requests_success,
  SUM(
    CASE WHEN (
      status IS NULL OR
      status < 200 OR
      status >= 300 OR
      error_code IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_failed,
  SUM({effective_input_expr}) AS input_tokens,
  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
    ) THEN 1 ELSE 0 END
  ) AS cost_covered_success,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      cost_usd_femto IS NOT NULL AND cost_usd_femto > 0
    ) THEN cost_usd_femto ELSE 0 END
  ) AS total_cost_usd_femto,
  SUM(duration_ms) AS total_duration_ms,
  MIN(CASE WHEN created_at_ms > 0 THEN created_at_ms ELSE created_at * 1000 END) AS first_request_created_at_ms,
  MAX(CASE WHEN created_at_ms > 0 THEN created_at_ms ELSE created_at * 1000 END) AS last_request_created_at_ms,
  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN duration_ms ELSE 0 END) AS success_duration_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN ttfb_ms ELSE 0 END
  ) AS success_ttfb_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN 1 ELSE 0 END
  ) AS success_ttfb_ms_count,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN (duration_ms - ttfb_ms) ELSE 0 END
  ) AS success_generation_ms_sum,
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      output_tokens IS NOT NULL AND
      ttfb_ms IS NOT NULL AND
      ttfb_ms < duration_ms
    ) THEN output_tokens ELSE 0 END
  ) AS success_output_tokens_for_rate_sum
FROM request_logs
WHERE excluded_from_stats = 0
{where_clause}
{cx2cc_filter_clause}
GROUP BY key
"#,
                effective_input_expr = effective_input_expr,
                where_clause = where_clause,
                cx2cc_filter_clause = cx2cc_filter_clause,
                day_bucket_sql = day_bucket_sql
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err!("failed to prepare day leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params_from_iter(where_params.clone()), |row| {
                    let key: String = row.get("key")?;
                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
                        total_duration_ms: row
                            .get::<_, Option<i64>>("total_duration_ms")?
                            .unwrap_or(0),
                        first_request_created_at_ms: row.get("first_request_created_at_ms")?,
                        last_request_created_at_ms: row.get("last_request_created_at_ms")?,
                        success_duration_ms_sum: row
                            .get::<_, Option<i64>>("success_duration_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_sum: row
                            .get::<_, Option<i64>>("success_ttfb_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_count: row
                            .get::<_, Option<i64>>("success_ttfb_ms_count")?
                            .unwrap_or(0),
                        success_generation_ms_sum: row
                            .get::<_, Option<i64>>("success_generation_ms_sum")?
                            .unwrap_or(0),
                        success_output_tokens_for_rate_sum: row
                            .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
                            .unwrap_or(0),
                        total_tokens: aggregated_total_tokens(row)?,
                        input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                        output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                        cache_creation_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_input_tokens")?
                            .unwrap_or(0),
                        cache_read_input_tokens: row
                            .get::<_, Option<i64>>("cache_read_input_tokens")?
                            .unwrap_or(0),
                        cache_creation_5m_input_tokens: 0,
                        cache_creation_1h_input_tokens: 0,
                        cost_covered_success: row
                            .get::<_, Option<i64>>("cost_covered_success")?
                            .unwrap_or(0),
                        total_cost_usd_femto: row
                            .get::<_, Option<i64>>("total_cost_usd_femto")?
                            .unwrap_or(0),
                    };

                    Ok(agg.into_leaderboard_row(key.clone(), key))
                })
                .map_err(|e| db_err!("failed to run day leaderboard query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| db_err!("failed to read day row: {e}"))?);
            }
            items
        }
        UsageScopeV2::Provider => {
            let effective_input_expr = sql_effective_input_tokens_expr_with_alias("r");
            let sql = format!(
                r#"
SELECT
  r.cli_key AS cli_key,
  r.final_provider_id AS provider_id,
  MAX(p.name) AS provider_name,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN r.status >= 200 AND r.status < 300 AND r.error_code IS NULL THEN 1 ELSE 0 END) AS requests_success,
  SUM(
    CASE WHEN (
      r.status IS NULL OR
      r.status < 200 OR
      r.status >= 300 OR
      r.error_code IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_failed,
  SUM({effective_input_expr}) AS input_tokens,
  SUM(COALESCE(r.output_tokens, 0)) AS output_tokens,
  SUM(COALESCE(r.cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(r.cache_read_input_tokens, 0)) AS cache_read_input_tokens,
  SUM(COALESCE(r.cache_creation_5m_input_tokens, 0)) AS cache_creation_5m_input_tokens,
  SUM(COALESCE(r.cache_creation_1h_input_tokens, 0)) AS cache_creation_1h_input_tokens,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.cost_usd_femto IS NOT NULL AND r.cost_usd_femto > 0
    ) THEN 1 ELSE 0 END
  ) AS cost_covered_success,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.cost_usd_femto IS NOT NULL AND r.cost_usd_femto > 0
    ) THEN r.cost_usd_femto ELSE 0 END
  ) AS total_cost_usd_femto,
  SUM(r.duration_ms) AS total_duration_ms,
  SUM(CASE WHEN r.status >= 200 AND r.status < 300 AND r.error_code IS NULL THEN r.duration_ms ELSE 0 END) AS success_duration_ms_sum,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.ttfb_ms IS NOT NULL AND
      r.ttfb_ms < r.duration_ms
    ) THEN r.ttfb_ms ELSE 0 END
  ) AS success_ttfb_ms_sum,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.ttfb_ms IS NOT NULL AND
      r.ttfb_ms < r.duration_ms
    ) THEN 1 ELSE 0 END
  ) AS success_ttfb_ms_count,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.output_tokens IS NOT NULL AND
      r.ttfb_ms IS NOT NULL AND
      r.ttfb_ms < r.duration_ms
    ) THEN (r.duration_ms - r.ttfb_ms) ELSE 0 END
  ) AS success_generation_ms_sum,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.output_tokens IS NOT NULL AND
      r.ttfb_ms IS NOT NULL AND
      r.ttfb_ms < r.duration_ms
    ) THEN r.output_tokens ELSE 0 END
  ) AS success_output_tokens_for_rate_sum
FROM request_logs r
LEFT JOIN providers p ON p.id = r.final_provider_id
WHERE r.excluded_from_stats = 0
AND r.final_provider_id IS NOT NULL
AND r.final_provider_id > 0
{provider_where_clause}
{provider_cx2cc_filter_clause}
GROUP BY r.cli_key, r.final_provider_id
"#,
                effective_input_expr = effective_input_expr,
                provider_where_clause = provider_where_clause,
                provider_cx2cc_filter_clause = provider_cx2cc_filter_clause
            );

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| db_err!("failed to prepare provider leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params_from_iter(provider_where_params.clone()), |row| {
                    let cli_key: String = row.get("cli_key")?;
                    let provider_id: i64 = row.get("provider_id")?;
                    let provider_name: Option<String> = row.get("provider_name")?;

                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
                        total_duration_ms: row
                            .get::<_, Option<i64>>("total_duration_ms")?
                            .unwrap_or(0),
                        first_request_created_at_ms: None,
                        last_request_created_at_ms: None,
                        success_duration_ms_sum: row
                            .get::<_, Option<i64>>("success_duration_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_sum: row
                            .get::<_, Option<i64>>("success_ttfb_ms_sum")?
                            .unwrap_or(0),
                        success_ttfb_ms_count: row
                            .get::<_, Option<i64>>("success_ttfb_ms_count")?
                            .unwrap_or(0),
                        success_generation_ms_sum: row
                            .get::<_, Option<i64>>("success_generation_ms_sum")?
                            .unwrap_or(0),
                        success_output_tokens_for_rate_sum: row
                            .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
                            .unwrap_or(0),
                        total_tokens: aggregated_total_tokens(row)?,
                        input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                        output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                        cache_creation_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_input_tokens")?
                            .unwrap_or(0),
                        cache_read_input_tokens: row
                            .get::<_, Option<i64>>("cache_read_input_tokens")?
                            .unwrap_or(0),
                        cache_creation_5m_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_5m_input_tokens")?
                            .unwrap_or(0),
                        cache_creation_1h_input_tokens: row
                            .get::<_, Option<i64>>("cache_creation_1h_input_tokens")?
                            .unwrap_or(0),
                        cost_covered_success: row
                            .get::<_, Option<i64>>("cost_covered_success")?
                            .unwrap_or(0),
                        total_cost_usd_femto: row
                            .get::<_, Option<i64>>("total_cost_usd_femto")?
                            .unwrap_or(0),
                    };

                    Ok((cli_key, provider_id, provider_name, agg))
                })
                .map_err(|e| db_err!("failed to run provider leaderboard query: {e}"))?;

            let fallback_name_sql = format!(
                r#"
SELECT attempts_json
FROM request_logs r
WHERE r.excluded_from_stats = 0
AND r.final_provider_id = ?1
AND r.cli_key = ?2
{provider_fallback_where_clause}
{provider_cx2cc_filter_clause}
LIMIT 1
"#,
                provider_fallback_where_clause = provider_fallback_where_clause,
                provider_cx2cc_filter_clause = provider_cx2cc_filter_clause
            );
            let mut stmt_fallback_name = conn
                .prepare(&fallback_name_sql)
                .map_err(|e| db_err!("failed to prepare provider name fallback query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(
                    row.map_err(|e| db_err!("failed to read provider leaderboard row: {e}"))?,
                );
            }

            let mut out = Vec::new();
            for (cli_key, provider_id, provider_name_db, agg) in items {
                let mut provider_name = provider_name_db
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty() && *v != "Unknown")
                    .map(str::to_string);

                if provider_name.is_none() {
                    let mut fallback_params: SqlValues =
                        vec![provider_id.into(), cli_key.clone().into()];
                    fallback_params.extend(provider_fallback_range_params.clone());
                    let attempts_json: Option<String> = stmt_fallback_name
                        .query_row(params_from_iter(fallback_params), |row| row.get(0))
                        .optional()
                        .map_err(|e| db_err!("failed to query provider name fallback: {e}"))?;

                    if let Some(attempts_json) = attempts_json {
                        let extracted = extract_final_provider(&cli_key, &attempts_json);
                        let extracted_name = extracted.provider_name.trim();
                        if !extracted_name.is_empty() && extracted_name != "Unknown" {
                            provider_name = Some(extracted_name.to_string());
                        }
                    }
                }

                let Some(provider_name) = provider_name else {
                    continue;
                };

                let provider_key = ProviderKey {
                    cli_key: cli_key.clone(),
                    provider_id,
                    provider_name: provider_name.clone(),
                };
                if !has_valid_provider_key(&provider_key) {
                    continue;
                }

                out.push(agg.into_leaderboard_row(
                    format!("{}:{}", cli_key, provider_id),
                    format!("{}/{}", cli_key, provider_name),
                ));
            }

            out
        }
    };

    if matches!(scope, UsageScopeV2::Day) {
        fill_missing_day_rows(conn, &mut out, start_ts, end_ts, day_start_hour)?;
        out.sort_by(|a, b| b.key.cmp(&a.key));
        // Truncate before summarizing so activity metrics (incl. per-hour
        // distribution) are only computed for the days that stay visible.
        out.truncate(effective_leaderboard_limit(limit));
        let min_day_key = out.last().map(|row| row.key.clone());
        let activity_rows = day_activity_rows_with_conn(
            conn,
            start_ts,
            end_ts,
            cli_key,
            provider_id,
            exclude_cx2cc_gateway_bridge,
            day_start_hour,
            min_day_key.as_deref(),
        )?;
        apply_day_activity_metrics(
            &mut out,
            summarize_activity_by_day(conn, activity_rows, development_time_gap_thresholds)?,
        );
    } else {
        out.sort_by(|a, b| {
            b.requests_total
                .cmp(&a.requests_total)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.key.cmp(&b.key))
        });
        out.truncate(effective_leaderboard_limit(limit));
    }
    Ok(out)
}

fn provider_name_from_event(row: &UsageEventAgg) -> Option<String> {
    let provider_id = row.bucket_provider_id?;
    if provider_id <= 0 {
        return None;
    }

    let mut provider_name = row
        .bucket_provider_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "Unknown")
        .map(str::to_string);

    if provider_name.is_none() {
        if let Some(attempts_json) = row.bucket_provider_attempts_json.as_deref() {
            let extracted = extract_final_provider(&row.cli_key, attempts_json);
            let extracted_name = extracted.provider_name.trim();
            if extracted.provider_id == provider_id
                && !extracted_name.is_empty()
                && extracted_name != "Unknown"
            {
                provider_name = Some(extracted_name.to_string());
            }
        }
    }

    let provider_name = provider_name?;
    let provider_key = ProviderKey {
        cli_key: row.cli_key.clone(),
        provider_id,
        provider_name: provider_name.clone(),
    };
    if !has_valid_provider_key(&provider_key) {
        return None;
    }
    Some(provider_name)
}

pub(super) struct FolderFilteredLeaderboardParams<'a> {
    pub(super) scope: UsageScopeV2,
    pub(super) start_ts: Option<i64>,
    pub(super) end_ts: Option<i64>,
    pub(super) cli_key: Option<&'a str>,
    pub(super) provider_id: Option<i64>,
    pub(super) folder_keys: &'a [String],
    pub(super) limit: Option<usize>,
    pub(super) exclude_cx2cc_gateway_bridge: bool,
    pub(super) day_start_hour: i64,
    pub(super) development_time_gap_thresholds: DevelopmentTimeGapThresholds,
}

pub(super) fn leaderboard_v2_folder_filtered_with_conn<F>(
    conn: &Connection,
    params: FolderFilteredLeaderboardParams<'_>,
    folder_lookup: F,
) -> Result<Vec<UsageLeaderboardRow>, String>
where
    F: FnOnce(&[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder>,
{
    let day_bucket_sql = local_day_bucket_sql("r.created_at", params.day_start_hour);
    let bucket_sql = match params.scope {
        UsageScopeV2::Cli | UsageScopeV2::Folder => None,
        UsageScopeV2::Provider => {
            Some("CASE WHEN r.final_provider_id IS NULL THEN NULL ELSE CAST(r.final_provider_id AS TEXT) END")
        }
        UsageScopeV2::Model => Some("COALESCE(NULLIF(r.requested_model, ''), 'Unknown')"),
        UsageScopeV2::Day => Some(day_bucket_sql.as_str()),
    };

    let rows = usage_event_rows(
        conn,
        params.start_ts,
        params.end_ts,
        params.cli_key,
        params.provider_id,
        bucket_sql,
        false,
        params.exclude_cx2cc_gateway_bridge,
    )?;
    let activity_rows = if matches!(params.scope, UsageScopeV2::Day | UsageScopeV2::Folder) {
        Some(day_activity_rows_with_conn(
            conn,
            params.start_ts,
            params.end_ts,
            params.cli_key,
            params.provider_id,
            params.exclude_cx2cc_gateway_bridge,
            params.day_start_hour,
            None,
        )?)
    } else {
        None
    };
    let lookup_keys = session_lookup_keys(&rows);
    let resolved = resolved_folder_map(folder_lookup(&lookup_keys));
    let rows = filter_rows_by_folder_keys(rows, &resolved, Some(params.folder_keys));

    let mut by_key: HashMap<String, (String, Option<String>, ProviderAgg)> = HashMap::new();
    for row in rows {
        let item = match params.scope {
            UsageScopeV2::Cli => {
                let key = row.cli_key.clone();
                Some((key.clone(), key, None))
            }
            UsageScopeV2::Provider => {
                let Some(provider_id) = row.bucket_provider_id else {
                    continue;
                };
                let Some(provider_name) = provider_name_from_event(&row) else {
                    continue;
                };
                Some((
                    format!("{}:{}", row.cli_key, provider_id),
                    format!("{}/{}", row.cli_key, provider_name),
                    None,
                ))
            }
            UsageScopeV2::Folder => {
                let folder = folder_identity_for_row(&row, &resolved);
                Some((folder.key, folder.name, folder.folder_path))
            }
            UsageScopeV2::Model | UsageScopeV2::Day => {
                let Some(key) = row.bucket_key.clone() else {
                    continue;
                };
                Some((key.clone(), key, None))
            }
        };
        let Some((key, name, folder_path)) = item else {
            continue;
        };
        let entry = by_key
            .entry(key)
            .or_insert_with(|| (name, folder_path, ProviderAgg::default()));
        let mut agg = row.agg;
        if !matches!(params.scope, UsageScopeV2::Day) {
            agg.first_request_created_at_ms = None;
            agg.last_request_created_at_ms = None;
        }
        entry.2.merge(agg);
    }

    let mut out: Vec<UsageLeaderboardRow> = by_key
        .into_iter()
        .map(|(key, (name, folder_path, agg))| {
            let mut row = agg.into_leaderboard_row(key, name);
            row.folder_path = folder_path;
            row
        })
        .collect();

    let filtered_activity_rows: Vec<UsageActivityRow> = activity_rows
        .into_iter()
        .flatten()
        .filter(|row| {
            params.folder_keys.is_empty()
                || params.folder_keys.iter().any(|folder_key| {
                    folder_identity_for_session(&row.cli_key, row.session_id.as_deref(), &resolved)
                        .key
                        == *folder_key
                })
        })
        .collect();

    match params.scope {
        UsageScopeV2::Day => {
            fill_missing_day_rows(
                conn,
                &mut out,
                params.start_ts,
                params.end_ts,
                params.day_start_hour,
            )?;
            out.sort_by(|a, b| b.key.cmp(&a.key));
            // Truncate before summarizing so activity metrics (incl. per-hour
            // distribution) are only computed for the days that stay visible.
            out.truncate(effective_leaderboard_limit(params.limit));
            let visible_day_keys: HashSet<String> = out.iter().map(|row| row.key.clone()).collect();
            let visible_activity_rows: Vec<UsageActivityRow> = filtered_activity_rows
                .into_iter()
                .filter(|row| visible_day_keys.contains(&row.day_key))
                .collect();
            apply_day_activity_metrics(
                &mut out,
                summarize_activity_by_day(
                    conn,
                    visible_activity_rows,
                    params.development_time_gap_thresholds,
                )?,
            );
        }
        UsageScopeV2::Folder => {
            apply_folder_activity_metrics(
                &mut out,
                summarize_activity_by_folder(
                    filtered_activity_rows,
                    &resolved,
                    params.development_time_gap_thresholds,
                ),
            );
            out.sort_by(|a, b| {
                b.total_tokens
                    .cmp(&a.total_tokens)
                    .then_with(|| b.requests_total.cmp(&a.requests_total))
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.key.cmp(&b.key))
            });
        }
        UsageScopeV2::Cli | UsageScopeV2::Provider | UsageScopeV2::Model => out.sort_by(|a, b| {
            b.requests_total
                .cmp(&a.requests_total)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.key.cmp(&b.key))
        }),
    }
    out.truncate(effective_leaderboard_limit(params.limit));
    Ok(out)
}

pub fn leaderboard_v2<F>(
    db: &db::Db,
    scope: &str,
    params: &UsageQueryParams,
    limit: Option<usize>,
    folder_lookup: F,
) -> crate::shared::error::AppResult<Vec<UsageLeaderboardRow>>
where
    F: FnOnce(&[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder>,
{
    let conn = db.open_connection()?;
    let scope = parse_scope_v2(scope)?;
    let resolved = resolve_query_params(&conn, params)?;
    if matches!(scope, UsageScopeV2::Folder) || resolved.folder_keys.is_some() {
        let folder_keys = resolved.folder_keys.as_deref().unwrap_or_default();
        return Ok(leaderboard_v2_folder_filtered_with_conn(
            &conn,
            FolderFilteredLeaderboardParams {
                scope,
                start_ts: resolved.start_ts,
                end_ts: resolved.end_ts,
                cli_key: resolved.cli_key,
                provider_id: resolved.provider_id,
                folder_keys,
                limit,
                exclude_cx2cc_gateway_bridge: resolved.exclude_cx2cc_gateway_bridge,
                day_start_hour: resolved.day_start_hour,
                development_time_gap_thresholds: resolved.development_time_gap_thresholds,
            },
            folder_lookup,
        )?);
    }

    Ok(leaderboard_v2_with_conn_day_start_and_thresholds(
        &conn,
        scope,
        resolved.start_ts,
        resolved.end_ts,
        resolved.cli_key,
        resolved.provider_id,
        limit,
        resolved.exclude_cx2cc_gateway_bridge,
        resolved.day_start_hour,
        resolved.development_time_gap_thresholds,
    )?)
}

#[cfg(test)]
mod tests {
    use super::{
        local_day_bucket_sql, summarize_day_activity as calculate_day_activity,
        weighted_idle_gap_ms as calculate_weighted_idle_gap_ms, DevelopmentTimeGapThresholds,
        UsageActivityRow, MAX_ESTIMATED_DEVELOPMENT_TIME_MS,
    };

    const FULL_IDLE_GAP_MS: i64 = 15 * 60 * 1000;
    const SESSION_BREAK_GAP_MS: i64 = 30 * 60 * 1000;

    fn summarize_day_activity(rows: &[UsageActivityRow]) -> Option<super::DayActivityMetrics> {
        calculate_day_activity(rows, DevelopmentTimeGapThresholds::default())
    }

    fn weighted_idle_gap_ms(gap_ms: i64) -> i64 {
        calculate_weighted_idle_gap_ms(gap_ms, DevelopmentTimeGapThresholds::default())
    }

    fn activity(start_ms: i64, duration_ms: i64, session_id: Option<&str>) -> UsageActivityRow {
        UsageActivityRow {
            cli_key: "codex".to_string(),
            session_id: session_id.map(str::to_string),
            day_key: "2026-07-20".to_string(),
            start_ms,
            duration_ms,
        }
    }

    #[test]
    fn local_day_bucket_sql_shifts_after_localtime_for_wall_clock_day_boundaries() {
        assert_eq!(
            local_day_bucket_sql("created_at", 0),
            "strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime')"
        );
        assert_eq!(
            local_day_bucket_sql("created_at", 5),
            "strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime', '-5 hours')"
        );
        assert_eq!(
            local_day_bucket_sql("r.created_at", 5),
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch', 'localtime', '-5 hours')"
        );
    }

    #[test]
    fn single_request_has_no_fixed_tail_compensation() {
        let metrics = summarize_day_activity(&[activity(1_000, 90_000, None)]).unwrap();
        assert_eq!(metrics.last_request_completed_at_ms, 91_000);
        assert_eq!(metrics.estimated_development_time_ms, 90_000);
    }

    #[test]
    fn overlapping_and_concurrent_requests_are_merged_by_interval_only() {
        let rows = [
            activity(0, 20_000, Some("reused")),
            activity(10_000, 20_000, Some("reused")),
            activity(10_000, 5_000, Some("different")),
        ];
        let metrics = summarize_day_activity(&rows).unwrap();
        assert_eq!(rows.iter().map(|row| row.duration_ms).sum::<i64>(), 45_000);
        assert_eq!(metrics.estimated_development_time_ms, 30_000);
        assert_eq!(metrics.last_request_completed_at_ms, 30_000);
    }

    #[test]
    fn reused_session_does_not_merge_non_overlapping_requests() {
        let rows = [
            activity(0, 60_000, Some("same-session")),
            activity(SESSION_BREAK_GAP_MS + 60_000, 60_000, Some("same-session")),
        ];
        let metrics = summarize_day_activity(&rows).unwrap();
        assert_eq!(metrics.estimated_development_time_ms, 120_000);
    }

    #[test]
    fn idle_gap_weighting_honors_boundaries_and_soft_decay() {
        let minute_ms = 60 * 1000;

        assert_eq!(
            weighted_idle_gap_ms(FULL_IDLE_GAP_MS - 1),
            FULL_IDLE_GAP_MS - 1
        );
        assert_eq!(weighted_idle_gap_ms(FULL_IDLE_GAP_MS), FULL_IDLE_GAP_MS);
        assert_eq!(
            weighted_idle_gap_ms(FULL_IDLE_GAP_MS + 1),
            FULL_IDLE_GAP_MS - 1
        );
        assert_eq!(
            weighted_idle_gap_ms(20 * minute_ms),
            13 * minute_ms + 20 * 1000
        );
        assert_eq!(
            weighted_idle_gap_ms(25 * minute_ms),
            8 * minute_ms + 20 * 1000
        );
        assert_eq!(weighted_idle_gap_ms(SESSION_BREAK_GAP_MS - 1), 1);
        assert_eq!(weighted_idle_gap_ms(SESSION_BREAK_GAP_MS), 0);
        assert_eq!(weighted_idle_gap_ms(SESSION_BREAK_GAP_MS + 1), 0);

        let fifteen_minute_gap = summarize_day_activity(&[
            activity(0, 60_000, None),
            activity(60_000 + FULL_IDLE_GAP_MS, 60_000, None),
        ])
        .unwrap();
        assert_eq!(
            fifteen_minute_gap.estimated_development_time_ms,
            17 * minute_ms
        );

        let thirty_minute_gap = summarize_day_activity(&[
            activity(0, 60_000, None),
            activity(60_000 + SESSION_BREAK_GAP_MS, 60_000, None),
        ])
        .unwrap();
        assert_eq!(
            thirty_minute_gap.estimated_development_time_ms,
            2 * minute_ms
        );
    }

    #[test]
    fn custom_idle_gap_thresholds_change_the_soft_decay_window() {
        let minute_ms = 60 * 1000;
        let thresholds = DevelopmentTimeGapThresholds {
            full_idle_gap_ms: 10 * minute_ms,
            session_break_gap_ms: 30 * minute_ms,
        };

        assert_eq!(
            calculate_weighted_idle_gap_ms(10 * minute_ms, thresholds),
            10 * minute_ms
        );
        assert_eq!(
            calculate_weighted_idle_gap_ms(15 * minute_ms, thresholds),
            11 * minute_ms + 15 * 1000
        );
        assert_eq!(
            calculate_weighted_idle_gap_ms(30 * minute_ms, thresholds),
            0
        );
        assert_eq!(
            calculate_weighted_idle_gap_ms(30 * minute_ms + 1, thresholds),
            0
        );

        let metrics = calculate_day_activity(
            &[
                activity(0, minute_ms, None),
                activity(16 * minute_ms, minute_ms, None),
            ],
            thresholds,
        )
        .unwrap();
        assert_eq!(
            metrics.estimated_development_time_ms,
            13 * minute_ms + 15 * 1000
        );
    }

    #[test]
    fn zero_negative_overflowing_and_long_durations_are_bounded() {
        let inactive =
            summarize_day_activity(&[activity(1_000, 0, None), activity(2_000, -500, None)])
                .unwrap();
        assert_eq!(inactive.estimated_development_time_ms, 0);
        assert_eq!(inactive.last_request_completed_at_ms, 2_000);

        let overflowing = summarize_day_activity(&[activity(i64::MAX - 5, 100, None)]).unwrap();
        assert_eq!(overflowing.last_request_completed_at_ms, i64::MAX);
        assert!(overflowing.estimated_development_time_ms >= 0);

        let long = summarize_day_activity(&[activity(0, i64::MAX, None)]).unwrap();
        assert_eq!(
            long.estimated_development_time_ms,
            MAX_ESTIMATED_DEVELOPMENT_TIME_MS
        );
    }
}
