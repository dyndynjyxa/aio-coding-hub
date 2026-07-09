use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use std::collections::HashMap;

use super::filters::{
    build_optional_range_cli_provider_filters, build_optional_range_filters_with_offset,
    sql_exclude_cx2cc_gateway_bridge_clause, SqlValues,
};
use super::folders::{
    filter_rows_by_folder_keys, resolved_folder_map, session_lookup_keys, usage_event_rows,
    UsageEventAgg,
};
use super::{
    extract_final_provider, has_valid_provider_key, parse_scope_v2, resolve_query_params,
    sql_effective_input_tokens_expr_with_alias, sql_effective_total_tokens_expr,
    sql_effective_total_tokens_expr_with_alias, ProviderAgg, ProviderKey, UsageLeaderboardRow,
    UsageQueryParams, UsageResolvedFolder, UsageScopeV2, UsageSessionLookupKey,
    SQL_EFFECTIVE_INPUT_TOKENS_EXPR,
};

fn local_day_bucket_sql(timestamp_expr: &str, day_start_hour: i64) -> String {
    if day_start_hour == 0 {
        return format!("strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime')");
    }
    format!(
        "strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime', '-{day_start_hour} hours')"
    )
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
    let effective_input_expr = SQL_EFFECTIVE_INPUT_TOKENS_EXPR;
    let effective_total_expr = sql_effective_total_tokens_expr();
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
                effective_total_expr = effective_total_expr.as_str(),
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
  SUM({effective_total_expr}) AS total_tokens,
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
                effective_total_expr = effective_total_expr.as_str(),
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
  SUM({effective_total_expr}) AS total_tokens,
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
                effective_total_expr = effective_total_expr.as_str(),
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
            let effective_total_expr = sql_effective_total_tokens_expr_with_alias("r");

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
  SUM({effective_total_expr}) AS total_tokens,
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
                effective_total_expr = effective_total_expr,
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
                        total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
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
        out.sort_by(|a, b| b.key.cmp(&a.key));
    } else {
        out.sort_by(|a, b| {
            b.requests_total
                .cmp(&a.requests_total)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.key.cmp(&b.key))
        });
    }
    if let Some(limit) = limit {
        out.truncate(limit.clamp(1, 200));
    } else {
        out.truncate(200);
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
        UsageScopeV2::Cli => None,
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
    let lookup_keys = session_lookup_keys(&rows);
    let resolved = resolved_folder_map(folder_lookup(&lookup_keys));
    let rows = filter_rows_by_folder_keys(rows, &resolved, Some(params.folder_keys));

    let mut by_key: HashMap<String, (String, ProviderAgg)> = HashMap::new();
    for row in rows {
        let item = match params.scope {
            UsageScopeV2::Cli => {
                let key = row.cli_key.clone();
                Some((key.clone(), key))
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
                ))
            }
            UsageScopeV2::Model | UsageScopeV2::Day => {
                let Some(key) = row.bucket_key.clone() else {
                    continue;
                };
                Some((key.clone(), key))
            }
        };
        let Some((key, name)) = item else {
            continue;
        };
        let entry = by_key
            .entry(key)
            .or_insert_with(|| (name, ProviderAgg::default()));
        let mut agg = row.agg;
        if !matches!(params.scope, UsageScopeV2::Day) {
            agg.first_request_created_at_ms = None;
            agg.last_request_created_at_ms = None;
        }
        entry.1.merge(agg);
    }

    let mut out: Vec<UsageLeaderboardRow> = by_key
        .into_iter()
        .map(|(key, (name, agg))| agg.into_leaderboard_row(key, name))
        .collect();

    if matches!(params.scope, UsageScopeV2::Day) {
        out.sort_by(|a, b| b.key.cmp(&a.key));
    } else {
        out.sort_by(|a, b| {
            b.requests_total
                .cmp(&a.requests_total)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.key.cmp(&b.key))
        });
    }
    if let Some(limit) = params.limit {
        out.truncate(limit.clamp(1, 200));
    } else {
        out.truncate(200);
    }
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
    if let Some(folder_keys) = resolved.folder_keys.as_deref() {
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
            },
            folder_lookup,
        )?);
    }

    Ok(leaderboard_v2_with_conn_day_start(
        &conn,
        scope,
        resolved.start_ts,
        resolved.end_ts,
        resolved.cli_key,
        resolved.provider_id,
        limit,
        resolved.exclude_cx2cc_gateway_bridge,
        resolved.day_start_hour,
    )?)
}

#[cfg(test)]
mod tests {
    use super::local_day_bucket_sql;

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
}
