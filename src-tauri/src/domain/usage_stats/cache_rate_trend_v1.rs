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
    UsageProviderCacheRateTrendRowV1, UsageQueryParams,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct ProviderCacheRateTrendQuery<'a> {
    pub period: UsagePeriodV2,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub limit: Option<usize>,
    pub exclude_cx2cc_gateway_bridge: bool,
}

pub(super) fn provider_cache_rate_trend_v1_with_conn(
    conn: &Connection,
    query: ProviderCacheRateTrendQuery<'_>,
) -> Result<Vec<UsageProviderCacheRateTrendRowV1>, String> {
    let bucket = bucket_for_period(query.period);
    let limit = normalize_trend_limit(query.limit);

    let (select_fields, group_by_fields) = bucket_select_and_group(bucket);
    let order_by_fields = match bucket {
        TrendBucketV1::Hour => "b.day ASC, b.hour ASC",
        TrendBucketV1::Day | TrendBucketV1::Month => "b.day ASC",
    };

    let effective_input_expr = sql_effective_input_tokens_expr_with_alias("r");
    let denom_expr = format!(
        "({effective_input_expr}) + COALESCE(r.cache_creation_input_tokens, 0) + COALESCE(r.cache_read_input_tokens, 0)",
        effective_input_expr = effective_input_expr
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
WITH bucketed AS (
  SELECT
    {select_fields},
    r.cli_key AS cli_key,
    r.final_provider_id AS provider_id,
    MAX(p.name) AS provider_name,
    SUM({denom_expr}) AS denom_tokens,
    SUM(COALESCE(r.cache_read_input_tokens, 0)) AS cache_read_input_tokens,
    COUNT(*) AS requests_success
  FROM request_logs r
  LEFT JOIN providers p ON p.id = r.final_provider_id
  WHERE r.excluded_from_stats = 0
  AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  {where_clause}
  {cx2cc_filter_clause}
  GROUP BY {group_by_fields}, r.cli_key, r.final_provider_id
),
top_providers AS (
  SELECT
    cli_key,
    provider_id,
    SUM(denom_tokens) AS denom_tokens
  FROM bucketed
  GROUP BY cli_key, provider_id
  ORDER BY denom_tokens DESC
  LIMIT ?{limit_bind_idx}
)
SELECT
  b.day,
  b.hour,
  b.cli_key,
  b.provider_id,
  b.provider_name,
  b.denom_tokens,
  b.cache_read_input_tokens,
  b.requests_success
FROM bucketed b
JOIN top_providers tp
  ON tp.cli_key = b.cli_key
 AND tp.provider_id = b.provider_id
ORDER BY {order_by_fields}, b.denom_tokens DESC
"#,
        denom_expr = denom_expr,
        select_fields = select_fields,
        group_by_fields = group_by_fields,
        order_by_fields = order_by_fields,
        where_clause = where_clause,
        cx2cc_filter_clause = cx2cc_filter_clause,
        limit_bind_idx = where_params.len() + 1,
    );

    #[derive(Debug, Clone)]
    struct RawRow {
        day: String,
        hour: Option<i64>,
        cli_key: String,
        provider_id: i64,
        provider_name: Option<String>,
        denom_tokens: i64,
        cache_read_input_tokens: i64,
        requests_success: i64,
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare provider cache trend query: {e}"))?;

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
                    denom_tokens: row
                        .get::<_, Option<i64>>("denom_tokens")?
                        .unwrap_or(0)
                        .max(0),
                    cache_read_input_tokens: row
                        .get::<_, Option<i64>>("cache_read_input_tokens")?
                        .unwrap_or(0)
                        .max(0),
                    requests_success: row
                        .get::<_, Option<i64>>("requests_success")?
                        .unwrap_or(0)
                        .max(0),
                })
            },
        )
        .map_err(|e| db_err!("failed to run provider cache trend query: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read cache trend row: {e}"))?);
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

        out.push(UsageProviderCacheRateTrendRowV1 {
            day: row.day,
            hour: row.hour,
            key: format!("{}:{}", row.cli_key, row.provider_id),
            name: format!("{}/{}", row.cli_key, provider_name),
            denom_tokens: row.denom_tokens,
            cache_read_input_tokens: row.cache_read_input_tokens,
            requests_success: row.requests_success,
        });
    }

    Ok(out)
}

pub fn provider_cache_rate_trend_v1(
    db: &db::Db,
    params: &UsageQueryParams,
    limit: Option<usize>,
) -> crate::shared::error::AppResult<Vec<UsageProviderCacheRateTrendRowV1>> {
    let conn = db.open_connection()?;
    let mut params = params.clone();
    params.day_start_hour = None;
    let resolved = resolve_query_params(&conn, &params)?;
    Ok(provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
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
