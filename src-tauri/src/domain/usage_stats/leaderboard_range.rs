use crate::db;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    compute_start_ts, normalize_cli_filter, parse_range, token_total, UsageDayRow, UsageProviderRow,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(super) struct ProviderKey {
    pub(super) cli_key: String,
    pub(super) provider_id: i64,
    pub(super) provider_name: String,
}

#[derive(Default)]
pub(super) struct ProviderAgg {
    pub(super) requests_total: i64,
    pub(super) requests_success: i64,
    pub(super) requests_failed: i64,
    pub(super) success_duration_ms_sum: i64,
    pub(super) success_ttfb_ms_sum: i64,
    pub(super) success_ttfb_ms_count: i64,
    pub(super) success_generation_ms_sum: i64,
    pub(super) success_output_tokens_for_rate_sum: i64,
    pub(super) input_tokens: i64,
    pub(super) output_tokens: i64,
    pub(super) total_tokens: i64,
    pub(super) cache_read_input_tokens: i64,
    pub(super) cache_creation_input_tokens: i64,
    pub(super) cache_creation_5m_input_tokens: i64,
    pub(super) cache_creation_1h_input_tokens: i64,
}

impl ProviderAgg {
    pub(super) fn merge(&mut self, add: ProviderAgg) {
        self.requests_total = self.requests_total.saturating_add(add.requests_total);
        self.requests_success = self.requests_success.saturating_add(add.requests_success);
        self.requests_failed = self.requests_failed.saturating_add(add.requests_failed);
        self.success_duration_ms_sum = self
            .success_duration_ms_sum
            .saturating_add(add.success_duration_ms_sum);
        self.success_ttfb_ms_sum = self
            .success_ttfb_ms_sum
            .saturating_add(add.success_ttfb_ms_sum);
        self.success_ttfb_ms_count = self
            .success_ttfb_ms_count
            .saturating_add(add.success_ttfb_ms_count);
        self.success_generation_ms_sum = self
            .success_generation_ms_sum
            .saturating_add(add.success_generation_ms_sum);
        self.success_output_tokens_for_rate_sum = self
            .success_output_tokens_for_rate_sum
            .saturating_add(add.success_output_tokens_for_rate_sum);
        self.input_tokens = self.input_tokens.saturating_add(add.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(add.output_tokens);
        self.total_tokens = self.total_tokens.saturating_add(add.total_tokens);
        self.cache_read_input_tokens = self
            .cache_read_input_tokens
            .saturating_add(add.cache_read_input_tokens);
        self.cache_creation_input_tokens = self
            .cache_creation_input_tokens
            .saturating_add(add.cache_creation_input_tokens);
        self.cache_creation_5m_input_tokens = self
            .cache_creation_5m_input_tokens
            .saturating_add(add.cache_creation_5m_input_tokens);
        self.cache_creation_1h_input_tokens = self
            .cache_creation_1h_input_tokens
            .saturating_add(add.cache_creation_1h_input_tokens);
    }

    pub(super) fn to_leaderboard_row(self, key: String, name: String) -> super::UsageLeaderboardRow {
        let avg_duration_ms = if self.requests_success > 0 {
            Some(self.success_duration_ms_sum / self.requests_success)
        } else {
            None
        };
        let avg_ttfb_ms = if self.success_ttfb_ms_count > 0 {
            Some(self.success_ttfb_ms_sum / self.success_ttfb_ms_count)
        } else {
            None
        };
        let avg_output_tokens_per_second = if self.success_generation_ms_sum > 0 {
            Some(
                self.success_output_tokens_for_rate_sum as f64
                    / (self.success_generation_ms_sum as f64 / 1000.0),
            )
        } else {
            None
        };

        super::UsageLeaderboardRow {
            key,
            name,
            requests_total: self.requests_total,
            requests_success: self.requests_success,
            requests_failed: self.requests_failed,
            total_tokens: self.total_tokens,
            io_total_tokens: self.input_tokens.saturating_add(self.output_tokens),
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens,
            avg_duration_ms,
            avg_ttfb_ms,
            avg_output_tokens_per_second,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AttemptRow {
    provider_id: i64,
    provider_name: String,
    outcome: String,
}

pub(super) fn extract_final_provider(cli_key: &str, attempts_json: &str) -> ProviderKey {
    let attempts: Vec<AttemptRow> = serde_json::from_str(attempts_json).unwrap_or_default();

    let picked = attempts
        .iter()
        .rev()
        .find(|a| a.outcome == "success")
        .or_else(|| attempts.last());

    match picked {
        Some(a) => ProviderKey {
            cli_key: cli_key.to_string(),
            provider_id: a.provider_id,
            provider_name: a.provider_name.clone(),
        },
        None => ProviderKey {
            cli_key: cli_key.to_string(),
            provider_id: 0,
            provider_name: "Unknown".to_string(),
        },
    }
}

pub(super) fn has_valid_provider_key(key: &ProviderKey) -> bool {
    if key.provider_id <= 0 {
        return false;
    }
    let name = key.provider_name.trim();
    if name.is_empty() {
        return false;
    }
    if name == "Unknown" {
        return false;
    }
    true
}

pub(super) fn is_success(status: Option<i64>, error_code: Option<&str>) -> bool {
    status.is_some_and(|v| (200..300).contains(&v)) && error_code.is_none()
}

fn resolve_range_filters<'a>(
    conn: &Connection,
    range: &str,
    cli_key: Option<&'a str>,
) -> Result<(Option<i64>, Option<&'a str>), String> {
    let range = parse_range(range)?;
    let start_ts = compute_start_ts(conn, range)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    Ok((start_ts, cli_key))
}

pub fn leaderboard_provider(
    db: &db::Db,
    range: &str,
    cli_key: Option<&str>,
    limit: usize,
) -> Result<Vec<UsageProviderRow>, String> {
    let conn = db.open_connection()?;
    let (start_ts, cli_key) = resolve_range_filters(&conn, range, cli_key)?;

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
	  total_tokens,
	  cache_read_input_tokens,
	  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens
FROM request_logs
WHERE excluded_from_stats = 0
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR cli_key = ?2)
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare provider leaderboard query: {e}"))?;

    let rows = stmt
        .query_map(params![start_ts, cli_key], |row| {
            let row_cli_key: String = row.get("cli_key")?;
            let attempts_json: String = row.get("attempts_json")?;
            let status: Option<i64> = row.get("status")?;
            let error_code: Option<String> = row.get("error_code")?;
            let duration_ms: i64 = row.get("duration_ms")?;
            let ttfb_ms: Option<i64> = row.get("ttfb_ms")?;

            let input_tokens: Option<i64> = row.get("input_tokens")?;
            let output_tokens: Option<i64> = row.get("output_tokens")?;
            let total_tokens: Option<i64> = row.get("total_tokens")?;
            let cache_read_input_tokens: Option<i64> = row.get("cache_read_input_tokens")?;
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
                    input_tokens: input_tokens.unwrap_or(0),
                    output_tokens: output_tokens.unwrap_or(0),
                    total_tokens: token_total(total_tokens, input_tokens, output_tokens),
                    cache_read_input_tokens: cache_read_input_tokens.unwrap_or(0),
                    cache_creation_input_tokens: cache_creation_input_tokens.unwrap_or(0),
                    cache_creation_5m_input_tokens: cache_creation_5m_input_tokens.unwrap_or(0),
                    cache_creation_1h_input_tokens: cache_creation_1h_input_tokens.unwrap_or(0),
                },
            ))
        })
        .map_err(|e| format!("DB_ERROR: failed to run provider leaderboard query: {e}"))?;

    let mut agg: HashMap<ProviderKey, ProviderAgg> = HashMap::new();
    for row in rows {
        let (key, add) =
            row.map_err(|e| format!("DB_ERROR: failed to read provider leaderboard row: {e}"))?;

        if !has_valid_provider_key(&key) {
            continue;
        }

        let entry = agg.entry(key).or_default();
        entry.merge(add);
    }

    let mut out: Vec<UsageProviderRow> = agg
        .into_iter()
        .map(|(k, v)| UsageProviderRow {
            cli_key: k.cli_key,
            provider_id: k.provider_id,
            provider_name: k.provider_name,
            requests_total: v.requests_total,
            requests_success: v.requests_success,
            requests_failed: v.requests_failed,
            avg_duration_ms: if v.requests_success > 0 {
                Some(v.success_duration_ms_sum / v.requests_success)
            } else {
                None
            },
            avg_ttfb_ms: if v.success_ttfb_ms_count > 0 {
                Some(v.success_ttfb_ms_sum / v.success_ttfb_ms_count)
            } else {
                None
            },
            avg_output_tokens_per_second: if v.success_generation_ms_sum > 0 {
                Some(
                    v.success_output_tokens_for_rate_sum as f64
                        / (v.success_generation_ms_sum as f64 / 1000.0),
                )
            } else {
                None
            },
            input_tokens: v.input_tokens,
            output_tokens: v.output_tokens,
            total_tokens: v.total_tokens,
            cache_read_input_tokens: v.cache_read_input_tokens,
            cache_creation_input_tokens: v.cache_creation_input_tokens,
            cache_creation_5m_input_tokens: v.cache_creation_5m_input_tokens,
            cache_creation_1h_input_tokens: v.cache_creation_1h_input_tokens,
        })
        .collect();

    out.sort_by(|a, b| {
        b.total_tokens
            .cmp(&a.total_tokens)
            .then_with(|| b.requests_total.cmp(&a.requests_total))
            .then_with(|| a.cli_key.cmp(&b.cli_key))
            .then_with(|| a.provider_name.cmp(&b.provider_name))
    });

    out.truncate(limit.max(1));
    Ok(out)
}

