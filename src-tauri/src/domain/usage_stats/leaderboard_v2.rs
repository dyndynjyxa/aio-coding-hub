use crate::db;
use rusqlite::{params, Connection};
use std::collections::HashMap;

use super::{
    compute_bounds_v2, effective_input_tokens, effective_total_tokens, extract_final_provider,
    has_valid_provider_key, is_success, normalize_cli_filter, parse_period_v2, parse_scope_v2,
    sql_effective_total_tokens_expr, ProviderAgg, ProviderKey, UsageLeaderboardRow, UsageScopeV2,
    SQL_EFFECTIVE_INPUT_TOKENS_EXPR,
};

pub(super) fn leaderboard_v2_with_conn(
    conn: &Connection,
    scope: UsageScopeV2,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    limit: usize,
) -> Result<Vec<UsageLeaderboardRow>, String> {
    let effective_input_expr = SQL_EFFECTIVE_INPUT_TOKENS_EXPR;
    let effective_total_expr = sql_effective_total_tokens_expr();

    let mut out: Vec<UsageLeaderboardRow> = match scope {
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
  SUM({effective_total_expr}) AS total_tokens,
  SUM({effective_input_expr}) AS input_tokens,
  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
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
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
GROUP BY cli_key
"#,
                effective_input_expr = effective_input_expr,
                effective_total_expr = effective_total_expr.as_str()
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("DB_ERROR: failed to prepare cli leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params![start_ts, end_ts, cli_key], |row| {
                    let key: String = row.get("key")?;
                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
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
                        total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
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
                    };

                    Ok(agg.to_leaderboard_row(key.clone(), key))
                })
                .map_err(|e| format!("DB_ERROR: failed to run cli leaderboard query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| format!("DB_ERROR: failed to read cli row: {e}"))?);
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
  SUM({effective_total_expr}) AS total_tokens,
  SUM({effective_input_expr}) AS input_tokens,
  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
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
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
GROUP BY COALESCE(NULLIF(requested_model, ''), 'Unknown')
"#,
                effective_input_expr = effective_input_expr,
                effective_total_expr = effective_total_expr.as_str()
            );
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("DB_ERROR: failed to prepare model leaderboard query: {e}"))?;

            let rows = stmt
                .query_map(params![start_ts, end_ts, cli_key], |row| {
                    let key: String = row.get("key")?;
                    let agg = ProviderAgg {
                        requests_total: row.get("requests_total")?,
                        requests_success: row
                            .get::<_, Option<i64>>("requests_success")?
                            .unwrap_or(0),
                        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
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
                        total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
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
                    };

                    Ok(agg.to_leaderboard_row(key.clone(), key))
                })
                .map_err(|e| format!("DB_ERROR: failed to run model leaderboard query: {e}"))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| format!("DB_ERROR: failed to read model row: {e}"))?);
            }
            items
        }
        UsageScopeV2::Provider => {
            let mut stmt = conn
                .prepare(
                    r#"
SELECT
  cli_key,
  attempts_json,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens
FROM request_logs
WHERE excluded_from_stats = 0
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR created_at < ?2)
AND (?3 IS NULL OR cli_key = ?3)
"#,
                )
                .map_err(|e| {
                    format!("DB_ERROR: failed to prepare provider leaderboard query: {e}")
                })?;

            let rows = stmt
                .query_map(params![start_ts, end_ts, cli_key], |row| {
                    let row_cli_key: String = row.get("cli_key")?;
                    let attempts_json: String = row.get("attempts_json")?;
                    let status: Option<i64> = row.get("status")?;
                    let error_code: Option<String> = row.get("error_code")?;
                    let duration_ms: i64 = row.get("duration_ms")?;
                    let ttfb_ms: Option<i64> = row.get("ttfb_ms")?;
                    let input_tokens: Option<i64> = row.get("input_tokens")?;
                    let output_tokens: Option<i64> = row.get("output_tokens")?;
                    let cache_read_input_tokens: Option<i64> =
                        row.get("cache_read_input_tokens")?;
                    let cache_creation_input_tokens: Option<i64> =
                        row.get("cache_creation_input_tokens")?;
                    let cache_creation_5m_input_tokens: Option<i64> =
                        row.get("cache_creation_5m_input_tokens")?;
                    let cache_creation_1h_input_tokens: Option<i64> =
                        row.get("cache_creation_1h_input_tokens")?;

                    let key = extract_final_provider(&row_cli_key, &attempts_json);
                    let success = is_success(status, error_code.as_deref());

                    let ttfb_ms = match ttfb_ms {
                        Some(v) if v < duration_ms => Some(v),
                        _ => None,
                    };
                    let ttfb_ms_for_rate = ttfb_ms.unwrap_or(duration_ms);
                    let generation_ms = duration_ms.saturating_sub(ttfb_ms_for_rate);
                    let (rate_generation_ms, rate_output_tokens) =
                        if success && generation_ms > 0 && output_tokens.is_some() {
                            (generation_ms, output_tokens.unwrap_or(0))
                        } else {
                            (0, 0)
                        };

                    let raw_input_tokens = input_tokens.unwrap_or(0);
                    let raw_output_tokens = output_tokens.unwrap_or(0);
                    let cache_read_input_tokens = cache_read_input_tokens.unwrap_or(0);
                    let cache_creation_input_tokens = cache_creation_input_tokens.unwrap_or(0);

                    let effective_input_tokens_value = effective_input_tokens(
                        &row_cli_key,
                        raw_input_tokens,
                        cache_read_input_tokens,
                    );
                    let effective_total_tokens_value = effective_total_tokens(
                        effective_input_tokens_value,
                        raw_output_tokens,
                        cache_creation_input_tokens,
                        cache_read_input_tokens,
                    );

                    Ok((
                        key,
                        ProviderAgg {
                            requests_total: 1,
                            requests_success: if success { 1 } else { 0 },
                            requests_failed: if success { 0 } else { 1 },
                            success_duration_ms_sum: if success { duration_ms } else { 0 },
                            success_ttfb_ms_sum: if success { ttfb_ms.unwrap_or(0) } else { 0 },
                            success_ttfb_ms_count: if success && ttfb_ms.is_some() { 1 } else { 0 },
                            success_generation_ms_sum: rate_generation_ms,
                            success_output_tokens_for_rate_sum: rate_output_tokens,
                            input_tokens: effective_input_tokens_value,
                            output_tokens: raw_output_tokens,
                            total_tokens: effective_total_tokens_value,
                            cache_read_input_tokens,
                            cache_creation_input_tokens,
                            cache_creation_5m_input_tokens: cache_creation_5m_input_tokens
                                .unwrap_or(0),
                            cache_creation_1h_input_tokens: cache_creation_1h_input_tokens
                                .unwrap_or(0),
                        },
                    ))
                })
                .map_err(|e| format!("DB_ERROR: failed to run provider leaderboard query: {e}"))?;

            let mut agg: HashMap<ProviderKey, ProviderAgg> = HashMap::new();
            for row in rows {
                let (key, add) = row.map_err(|e| {
                    format!("DB_ERROR: failed to read provider leaderboard row: {e}")
                })?;

                if !has_valid_provider_key(&key) {
                    continue;
                }

                let entry = agg.entry(key).or_default();
                entry.merge(add);
            }

            agg.into_iter()
                .map(|(k, v)| {
                    let key = format!("{}:{}", k.cli_key, k.provider_id);
                    let name = format!("{}/{}", k.cli_key, k.provider_name);
                    v.to_leaderboard_row(key, name)
                })
                .collect()
        }
    };

    out.sort_by(|a, b| {
        b.requests_total
            .cmp(&a.requests_total)
            .then_with(|| b.total_tokens.cmp(&a.total_tokens))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.key.cmp(&b.key))
    });
    out.truncate(limit.clamp(1, 200));
    Ok(out)
}

pub fn leaderboard_v2(
    db: &db::Db,
    scope: &str,
    period: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    limit: usize,
) -> Result<Vec<UsageLeaderboardRow>, String> {
    let conn = db.open_connection()?;
    let scope = parse_scope_v2(scope)?;
    let period = parse_period_v2(period)?;
    let (start_ts, end_ts) = compute_bounds_v2(&conn, period, start_ts, end_ts)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    leaderboard_v2_with_conn(&conn, scope, start_ts, end_ts, cli_key, limit)
}
