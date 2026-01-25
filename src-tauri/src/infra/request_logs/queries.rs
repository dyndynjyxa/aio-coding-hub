//! Usage: Request log queries and attempts decoding.

use crate::db;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;

use super::costing::cost_usd_from_femto;
use super::{RequestLogDetail, RequestLogRouteHop, RequestLogSummary};

/// Common SELECT fields for request_logs queries (summary view).
const REQUEST_LOG_SUMMARY_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  method,
  path,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  attempts_json,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  cost_multiplier,
  created_at_ms,
  created_at
";

/// Common SELECT fields for request_logs queries (detail view).
const REQUEST_LOG_DETAIL_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  method,
  path,
  query,
  excluded_from_stats,
  special_settings_json,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  attempts_json,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  usage_json,
  requested_model,
  cost_usd_femto,
  cost_multiplier,
  created_at_ms,
  created_at
";

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct AttemptRow {
    provider_id: i64,
    provider_name: String,
    outcome: String,
    session_reuse: Option<bool>,
}

pub(super) fn parse_attempts(attempts_json: &str) -> Vec<AttemptRow> {
    serde_json::from_str(attempts_json).unwrap_or_default()
}

pub(super) fn start_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    match attempts.first() {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn final_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    let picked = attempts
        .iter()
        .rev()
        .find(|a| a.outcome == "success")
        .or_else(|| attempts.last());

    match picked {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn route_from_attempts(attempts: &[AttemptRow]) -> Vec<RequestLogRouteHop> {
    let mut out = Vec::new();
    let mut last_provider_id: i64 = 0;
    for attempt in attempts {
        if attempt.provider_id <= 0 {
            continue;
        }
        if attempt.provider_id == last_provider_id {
            continue;
        }
        last_provider_id = attempt.provider_id;

        let ok = attempts
            .iter()
            .any(|row| row.provider_id == attempt.provider_id && row.outcome == "success");

        out.push(RequestLogRouteHop {
            provider_id: attempt.provider_id,
            provider_name: attempt.provider_name.clone(),
            ok,
        });
    }
    out
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<RequestLogSummary, rusqlite::Error> {
    let attempts_json: String = row.get("attempts_json")?;
    let attempts = parse_attempts(&attempts_json);
    let attempt_count = attempts.len() as i64;
    let has_failover = attempt_count > 1;
    let (start_provider_id, start_provider_name) = start_provider_from_attempts(&attempts);
    let (final_provider_id, final_provider_name) = final_provider_from_attempts(&attempts);
    let route = route_from_attempts(&attempts);
    let session_reuse = attempts
        .iter()
        .any(|row| row.session_reuse.unwrap_or(false));
    let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

    Ok(RequestLogSummary {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        method: row.get("method")?,
        path: row.get("path")?,
        requested_model: row.get("requested_model")?,
        status: row.get("status")?,
        error_code: row.get("error_code")?,
        duration_ms: row.get("duration_ms")?,
        ttfb_ms: row.get("ttfb_ms")?,
        attempt_count,
        has_failover,
        start_provider_id,
        start_provider_name,
        final_provider_id,
        final_provider_name,
        route,
        session_reuse,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
        total_tokens: row.get("total_tokens")?,
        cache_read_input_tokens: row.get("cache_read_input_tokens")?,
        cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
        cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
        cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
        cost_usd,
        cost_multiplier: row.get("cost_multiplier")?,
        created_at_ms: row.get("created_at_ms")?,
        created_at: row.get("created_at")?,
    })
}

pub fn list_recent(
    db: &db::Db,
    cli_key: &str,
    limit: usize,
) -> Result<Vec<RequestLogSummary>, String> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let sql = format!("SELECT{}FROM request_logs WHERE cli_key = ?1 ORDER BY created_at_ms DESC, id DESC LIMIT ?2", REQUEST_LOG_SUMMARY_FIELDS);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key, limit as i64], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read request_log row: {e}"))?);
    }
    Ok(items)
}

