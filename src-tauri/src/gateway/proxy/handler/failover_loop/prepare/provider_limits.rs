//! Usage: Provider spend-limit gating (5h/daily/weekly/monthly/total).

use super::context::CommonCtx;
use crate::providers;
use crate::shared::error::db_err;
use rusqlite::{params, Connection};

pub(super) struct ProviderLimitsInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) provider: &'a providers::ProviderForGateway,
    pub(super) earliest_available_unix: &'a mut Option<i64>,
    pub(super) skipped_limits: &'a mut usize,
}

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const WINDOW_5H_SECS: i64 = 5 * 60 * 60;
const WINDOW_24H_SECS: i64 = 24 * 60 * 60;

fn update_earliest(earliest: &mut Option<i64>, candidate: i64) {
    if candidate <= 0 {
        return;
    }
    match earliest {
        Some(existing) if *existing <= candidate => {}
        _ => *earliest = Some(candidate),
    }
}

fn update_latest(latest: &mut Option<i64>, candidate: i64) {
    if candidate <= 0 {
        return;
    }
    match latest {
        Some(existing) if *existing >= candidate => {}
        _ => *latest = Some(candidate),
    }
}

fn limit_usd_to_femto(limit_usd: f64) -> Option<i128> {
    if !limit_usd.is_finite() || limit_usd < 0.0 {
        return None;
    }
    if limit_usd == 0.0 {
        return Some(0);
    }

    let limit_femto = (limit_usd * USD_FEMTO_DENOM).round();
    if !limit_femto.is_finite() {
        return None;
    }

    let limit_femto = limit_femto as i128;
    if limit_femto <= 0 {
        // Ensure tiny positive limits never collapse to zero due to rounding.
        return Some(1);
    }

    Some(limit_femto)
}

fn limit_exceeded(limit_usd: f64, spent_femto: f64) -> bool {
    let Some(limit_femto) = limit_usd_to_femto(limit_usd) else {
        return false;
    };
    spent_femto.max(0.0) >= limit_femto as f64
}

fn has_any_limit(provider: &providers::ProviderForGateway) -> bool {
    provider.limit_5h_usd.is_some()
        || provider.limit_daily_usd.is_some()
        || provider.limit_weekly_usd.is_some()
        || provider.limit_monthly_usd.is_some()
        || provider.limit_total_usd.is_some()
}

#[derive(Debug, Clone, Copy, Default)]
struct SpendSums {
    spent_5h: f64,
    spent_daily_rolling: f64,
    spent_daily_fixed: f64,
    spent_weekly: f64,
    spent_monthly: f64,
    spent_total: f64,
}

fn min_start_ts(values: &[Option<i64>]) -> Option<i64> {
    values.iter().copied().flatten().min()
}

#[derive(Debug, Clone, Copy)]
struct SpendQueryBounds {
    start_5h: Option<i64>,
    start_daily_rolling: Option<i64>,
    start_daily_fixed: Option<i64>,
    start_weekly: Option<i64>,
    start_monthly: Option<i64>,
    end_ts: i64,
    min_start: Option<i64>,
}