pub fn leaderboard_day(
    db: &db::Db,
    range: &str,
    cli_key: Option<&str>,
    limit: usize,
) -> Result<Vec<UsageDayRow>, String> {
    let conn = db.open_connection()?;
    let (start_ts, cli_key) = resolve_range_filters(&conn, range, cli_key)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') AS day,
  COUNT(*) AS requests_total,
  SUM(COALESCE(input_tokens, 0)) AS input_tokens,
  SUM(COALESCE(output_tokens, 0)) AS output_tokens,
  SUM(COALESCE(total_tokens, COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0))) AS total_tokens,
  SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
  SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
  SUM(COALESCE(cache_creation_5m_input_tokens, 0)) AS cache_creation_5m_input_tokens,
  SUM(COALESCE(cache_creation_1h_input_tokens, 0)) AS cache_creation_1h_input_tokens
FROM request_logs
WHERE excluded_from_stats = 0
AND (?1 IS NULL OR created_at >= ?1)
AND (?2 IS NULL OR cli_key = ?2)
GROUP BY day
ORDER BY total_tokens DESC, day DESC
LIMIT ?3
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare day leaderboard query: {e}"))?;

    let rows = stmt
        .query_map(params![start_ts, cli_key, limit as i64], |row| {
            Ok(UsageDayRow {
                day: row.get("day")?,
                requests_total: row.get("requests_total")?,
                input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
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
        .map_err(|e| format!("DB_ERROR: failed to run day leaderboard query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read day row: {e}"))?);
    }
    Ok(out)
}
