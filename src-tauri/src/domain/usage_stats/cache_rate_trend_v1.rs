use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use std::collections::HashMap;

use super::filters::{
    build_optional_range_cli_provider_filters, build_optional_range_filters_with_offset,
    sql_exclude_cx2cc_gateway_bridge_clause, SqlValues,
};
use super::{
    extract_final_provider, has_valid_provider_key, resolve_query_params,
    sql_effective_input_tokens_expr_with_alias, ProviderKey, UsagePeriodV2,
    UsageProviderCacheRateTrendRowV1, UsageQueryParams,
};

#[derive(Debug, Clone, Copy)]
enum TrendBucketV1 {
    Hour,
    Day,
    Month,
}

fn bucket_for_period(period: UsagePeriodV2) -> TrendBucketV1 {
    match period {
        UsagePeriodV2::Daily => TrendBucketV1::Hour,
        UsagePeriodV2::AllTime => TrendBucketV1::Month,
        UsagePeriodV2::Weekly | UsagePeriodV2::Monthly | UsagePeriodV2::Custom => {
            TrendBucketV1::Day
        }
    }
}

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
    let limit = match query.limit {
        None => -1,
        Some(0) => -1,
        Some(v) => v.clamp(1, 200) as i64,
    };

    let (select_fields, group_by_fields, order_by_fields) = match bucket {
        TrendBucketV1::Hour => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch','localtime') AS day, CAST(strftime('%H', r.created_at, 'unixepoch','localtime') AS INTEGER) AS hour",
            "day, hour",
            "day ASC, hour ASC",
        ),
        TrendBucketV1::Day => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch','localtime') AS day, NULL AS hour",
            "day",
            "day ASC",
        ),
        TrendBucketV1::Month => (
            "strftime('%Y-%m', r.created_at, 'unixepoch','localtime') AS day, NULL AS hour",
            "day",
            "day ASC",
        ),
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
  SUM({denom_expr}) AS denom_tokens,
  SUM(COALESCE(r.cache_read_input_tokens, 0)) AS cache_read_input_tokens,
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
ORDER BY {order_by_fields}, denom_tokens DESC
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

    let fallback_sql = format!(
        r#"
SELECT attempts_json
FROM request_logs r
WHERE r.excluded_from_stats = 0
AND r.final_provider_id = ?1
AND r.cli_key = ?2
{fallback_where_clause}
{cx2cc_filter_clause}
LIMIT 1
"#,
        fallback_where_clause = fallback_where_clause,
        cx2cc_filter_clause = cx2cc_filter_clause
    );
    let mut stmt_fallback_name = conn
        .prepare(&fallback_sql)
        .map_err(|e| db_err!("failed to prepare provider name fallback query: {e}"))?;

    let mut name_cache: HashMap<(String, i64), Option<String>> = HashMap::new();

    let mut out = Vec::new();
    for row in items {
        let name_key = (row.cli_key.clone(), row.provider_id);
        let provider_name = match name_cache.get(&name_key) {
            Some(v) => v.clone(),
            None => {
                let mut provider_name = row
                    .provider_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty() && *v != "Unknown")
                    .map(str::to_string);

                if provider_name.is_none() {
                    let mut fallback_params: SqlValues =
                        vec![row.provider_id.into(), row.cli_key.clone().into()];
                    fallback_params.extend(fallback_range_params.clone());
                    let attempts_json: Option<String> = stmt_fallback_name
                        .query_row(params_from_iter(fallback_params), |r| r.get(0))
                        .optional()
                        .map_err(|e| db_err!("failed to query provider name fallback: {e}"))?;

                    if let Some(attempts_json) = attempts_json {
                        let extracted = extract_final_provider(&row.cli_key, &attempts_json);
                        let extracted_name = extracted.provider_name.trim();
                        if !extracted_name.is_empty() && extracted_name != "Unknown" {
                            provider_name = Some(extracted_name.to_string());
                        }
                    }
                }

                if let Some(provider_name_str) = provider_name.as_deref() {
                    let key = ProviderKey {
                        cli_key: row.cli_key.clone(),
                        provider_id: row.provider_id,
                        provider_name: provider_name_str.to_string(),
                    };
                    if !has_valid_provider_key(&key) {
                        provider_name = None;
                    }
                }

                name_cache.insert(name_key.clone(), provider_name.clone());
                provider_name
            }
        };

        let Some(provider_name) = provider_name else {
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

#[allow(clippy::too_many_arguments)]
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
