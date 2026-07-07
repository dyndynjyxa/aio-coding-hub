use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection, Row};
use std::collections::{HashMap, HashSet};

use super::filters::{
    build_optional_range_cli_provider_filters, sql_exclude_cx2cc_gateway_bridge_clause,
};
use super::{
    sql_effective_input_tokens_expr_with_alias, sql_effective_total_tokens_expr_with_alias,
    ProviderAgg, UsageFolderOptionV1,
};

pub(super) const UNKNOWN_FOLDER_KEY: &str = "__unknown__";
pub(super) const UNKNOWN_FOLDER_NAME: &str = "未知文件夹";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UsageSessionLookupKey {
    pub cli_key: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct UsageResolvedFolder {
    pub cli_key: String,
    pub session_id: String,
    pub folder_name: String,
    pub folder_path: String,
}

#[derive(Debug, Clone)]
pub(super) struct UsageEventAgg {
    pub cli_key: String,
    pub session_id: Option<String>,
    pub bucket_key: Option<String>,
    pub bucket_provider_id: Option<i64>,
    pub bucket_provider_name: Option<String>,
    pub bucket_provider_attempts_json: Option<String>,
    pub hour: Option<i64>,
    pub requests_with_usage: i64,
    pub agg: ProviderAgg,
}

#[derive(Debug, Clone)]
pub(super) struct FolderIdentity {
    pub key: String,
    pub name: String,
    pub folder_path: Option<String>,
}

fn can_lookup_folder(cli_key: &str) -> bool {
    matches!(cli_key, "claude" | "codex")
}

fn lookup_key(cli_key: &str, session_id: &str) -> String {
    format!("{cli_key}:{session_id}")
}

pub(super) fn session_lookup_keys(rows: &[UsageEventAgg]) -> Vec<UsageSessionLookupKey> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for row in rows {
        let Some(session_id) = row.session_id.as_deref() else {
            continue;
        };
        if !can_lookup_folder(&row.cli_key) {
            continue;
        }
        let key = lookup_key(&row.cli_key, session_id);
        if !seen.insert(key) {
            continue;
        }
        out.push(UsageSessionLookupKey {
            cli_key: row.cli_key.clone(),
            session_id: session_id.to_string(),
        });
    }
    out
}

pub(super) fn resolved_folder_map(
    rows: Vec<UsageResolvedFolder>,
) -> HashMap<String, UsageResolvedFolder> {
    let mut out = HashMap::new();
    for row in rows {
        if row.folder_path.trim().is_empty() || row.folder_name.trim().is_empty() {
            continue;
        }
        out.insert(lookup_key(&row.cli_key, &row.session_id), row);
    }
    out
}

pub(super) fn unknown_folder_identity() -> FolderIdentity {
    FolderIdentity {
        key: UNKNOWN_FOLDER_KEY.to_string(),
        name: UNKNOWN_FOLDER_NAME.to_string(),
        folder_path: None,
    }
}

pub(super) fn folder_identity_for_row(
    row: &UsageEventAgg,
    resolved: &HashMap<String, UsageResolvedFolder>,
) -> FolderIdentity {
    let Some(session_id) = row.session_id.as_deref() else {
        return unknown_folder_identity();
    };
    if !can_lookup_folder(&row.cli_key) {
        return unknown_folder_identity();
    }
    let key = lookup_key(&row.cli_key, session_id);
    let Some(folder) = resolved.get(&key) else {
        return unknown_folder_identity();
    };
    let folder_path = folder.folder_path.trim();
    let folder_name = folder.folder_name.trim();
    if folder_path.is_empty() || folder_name.is_empty() {
        return unknown_folder_identity();
    }
    FolderIdentity {
        key: folder_path.to_string(),
        name: folder_name.to_string(),
        folder_path: Some(folder_path.to_string()),
    }
}

