//! Usage: Request log queries and attempts decoding.

use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::Deserialize;
use std::collections::HashMap;

use super::costing::cost_usd_from_femto;
use super::{RequestLogDetail, RequestLogRouteHop, RequestLogSummary};

const CLAUDE_VISIBLE_LOG_PATH: &str = "/v1/messages";
const CLAUDE_VISIBLE_LOG_CONDITION: &str = "(cli_key != 'claude' OR path = '/v1/messages')";

/// Common SELECT fields for request_logs queries (summary view).
const REQUEST_LOG_SUMMARY_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  session_id,
  method,
  path,
  excluded_from_stats,
  special_settings_json,
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
  created_at,
  provider_chain_json,
  error_details_json
";

/// Common SELECT fields for request_logs queries (detail view).
const REQUEST_LOG_DETAIL_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  session_id,
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
  created_at,
  provider_chain_json,
  error_details_json
";

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub(super) struct AttemptRow {
    provider_id: i64,
    provider_name: String,
    outcome: String,
    status: Option<i64>,
    error_code: Option<String>,
    decision: Option<String>,
    reason: Option<String>,
    session_reuse: Option<bool>,
}

pub(super) fn parse_attempts(attempts_json: &str) -> Vec<AttemptRow> {
    serde_json::from_str(attempts_json).unwrap_or_default()
}