pub fn list_recent_all(db: &db::Db, limit: usize) -> Result<Vec<RequestLogSummary>, String> {
    let conn = db.open_connection()?;

    let sql = format!("SELECT{}FROM request_logs ORDER BY created_at_ms DESC, id DESC LIMIT ?1", REQUEST_LOG_SUMMARY_FIELDS);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![limit as i64], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read request_log row: {e}"))?);
    }
    Ok(items)
}

pub fn list_after_id(
    db: &db::Db,
    cli_key: &str,
    after_id: i64,
    limit: usize,
) -> Result<Vec<RequestLogSummary>, String> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = format!("SELECT{}FROM request_logs WHERE cli_key = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3", REQUEST_LOG_SUMMARY_FIELDS);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key, after_id, limit as i64], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read request_log row: {e}"))?);
    }
    Ok(items)
}

pub fn list_after_id_all(
    db: &db::Db,
    after_id: i64,
    limit: usize,
) -> Result<Vec<RequestLogSummary>, String> {
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = format!("SELECT{}FROM request_logs WHERE id > ?1 ORDER BY id ASC LIMIT ?2", REQUEST_LOG_SUMMARY_FIELDS);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![after_id, limit as i64], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read request_log row: {e}"))?);
    }
    Ok(items)
}

pub fn get_by_id(db: &db::Db, log_id: i64) -> Result<RequestLogDetail, String> {
    let conn = db.open_connection()?;
    let sql = format!("SELECT{}FROM request_logs WHERE id = ?1", REQUEST_LOG_DETAIL_FIELDS);
    conn.query_row(
        &sql,
        params![log_id],
        |row| {
            let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

            Ok(RequestLogDetail {
                id: row.get("id")?,
                trace_id: row.get("trace_id")?,
                cli_key: row.get("cli_key")?,
                method: row.get("method")?,
                path: row.get("path")?,
                query: row.get("query")?,
                excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
                special_settings_json: row.get("special_settings_json")?,
                status: row.get("status")?,
                error_code: row.get("error_code")?,
                duration_ms: row.get("duration_ms")?,
                ttfb_ms: row.get("ttfb_ms")?,
                attempts_json: row.get("attempts_json")?,
                input_tokens: row.get("input_tokens")?,
                output_tokens: row.get("output_tokens")?,
                total_tokens: row.get("total_tokens")?,
                cache_read_input_tokens: row.get("cache_read_input_tokens")?,
                cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
                cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
                cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
                usage_json: row.get("usage_json")?,
                requested_model: row.get("requested_model")?,
                cost_usd,
                cost_multiplier: row.get("cost_multiplier")?,
                created_at_ms: row.get("created_at_ms")?,
                created_at: row.get("created_at")?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query request_log: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: request_log not found".to_string())
}

pub fn get_by_trace_id(db: &db::Db, trace_id: &str) -> Result<Option<RequestLogDetail>, String> {
    if trace_id.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: trace_id is required".to_string());
    }

    let conn = db.open_connection()?;
    let sql = format!("SELECT{}FROM request_logs WHERE trace_id = ?1", REQUEST_LOG_DETAIL_FIELDS);
    conn.query_row(
        &sql,
        params![trace_id],
        |row| {
            let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

            Ok(RequestLogDetail {
                id: row.get("id")?,
                trace_id: row.get("trace_id")?,
                cli_key: row.get("cli_key")?,
                method: row.get("method")?,
                path: row.get("path")?,
                query: row.get("query")?,
                excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
                special_settings_json: row.get("special_settings_json")?,
                status: row.get("status")?,
                error_code: row.get("error_code")?,
                duration_ms: row.get("duration_ms")?,
                ttfb_ms: row.get("ttfb_ms")?,
                attempts_json: row.get("attempts_json")?,
                input_tokens: row.get("input_tokens")?,
                output_tokens: row.get("output_tokens")?,
                total_tokens: row.get("total_tokens")?,
                cache_read_input_tokens: row.get("cache_read_input_tokens")?,
                cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
                cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
                cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
                usage_json: row.get("usage_json")?,
                requested_model: row.get("requested_model")?,
                cost_usd,
                cost_multiplier: row.get("cost_multiplier")?,
                created_at_ms: row.get("created_at_ms")?,
                created_at: row.get("created_at")?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query request_log: {e}"))
}
