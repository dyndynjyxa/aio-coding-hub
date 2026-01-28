//! Usage: Cost analytics queries and backfill jobs backed by sqlite.

use crate::cost;
use crate::db;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const SQL_MODEL_KEY_EXPR: &str = "COALESCE(NULLIF(TRIM(requested_model), ''), 'Unknown')";

#[derive(Debug, Clone, Serialize)]
pub struct CostSummaryV1 {
    pub requests_total: i64,
    pub requests_success: i64,
    pub requests_failed: i64,
    pub cost_covered_success: i64,
    pub total_cost_usd: f64,
    pub avg_cost_usd_per_covered_success: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostTrendRowV1 {
    pub day: String,
    pub hour: Option<i64>,
    pub cost_usd: f64,
    pub requests_success: i64,
    pub cost_covered_success: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostProviderBreakdownRowV1 {
    pub cli_key: String,
    pub provider_id: i64,
    pub provider_name: String,
    pub requests_success: i64,
    pub cost_covered_success: i64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostModelBreakdownRowV1 {
    pub model: String,
    pub requests_success: i64,
    pub cost_covered_success: i64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostTopRequestRowV1 {
    pub log_id: i64,
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub requested_model: Option<String>,
    pub provider_id: i64,
    pub provider_name: String,
    pub duration_ms: i64,
    pub ttfb_ms: Option<i64>,
    pub cost_usd: f64,
    pub cost_multiplier: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostScatterCliProviderModelRowV1 {
    pub cli_key: String,
    pub provider_name: String,
    pub model: String,
    pub requests_success: i64,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostBackfillReportV1 {
    pub scanned: i64,
    pub updated: i64,
    pub skipped_no_model: i64,
    pub skipped_no_usage: i64,
    pub skipped_no_price: i64,
    pub skipped_other: i64,
    pub capped: bool,
    pub max_rows: i64,
}

#[derive(Debug, Clone, Copy)]
enum CostPeriodV1 {
    Daily,
    Weekly,
    Monthly,
    AllTime,
    Custom,
}

#[derive(Debug, Clone, Copy)]
enum TrendBucket {
    Hour,
    Day,
}

fn parse_period_v1(input: &str) -> Result<CostPeriodV1, String> {
    match input {
        "daily" => Ok(CostPeriodV1::Daily),
        "weekly" => Ok(CostPeriodV1::Weekly),
        "monthly" => Ok(CostPeriodV1::Monthly),
        "allTime" | "all_time" | "all" => Ok(CostPeriodV1::AllTime),
        "custom" => Ok(CostPeriodV1::Custom),
        _ => Err(format!("SEC_INVALID_INPUT: unknown period={input}")),
    }
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn normalize_cli_filter(cli_key: Option<&str>) -> Result<Option<&str>, String> {
    if let Some(k) = cli_key {
        validate_cli_key(k)?;
        return Ok(Some(k));
    }
    Ok(None)
}

fn normalize_provider_id_filter(provider_id: Option<i64>) -> Result<Option<i64>, String> {
    if let Some(id) = provider_id {
        if id <= 0 {
            return Err("SEC_INVALID_INPUT: provider_id must be > 0".to_string());
        }
        return Ok(Some(id));
    }
    Ok(None)
}

fn normalize_model_filter(model: Option<&str>) -> Option<String> {
    let raw = model?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(if raw.len() > 200 {
        raw[..200].to_string()
    } else {
        raw.to_string()
    })
}

fn compute_start_ts(conn: &Connection, period: CostPeriodV1) -> Result<Option<i64>, String> {
    let sql = match period {
        CostPeriodV1::AllTime | CostPeriodV1::Custom => return Ok(None),
        CostPeriodV1::Daily => {
            "SELECT CAST(strftime('%s','now','localtime','start of day','utc') AS INTEGER)"
        }
        CostPeriodV1::Weekly => {
            "SELECT CAST(strftime('%s','now','localtime','start of day','-6 days','utc') AS INTEGER)"
        }
        CostPeriodV1::Monthly => {
            "SELECT CAST(strftime('%s','now','localtime','start of month','utc') AS INTEGER)"
        }
    };

    let ts = conn
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("DB_ERROR: failed to compute period start ts: {e}"))?;
    Ok(Some(ts))
}

fn compute_bounds_v1(
    conn: &Connection,
    period: CostPeriodV1,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<(Option<i64>, Option<i64>, TrendBucket), String> {
    let bucket = match period {
        CostPeriodV1::Daily => TrendBucket::Hour,
        _ => TrendBucket::Day,
    };

    match period {
        CostPeriodV1::Custom => {
            let start_ts = start_ts
                .ok_or_else(|| "SEC_INVALID_INPUT: custom period requires start_ts".to_string())?;
            let end_ts = end_ts
                .ok_or_else(|| "SEC_INVALID_INPUT: custom period requires end_ts".to_string())?;
            if start_ts >= end_ts {
                return Err(
                    "SEC_INVALID_INPUT: custom range requires start_ts < end_ts".to_string()
                );
            }
            Ok((Some(start_ts), Some(end_ts), bucket))
        }
        _ => Ok((compute_start_ts(conn, period)?, None, bucket)),
    }
}

fn cost_usd_from_femto(v: i64) -> f64 {
    (v.max(0) as f64) / USD_FEMTO_DENOM
}

pub fn summary_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
) -> Result<CostSummaryV1, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();

    let sql = format!(
        r#"
SELECT
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
  SUM(
    CASE WHEN (
      status >= 200 AND status < 300 AND error_code IS NULL AND
      cost_usd_femto IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS cost_covered_success,
  SUM(COALESCE(cost_usd_femto, 0)) AS total_cost_usd_femto
FROM request_logs
WHERE excluded_from_stats = 0
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
AND (?4 IS NULL OR final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
"#,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    conn.query_row(
        &sql,
        params![start_ts, end_ts, cli_key, provider_id, model],
        |row| {
            let requests_total: i64 = row.get("requests_total")?;
            let requests_success: i64 = row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
            let requests_failed: i64 = row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0);
            let cost_covered_success: i64 = row
                .get::<_, Option<i64>>("cost_covered_success")?
                .unwrap_or(0);
            let total_cost_usd_femto: i64 = row
                .get::<_, Option<i64>>("total_cost_usd_femto")?
                .unwrap_or(0)
                .max(0);

            let total_cost_usd = cost_usd_from_femto(total_cost_usd_femto);
            let avg_cost_usd_per_covered_success = if cost_covered_success > 0 {
                Some(total_cost_usd / (cost_covered_success as f64))
            } else {
                None
            };

            Ok(CostSummaryV1 {
                requests_total: requests_total.max(0),
                requests_success: requests_success.max(0),
                requests_failed: requests_failed.max(0),
                cost_covered_success: cost_covered_success.max(0),
                total_cost_usd,
                avg_cost_usd_per_covered_success,
            })
        },
    )
    .map_err(|e| format!("DB_ERROR: failed to query cost summary: {e}"))
}

pub fn trend_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
) -> Result<Vec<CostTrendRowV1>, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, bucket) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();

    let (select_fields, group_by_fields, order_by_fields) = match bucket {
        TrendBucket::Hour => (
            "strftime('%Y-%m-%d', created_at, 'unixepoch','localtime') AS day, CAST(strftime('%H', created_at, 'unixepoch','localtime') AS INTEGER) AS hour",
            "day, hour",
            "day ASC, hour ASC",
        ),
        TrendBucket::Day => (
            "strftime('%Y-%m-%d', created_at, 'unixepoch','localtime') AS day, NULL AS hour",
            "day",
            "day ASC",
        ),
    };

    let sql = format!(
        r#"
SELECT
  {select_fields},
  COUNT(*) AS requests_success,
  SUM(CASE WHEN cost_usd_femto IS NOT NULL THEN 1 ELSE 0 END) AS cost_covered_success,
  SUM(COALESCE(cost_usd_femto, 0)) AS total_cost_usd_femto
FROM request_logs
WHERE excluded_from_stats = 0
AND status >= 200 AND status < 300 AND error_code IS NULL
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
AND (?4 IS NULL OR final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
GROUP BY {group_by_fields}
ORDER BY {order_by_fields}
"#,
        select_fields = select_fields,
        group_by_fields = group_by_fields,
        order_by_fields = order_by_fields,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare cost trend query: {e}"))?;
    let rows = stmt
        .query_map(
            params![start_ts, end_ts, cli_key, provider_id, model],
            |row| {
                let day: String = row.get("day")?;
                let hour: Option<i64> = row.get("hour")?;
                let requests_success: i64 =
                    row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
                let cost_covered_success: i64 = row
                    .get::<_, Option<i64>>("cost_covered_success")?
                    .unwrap_or(0);
                let total_cost_usd_femto: i64 = row
                    .get::<_, Option<i64>>("total_cost_usd_femto")?
                    .unwrap_or(0)
                    .max(0);

                Ok(CostTrendRowV1 {
                    day,
                    hour,
                    cost_usd: cost_usd_from_femto(total_cost_usd_femto),
                    requests_success: requests_success.max(0),
                    cost_covered_success: cost_covered_success.max(0),
                })
            },
        )
        .map_err(|e| format!("DB_ERROR: failed to run cost trend query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read cost trend row: {e}"))?);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn breakdown_provider_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
    limit: usize,
) -> Result<Vec<CostProviderBreakdownRowV1>, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();
    let limit = limit.clamp(1, 200) as i64;

    let sql = format!(
        r#"
SELECT
  r.cli_key AS cli_key,
  COALESCE(r.final_provider_id, 0) AS provider_id,
  COALESCE(p.name, 'Unknown') AS provider_name,
  COUNT(*) AS requests_success,
  SUM(CASE WHEN r.cost_usd_femto IS NOT NULL THEN 1 ELSE 0 END) AS cost_covered_success,
  SUM(COALESCE(r.cost_usd_femto, 0)) AS total_cost_usd_femto
FROM request_logs r
LEFT JOIN providers p ON p.id = r.final_provider_id
WHERE r.excluded_from_stats = 0
AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
AND (?1 IS NULL OR r.created_at >= ?1)
AND (?2 IS NULL OR r.created_at < ?2)
AND (?3 IS NULL OR r.cli_key = ?3)
AND (?4 IS NULL OR r.final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
GROUP BY r.cli_key, provider_id, provider_name
ORDER BY total_cost_usd_femto DESC, requests_success DESC, provider_name ASC
LIMIT ?6
"#,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare provider breakdown query: {e}"))?;
    let rows = stmt
        .query_map(
            params![start_ts, end_ts, cli_key, provider_id, model, limit],
            |row| {
                let cli_key: String = row.get("cli_key")?;
                let provider_id: i64 = row.get("provider_id")?;
                let provider_name: String = row.get("provider_name")?;
                let requests_success: i64 =
                    row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
                let cost_covered_success: i64 = row
                    .get::<_, Option<i64>>("cost_covered_success")?
                    .unwrap_or(0);
                let total_cost_usd_femto: i64 = row
                    .get::<_, Option<i64>>("total_cost_usd_femto")?
                    .unwrap_or(0)
                    .max(0);

                Ok(CostProviderBreakdownRowV1 {
                    cli_key,
                    provider_id: provider_id.max(0),
                    provider_name,
                    requests_success: requests_success.max(0),
                    cost_covered_success: cost_covered_success.max(0),
                    cost_usd: cost_usd_from_femto(total_cost_usd_femto),
                })
            },
        )
        .map_err(|e| format!("DB_ERROR: failed to run provider breakdown query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read provider row: {e}"))?);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn breakdown_model_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
    limit: usize,
) -> Result<Vec<CostModelBreakdownRowV1>, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();
    let limit = limit.clamp(1, 200) as i64;

    let sql = format!(
        r#"
SELECT
  {model_key_expr} AS model_key,
  COUNT(*) AS requests_success,
  SUM(CASE WHEN cost_usd_femto IS NOT NULL THEN 1 ELSE 0 END) AS cost_covered_success,
  SUM(COALESCE(cost_usd_femto, 0)) AS total_cost_usd_femto
FROM request_logs
WHERE excluded_from_stats = 0
AND status >= 200 AND status < 300 AND error_code IS NULL
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
AND (?4 IS NULL OR final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
GROUP BY model_key
ORDER BY total_cost_usd_femto DESC, requests_success DESC, model_key ASC
LIMIT ?6
"#,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare model breakdown query: {e}"))?;
    let rows = stmt
        .query_map(
            params![start_ts, end_ts, cli_key, provider_id, model, limit],
            |row| {
                let model: String = row.get("model_key")?;
                let requests_success: i64 =
                    row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
                let cost_covered_success: i64 = row
                    .get::<_, Option<i64>>("cost_covered_success")?
                    .unwrap_or(0);
                let total_cost_usd_femto: i64 = row
                    .get::<_, Option<i64>>("total_cost_usd_femto")?
                    .unwrap_or(0)
                    .max(0);

                Ok(CostModelBreakdownRowV1 {
                    model,
                    requests_success: requests_success.max(0),
                    cost_covered_success: cost_covered_success.max(0),
                    cost_usd: cost_usd_from_femto(total_cost_usd_femto),
                })
            },
        )
        .map_err(|e| format!("DB_ERROR: failed to run model breakdown query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read model row: {e}"))?);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn scatter_cli_provider_model_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
    limit: usize,
) -> Result<Vec<CostScatterCliProviderModelRowV1>, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();
    let limit = limit.clamp(1, 5000) as i64;