fn sum_cost_usd_femto_windows(
    conn: &Connection,
    provider_id: i64,
    bounds: SpendQueryBounds,
) -> crate::shared::error::AppResult<SpendSums> {
    let SpendQueryBounds {
        start_5h,
        start_daily_rolling,
        start_daily_fixed,
        start_weekly,
        start_monthly,
        end_ts,
        min_start,
    } = bounds;

    conn.query_row(
        r#"
SELECT
  TOTAL(CASE WHEN created_at >= ?2 THEN CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END ELSE 0 END) AS spent_5h,
  TOTAL(CASE WHEN created_at >= ?3 THEN CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END ELSE 0 END) AS spent_daily_rolling,
  TOTAL(CASE WHEN created_at >= ?4 THEN CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END ELSE 0 END) AS spent_daily_fixed,
  TOTAL(CASE WHEN created_at >= ?5 THEN CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END ELSE 0 END) AS spent_weekly,
  TOTAL(CASE WHEN created_at >= ?6 THEN CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END ELSE 0 END) AS spent_monthly,
  TOTAL(CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END) AS spent_total
FROM request_logs
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300 AND error_code IS NULL
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at < ?7
  AND (?8 IS NULL OR created_at >= ?8)
"#,
        params![
            provider_id,
            start_5h,
            start_daily_rolling,
            start_daily_fixed,
            start_weekly,
            start_monthly,
            end_ts,
            min_start
        ],
        |row| {
            Ok(SpendSums {
                spent_5h: row.get::<_, f64>("spent_5h")?.max(0.0),
                spent_daily_rolling: row
                    .get::<_, f64>("spent_daily_rolling")?
                    .max(0.0),
                spent_daily_fixed: row
                    .get::<_, f64>("spent_daily_fixed")?
                    .max(0.0),
                spent_weekly: row
                    .get::<_, f64>("spent_weekly")?
                    .max(0.0),
                spent_monthly: row
                    .get::<_, f64>("spent_monthly")?
                    .max(0.0),
                spent_total: row.get::<_, f64>("spent_total")?.max(0.0),
            })
        },
    )
    .map_err(|e| db_err!("failed to sum provider cost windows: {e}"))
}

fn fetch_cost_buckets(
    conn: &Connection,
    provider_id: i64,
    start_ts: i64,
    end_ts: i64,
) -> crate::shared::error::AppResult<Vec<(i64, i128)>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      created_at,
      CASE WHEN cost_usd_femto < 0 THEN 0 ELSE cost_usd_femto END AS cost
    FROM request_logs
    WHERE excluded_from_stats = 0
      AND status >= 200 AND status < 300 AND error_code IS NULL
      AND cost_usd_femto IS NOT NULL
      AND final_provider_id = ?1
      AND created_at >= ?2 AND created_at < ?3
    ORDER BY created_at ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare provider cost bucket query: {e}"))?;

    let rows = stmt
        .query_map(params![provider_id, start_ts, end_ts], |row| {
            let ts: i64 = row.get(0)?;
            let cost: i64 = row.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0);
            Ok((ts, cost))
        })
        .map_err(|e| db_err!("failed to query provider cost buckets: {e}"))?;

    let mut out: Vec<(i64, i128)> = Vec::new();
    for row in rows {
        let (ts, cost) = row.map_err(|e| db_err!("failed to read provider cost bucket: {e}"))?;
        match out.last_mut() {
            Some((last_ts, last_cost)) if *last_ts == ts => {
                *last_cost = (*last_cost).saturating_add(cost as i128);
            }
            _ => out.push((ts, cost as i128)),
        }
    }
    Ok(out)
}

fn compute_next_available_rolling_from_buckets(
    buckets: &[(i64, i128)],
    window_start: i64,
    window_secs: i64,
    limit_femto: i128,
) -> Option<i64> {
    if window_secs <= 0 {
        return None;
    }
    if limit_femto <= 0 {
        return None;
    }

    let mut total: i128 = 0;
    for (ts, cost) in buckets.iter().copied() {
        if ts < window_start {
            continue;
        }
        total = total.saturating_add(cost.max(0));
    }
    if total < limit_femto {
        return None;
    }

    let threshold = total.saturating_sub(limit_femto).saturating_add(1);
    let mut prefix: i128 = 0;
    for (ts, cost) in buckets.iter().copied() {
        if ts < window_start {
            continue;
        }
        prefix = prefix.saturating_add(cost.max(0));
        if prefix >= threshold {
            return Some(ts.saturating_add(1).saturating_add(window_secs));
        }
    }

    None
}

fn parse_reset_time_hms_lossy(input: &str) -> (u8, u8, u8) {
    let trimmed = input.trim();
    let mut parts = trimmed.split(':');

    let h_raw = parts.next().unwrap_or("0");
    let m_raw = parts.next().unwrap_or("0");
    let s_raw = parts.next().unwrap_or("0");

    let h = h_raw.parse::<u8>().ok().filter(|v| *v <= 23).unwrap_or(0);
    let m = m_raw.parse::<u8>().ok().filter(|v| *v <= 59).unwrap_or(0);
    let s = s_raw.parse::<u8>().ok().filter(|v| *v <= 59).unwrap_or(0);
    (h, m, s)
}

