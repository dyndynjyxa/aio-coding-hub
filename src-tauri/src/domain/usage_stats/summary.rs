use crate::db;
use rusqlite::{params_from_iter, Connection};

use super::filters::build_optional_range_cli_provider_filters;
use super::{
    compute_start_ts, normalize_cli_filter, parse_range, resolve_query_params,
    sql_effective_total_tokens_expr, UsageQueryParams, UsageSummary,
    SQL_EFFECTIVE_INPUT_TOKENS_EXPR,
};

fn build_summary_where_clause(
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
) -> (String, Vec<rusqlite::types::Value>) {
    let (filter_sql, values) = build_optional_range_cli_provider_filters(
        "created_at",
        "cli_key",
        "final_provider_id",
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    );
    let clause = format!("excluded_from_stats = 0{filter_sql}");
    (clause, values)
}

pub(super) fn summary_query(
    conn: &Connection,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
) -> Result<UsageSummary, String> {
    let effective_input_expr = SQL_EFFECTIVE_INPUT_TOKENS_EXPR;
    let effective_total_expr = sql_effective_total_tokens_expr();
    let (where_sql, params_vec) =
        build_summary_where_clause(start_ts, end_ts, cli_key, provider_id);
    let sql = format!(
        r#"
	SELECT
	  COUNT(*) AS requests_total,
	  SUM(
	    CASE WHEN (
      total_tokens IS NOT NULL OR
      input_tokens IS NOT NULL OR
      output_tokens IS NOT NULL OR
      cache_read_input_tokens IS NOT NULL OR
      cache_creation_input_tokens IS NOT NULL OR
      cache_creation_5m_input_tokens IS NOT NULL OR
      cache_creation_1h_input_tokens IS NOT NULL OR
      usage_json IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_with_usage,
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
	  ) AS success_output_tokens_for_rate_sum,
	  SUM({effective_input_expr}) AS input_tokens,
	  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
	  SUM({effective_total_expr}) AS total_tokens,
	  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(cache_creation_5m_input_tokens, 0)) AS cache_creation_5m_input_tokens,
  SUM(COALESCE(cache_creation_1h_input_tokens, 0)) AS cache_creation_1h_input_tokens
	FROM request_logs
	WHERE {where_sql}
	"#,
        effective_input_expr = effective_input_expr,
        effective_total_expr = effective_total_expr.as_str(),
        where_sql = where_sql
    );

    conn.query_row(&sql, params_from_iter(params_vec), |row| {
        let requests_success = row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0);
        let success_duration_ms_sum = row
            .get::<_, Option<i64>>("success_duration_ms_sum")?
            .unwrap_or(0);
        let success_ttfb_ms_sum = row
            .get::<_, Option<i64>>("success_ttfb_ms_sum")?
            .unwrap_or(0);
        let success_ttfb_ms_count = row
            .get::<_, Option<i64>>("success_ttfb_ms_count")?
            .unwrap_or(0);
        let success_generation_ms_sum = row
            .get::<_, Option<i64>>("success_generation_ms_sum")?
            .unwrap_or(0);
        let success_output_tokens_for_rate_sum = row
            .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
            .unwrap_or(0);

        let avg_duration_ms = if requests_success > 0 {
            Some(success_duration_ms_sum / requests_success)
        } else {
            None
        };
        let avg_ttfb_ms = if success_ttfb_ms_count > 0 {
            Some(success_ttfb_ms_sum / success_ttfb_ms_count)
        } else {
            None
        };
        let avg_output_tokens_per_second = if success_generation_ms_sum > 0 {
            Some(
                success_output_tokens_for_rate_sum as f64
                    / (success_generation_ms_sum as f64 / 1000.0),
            )
        } else {
            None
        };

        let input_tokens = row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0);
        let output_tokens = row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0);
        let io_total_tokens = input_tokens.saturating_add(output_tokens);

        Ok(UsageSummary {
            requests_total: row.get::<_, i64>("requests_total")?,
            requests_with_usage: row
                .get::<_, Option<i64>>("requests_with_usage")?
                .unwrap_or(0),
            requests_success,
            requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
            cost_covered_success: row
                .get::<_, Option<i64>>("cost_covered_success")?
                .unwrap_or(0),
            avg_duration_ms,
            avg_ttfb_ms,
            avg_output_tokens_per_second,
            input_tokens,
            output_tokens,
            io_total_tokens,
            total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
            cache_read_input_tokens: row
                .get::<_, Option<i64>>("cache_read_input_tokens")?
                .unwrap_or(0),
            cache_creation_input_tokens: row
                .get::<_, Option<i64>>("cache_creation_input_tokens")?
                .unwrap_or(0),
            cache_creation_5m_input_tokens: row
                .get::<_, Option<i64>>("cache_creation_5m_input_tokens")?
                .unwrap_or(0),
            cache_creation_1h_input_tokens: row
                .get::<_, Option<i64>>("cache_creation_1h_input_tokens")?
                .unwrap_or(0),
        })
    })
    .map_err(|e| format!("DB_ERROR: failed to query usage summary: {e}"))
}

pub fn summary(
    db: &db::Db,
    range: &str,
    cli_key: Option<&str>,
) -> crate::shared::error::AppResult<UsageSummary> {
    let conn = db.open_connection()?;
    let range = parse_range(range)?;
    let start_ts = compute_start_ts(&conn, range)?;
    let cli_key = normalize_cli_filter(cli_key)?;

    Ok(summary_query(&conn, start_ts, None, cli_key, None)?)
}

pub fn summary_v2(
    db: &db::Db,
    params: &UsageQueryParams,
) -> crate::shared::error::AppResult<UsageSummary> {
    let conn = db.open_connection()?;
    let resolved = resolve_query_params(&conn, params)?;
    Ok(summary_query(
        &conn,
        resolved.start_ts,
        resolved.end_ts,
        resolved.cli_key,
        resolved.provider_id,
    )?)
}