pub(super) fn row_to_agg(row: &Row<'_>) -> rusqlite::Result<ProviderAgg> {
    Ok(ProviderAgg {
        requests_total: row.get("requests_total")?,
        requests_success: row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0),
        requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
        total_duration_ms: row.get::<_, Option<i64>>("total_duration_ms")?.unwrap_or(0),
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
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn usage_event_rows(
    conn: &Connection,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
    bucket_sql: Option<&str>,
    include_hour: bool,
    exclude_cx2cc_gateway_bridge: bool,
) -> Result<Vec<UsageEventAgg>, String> {
    let effective_input_expr = sql_effective_input_tokens_expr_with_alias("r");
    let effective_total_expr = sql_effective_total_tokens_expr_with_alias("r");
    let (where_clause, where_params) = build_optional_range_cli_provider_filters(
        "r.created_at",
        "r.cli_key",
        "r.final_provider_id",
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    );
    let cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), exclude_cx2cc_gateway_bridge);

    let bucket_select = match bucket_sql {
        Some(sql) => format!(",\n  {sql} AS bucket_key"),
        None => String::new(),
    };
    let bucket_group = if bucket_sql.is_some() {
        ", bucket_key"
    } else {
        ""
    };
    let hour_select = if include_hour {
        ",\n  CAST(strftime('%H', r.created_at, 'unixepoch', 'localtime') AS INTEGER) AS hour"
    } else {
        ""
    };
    let hour_group = if include_hour { ", hour" } else { "" };

    let sql = format!(
        r#"
SELECT
  r.cli_key,
  NULLIF(TRIM(COALESCE(r.session_id, '')), '') AS session_id{bucket_select}{hour_select},
  MAX(r.final_provider_id) AS bucket_provider_id,
  MAX(NULLIF(TRIM(COALESCE(p.name, '')), '')) AS bucket_provider_name,
  MAX(r.attempts_json) AS bucket_provider_attempts_json,
  COUNT(*) AS requests_total,
  SUM(
    CASE WHEN (
      r.total_tokens IS NOT NULL OR
      r.input_tokens IS NOT NULL OR
      r.output_tokens IS NOT NULL OR
      r.cache_read_input_tokens IS NOT NULL OR
      r.cache_creation_input_tokens IS NOT NULL OR
      r.cache_creation_5m_input_tokens IS NOT NULL OR
      r.cache_creation_1h_input_tokens IS NOT NULL OR
      r.usage_json IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS requests_with_usage,
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
      r.cost_usd_femto IS NOT NULL
    ) THEN 1 ELSE 0 END
  ) AS cost_covered_success,
  SUM(
    CASE WHEN (
      r.status >= 200 AND r.status < 300 AND r.error_code IS NULL AND
      r.cost_usd_femto IS NOT NULL AND r.cost_usd_femto > 0
    ) THEN r.cost_usd_femto ELSE 0 END
  ) AS total_cost_usd_femto,
  SUM(r.duration_ms) AS total_duration_ms,
  MIN(CASE WHEN r.created_at_ms > 0 THEN r.created_at_ms ELSE r.created_at * 1000 END) AS first_request_created_at_ms,
  MAX(CASE WHEN r.created_at_ms > 0 THEN r.created_at_ms ELSE r.created_at * 1000 END) AS last_request_created_at_ms,
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
{where_clause}
{cx2cc_filter_clause}
GROUP BY r.cli_key, session_id{bucket_group}{hour_group}
"#,
        bucket_select = bucket_select,
        hour_select = hour_select,
        effective_input_expr = effective_input_expr,
        effective_total_expr = effective_total_expr.as_str(),
        where_clause = where_clause,
        cx2cc_filter_clause = cx2cc_filter_clause,
        bucket_group = bucket_group,
        hour_group = hour_group
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare usage event query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(where_params), |row| {
            Ok(UsageEventAgg {
                cli_key: row.get("cli_key")?,
                session_id: row.get("session_id")?,
                bucket_key: if bucket_sql.is_some() {
                    row.get("bucket_key")?
                } else {
                    None
                },
                bucket_provider_id: row.get("bucket_provider_id")?,
                bucket_provider_name: row.get("bucket_provider_name")?,
                bucket_provider_attempts_json: row.get("bucket_provider_attempts_json")?,
                hour: if include_hour { row.get("hour")? } else { None },
                requests_with_usage: row
                    .get::<_, Option<i64>>("requests_with_usage")?
                    .unwrap_or(0),
                agg: row_to_agg(row)?,
            })
        })
        .map_err(|e| db_err!("failed to run usage event query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read usage event row: {e}"))?);
    }
    Ok(out)
}

pub(super) fn filter_rows_by_folder_keys(
    rows: Vec<UsageEventAgg>,
    resolved: &HashMap<String, UsageResolvedFolder>,
    folder_keys: Option<&[String]>,
) -> Vec<UsageEventAgg> {
    let Some(folder_keys) = folder_keys else {
        return rows;
    };
    let wanted: HashSet<&str> = folder_keys.iter().map(String::as_str).collect();
    if wanted.is_empty() {
        return rows;
    }

    rows.into_iter()
        .filter(|row| {
            let identity = folder_identity_for_row(row, resolved);
            wanted.contains(identity.key.as_str())
        })
        .collect()
}

pub(super) fn folder_options_from_rows(
    rows: &[UsageEventAgg],
    resolved: &HashMap<String, UsageResolvedFolder>,
) -> Vec<UsageFolderOptionV1> {
    let mut by_folder: HashMap<String, (FolderIdentity, ProviderAgg)> = HashMap::new();
    for row in rows {
        let identity = folder_identity_for_row(row, resolved);
        let entry = by_folder
            .entry(identity.key.clone())
            .or_insert_with(|| (identity, ProviderAgg::default()));
        entry.1.merge(row.agg.clone());
    }

    let mut out: Vec<UsageFolderOptionV1> = by_folder
        .into_values()
        .map(|(identity, agg)| UsageFolderOptionV1 {
            key: identity.key,
            name: identity.name,
            folder_path: identity.folder_path,
            requests_total: agg.requests_total,
            total_tokens: agg.total_tokens,
        })
        .collect();

    out.sort_by(|a, b| {
        b.total_tokens
            .cmp(&a.total_tokens)
            .then_with(|| b.requests_total.cmp(&a.requests_total))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.key.cmp(&b.key))
    });
    out
}
