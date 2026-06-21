//! Usage: Provider limit usage queries - calculates current spending against configured limits.

use crate::db;
use crate::providers::DailyResetMode;
use crate::shared::error::db_err;
use rusqlite::{params, params_from_iter, Connection};
use serde::Serialize;
use std::collections::HashMap;

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const WINDOW_5H_SECS: i64 = 5 * 60 * 60;
const MAX_PROVIDERS_PER_USAGE_QUERY: usize = 300;

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

fn current_unix_seconds(conn: &Connection) -> crate::shared::error::AppResult<i64> {
    conn.query_row("SELECT CAST(strftime('%s', 'now') AS INTEGER)", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| db_err!("failed to get current timestamp: {e}"))
}

fn values_clause(row_count: usize, column_count: usize) -> String {
    let row = format!("({})", crate::db::sql_placeholders(column_count));
    std::iter::repeat_n(row, row_count)
        .collect::<Vec<_>>()
        .join(",")
}

/// Resolves fixed 5h window starts in batches. Valid stored windows are used
/// directly; expired/null windows fall back to the first successful request in
/// the recent 5h window, matching the previous per-provider behavior.
fn resolve_5h_starts(
    conn: &Connection,
    provider_windows: &[(i64, Option<i64>)],
) -> crate::shared::error::AppResult<HashMap<i64, i64>> {
    let now_unix = current_unix_seconds(conn)?;
    let recent_threshold = now_unix.saturating_sub(WINDOW_5H_SECS);
    let mut out = HashMap::with_capacity(provider_windows.len());
    let mut expired_ids = Vec::new();

    for (provider_id, stored_window) in provider_windows.iter().copied() {
        if let Some(start_ts) = stored_window {
            if now_unix < start_ts.saturating_add(WINDOW_5H_SECS) {
                out.insert(provider_id, start_ts);
                continue;
            }
        }
        expired_ids.push(provider_id);
    }

    for chunk in expired_ids.chunks(MAX_PROVIDERS_PER_USAGE_QUERY) {
        if chunk.is_empty() {
            continue;
        }
        let values = values_clause(chunk.len(), 1);
        let sql = format!(
            r#"
WITH candidates(provider_id) AS (VALUES {values})
SELECT
  c.provider_id,
  MIN(r.created_at) AS first_request_ts
FROM candidates c
LEFT JOIN request_logs r
  ON r.final_provider_id = c.provider_id
 AND r.excluded_from_stats = 0
 AND r.status >= 200 AND r.status < 300
 AND r.error_code IS NULL
 AND r.created_at >= ?
GROUP BY c.provider_id
"#
        );

        let mut params_vec: Vec<i64> = chunk.to_vec();
        params_vec.push(recent_threshold);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare 5h window query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(params_vec.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .map_err(|e| db_err!("failed to query 5h windows: {e}"))?;

        for row in rows {
            let (provider_id, first_request_ts) =
                row.map_err(|e| db_err!("failed to read 5h window row: {e}"))?;
            out.insert(provider_id, first_request_ts.unwrap_or(now_unix));
        }
    }

    Ok(out)
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
            conn.query_row(
                r#"
                SELECT CASE
                    WHEN strftime('%H:%M:%S', 'now', 'localtime') >= ?1
                    THEN CAST(strftime('%s', date('now', 'localtime') || ' ' || ?1, 'utc') AS INTEGER)
                    ELSE CAST(strftime('%s', date('now', 'localtime', '-1 day') || ' ' || ?1, 'utc') AS INTEGER)
                END
                "#,
                params![daily_reset_time],
                |row| row.get::<_, i64>(0),
            )
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

#[derive(Debug, Clone)]
struct ProviderLimitCandidate {
    provider_id: i64,
    cli_key: String,
    name: String,
    enabled: bool,
    limit_5h_usd: Option<f64>,
    limit_daily_usd: Option<f64>,
    daily_reset_mode_raw: String,
    daily_reset_time: String,
    limit_weekly_usd: Option<f64>,
    limit_monthly_usd: Option<f64>,
    limit_total_usd: Option<f64>,
    window_5h_start_ts: i64,
    window_daily_start_ts: i64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProviderUsageSums {
    usage_5h_femto: i64,
    usage_daily_femto: i64,
    usage_weekly_femto: i64,
    usage_monthly_femto: i64,
    usage_total_femto: i64,
}

fn aggregate_costs_for_providers(
    conn: &Connection,
    providers: &[ProviderLimitCandidate],
    ts_weekly: i64,
    ts_monthly: i64,
) -> crate::shared::error::AppResult<HashMap<i64, ProviderUsageSums>> {
    let mut out = HashMap::with_capacity(providers.len());
    if providers.is_empty() {
        return Ok(out);
    }

    for chunk in providers.chunks(MAX_PROVIDERS_PER_USAGE_QUERY) {
        let values = values_clause(chunk.len(), 3);
        let sql = format!(
            r#"
WITH provider_windows(provider_id, ts_5h, ts_daily) AS (VALUES {values})
SELECT
  w.provider_id,
  COALESCE(SUM(CASE WHEN r.created_at >= w.ts_5h THEN r.cost_usd_femto ELSE 0 END), 0) AS usage_5h_femto,
  COALESCE(SUM(CASE WHEN r.created_at >= w.ts_daily THEN r.cost_usd_femto ELSE 0 END), 0) AS usage_daily_femto,
  COALESCE(SUM(CASE WHEN r.created_at >= ? THEN r.cost_usd_femto ELSE 0 END), 0) AS usage_weekly_femto,
  COALESCE(SUM(CASE WHEN r.created_at >= ? THEN r.cost_usd_femto ELSE 0 END), 0) AS usage_monthly_femto,
  COALESCE(SUM(r.cost_usd_femto), 0) AS usage_total_femto
FROM provider_windows w
LEFT JOIN request_logs r
  ON r.final_provider_id = w.provider_id
 AND r.excluded_from_stats = 0
 AND r.status >= 200 AND r.status < 300
 AND r.error_code IS NULL
 AND r.cost_usd_femto IS NOT NULL
GROUP BY w.provider_id
"#
        );

        let mut params_vec = Vec::with_capacity(chunk.len() * 3 + 2);
        for provider in chunk {
            params_vec.push(provider.provider_id);
            params_vec.push(provider.window_5h_start_ts);
            params_vec.push(provider.window_daily_start_ts);
        }
        params_vec.push(ts_weekly);
        params_vec.push(ts_monthly);

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare provider usage query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(params_vec.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ProviderUsageSums {
                        usage_5h_femto: row.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0),
                        usage_daily_femto: row.get::<_, Option<i64>>(2)?.unwrap_or(0).max(0),
                        usage_weekly_femto: row.get::<_, Option<i64>>(3)?.unwrap_or(0).max(0),
                        usage_monthly_femto: row.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0),
                        usage_total_femto: row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0),
                    },
                ))
            })
            .map_err(|e| db_err!("failed to query provider usage: {e}"))?;

        for row in rows {
            let (provider_id, sums) =
                row.map_err(|e| db_err!("failed to read provider usage row: {e}"))?;
            out.insert(provider_id, sums);
        }
    }

    Ok(out)
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
            limit_total_usd,
            window_5h_start_ts
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
                row.get::<_, Option<i64>>("window_5h_start_ts")?,
            ))
        })
        .map_err(|e| db_err!("failed to query providers: {e}"))?;

    let mut raw_rows = Vec::new();
    let mut provider_windows = Vec::new();
    let mut daily_window_cache: HashMap<(String, String), i64> = HashMap::new();

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
            stored_5h_start_ts,
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

        // Compute daily timestamp based on provider's reset mode. Cache by
        // mode/time because most providers share the same reset settings.
        let daily_cache_key = (
            daily_reset_mode.as_str().to_string(),
            daily_reset_time.clone(),
        );
        let ts_daily = match daily_window_cache.get(&daily_cache_key).copied() {
            Some(ts) => ts,
            None => {
                let ts = compute_ts_daily(&conn, daily_reset_mode, &daily_reset_time)?;
                daily_window_cache.insert(daily_cache_key, ts);
                ts
            }
        };

        raw_rows.push((
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            stored_5h_start_ts,
            ts_daily,
        ));
        provider_windows.push((provider_id, stored_5h_start_ts));
    }

    if raw_rows.is_empty() {
        return Ok(Vec::new());
    }

    let starts_5h = resolve_5h_starts(&conn, &provider_windows)?;
    let mut candidates = Vec::with_capacity(raw_rows.len());
    for (
        provider_id,
        cli_key,
        name,
        enabled,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode_raw,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        _stored_5h_start_ts,
        ts_daily,
    ) in raw_rows
    {
        let ts_5h = starts_5h
            .get(&provider_id)
            .copied()
            .ok_or_else(|| db_err!("failed to resolve 5h window for provider_id={provider_id}"))?;
        candidates.push(ProviderLimitCandidate {
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            window_5h_start_ts: ts_5h,
            window_daily_start_ts: ts_daily,
        });
    }

    let usage_by_provider =
        aggregate_costs_for_providers(&conn, &candidates, ts_weekly, ts_monthly)?;
    let out = candidates
        .into_iter()
        .map(|provider| {
            let sums = usage_by_provider
                .get(&provider.provider_id)
                .copied()
                .unwrap_or_default();
            ProviderLimitUsageRow {
                cli_key: provider.cli_key,
                provider_id: provider.provider_id,
                provider_name: provider.name,
                enabled: provider.enabled,
                limit_5h_usd: provider.limit_5h_usd,
                limit_daily_usd: provider.limit_daily_usd,
                daily_reset_mode: if provider.limit_daily_usd.is_some() {
                    Some(provider.daily_reset_mode_raw)
                } else {
                    None
                },
                daily_reset_time: if provider.limit_daily_usd.is_some() {
                    Some(provider.daily_reset_time)
                } else {
                    None
                },
                limit_weekly_usd: provider.limit_weekly_usd,
                limit_monthly_usd: provider.limit_monthly_usd,
                limit_total_usd: provider.limit_total_usd,
                usage_5h_usd: cost_usd_from_femto(sums.usage_5h_femto),
                usage_daily_usd: cost_usd_from_femto(sums.usage_daily_femto),
                usage_weekly_usd: cost_usd_from_femto(sums.usage_weekly_femto),
                usage_monthly_usd: cost_usd_from_femto(sums.usage_monthly_femto),
                usage_total_usd: cost_usd_from_femto(sums.usage_total_femto),
                window_5h_start_ts: provider.window_5h_start_ts,
                window_daily_start_ts: provider.window_daily_start_ts,
                window_weekly_start_ts: ts_weekly,
                window_monthly_start_ts: ts_monthly,
            }
        })
        .collect();

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{self, DailyResetMode, ProviderBaseUrlMode, ProviderUpsertParams};
    use rusqlite::params;

    const FEMTO: i64 = 1_000_000_000_000_000;

    fn create_limited_provider(db: &db::Db, name: &str) -> i64 {
        providers::upsert(
            db,
            ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.com".to_string()],
                base_url_mode: ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some("sk-test".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: None,
                claude_models: None,
                limit_5h_usd: Some(10.0),
                limit_daily_usd: Some(10.0),
                daily_reset_mode: Some(DailyResetMode::Rolling),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: Some(10.0),
                limit_monthly_usd: Some(10.0),
                limit_total_usd: Some(10.0),
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("create provider")
        .id
    }

    fn insert_log_with_exclusion(
        conn: &Connection,
        provider_id: i64,
        created_at: i64,
        cost_femto: i64,
        excluded_from_stats: i64,
    ) {
        conn.execute(
            r#"
INSERT INTO request_logs(
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id
) VALUES (?1, 'codex', 'POST', '/v1/chat/completions', ?2, ?3, 10,
  '[]', ?4, ?5, ?6, ?7, ?8)
"#,
            params![
                format!("trace-{provider_id}-{created_at}-{cost_femto}"),
                if excluded_from_stats == 0 {
                    200i64
                } else {
                    499i64
                },
                if excluded_from_stats == 0 {
                    None
                } else {
                    Some("GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP")
                },
                created_at,
                created_at.saturating_mul(1000),
                cost_femto,
                excluded_from_stats,
                provider_id
            ],
        )
        .expect("insert request log");
    }

    fn insert_log(conn: &Connection, provider_id: i64, created_at: i64, cost_femto: i64) {
        insert_log_with_exclusion(conn, provider_id, created_at, cost_femto, 0);
    }

    #[test]
    fn list_v1_batches_provider_usage_without_changing_window_totals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("test.db")).expect("init db");
        let provider_id = create_limited_provider(&db, "limited");
        let conn = db.open_connection().expect("open db");
        let now = current_unix_seconds(&conn).expect("now");
        let start_5h = now.saturating_sub(60 * 60);

        conn.execute(
            "UPDATE providers SET window_5h_start_ts = ?1 WHERE id = ?2",
            params![start_5h, provider_id],
        )
        .expect("set 5h window");

        insert_log(&conn, provider_id, now.saturating_sub(30 * 60), FEMTO);
        insert_log(
            &conn,
            provider_id,
            now.saturating_sub(2 * 60 * 60),
            2 * FEMTO,
        );
        insert_log(&conn, provider_id, now.saturating_sub(15 * 60), -2 * FEMTO);
        drop(conn);

        let rows = list_v1(&db, Some("codex")).expect("list usage");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.provider_id, provider_id);
        assert_eq!(row.window_5h_start_ts, start_5h);
        assert!((row.usage_5h_usd - 0.0).abs() < f64::EPSILON);
        assert!((row.usage_daily_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_total_usd - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn list_v1_excludes_lifecycle_interruption_rows_from_provider_usage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("test.db")).expect("init db");
        let provider_id = create_limited_provider(&db, "limited");
        let conn = db.open_connection().expect("open db");
        let now = current_unix_seconds(&conn).expect("now");

        insert_log(&conn, provider_id, now.saturating_sub(60), FEMTO);
        insert_log_with_exclusion(&conn, provider_id, now.saturating_sub(30), 99 * FEMTO, 1);
        drop(conn);

        let rows = list_v1(&db, Some("codex")).expect("list usage");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.provider_id, provider_id);
        assert!((row.usage_5h_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_daily_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_total_usd - 1.0).abs() < f64::EPSILON);
    }
}
