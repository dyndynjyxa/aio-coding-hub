use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection, OptionalExtension, Statement};
use std::collections::HashMap;

use super::filters::SqlValues;
use super::{extract_final_provider, has_valid_provider_key, ProviderKey, UsagePeriodV2};

#[derive(Debug, Clone, Copy)]
pub(super) enum TrendBucketV1 {
    Hour,
    Day,
    Month,
}

pub(super) fn bucket_for_period(period: UsagePeriodV2) -> TrendBucketV1 {
    match period {
        UsagePeriodV2::Daily => TrendBucketV1::Hour,
        UsagePeriodV2::AllTime => TrendBucketV1::Month,
        UsagePeriodV2::Weekly | UsagePeriodV2::Monthly | UsagePeriodV2::Custom => {
            TrendBucketV1::Day
        }
    }
}

/// SQL LIMIT for the top-provider CTE: `None`/`Some(0)` = unlimited (-1),
/// otherwise clamped to 1..=200.
pub(super) fn normalize_trend_limit(limit: Option<usize>) -> i64 {
    match limit {
        None | Some(0) => -1,
        Some(v) => v.clamp(1, 200) as i64,
    }
}

/// (select_fields, group_by_fields) for a trend bucket, aliased on `r`.
pub(super) fn bucket_select_and_group(bucket: TrendBucketV1) -> (&'static str, &'static str) {
    match bucket {
        TrendBucketV1::Hour => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch','localtime') AS day, CAST(strftime('%H', r.created_at, 'unixepoch','localtime') AS INTEGER) AS hour",
            "day, hour",
        ),
        TrendBucketV1::Day => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch','localtime') AS day, NULL AS hour",
            "day",
        ),
        TrendBucketV1::Month => (
            "strftime('%Y-%m', r.created_at, 'unixepoch','localtime') AS day, NULL AS hour",
            "day",
        ),
    }
}

/// Resolves a display name per (cli_key, provider_id): trims the joined
/// `providers.name`, falls back to `attempts_json` extraction, and drops
/// names that fail `has_valid_provider_key`. Memoized per key.
pub(super) struct ProviderNameResolver<'conn> {
    stmt: Statement<'conn>,
    range_params: SqlValues,
    cache: HashMap<(String, i64), Option<String>>,
}

impl<'conn> ProviderNameResolver<'conn> {
    pub(super) fn new(
        conn: &'conn Connection,
        fallback_where_clause: &str,
        cx2cc_filter_clause: &str,
        range_params: SqlValues,
    ) -> Result<Self, String> {
        let sql = format!(
            r#"
SELECT attempts_json
FROM request_logs r
WHERE r.excluded_from_stats = 0
AND r.final_provider_id = ?1
AND r.cli_key = ?2
{fallback_where_clause}
{cx2cc_filter_clause}
LIMIT 1
"#
        );
        let stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare provider name fallback query: {e}"))?;
        Ok(Self {
            stmt,
            range_params,
            cache: HashMap::new(),
        })
    }

    pub(super) fn resolve(
        &mut self,
        cli_key: &str,
        provider_id: i64,
        joined_name: Option<&str>,
    ) -> Result<Option<String>, String> {
        let name_key = (cli_key.to_string(), provider_id);
        if let Some(v) = self.cache.get(&name_key) {
            return Ok(v.clone());
        }

        let mut provider_name = joined_name
            .map(str::trim)
            .filter(|v| !v.is_empty() && *v != "Unknown")
            .map(str::to_string);

        if provider_name.is_none() {
            let mut fallback_params: SqlValues =
                vec![provider_id.into(), cli_key.to_string().into()];
            fallback_params.extend(self.range_params.clone());
            let attempts_json: Option<String> = self
                .stmt
                .query_row(params_from_iter(fallback_params), |r| r.get(0))
                .optional()
                .map_err(|e| db_err!("failed to query provider name fallback: {e}"))?;

            if let Some(attempts_json) = attempts_json {
                let extracted = extract_final_provider(cli_key, &attempts_json);
                let extracted_name = extracted.provider_name.trim();
                if !extracted_name.is_empty() && extracted_name != "Unknown" {
                    provider_name = Some(extracted_name.to_string());
                }
            }
        }

        if let Some(provider_name_str) = provider_name.as_deref() {
            let key = ProviderKey {
                cli_key: cli_key.to_string(),
                provider_id,
                provider_name: provider_name_str.to_string(),
            };
            if !has_valid_provider_key(&key) {
                provider_name = None;
            }
        }

        self.cache.insert(name_key, provider_name.clone());
        Ok(provider_name)
    }
}