fn compute_daily_fixed_bounds(
    conn: &Connection,
    now_unix: i64,
    reset_time: &str,
) -> crate::shared::error::AppResult<(i64, i64)> {
    let (h, m, s) = parse_reset_time_hms_lossy(reset_time);
    let mod_h = format!("+{h} hours");
    let mod_m = format!("+{m} minutes");
    let mod_s = format!("+{s} seconds");

    conn.query_row(
        r#"
WITH bounds AS (
  SELECT
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', ?2, ?3, ?4, 'utc') AS INTEGER) AS today_reset,
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day','-1 day', ?2, ?3, ?4, 'utc') AS INTEGER) AS yesterday_reset,
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day','+1 day', ?2, ?3, ?4, 'utc') AS INTEGER) AS tomorrow_reset
)
SELECT
  CASE WHEN ?1 >= today_reset THEN today_reset ELSE yesterday_reset END AS start_ts,
  CASE WHEN ?1 < today_reset THEN today_reset ELSE tomorrow_reset END AS next_reset
FROM bounds
"#,
        params![now_unix, mod_h, mod_m, mod_s],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute daily reset bounds: {e}"))
}

fn compute_weekly_bounds(
    conn: &Connection,
    now_unix: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    conn.query_row(
        r#"
WITH w AS (
  SELECT (CAST(strftime('%w', ?1, 'unixepoch','localtime') AS INTEGER) + 6) % 7 AS offset
)
SELECT
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', printf('-%d days', offset), 'utc') AS INTEGER) AS start_ts,
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', printf('+%d days', 7 - offset), 'utc') AS INTEGER) AS next_reset
FROM w
"#,
        params![now_unix],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute weekly bounds: {e}"))
}

fn compute_monthly_bounds(
    conn: &Connection,
    now_unix: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    conn.query_row(
        r#"
SELECT
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of month','utc') AS INTEGER) AS start_ts,
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of month','+1 month','utc') AS INTEGER) AS next_reset
"#,
        params![now_unix],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute monthly bounds: {e}"))
}

/// Resolve the fixed 5h window start for a provider.
/// Reads stored `window_5h_start_ts`; if NULL or expired, sets it to `now_unix` (the current request time).
fn resolve_fixed_5h_start(
    conn: &Connection,
    provider_id: i64,
    now_unix: i64,
) -> crate::shared::error::AppResult<i64> {
    let stored: Option<i64> = conn
        .query_row(
            "SELECT window_5h_start_ts FROM providers WHERE id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .map_err(|e| db_err!("failed to read window_5h_start_ts: {e}"))?;

    if let Some(start_ts) = stored {
        let window_end = start_ts.saturating_add(WINDOW_5H_SECS);
        if now_unix < window_end {
            return Ok(start_ts);
        }
    }

    // Window expired or null -> start a new window from the current request
    conn.execute(
        "UPDATE providers SET window_5h_start_ts = ?1 WHERE id = ?2",
        params![now_unix, provider_id],
    )
    .map_err(|e| db_err!("failed to update window_5h_start_ts: {e}"))?;

    Ok(now_unix)
}

pub(super) fn gate_provider<R: tauri::Runtime>(input: ProviderLimitsInput<'_, R>) -> bool {
    let ProviderLimitsInput {
        ctx,
        provider,
        earliest_available_unix,
        skipped_limits,
    } = input;

    let has_oauth_quota_gate = provider.auth_mode == "oauth";
    let has_spend_limit = has_any_limit(provider);
    if !has_oauth_quota_gate && !has_spend_limit {
        return true;
    }

    let conn = match ctx.state.db.open_connection() {
        Ok(conn) => conn,
        Err(_) => return true,
    };

    let now_unix = ctx.created_at;
    let end_unix = now_unix.saturating_add(1);

    if has_oauth_quota_gate {
        match crate::domain::provider_oauth_limits::gate_snapshot(&conn, provider.id, now_unix) {
            Ok(crate::domain::provider_oauth_limits::OAuthLimitGate::Allow) => {}
            Ok(crate::domain::provider_oauth_limits::OAuthLimitGate::Limited { reset_at }) => {
                *skipped_limits = skipped_limits.saturating_add(1);
                if let Some(reset_at) = reset_at {
                    update_earliest(earliest_available_unix, reset_at);
                }
                return false;
            }
            Err(err) => {
                tracing::warn!(
                    provider_id = provider.id,
                    provider_name = %provider.name,
                    "failed to gate OAuth provider quota snapshot: {err}"
                );
            }
        }
    }

    if !has_spend_limit {
        return true;
    }

    // Use fixed window for 5h limit
    let start_5h = if provider.limit_5h_usd.is_some() {
        match resolve_fixed_5h_start(&conn, provider.id, now_unix) {
            Ok(ts) => Some(ts),
            Err(_) => return true,
        }
    } else {
        None
    };

    let (start_daily_rolling, start_daily_fixed, next_daily_fixed) =
        match (provider.limit_daily_usd, provider.daily_reset_mode) {
            (Some(_), providers::DailyResetMode::Rolling) => {
                (Some(now_unix.saturating_sub(WINDOW_24H_SECS)), None, None)
            }
            (Some(_), providers::DailyResetMode::Fixed) => {
                let (start, next) = match compute_daily_fixed_bounds(
                    &conn,
                    now_unix,
                    provider.daily_reset_time.as_str(),
                ) {
                    Ok(v) => v,
                    Err(_) => return true,
                };
                (None, Some(start), Some(next))
            }
            _ => (None, None, None),
        };

    let (start_weekly, next_weekly) = if provider.limit_weekly_usd.is_some() {
        match compute_weekly_bounds(&conn, now_unix) {
            Ok((start, next)) => (Some(start), Some(next)),
            Err(_) => return true,
        }
    } else {
        (None, None)
    };

    let (start_monthly, next_monthly) = if provider.limit_monthly_usd.is_some() {
        match compute_monthly_bounds(&conn, now_unix) {
            Ok((start, next)) => (Some(start), Some(next)),
            Err(_) => return true,
        }
    } else {
        (None, None)
    };

    let needs_total = provider.limit_total_usd.is_some();
    let min_start = if needs_total {
        None
    } else {
        min_start_ts(&[
            start_5h,
            start_daily_rolling,
            start_daily_fixed,
            start_weekly,
            start_monthly,
        ])
    };

    let sums = match sum_cost_usd_femto_windows(
        &conn,
        provider.id,
        SpendQueryBounds {
            start_5h,
            start_daily_rolling,
            start_daily_fixed,
            start_weekly,
            start_monthly,
            end_ts: end_unix,
            min_start,
        },
    ) {
        Ok(v) => v,
        Err(_) => return true,
    };

    let mut exceeded = false;
    let mut provider_next_available: Option<i64> = None;
    let mut need_rolling_5h = false;
    let mut need_rolling_daily = false;

    if let Some(limit) = provider.limit_5h_usd {
        if limit_exceeded(limit, sums.spent_5h) {
            exceeded = true;
            need_rolling_5h = true;
        }
    }

    if let Some(limit) = provider.limit_daily_usd {
        match provider.daily_reset_mode {
            providers::DailyResetMode::Rolling => {
                if limit_exceeded(limit, sums.spent_daily_rolling) {
                    exceeded = true;
                    need_rolling_daily = true;
                }
            }
            providers::DailyResetMode::Fixed => {
                if limit_exceeded(limit, sums.spent_daily_fixed) {
                    exceeded = true;
                    if let Some(next_reset) = next_daily_fixed {
                        update_latest(&mut provider_next_available, next_reset);
                    }
                }
            }
        }
    }

    if let Some(limit) = provider.limit_weekly_usd {
        if limit_exceeded(limit, sums.spent_weekly) {
            exceeded = true;
            if let Some(next_reset) = next_weekly {
                update_latest(&mut provider_next_available, next_reset);
            }
        }
    }

    if let Some(limit) = provider.limit_monthly_usd {
        if limit_exceeded(limit, sums.spent_monthly) {
            exceeded = true;
            if let Some(next_reset) = next_monthly {
                update_latest(&mut provider_next_available, next_reset);
            }
        }
    }

    if let Some(limit) = provider.limit_total_usd {
        if limit_exceeded(limit, sums.spent_total) {
            exceeded = true;
        }
    }

    if !exceeded {
        return true;
    }

    if need_rolling_5h || need_rolling_daily {
        let mut buckets_start: Option<i64> = None;
        if need_rolling_daily {
            buckets_start = start_daily_rolling;
        }
        if need_rolling_5h {
            if let Some(start_5h) = start_5h {
                buckets_start = Some(match buckets_start {
                    Some(existing) => existing.min(start_5h),
                    None => start_5h,
                });
            }
        }

        if let Some(buckets_start) = buckets_start {
            if let Ok(buckets) = fetch_cost_buckets(&conn, provider.id, buckets_start, end_unix) {
                if need_rolling_5h {
                    if let (Some(start_5h), Some(limit_usd)) = (start_5h, provider.limit_5h_usd) {
                        if let Some(limit_femto) = limit_usd_to_femto(limit_usd) {
                            if let Some(next) = compute_next_available_rolling_from_buckets(
                                &buckets,
                                start_5h,
                                WINDOW_5H_SECS,
                                limit_femto,
                            ) {
                                update_latest(&mut provider_next_available, next);
                            }
                        }
                    }
                }

                if need_rolling_daily {
                    if let (Some(start_24h), Some(limit_usd)) =
                        (start_daily_rolling, provider.limit_daily_usd)
                    {
                        if let Some(limit_femto) = limit_usd_to_femto(limit_usd) {
                            if let Some(next) = compute_next_available_rolling_from_buckets(
                                &buckets,
                                start_24h,
                                WINDOW_24H_SECS,
                                limit_femto,
                            ) {
                                update_latest(&mut provider_next_available, next);
                            }
                        }
                    }
                }
            }
        }
    }

    *skipped_limits = skipped_limits.saturating_add(1);
    if let Some(next) = provider_next_available {
        update_earliest(earliest_available_unix, next);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::context::CommonCtxArgs;
    use super::*;
    use crate::gateway::active_requests::ActiveRequestRegistry;
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
    use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
    use crate::gateway::runtime::GatewayAppState;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn gateway_test_state(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: crate::db::Db,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(1);
        GatewayAppState {
            app,
            db,
            log_tx,
            circuit: Arc::new(crate::circuit_breaker::CircuitBreaker::new(
                crate::circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            session: Arc::new(crate::session_manager::SessionManager::new()),
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: Arc::new(Mutex::new(RecentErrorCache::default())),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline: GatewayPluginPipeline::empty_shared(),
            active_requests: Arc::new(ActiveRequestRegistry::default()),
            responses_ws: Arc::new(crate::gateway::responses_ws::state::Runtime::new(false)),
        }
    }

    fn provider_with_5h_limit(id: i64) -> providers::ProviderForGateway {
        providers::ProviderForGateway {
            custom_headers: Vec::new(),
            id,
            name: "overflow-provider".to_string(),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: "test-key".to_string(),
            claude_models: providers::ClaudeModels::default(),
            model_policy: Some(providers::ProviderModelPolicyV1::all()),
            model_policy_status: providers::ProviderModelPolicyStatus::Ready,
            limit_5h_usd: Some(9_000.0),
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            supports_websockets: false,
            extension_values: vec![],
        }
    }

    #[test]
    fn cost_queries_allow_totals_above_i64_max() {
        const COST_TERM_FEMTO: i64 = 3_i64 << 61;

        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            r#"
CREATE TABLE request_logs (
  excluded_from_stats INTEGER NOT NULL,
  status INTEGER,
  error_code TEXT,
  cost_usd_femto INTEGER,
  final_provider_id INTEGER,
  created_at INTEGER NOT NULL
);
"#,
        )
        .expect("create request_logs");
        for _ in 0..2 {
            conn.execute(
                "INSERT INTO request_logs VALUES (0, 200, NULL, ?1, 7, 10)",
                [COST_TERM_FEMTO],
            )
            .expect("insert request log");
        }

        let sums = sum_cost_usd_femto_windows(
            &conn,
            7,
            SpendQueryBounds {
                start_5h: Some(0),
                start_daily_rolling: Some(0),
                start_daily_fixed: Some(0),
                start_weekly: Some(0),
                start_monthly: Some(0),
                end_ts: 20,
                min_start: None,
            },
        )
        .expect("sum cost windows");
        let expected = COST_TERM_FEMTO as f64 * 2.0;

        for actual in [
            sums.spent_5h,
            sums.spent_daily_rolling,
            sums.spent_daily_fixed,
            sums.spent_weekly,
            sums.spent_monthly,
            sums.spent_total,
        ] {
            assert_eq!(actual, expected);
            assert!(limit_exceeded(9_000.0, actual));
        }

        let buckets = fetch_cost_buckets(&conn, 7, 0, 20).expect("fetch cost buckets");
        assert_eq!(buckets, vec![(10, COST_TERM_FEMTO as i128 * 2)]);
        assert!(buckets[0].1 > i64::MAX as i128);
        assert_eq!(
            compute_next_available_rolling_from_buckets(
                &buckets,
                0,
                WINDOW_5H_SECS,
                i64::MAX as i128,
            ),
            Some(10 + 1 + WINDOW_5H_SECS)
        );
    }

    #[test]
    fn gate_provider_rejects_cost_total_above_i64_max() {
        const COST_TERM_FEMTO: i64 = 3_i64 << 61;
        const NOW_UNIX: i64 = 1_000;
        const WINDOW_START_UNIX: i64 = 900;
        const COST_CREATED_AT: i64 = 950;

        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("provider-limit-overflow.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open connection");
        conn.execute(
            r#"
INSERT INTO providers (
  cli_key, name, base_url, api_key_plaintext, created_at, updated_at, window_5h_start_ts
) VALUES ('codex', 'overflow-provider', 'https://example.com', 'test-key', 1, 1, ?1)
"#,
            [WINDOW_START_UNIX],
        )
        .expect("insert provider");
        let provider_id = conn.last_insert_rowid();

        for index in 0..2 {
            conn.execute(
                r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, duration_ms, attempts_json,
  created_at, cost_usd_femto, excluded_from_stats, final_provider_id
) VALUES (?1, 'codex', 'POST', '/v1/responses', 200, 1, '[]', ?2, ?3, 0, ?4)
"#,
                params![
                    format!("provider-limit-overflow-{index}"),
                    COST_CREATED_AT,
                    COST_TERM_FEMTO,
                    provider_id
                ],
            )
            .expect("insert request log");
        }
        drop(conn);

        let state = gateway_test_state(app.handle().clone(), db);
        let provider = provider_with_5h_limit(provider_id);
        let cli_key = "codex".to_string();
        let forwarded_path = "/v1/responses".to_string();
        let method_hint = "POST".to_string();
        let query = None;
        let trace_id = "provider-limit-gate".to_string();
        let session_id = None;
        let requested_model = None;
        let cx2cc_settings = crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default();
        let special_settings = Arc::new(Mutex::new(Vec::new()));
        let response_fixer_config = crate::gateway::response_fixer::ResponseFixerConfig {
            fix_encoding: false,
            fix_sse_format: false,
            fix_truncated_json: false,
            max_json_depth: crate::gateway::response_fixer::DEFAULT_MAX_JSON_DEPTH,
            max_fix_size: crate::gateway::response_fixer::DEFAULT_MAX_FIX_SIZE,
        };
        let ctx = CommonCtx::from(CommonCtxArgs {
            ws_request: None,
            state: &state,
            cli_key: &cli_key,
            forwarded_path: &forwarded_path,
            observe: true,
            method_hint: &method_hint,
            query: &query,
            trace_id: &trace_id,
            started: std::time::Instant::now(),
            created_at_ms: NOW_UNIX * 1_000,
            created_at: NOW_UNIX,
            session_id: &session_id,
            requested_model: &requested_model,
            cx2cc_settings: &cx2cc_settings,
            effective_sort_mode_id: None,
            special_settings: &special_settings,
            provider_health_neutral: false,
            provider_cooldown_secs: 0,
            upstream_first_byte_timeout_secs: 0,
            upstream_first_byte_timeout: None,
            upstream_stream_idle_timeout: None,
            upstream_request_timeout_non_streaming: None,
            verbose_provider_error: false,
            max_attempts_per_provider: 1,
            codex_priority_billing_source: crate::settings::CodexPriorityBillingSource::default(),
            enable_response_fixer: false,
            response_fixer_stream_config: response_fixer_config,
            response_fixer_non_stream_config: response_fixer_config,
            introspection_body: &[],
        });
        let mut earliest_available_unix = None;
        let mut skipped_limits = 0;

        assert!(!gate_provider(ProviderLimitsInput {
            ctx,
            provider: &provider,
            earliest_available_unix: &mut earliest_available_unix,
            skipped_limits: &mut skipped_limits,
        }));
        assert_eq!(skipped_limits, 1);
        assert_eq!(
            earliest_available_unix,
            Some(COST_CREATED_AT + 1 + WINDOW_5H_SECS)
        );
    }

    #[test]
    fn rolling_next_available_returns_cutoff_plus_window_plus_1() {
        let window_secs = 5;
        let window_start = 100;
        let limit_femto: i128 = 100;

        let buckets = vec![(100, 60), (101, 50)];

        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        assert_eq!(next, 100 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_handles_equal_to_limit_as_exceeded() {
        let window_secs = 10;
        let window_start = 1_000;
        let limit_femto: i128 = 100;

        let buckets = vec![(1_000, 100)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        assert_eq!(next, 1_000 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_returns_none_when_under_limit() {
        let window_secs = 10;
        let window_start = 100;
        let limit_femto: i128 = 200;

        let buckets = vec![(100, 50), (101, 49)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        );
        assert!(next.is_none());
    }

    #[test]
    fn rolling_next_available_ignores_buckets_before_window_start() {
        let window_secs = 10;
        let window_start = 200;
        let limit_femto: i128 = 100;

        // Buckets before window_start should be ignored
        let buckets = vec![(100, 1000), (150, 1000), (200, 50), (201, 50)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        // First bucket at 200 pushes over limit
        assert_eq!(next, 200 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_handles_zero_or_negative_limit() {
        let buckets = vec![(100, 50)];
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 10, 0).is_none());
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 10, -1).is_none());
    }

    #[test]
    fn rolling_next_available_handles_zero_or_negative_window() {
        let buckets = vec![(100, 50)];
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 0, 100).is_none());
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, -1, 100).is_none());
    }

    #[test]
    fn limit_usd_to_femto_conversion() {
        assert_eq!(limit_usd_to_femto(1.0), Some(1_000_000_000_000_000));
        assert_eq!(limit_usd_to_femto(0.001), Some(1_000_000_000_000));
        assert_eq!(limit_usd_to_femto(0.0), Some(0));
    }

    #[test]
    fn limit_usd_to_femto_tiny_positive_never_rounds_to_zero() {
        assert_eq!(limit_usd_to_femto(1e-18), Some(1));
    }

    #[test]
    fn limit_usd_to_femto_handles_invalid_inputs() {
        assert!(limit_usd_to_femto(f64::NAN).is_none());
        assert!(limit_usd_to_femto(f64::INFINITY).is_none());
        assert!(limit_usd_to_femto(f64::NEG_INFINITY).is_none());
        assert!(limit_usd_to_femto(-1.0).is_none());
    }

    #[test]
    fn limit_exceeded_checks_correctly() {
        // 1 USD limit = 1_000_000_000_000_000 femto
        let limit_usd = 1.0;
        let limit_femto = 1_000_000_000_000_000_f64;

        // Exactly at limit - should be exceeded
        assert!(limit_exceeded(limit_usd, limit_femto));

        // Under limit
        assert!(!limit_exceeded(limit_usd, limit_femto - 1.0));

        // Over limit
        assert!(limit_exceeded(limit_usd, limit_femto + 1.0));

        // Negative spent should not exceed
        assert!(!limit_exceeded(limit_usd, -100.0));

        // Zero limit is explicitly treated as immediate limit hit
        assert!(limit_exceeded(0.0, 0.0));
    }

    #[test]
    fn limit_exceeded_handles_invalid_limit() {
        // Invalid limits should never be "exceeded" (fail open)
        assert!(!limit_exceeded(f64::NAN, 1_000_000.0));
        assert!(!limit_exceeded(-1.0, 1_000_000.0));
    }

    #[test]
    fn update_earliest_selects_minimum() {
        let mut earliest: Option<i64> = None;

        update_earliest(&mut earliest, 100);
        assert_eq!(earliest, Some(100));

        update_earliest(&mut earliest, 200);
        assert_eq!(earliest, Some(100)); // Should keep 100

        update_earliest(&mut earliest, 50);
        assert_eq!(earliest, Some(50)); // Should update to 50
    }

    #[test]
    fn update_earliest_ignores_non_positive() {
        let mut earliest: Option<i64> = Some(100);
        update_earliest(&mut earliest, 0);
        assert_eq!(earliest, Some(100));

        update_earliest(&mut earliest, -50);
        assert_eq!(earliest, Some(100));
    }

    #[test]
    fn update_latest_selects_maximum() {
        let mut latest: Option<i64> = None;

        update_latest(&mut latest, 100);
        assert_eq!(latest, Some(100));

        update_latest(&mut latest, 50);
        assert_eq!(latest, Some(100)); // Should keep 100

        update_latest(&mut latest, 200);
        assert_eq!(latest, Some(200)); // Should update to 200
    }

    #[test]
    fn update_latest_ignores_non_positive() {
        let mut latest: Option<i64> = Some(100);
        update_latest(&mut latest, 0);
        assert_eq!(latest, Some(100));

        update_latest(&mut latest, -50);
        assert_eq!(latest, Some(100));
    }

    #[test]
    fn parse_reset_time_hms_lossy_valid_inputs() {
        assert_eq!(parse_reset_time_hms_lossy("00:00:00"), (0, 0, 0));
        assert_eq!(parse_reset_time_hms_lossy("12:30:45"), (12, 30, 45));
        assert_eq!(parse_reset_time_hms_lossy("23:59:59"), (23, 59, 59));
        assert_eq!(parse_reset_time_hms_lossy("  09:15:30  "), (9, 15, 30));
    }

    #[test]
    fn parse_reset_time_hms_lossy_partial_inputs() {
        assert_eq!(parse_reset_time_hms_lossy("12"), (12, 0, 0));
        assert_eq!(parse_reset_time_hms_lossy("12:30"), (12, 30, 0));
    }

    #[test]
    fn parse_reset_time_hms_lossy_invalid_inputs() {
        // Invalid hour (> 23) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("25:30:00"), (0, 30, 0));
        // Invalid minute (> 59) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("12:60:00"), (12, 0, 0));
        // Invalid second (> 59) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("12:30:60"), (12, 30, 0));
        // Non-numeric should default to 0
        assert_eq!(parse_reset_time_hms_lossy("abc:def:ghi"), (0, 0, 0));
        // Empty string
        assert_eq!(parse_reset_time_hms_lossy(""), (0, 0, 0));
    }

    #[test]
    fn min_start_ts_returns_minimum() {
        assert_eq!(min_start_ts(&[Some(100), Some(50), Some(200)]), Some(50));
        assert_eq!(min_start_ts(&[None, Some(100), None]), Some(100));
        assert_eq!(min_start_ts(&[None, None, None]), None);
        assert_eq!(min_start_ts(&[]), None);
    }
}