pub(super) fn start_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    let first = attempts
        .iter()
        .find(|a| a.outcome != "skipped")
        .or_else(|| attempts.first());

    match first {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn final_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    let picked = attempts
        .iter()
        .rev()
        .find(|a| a.outcome == "success")
        .or_else(|| attempts.iter().rev().find(|a| a.outcome != "skipped"))
        .or_else(|| attempts.last());

    match picked {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn route_from_attempts(attempts: &[AttemptRow]) -> Vec<RequestLogRouteHop> {
    let mut out: Vec<RequestLogRouteHop> = Vec::new();
    let mut last_provider_id: i64 = 0;
    let mut last_hop_attempt_count: i64 = 0;
    for attempt in attempts {
        if attempt.provider_id <= 0 {
            continue;
        }
        if attempt.outcome == "skipped" {
            continue;
        }
        if attempt.provider_id == last_provider_id {
            // 同一 provider 连续尝试，累加计数
            last_hop_attempt_count += 1;
            if let Some(hop) = out.last_mut() {
                hop.attempts = last_hop_attempt_count;
            }
            continue;
        }
        last_provider_id = attempt.provider_id;
        last_hop_attempt_count = 1;

        let ok = attempts
            .iter()
            .any(|row| row.provider_id == attempt.provider_id && row.outcome == "success");

        let picked = if ok {
            attempts
                .iter()
                .find(|row| row.provider_id == attempt.provider_id && row.outcome == "success")
                .or_else(|| {
                    attempts
                        .iter()
                        .rev()
                        .find(|row| row.provider_id == attempt.provider_id)
                })
        } else {
            attempts
                .iter()
                .rev()
                .find(|row| row.provider_id == attempt.provider_id)
        };

        let (status, error_code, decision, reason) = match picked {
            Some(row) => (
                row.status,
                row.error_code.clone(),
                row.decision.clone(),
                row.reason.clone(),
            ),
            None => (None, None, None, None),
        };

        out.push(RequestLogRouteHop {
            provider_id: attempt.provider_id,
            provider_name: attempt.provider_name.clone(),
            ok,
            attempts: 1,
            skipped: false,
            status,
            error_code,
            decision,
            reason,
        });
    }
    out
}

#[derive(Debug, Clone, Default)]
struct SourceProviderInfo {
    source_provider_id: Option<i64>,
    source_provider_name: Option<String>,
}

fn normalize_source_provider_name(name: Option<String>) -> Option<String> {
    name.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn load_source_provider_info_map(
    conn: &Connection,
    bridge_provider_ids: &[i64],
) -> crate::shared::error::AppResult<HashMap<i64, SourceProviderInfo>> {
    let ids: Vec<i64> = bridge_provider_ids
        .iter()
        .copied()
        .filter(|id| *id > 0)
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql = format!(
        r#"
SELECT
  bridge.id,
  bridge.source_provider_id,
  source.name
FROM providers bridge
LEFT JOIN providers source ON source.id = bridge.source_provider_id
WHERE bridge.id IN ({placeholders})
  AND bridge.source_provider_id IS NOT NULL
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare provider source query: {e}"))?;
    let mut rows = stmt
        .query(params_from_iter(ids.iter()))
        .map_err(|e| db_err!("failed to query provider sources: {e}"))?;

    let mut out = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| db_err!("failed to read provider source row: {e}"))?
    {
        let bridge_id: i64 = row
            .get(0)
            .map_err(|e| db_err!("invalid provider source bridge id: {e}"))?;
        let source_provider_id: Option<i64> = row
            .get(1)
            .map_err(|e| db_err!("invalid provider source id: {e}"))?;
        let source_provider_name: Option<String> = row
            .get(2)
            .map_err(|e| db_err!("invalid provider source name: {e}"))?;

        out.insert(
            bridge_id,
            SourceProviderInfo {
                source_provider_id,
                source_provider_name: normalize_source_provider_name(source_provider_name),
            },
        );
    }

    Ok(out)
}

fn attach_source_provider_info(
    conn: &Connection,
    items: &mut [RequestLogSummary],
) -> crate::shared::error::AppResult<()> {
    let ids: Vec<i64> = items.iter().map(|item| item.final_provider_id).collect();
    let info_by_bridge_id = load_source_provider_info_map(conn, &ids)?;

    for item in items.iter_mut() {
        if let Some(info) = info_by_bridge_id.get(&item.final_provider_id) {
            item.final_provider_source_id = info.source_provider_id;
            item.final_provider_source_name = info.source_provider_name.clone();
        }
    }

    Ok(())
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<RequestLogSummary, rusqlite::Error> {
    let attempts_json: String = row.get("attempts_json")?;
    let attempts = parse_attempts(&attempts_json);
    let attempt_count = attempts.len() as i64;
    let (start_provider_id, start_provider_name) = start_provider_from_attempts(&attempts);
    let (final_provider_id, final_provider_name) = final_provider_from_attempts(&attempts);
    let route = route_from_attempts(&attempts);
    // has_failover: 真正切换过 provider（route 中有多个 hop，skipped 已被过滤）
    let has_failover = route.len() > 1;
    let session_reuse = attempts
        .iter()
        .any(|row| row.session_reuse.unwrap_or(false));
    let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

    Ok(RequestLogSummary {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        session_id: row.get("session_id")?,
        method: row.get("method")?,
        path: row.get("path")?,
        excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
        special_settings_json: row.get("special_settings_json")?,
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
        final_provider_source_id: None,
        final_provider_source_name: None,
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
        provider_chain_json: row.get("provider_chain_json").unwrap_or(None),
        error_details_json: row.get("error_details_json").unwrap_or(None),
    })
}

fn row_to_detail(row: &rusqlite::Row<'_>) -> Result<RequestLogDetail, rusqlite::Error> {
    let attempts_json: String = row.get("attempts_json")?;
    let attempts = parse_attempts(&attempts_json);
    let (final_provider_id, final_provider_name) = final_provider_from_attempts(&attempts);
    let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

    Ok(RequestLogDetail {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        session_id: row.get("session_id")?,
        method: row.get("method")?,
        path: row.get("path")?,
        query: row.get("query")?,
        excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
        special_settings_json: row.get("special_settings_json")?,
        status: row.get("status")?,
        error_code: row.get("error_code")?,
        duration_ms: row.get("duration_ms")?,
        ttfb_ms: row.get("ttfb_ms")?,
        attempts_json,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
        total_tokens: row.get("total_tokens")?,
        cache_read_input_tokens: row.get("cache_read_input_tokens")?,
        cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
        cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
        cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
        usage_json: row.get("usage_json")?,
        requested_model: row.get("requested_model")?,
        final_provider_id,
        final_provider_name,
        final_provider_source_id: None,
        final_provider_source_name: None,
        cost_usd,
        cost_multiplier: row.get("cost_multiplier")?,
        created_at_ms: row.get("created_at_ms")?,
        created_at: row.get("created_at")?,
        provider_chain_json: row.get("provider_chain_json").unwrap_or(None),
        error_details_json: row.get("error_details_json").unwrap_or(None),
    })
}

fn attach_source_provider_info_to_detail(
    conn: &Connection,
    item: &mut RequestLogDetail,
) -> crate::shared::error::AppResult<()> {
    let info_by_bridge_id = load_source_provider_info_map(conn, &[item.final_provider_id])?;
    if let Some(info) = info_by_bridge_id.get(&item.final_provider_id) {
        item.final_provider_source_id = info.source_provider_id;
        item.final_provider_source_name = info.source_provider_name.clone();
    }
    Ok(())
}

pub fn list_recent(
    db: &db::Db,
    cli_key: &str,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let sql = if cli_key == "claude" {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND path = ?2 ORDER BY created_at_ms DESC, id DESC LIMIT ?3",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    } else {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 ORDER BY created_at_ms DESC, id DESC LIMIT ?2",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = if cli_key == "claude" {
        stmt.query_map(
            params![cli_key, CLAUDE_VISIBLE_LOG_PATH, limit as i64],
            row_to_summary,
        )
    } else {
        stmt.query_map(params![cli_key, limit as i64], row_to_summary)
    }
    .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_recent_all(
    db: &db::Db,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    let conn = db.open_connection()?;

    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} ORDER BY created_at_ms DESC, id DESC LIMIT ?1",
        REQUEST_LOG_SUMMARY_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![limit as i64], row_to_summary)
        .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_after_id(
    db: &db::Db,
    cli_key: &str,
    after_id: i64,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = if cli_key == "claude" {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND path = ?2 AND id > ?3 ORDER BY id ASC LIMIT ?4",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    } else {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = if cli_key == "claude" {
        stmt.query_map(
            params![cli_key, CLAUDE_VISIBLE_LOG_PATH, after_id, limit as i64],
            row_to_summary,
        )
    } else {
        stmt.query_map(params![cli_key, after_id, limit as i64], row_to_summary)
    }
    .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_after_id_all(
    db: &db::Db,
    after_id: i64,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} AND id > ?1 ORDER BY id ASC LIMIT ?2",
        REQUEST_LOG_SUMMARY_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![after_id, limit as i64], row_to_summary)
        .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn get_by_id(db: &db::Db, log_id: i64) -> crate::shared::error::AppResult<RequestLogDetail> {
    let conn = db.open_connection()?;
    let sql = format!(
        "SELECT{}FROM request_logs WHERE id = ?1 AND {}",
        REQUEST_LOG_DETAIL_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut item = conn
        .query_row(&sql, params![log_id], row_to_detail)
        .optional()
        .map_err(|e| db_err!("failed to query request_log: {e}"))?
        .ok_or_else(|| {
            crate::shared::error::AppError::from("DB_NOT_FOUND: request_log not found".to_string())
        })?;
    attach_source_provider_info_to_detail(&conn, &mut item)?;
    Ok(item)
}

pub fn get_by_trace_id(
    db: &db::Db,
    trace_id: &str,
) -> crate::shared::error::AppResult<Option<RequestLogDetail>> {
    if trace_id.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: trace_id is required".to_string().into());
    }

    let conn = db.open_connection()?;
    let sql = format!(
        "SELECT{}FROM request_logs WHERE trace_id = ?1 AND {}",
        REQUEST_LOG_DETAIL_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut item = conn
        .query_row(&sql, params![trace_id], row_to_detail)
        .optional()
        .map_err(|e| db_err!("failed to query request_log: {e}"))?;
    if let Some(detail) = item.as_mut() {
        attach_source_provider_info_to_detail(&conn, detail)?;
    }
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::{
        final_provider_from_attempts, get_by_id, get_by_trace_id, list_after_id_all, list_recent,
        list_recent_all, load_source_provider_info_map, parse_attempts, route_from_attempts,
        start_provider_from_attempts,
    };
    use crate::db;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn seed_request_log(conn: &Connection, id: i64, trace_id: &str, cli_key: &str, path: &str) {
        conn.execute(
            r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, session_id, method, path, query, excluded_from_stats,
  special_settings_json, status, error_code, duration_ms, ttfb_ms, attempts_json,
  input_tokens, output_tokens, total_tokens, cache_read_input_tokens,
  cache_creation_input_tokens, cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens, usage_json, requested_model, cost_usd_femto,
  cost_multiplier, created_at_ms, created_at, final_provider_id
) VALUES (?1, ?2, ?3, NULL, 'POST', ?4, NULL, 0, NULL, 200, NULL, 10, 5, '[]',
  NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'model', NULL, 1.0, ?5, ?6, 0)
"#,
            rusqlite::params![id, trace_id, cli_key, path, id * 1000, id],
        )
        .unwrap();
    }

    #[test]
    fn route_excludes_skipped_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_RATE_LIMITED","decision":"skip","reason":"provider skipped by rate limit"},
                {"provider_id":2,"provider_name":"B","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        // skipped 的 provider 不出现在 route 中
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 2);
        assert!(!route[0].skipped);
        assert!(route[0].ok);
        assert_eq!(route[0].attempts, 1);
    }

    #[test]
    fn start_and_final_provider_prefer_non_skipped_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_RATE_LIMITED","decision":"skip","reason":"provider skipped by rate limit"},
                {"provider_id":2,"provider_name":"B","outcome":"failed","status":429,"error_code":"GW_UPSTREAM_4XX","decision":"abort","reason":"status=429"}
            ]"#,
        );

        let (start_id, start_name) = start_provider_from_attempts(&attempts);
        assert_eq!(start_id, 2);
        assert_eq!(start_name, "B");

        let (final_id, final_name) = final_provider_from_attempts(&attempts);
        assert_eq!(final_id, 2);
        assert_eq!(final_name, "B");
    }

    #[test]
    fn route_counts_consecutive_same_provider_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"retry","reason":"status=500"},
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"retry","reason":"status=500"},
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"failover","reason":"status=500"},
                {"provider_id":2,"provider_name":"B","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 2);
        assert_eq!(route[0].provider_id, 1);
        assert_eq!(route[0].attempts, 3);
        assert_eq!(route[0].provider_name, "A");
        assert!(!route[0].ok);
        assert_eq!(route[1].provider_id, 2);
        assert_eq!(route[1].attempts, 1);
        assert_eq!(route[1].provider_name, "B");
        assert!(route[1].ok);
    }

    #[test]
    fn route_single_provider_single_attempt() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 1);
        assert_eq!(route[0].attempts, 1);
        assert!(route[0].ok);
    }

    #[test]
    fn started_attempt_still_resolves_provider_for_abort_logs() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":12,"provider_name":"Claude Bridge","outcome":"started","status":null,"error_code":null,"decision":null,"reason":null}
            ]"#,
        );

        let (final_id, final_name) = final_provider_from_attempts(&attempts);
        assert_eq!(final_id, 12);
        assert_eq!(final_name, "Claude Bridge");

        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 12);
        assert_eq!(route[0].provider_name, "Claude Bridge");
        assert!(!route[0].ok);
    }

    #[test]
    fn loads_source_provider_names_for_bridge_providers() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  source_provider_id INTEGER
);
INSERT INTO providers (id, name, source_provider_id) VALUES (7, 'OpenAI Primary', NULL);
INSERT INTO providers (id, name, source_provider_id) VALUES (12, 'Claude Bridge', 7);
"#,
        )
        .unwrap();

        let info = load_source_provider_info_map(&conn, &[12, 99]).unwrap();
        let bridge = info.get(&12).expect("bridge provider source info");

        assert_eq!(bridge.source_provider_id, Some(7));
        assert_eq!(
            bridge.source_provider_name.as_deref(),
            Some("OpenAI Primary")
        );
        assert!(!info.contains_key(&99));
    }

    #[test]
    fn list_queries_hide_claude_non_messages_rows() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-claude-messages", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "trace-claude-count",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "trace-codex", "codex", "/v1/responses");
        drop(conn);

        let all = list_recent_all(&db, 10).unwrap();
        assert_eq!(
            all.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![3, 1]
        );

        let claude = list_recent(&db, "claude", 10).unwrap();
        assert_eq!(
            claude.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1]
        );

        let after = list_after_id_all(&db, 1, 10).unwrap();
        assert_eq!(
            after.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![3]
        );
    }

    #[test]
    fn detail_queries_hide_claude_non_messages_rows() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-claude-messages", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "trace-claude-count",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "trace-codex", "codex", "/v1/responses");
        drop(conn);

        let visible = get_by_id(&db, 1).unwrap();
        assert_eq!(visible.id, 1);

        let hidden = get_by_id(&db, 2).unwrap_err().to_string();
        assert!(hidden.contains("request_log not found"));

        let hidden_by_trace = get_by_trace_id(&db, "trace-claude-count").unwrap();
        assert!(hidden_by_trace.is_none());

        let visible_by_trace = get_by_trace_id(&db, "trace-codex").unwrap();
        assert_eq!(visible_by_trace.as_ref().map(|item| item.id), Some(3));
    }

    #[test]
    fn summary_and_detail_expose_session_id() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        conn.execute(
            r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, session_id, method, path, query, excluded_from_stats,
  special_settings_json, status, error_code, duration_ms, ttfb_ms, attempts_json,
  input_tokens, output_tokens, total_tokens, cache_read_input_tokens,
  cache_creation_input_tokens, cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens, usage_json, requested_model, cost_usd_femto,
  cost_multiplier, created_at_ms, created_at, final_provider_id
) VALUES (?1, ?2, ?3, ?4, 'POST', ?5, NULL, 0, NULL, 200, NULL, 10, 5, '[]',
  NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'model', NULL, 1.0, ?6, ?7, 0)
"#,
            rusqlite::params![
                11_i64,
                "trace-session-id",
                "codex",
                "sess-123",
                "/v1/responses",
                11_000_i64,
                11_i64
            ],
        )
        .unwrap();
        drop(conn);

        let summary = list_recent_all(&db, 10).unwrap();
        assert_eq!(summary[0].session_id.as_deref(), Some("sess-123"));

        let detail = get_by_id(&db, 11).unwrap();
        assert_eq!(detail.session_id.as_deref(), Some("sess-123"));
    }
}
