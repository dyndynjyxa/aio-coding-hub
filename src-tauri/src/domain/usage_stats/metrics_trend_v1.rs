use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection};

use super::filters::{
    build_optional_range_cli_provider_filters, build_optional_range_filters_with_offset,
    sql_exclude_cx2cc_gateway_bridge_clause,
};
use super::trend_common::{
    bucket_for_period, bucket_select_and_group, normalize_trend_limit, ProviderNameResolver,
    TrendBucketV1,
};
use super::{
    resolve_query_params, sql_effective_input_tokens_expr_with_alias, UsagePeriodV2,
    UsageProviderMetricsTrendRowV1, UsageQueryParams,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct ProviderMetricsTrendQuery<'a> {
    pub period: UsagePeriodV2,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub limit: Option<usize>,
    pub exclude_cx2cc_gateway_bridge: bool,
}

// Only ttfb_ms present AND strictly below duration counts toward ttfb/rate
// aggregation — mirrors summary.rs / folders.rs validity guard so a bogus
// ttfb (e.g. >= duration, or NULL on non-streaming) never skews the average.
const TTFB_VALID: &str = "r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";

pub(super) fn provider_metrics_trend_v1_with_conn(
    conn: &Connection,
    query: ProviderMetricsTrendQuery<'_>,
) -> Result<Vec<UsageProviderMetricsTrendRowV1>, String> {
    let bucket = bucket_for_period(query.period);
    let limit = normalize_trend_limit(query.limit);

    let (select_fields, group_by_fields) = bucket_select_and_group(bucket);
    let order_by_fields = match bucket {
        TrendBucketV1::Hour => "day ASC, hour ASC",
        TrendBucketV1::Day | TrendBucketV1::Month => "day ASC",
    };

    let effective_input_expr = sql_effective_input_tokens_expr_with_alias("r");
    let denom_expr = format!(
        "({effective_input_expr}) + COALESCE(r.cache_creation_input_tokens, 0) + COALESCE(r.cache_read_input_tokens, 0)"
    );
    let (where_clause, where_params) = build_optional_range_cli_provider_filters(
        "r.created_at",
        "r.cli_key",
        "r.final_provider_id",
        query.start_ts,
        query.end_ts,
        query.cli_key,
        query.provider_id,
    );
    let (fallback_where_clause, fallback_range_params) =
        build_optional_range_filters_with_offset("r.created_at", query.start_ts, query.end_ts, 2);
    let cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);

    let sql = format!(
        r#"
WITH top_providers AS (
  SELECT
    r.cli_key AS cli_key,
    r.final_provider_id AS provider_id,
    SUM({denom_expr}) AS denom_tokens
  FROM request_logs r
  WHERE r.excluded_from_stats = 0
  AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  {where_clause}
  {cx2cc_filter_clause}
  GROUP BY r.cli_key, r.final_provider_id
  ORDER BY denom_tokens DESC
  LIMIT ?{limit_bind_idx}
)
SELECT
  {select_fields},
  r.cli_key AS cli_key,
  r.final_provider_id AS provider_id,
  MAX(p.name) AS provider_name,
  SUM(r.duration_ms) AS duration_ms_sum,
  SUM(CASE WHEN {ttfb_valid} THEN r.ttfb_ms ELSE 0 END) AS ttfb_ms_sum,
  SUM(CASE WHEN {ttfb_valid} THEN 1 ELSE 0 END) AS ttfb_ms_count,
  SUM(CASE WHEN {ttfb_valid} THEN (r.duration_ms - r.ttfb_ms) ELSE 0 END) AS generation_ms_sum,
  SUM(CASE WHEN {ttfb_valid} THEN COALESCE(r.output_tokens, 0) ELSE 0 END) AS output_tokens_for_rate_sum,
  COUNT(*) AS requests_success
FROM request_logs r
JOIN top_providers tp
  ON tp.cli_key = r.cli_key
 AND tp.provider_id = r.final_provider_id
LEFT JOIN providers p ON p.id = r.final_provider_id
WHERE r.excluded_from_stats = 0
AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
AND r.final_provider_id IS NOT NULL
AND r.final_provider_id > 0
{where_clause}
{cx2cc_filter_clause}
GROUP BY {group_by_fields}, r.cli_key, r.final_provider_id
ORDER BY {order_by_fields}, requests_success DESC
"#,
        limit_bind_idx = where_params.len() + 1,
        ttfb_valid = TTFB_VALID,
    );

    #[derive(Debug, Clone)]
    struct RawRow {
        day: String,
        hour: Option<i64>,
        cli_key: String,
        provider_id: i64,
        provider_name: Option<String>,
        duration_ms_sum: i64,
        ttfb_ms_sum: i64,
        ttfb_ms_count: i64,
        generation_ms_sum: i64,
        output_tokens_for_rate_sum: i64,
        requests_success: i64,
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare provider metrics trend query: {e}"))?;

    let rows = stmt
        .query_map(
            params_from_iter({
                let mut params = where_params.clone();
                params.push(limit.into());
                params
            }),
            |row| {
                Ok(RawRow {
                    day: row.get("day")?,
                    hour: row.get("hour")?,
                    cli_key: row.get("cli_key")?,
                    provider_id: row.get("provider_id")?,
                    provider_name: row.get("provider_name")?,
                    duration_ms_sum: row
                        .get::<_, Option<i64>>("duration_ms_sum")?
                        .unwrap_or(0)
                        .max(0),
                    ttfb_ms_sum: row
                        .get::<_, Option<i64>>("ttfb_ms_sum")?
                        .unwrap_or(0)
                        .max(0),
                    ttfb_ms_count: row
                        .get::<_, Option<i64>>("ttfb_ms_count")?
                        .unwrap_or(0)
                        .max(0),
                    generation_ms_sum: row
                        .get::<_, Option<i64>>("generation_ms_sum")?
                        .unwrap_or(0)
                        .max(0),
                    output_tokens_for_rate_sum: row
                        .get::<_, Option<i64>>("output_tokens_for_rate_sum")?
                        .unwrap_or(0)
                        .max(0),
                    requests_success: row
                        .get::<_, Option<i64>>("requests_success")?
                        .unwrap_or(0)
                        .max(0),
                })
            },
        )
        .map_err(|e| db_err!("failed to run provider metrics trend query: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read metrics trend row: {e}"))?);
    }

    let mut name_resolver = ProviderNameResolver::new(
        conn,
        &fallback_where_clause,
        &cx2cc_filter_clause,
        fallback_range_params,
    )?;

    let mut out = Vec::new();
    for row in items {
        let Some(provider_name) =
            name_resolver.resolve(&row.cli_key, row.provider_id, row.provider_name.as_deref())?
        else {
            continue;
        };

        let avg_duration_ms = if row.requests_success > 0 {
            Some(row.duration_ms_sum / row.requests_success)
        } else {
            None
        };
        let avg_ttfb_ms = if row.ttfb_ms_count > 0 {
            Some(row.ttfb_ms_sum / row.ttfb_ms_count)
        } else {
            None
        };
        let avg_output_tokens_per_second = if row.generation_ms_sum > 0 {
            Some(row.output_tokens_for_rate_sum as f64 / (row.generation_ms_sum as f64 / 1000.0))
        } else {
            None
        };

        out.push(UsageProviderMetricsTrendRowV1 {
            day: row.day,
            hour: row.hour,
            key: format!("{}:{}", row.cli_key, row.provider_id),
            name: format!("{}/{}", row.cli_key, provider_name),
            avg_duration_ms,
            avg_ttfb_ms,
            avg_output_tokens_per_second,
            requests_success: row.requests_success,
        });
    }

    Ok(out)
}

pub fn provider_metrics_trend_v1(
    db: &db::Db,
    params: &UsageQueryParams,
    limit: Option<usize>,
) -> crate::shared::error::AppResult<Vec<UsageProviderMetricsTrendRowV1>> {
    let conn = db.open_connection()?;
    // Trends bucket by calendar day/hour; a custom day-start offset would make
    // the range and the bucket labels disagree (same guard as cache trend).
    let mut params = params.clone();
    params.day_start_hour = None;
    let resolved = resolve_query_params(&conn, &params)?;
    Ok(provider_metrics_trend_v1_with_conn(
        &conn,
        ProviderMetricsTrendQuery {
            period: resolved.period,
            start_ts: resolved.start_ts,
            end_ts: resolved.end_ts,
            cli_key: resolved.cli_key,
            provider_id: resolved.provider_id,
            limit,
            exclude_cx2cc_gateway_bridge: resolved.exclude_cx2cc_gateway_bridge,
        },
    )?)
}