    let sql = format!(
        r#"
SELECT
  r.cli_key AS cli_key,
  COALESCE(p.name, 'Unknown') AS provider_name,
  {model_key_expr} AS model_key,
  COUNT(*) AS requests_success,
  SUM(r.cost_usd_femto) AS total_cost_usd_femto,
  SUM(CASE WHEN r.duration_ms IS NULL OR r.duration_ms < 0 THEN 0 ELSE r.duration_ms END) AS total_duration_ms
FROM request_logs r
LEFT JOIN providers p ON p.id = r.final_provider_id
WHERE r.excluded_from_stats = 0
AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
AND r.cost_usd_femto IS NOT NULL
AND (?1 IS NULL OR r.created_at >= ?1)
AND (?2 IS NULL OR r.created_at < ?2)
AND (?3 IS NULL OR r.cli_key = ?3)
AND (?4 IS NULL OR r.final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
GROUP BY r.cli_key, provider_name, model_key
ORDER BY total_cost_usd_femto DESC, total_duration_ms DESC, requests_success DESC, cli_key ASC, provider_name ASC, model_key ASC
LIMIT ?6
"#,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare cost scatter query: {e}"))?;
    let rows = stmt
        .query_map(
            params![start_ts, end_ts, cli_key, provider_id, model, limit],
            |row| {
                let cli_key: String = row.get("cli_key")?;
                let provider_name: String = row.get("provider_name")?;
                let model: String = row.get("model_key")?;
                let requests_success: i64 =
                    row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
                let total_cost_usd_femto: i64 = row
                    .get::<_, Option<i64>>("total_cost_usd_femto")?
                    .unwrap_or(0)
                    .max(0);
                let total_duration_ms: i64 = row
                    .get::<_, Option<i64>>("total_duration_ms")?
                    .unwrap_or(0)
                    .max(0);

                Ok(CostScatterCliProviderModelRowV1 {
                    cli_key,
                    provider_name,
                    model,
                    requests_success: requests_success.max(0),
                    total_cost_usd: cost_usd_from_femto(total_cost_usd_femto),
                    total_duration_ms: total_duration_ms.max(0),
                })
            },
        )
        .map_err(|e| format!("DB_ERROR: failed to run cost scatter query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read cost scatter row: {e}"))?);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn top_requests_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
    limit: usize,
) -> Result<Vec<CostTopRequestRowV1>, String> {
    let conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();
    let limit = limit.clamp(1, 200) as i64;

    let sql = format!(
        r#"
SELECT
  r.id AS log_id,
  r.trace_id AS trace_id,
  r.cli_key AS cli_key,
  r.method AS method,
  r.path AS path,
  r.requested_model AS requested_model,
  COALESCE(r.final_provider_id, 0) AS provider_id,
  COALESCE(p.name, 'Unknown') AS provider_name,
  r.duration_ms AS duration_ms,
  r.ttfb_ms AS ttfb_ms,
  r.cost_usd_femto AS cost_usd_femto,
  r.cost_multiplier AS cost_multiplier,
  r.created_at AS created_at
FROM request_logs r
LEFT JOIN providers p ON p.id = r.final_provider_id
WHERE r.excluded_from_stats = 0
AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
AND r.cost_usd_femto IS NOT NULL
AND (?1 IS NULL OR r.created_at >= ?1)
AND (?2 IS NULL OR r.created_at < ?2)
AND (?3 IS NULL OR r.cli_key = ?3)
AND (?4 IS NULL OR r.final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
ORDER BY r.cost_usd_femto DESC, r.created_at_ms DESC, r.id DESC
LIMIT ?6
"#,
        model_key_expr = SQL_MODEL_KEY_EXPR
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare top requests query: {e}"))?;
    let rows = stmt
        .query_map(
            params![start_ts, end_ts, cli_key, provider_id, model, limit],
            |row| {
                let log_id: i64 = row.get("log_id")?;
                let trace_id: String = row.get("trace_id")?;
                let cli_key: String = row.get("cli_key")?;
                let method: String = row.get("method")?;
                let path: String = row.get("path")?;
                let requested_model: Option<String> = row.get("requested_model")?;
                let provider_id: i64 = row.get("provider_id")?;
                let provider_name: String = row.get("provider_name")?;
                let duration_ms: i64 = row.get("duration_ms")?;
                let ttfb_ms: Option<i64> = row.get("ttfb_ms")?;
                let cost_usd_femto: i64 = row.get("cost_usd_femto")?;
                let cost_multiplier: f64 = row.get("cost_multiplier")?;
                let created_at: i64 = row.get("created_at")?;

                Ok(CostTopRequestRowV1 {
                    log_id,
                    trace_id,
                    cli_key,
                    method,
                    path,
                    requested_model,
                    provider_id: provider_id.max(0),
                    provider_name,
                    duration_ms: duration_ms.max(0),
                    ttfb_ms,
                    cost_usd: cost_usd_from_femto(cost_usd_femto),
                    cost_multiplier,
                    created_at,
                })
            },
        )
        .map_err(|e| format!("DB_ERROR: failed to run top requests query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read top request row: {e}"))?);
    }
    Ok(out)
}

