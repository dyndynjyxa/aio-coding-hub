//! Usage: Provider limit usage queries - calculates current spending against configured limits.

use crate::db;
use crate::providers::DailyResetMode;
use crate::shared::error::db_err;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderLimitUsageRow {
    pub cli_key: String,
    pub provider_id: i64,
    pub provider_name: String,
    pub enabled: bool,
    // Limits (null if not configured)
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<String>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    // Current usage for each window
    pub usage_5h_usd: f64,
    pub usage_daily_usd: f64,
    pub usage_weekly_usd: f64,
    pub usage_monthly_usd: f64,
    pub usage_total_usd: f64,
    // Window start timestamps (unix seconds) for UI display
    pub window_5h_start_ts: i64,
    pub window_daily_start_ts: i64,
    pub window_weekly_start_ts: i64,
    pub window_monthly_start_ts: i64,
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn normalize_cli_filter(cli_key: Option<&str>) -> crate::shared::error::AppResult<Option<&str>> {
    if let Some(k) = cli_key {
        validate_cli_key(k)?;
        return Ok(Some(k));
    }
    Ok(None)
}

fn cost_usd_from_femto(v: i64) -> f64 {
    (v.max(0) as f64) / USD_FEMTO_DENOM
}

/// Computes the start timestamp for the 5h window for a specific provider (fixed window mode)
/// Returns the stored window_5h_start_ts from providers table, or computes from first request if expired/null.
fn compute_ts_5h(conn: &Connection, provider_id: i64) -> crate::shared::error::AppResult<i64> {
    const WINDOW_5H_SECS: i64 = 5 * 60 * 60;

    // Get current time
    let now_unix = conn
        .query_row("SELECT CAST(strftime('%s', 'now') AS INTEGER)", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| db_err!("failed to get current timestamp: {e}"))?;

    // Read stored window_5h_start_ts
    let stored_window: Option<i64> = conn
        .query_row(
            "SELECT window_5h_start_ts FROM providers WHERE id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .map_err(|e| db_err!("failed to read window_5h_start_ts: {e}"))?;

    // Check if stored window is still valid (not expired)
    if let Some(start_ts) = stored_window {
        let window_end = start_ts.saturating_add(WINDOW_5H_SECS);
        if now_unix < window_end {
            // Window still valid
            return Ok(start_ts);
        }
    }

    // Window expired or null -> find first request timestamp in recent 5h
    let recent_threshold = now_unix.saturating_sub(WINDOW_5H_SECS);
    let first_request_ts: Option<i64> = conn
        .query_row(
            r#"
            SELECT MIN(created_at)
            FROM request_logs
            WHERE final_provider_id = ?1
              AND excluded_from_stats = 0
              AND status >= 200 AND status < 300
              AND error_code IS NULL
              AND created_at >= ?2
            "#,
            params![provider_id, recent_threshold],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query first request timestamp: {e}"))?
        .flatten();

    // Return first request timestamp if exists, otherwise use now
    Ok(first_request_ts.unwrap_or(now_unix))
}

/// Computes the start timestamp for the daily window based on reset mode
fn compute_ts_daily(
    conn: &Connection,
    daily_reset_mode: DailyResetMode,
    daily_reset_time: &str,
) -> crate::shared::error::AppResult<i64> {
    match daily_reset_mode {
        DailyResetMode::Rolling => {
            // Rolling: now - 24 hours
            conn.query_row(
                "SELECT CAST(strftime('%s', 'now', '-24 hours') AS INTEGER)",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| db_err!("failed to compute rolling daily timestamp: {e}"))
        }
        DailyResetMode::Fixed => {
            // Fixed: start of day based on daily_reset_time in local timezone
            // daily_reset_time is in format "HH:MM:SS"
            let sql = format!(
                r#"
                SELECT CASE
                    WHEN strftime('%H:%M:%S', 'now', 'localtime') >= '{reset_time}'
                    THEN CAST(strftime('%s', date('now', 'localtime') || ' {reset_time}', 'utc') AS INTEGER)
                    ELSE CAST(strftime('%s', date('now', 'localtime', '-1 day') || ' {reset_time}', 'utc') AS INTEGER)
                END
                "#,
                reset_time = daily_reset_time
            );
            conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
                .map_err(|e| db_err!("failed to compute fixed daily timestamp: {e}"))
        }
    }
}

/// Computes the start timestamp for the weekly window (Monday 00:00:00 local time)
fn compute_ts_weekly(conn: &Connection) -> crate::shared::error::AppResult<i64> {
    // Get Monday of current week at 00:00:00 local time, converted to UTC
    conn.query_row(
        r#"
        SELECT CAST(strftime('%s',
            date('now', 'localtime', 'weekday 0', '-6 days') || ' 00:00:00',
            'utc'
        ) AS INTEGER)
        "#,
        [],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| db_err!("failed to compute weekly timestamp: {e}"))
}

/// Computes the start timestamp for the monthly window (1st of month 00:00:00 local time)
fn compute_ts_monthly(conn: &Connection) -> crate::shared::error::AppResult<i64> {
    conn.query_row(
        "SELECT CAST(strftime('%s', date('now', 'localtime', 'start of month') || ' 00:00:00', 'utc') AS INTEGER)",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| db_err!("failed to compute monthly timestamp: {e}"))
}

/// Aggregates cost_usd from request_logs for a specific provider within a time window
fn aggregate_cost_for_provider(
    conn: &Connection,
    provider_id: i64,
    start_ts: Option<i64>,
) -> crate::shared::error::AppResult<i64> {
    let sql = r#"
        SELECT COALESCE(SUM(cost_usd_femto), 0)
        FROM request_logs
        WHERE final_provider_id = ?1
          AND excluded_from_stats = 0
          AND status >= 200 AND status < 300 AND error_code IS NULL
          AND (?2 IS NULL OR created_at >= ?2)
    "#;

    conn.query_row(sql, params![provider_id, start_ts], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| db_err!("failed to aggregate cost: {e}"))
}

pub fn list_v1(
    db: &db::Db,
    cli_key: Option<&str>,
) -> crate::shared::error::AppResult<Vec<ProviderLimitUsageRow>> {
    let cli_key = normalize_cli_filter(cli_key)?;
    let conn = db.open_connection()?;

    // Pre-compute common time windows (5h is computed per-provider below)
    let ts_weekly = compute_ts_weekly(&conn)?;
    let ts_monthly = compute_ts_monthly(&conn)?;

    // Query all providers with at least one limit configured
    let sql = r#"
        SELECT
            id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd
        FROM providers
        WHERE (?1 IS NULL OR cli_key = ?1)
          AND (
            limit_5h_usd IS NOT NULL OR
            limit_daily_usd IS NOT NULL OR
            limit_weekly_usd IS NOT NULL OR
            limit_monthly_usd IS NOT NULL OR
            limit_total_usd IS NOT NULL
          )
        ORDER BY cli_key ASC, sort_order ASC, id DESC
    "#;

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| db_err!("failed to prepare providers query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], |row| {
            let daily_reset_mode_raw: String = row.get("daily_reset_mode")?;
            let daily_reset_time_raw: String = row.get("daily_reset_time")?;

            Ok((
                row.get::<_, i64>("id")?,
                row.get::<_, String>("cli_key")?,
                row.get::<_, String>("name")?,
                row.get::<_, i64>("enabled")? != 0,
                row.get::<_, Option<f64>>("limit_5h_usd")?,
                row.get::<_, Option<f64>>("limit_daily_usd")?,
                daily_reset_mode_raw,
                daily_reset_time_raw,
                row.get::<_, Option<f64>>("limit_weekly_usd")?,
                row.get::<_, Option<f64>>("limit_monthly_usd")?,
                row.get::<_, Option<f64>>("limit_total_usd")?,
            ))
        })
        .map_err(|e| db_err!("failed to query providers: {e}"))?;

    let mut out = Vec::new();

    for row in rows {
        let (
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time_raw,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
        ) = row.map_err(|e| db_err!("failed to read provider row: {e}"))?;

        // Parse daily reset mode for computation
        let daily_reset_mode = match daily_reset_mode_raw.as_str() {
            "rolling" => DailyResetMode::Rolling,
            _ => DailyResetMode::Fixed,
        };

        // Normalize daily_reset_time (defaults to "00:00:00")
        let daily_reset_time = if daily_reset_time_raw.trim().is_empty() {
            "00:00:00".to_string()
        } else {
            daily_reset_time_raw.clone()
        };

        // Compute daily timestamp based on provider's reset mode
        let ts_daily = compute_ts_daily(&conn, daily_reset_mode, &daily_reset_time)?;

        // Compute 5h window start per provider (fixed window mode)
        let ts_5h = compute_ts_5h(&conn, provider_id)?;

        // Aggregate costs for each time window
        let usage_5h_femto = aggregate_cost_for_provider(&conn, provider_id, Some(ts_5h))?;
        let usage_daily_femto = aggregate_cost_for_provider(&conn, provider_id, Some(ts_daily))?;
        let usage_weekly_femto = aggregate_cost_for_provider(&conn, provider_id, Some(ts_weekly))?;
        let usage_monthly_femto =
            aggregate_cost_for_provider(&conn, provider_id, Some(ts_monthly))?;
        let usage_total_femto = aggregate_cost_for_provider(&conn, provider_id, None)?;

        out.push(ProviderLimitUsageRow {
            cli_key,
            provider_id,
            provider_name: name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode: if limit_daily_usd.is_some() {
                Some(daily_reset_mode_raw)
            } else {
                None
            },
            daily_reset_time: if limit_daily_usd.is_some() {
                Some(daily_reset_time)
            } else {
                None
            },
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            usage_5h_usd: cost_usd_from_femto(usage_5h_femto),
            usage_daily_usd: cost_usd_from_femto(usage_daily_femto),
            usage_weekly_usd: cost_usd_from_femto(usage_weekly_femto),
            usage_monthly_usd: cost_usd_from_femto(usage_monthly_femto),
            usage_total_usd: cost_usd_from_femto(usage_total_femto),
            window_5h_start_ts: ts_5h,
            window_daily_start_ts: ts_daily,
            window_weekly_start_ts: ts_weekly,
            window_monthly_start_ts: ts_monthly,
        });
    }

    Ok(out)
}