fn has_any_cost_usage(usage: &cost::CostUsage) -> bool {
    usage.input_tokens > 0
        || usage.output_tokens > 0
        || usage.cache_read_input_tokens > 0
        || usage.cache_creation_input_tokens > 0
        || usage.cache_creation_5m_input_tokens > 0
        || usage.cache_creation_1h_input_tokens > 0
}

#[allow(clippy::too_many_arguments)]
pub fn backfill_missing_v1(
    db: &db::Db,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    model: Option<&str>,
    max_rows: usize,
) -> Result<CostBackfillReportV1, String> {
    let mut conn = db.open_connection()?;

    let period = parse_period_v1(period)?;
    let (start_ts, end_ts, _) = compute_bounds_v1(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    let provider_id = normalize_provider_id_filter(provider_id)?;
    let model = normalize_model_filter(model);
    let model = model.as_deref();

    let max_rows = max_rows.clamp(1, 10_000) as i64;

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start sqlite transaction: {e}"))?;

    let mut report = CostBackfillReportV1 {
        scanned: 0,
        updated: 0,
        skipped_no_model: 0,
        skipped_no_usage: 0,
        skipped_no_price: 0,
        skipped_other: 0,
        capped: false,
        max_rows,
    };

    {
        let mut stmt_candidates = tx
            .prepare(&format!(
                r#"
SELECT
  id,
  cli_key,
  requested_model,
  cost_multiplier,
  input_tokens,
  output_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens
FROM request_logs
WHERE excluded_from_stats = 0
AND status >= 200 AND status < 300 AND error_code IS NULL
AND cost_usd_femto IS NULL
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
AND (?4 IS NULL OR final_provider_id = ?4)
AND (?5 IS NULL OR {model_key_expr} = ?5)
ORDER BY created_at_ms DESC, id DESC
LIMIT ?6
"#,
                model_key_expr = SQL_MODEL_KEY_EXPR
            ))
            .map_err(|e| format!("DB_ERROR: failed to prepare backfill candidates query: {e}"))?;

        let mut stmt_price = tx
            .prepare("SELECT price_json FROM model_prices WHERE cli_key = ?1 AND model = ?2")
            .map_err(|e| format!("DB_ERROR: failed to prepare model_prices query: {e}"))?;

        let mut stmt_update = tx
            .prepare(
                "UPDATE request_logs SET cost_usd_femto = ?1 WHERE id = ?2 AND cost_usd_femto IS NULL",
            )
            .map_err(|e| format!("DB_ERROR: failed to prepare backfill update: {e}"))?;

        let rows = stmt_candidates
            .query_map(
                params![start_ts, end_ts, cli_key, provider_id, model, max_rows],
                |row| {
                    Ok((
                        row.get::<_, i64>("id")?,
                        row.get::<_, String>("cli_key")?,
                        row.get::<_, Option<String>>("requested_model")?,
                        row.get::<_, f64>("cost_multiplier")?,
                        row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                        row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                        row.get::<_, Option<i64>>("cache_read_input_tokens")?
                            .unwrap_or(0),
                        row.get::<_, Option<i64>>("cache_creation_input_tokens")?
                            .unwrap_or(0),
                        row.get::<_, Option<i64>>("cache_creation_5m_input_tokens")?
                            .unwrap_or(0),
                        row.get::<_, Option<i64>>("cache_creation_1h_input_tokens")?
                            .unwrap_or(0),
                    ))
                },
            )
            .map_err(|e| format!("DB_ERROR: failed to run backfill candidates query: {e}"))?;

        for row in rows {
            let (
                id,
                cli_key,
                requested_model,
                cost_multiplier,
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                cache_creation_5m_input_tokens,
                cache_creation_1h_input_tokens,
            ) = row.map_err(|e| format!("DB_ERROR: failed to read backfill candidate row: {e}"))?;

            report.scanned = report.scanned.saturating_add(1);

            let model = requested_model
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| if v.len() > 200 { &v[..200] } else { v });

            let Some(model) = model else {
                report.skipped_no_model = report.skipped_no_model.saturating_add(1);
                continue;
            };

            let usage = cost::CostUsage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                cache_creation_5m_input_tokens,
                cache_creation_1h_input_tokens,
            };

            if !has_any_cost_usage(&usage) {
                report.skipped_no_usage = report.skipped_no_usage.saturating_add(1);
                continue;
            }

            let price_json: Option<String> = stmt_price
                .query_row(params![cli_key, model], |row| row.get::<_, String>(0))
                .optional()
                .unwrap_or(None);

            let Some(price_json) = price_json else {
                report.skipped_no_price = report.skipped_no_price.saturating_add(1);
                continue;
            };

            let multiplier = if cost_multiplier.is_finite() && cost_multiplier > 0.0 {
                cost_multiplier
            } else {
                1.0
            };

            let cost_usd_femto =
                cost::calculate_cost_usd_femto(&usage, &price_json, multiplier, &cli_key, model);
            let Some(cost_usd_femto) = cost_usd_femto else {
                report.skipped_other = report.skipped_other.saturating_add(1);
                continue;
            };

            let changed = stmt_update
                .execute(params![cost_usd_femto, id])
                .map_err(|e| format!("DB_ERROR: failed to update cost_usd_femto: {e}"))?;
            if changed > 0 {
                report.updated = report.updated.saturating_add(1);
            } else {
                report.skipped_other = report.skipped_other.saturating_add(1);
            }
        }
    }

    report.capped = report.scanned >= max_rows;

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit backfill transaction: {e}"))?;

    Ok(report)
}
