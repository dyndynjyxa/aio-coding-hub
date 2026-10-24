//! Usage: Estimate OAuth window capacity from local cost and upstream utilization.

use crate::shared::error::{db_err, AppResult};
use rusqlite::{params, Connection};

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const WINDOW_COST_SQL: &str = r#"
SELECT
  TOTAL(CASE WHEN cost_usd_femto >= 0 THEN cost_usd_femto ELSE 0 END),
  COUNT(*),
  COUNT(CASE WHEN cost_usd_femto IS NULL OR cost_usd_femto < 0 THEN 1 END)
FROM (
  SELECT cost_usd_femto FROM request_logs
  WHERE final_provider_id = ?1 AND created_at >= ?2 AND created_at <= ?3
    AND excluded_from_stats = 0 AND status >= 200 AND status < 300 AND error_code IS NULL
  UNION ALL
  SELECT cost_usd_femto FROM request_logs
  WHERE cost_source_provider_id = ?1 AND created_at >= ?2 AND created_at <= ?3
    AND final_provider_id IS NOT ?1
    AND excluded_from_stats = 0 AND status >= 200 AND status < 300 AND error_code IS NULL
)
"#;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct OAuthQuotaEstimate {
    pub used_usd: Option<f64>,
    pub estimated_total_usd: Option<f64>,
    pub estimated_remaining_usd: Option<f64>,
    pub window_start: i64,
    pub window_end: i64,
    pub sample_count: i64,
    pub unpriced_count: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthQuotaEstimates {
    pub short: Option<OAuthQuotaEstimate>,
    pub weekly: Option<OAuthQuotaEstimate>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OAuthQuotaWindow {
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub cost_start_at: Option<i64>,
}

/// A reset-aligned window keeps the numerator and upstream utilization on the
/// same period. Unknown/expired windows and unsupported providers yield None.
pub(crate) fn estimate_window(
    conn: &Connection,
    provider_id: i64,
    window: OAuthQuotaWindow,
    now_unix: i64,
) -> AppResult<Option<OAuthQuotaEstimate>> {
    let Some(remaining) = window
        .remaining_percent
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
    else {
        return Ok(None);
    };
    let Some((reset_at, duration)) = window.reset_at.zip(window.duration_seconds) else {
        return Ok(None);
    };
    if provider_id <= 0 || duration <= 0 || duration > 31 * 24 * 3600 || reset_at <= now_unix {
        return Ok(None);
    }
    let Some(start_at) = reset_at
        .checked_sub(duration)
        .filter(|start| *start <= now_unix)
    else {
        return Ok(None);
    };
    let cost_start_at = start_at.max(window.cost_start_at.unwrap_or(start_at));

    // TOTAL avoids SQLite integer-SUM overflow. Like sub2api, estimate from
    // recorded costs even when other requests are unpriced. Keep their count
    // so the UI can explain that the estimate may be incomplete.
    let (cost_femto, sample_count, unpriced_count): (f64, i64, i64) = conn
        .query_row(
            WINDOW_COST_SQL,
            params![provider_id, cost_start_at, now_unix],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| db_err!("failed to query OAuth quota estimate costs: {e}"))?;
    let used_usd = (sample_count > unpriced_count && cost_femto.is_finite())
        .then_some(cost_femto / USD_FEMTO_DENOM);
    let used_percent = 100.0 - remaining;
    let estimated_total_usd = used_usd
        .filter(|used| *used > 0.0 && used_percent > 0.0)
        .map(|used| used * 100.0 / used_percent)
        .filter(|total| total.is_finite() && *total > 0.0);

    Ok(Some(OAuthQuotaEstimate {
        used_usd,
        estimated_total_usd,
        estimated_remaining_usd: estimated_total_usd.map(|total| total * remaining / 100.0),
        window_start: start_at,
        window_end: reset_at,
        sample_count,
        unpriced_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;
    const START: i64 = NOW - 3600;
    const RESET: i64 = START + 5 * 3600;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE request_logs (
              final_provider_id INTEGER, created_at INTEGER, status INTEGER,
              excluded_from_stats INTEGER DEFAULT 0, error_code TEXT, cost_usd_femto INTEGER,
              special_settings_json TEXT, cost_source_provider_id INTEGER
            );",
        )
        .unwrap();
        conn
    }

    fn insert(conn: &Connection, provider: i64, at: i64, cost: Option<i64>) {
        conn.execute(
            "INSERT INTO request_logs(final_provider_id, created_at, status, cost_usd_femto)
             VALUES (?1, ?2, 200, ?3)",
            params![provider, at, cost],
        )
        .unwrap();
    }

    fn window(remaining: f64) -> OAuthQuotaWindow {
        OAuthQuotaWindow {
            remaining_percent: Some(remaining),
            reset_at: Some(RESET),
            duration_seconds: Some(5 * 3600),
            cost_start_at: None,
        }
    }

    #[test]
    fn quota_estimate_matches_sub2api_formula_and_actual_window() {
        let conn = database();
        insert(&conn, 1, START, Some(10_000_000_000_000_000));
        insert(&conn, 1, NOW, Some(10_000_000_000_000_000));
        let estimate = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(estimate.used_usd, Some(20.0));
        assert_eq!(estimate.estimated_total_usd, Some(100.0));
        assert_eq!(estimate.estimated_remaining_usd, Some(80.0));
        assert_eq!((estimate.window_start, estimate.window_end), (START, RESET));
        assert_eq!(estimate.sample_count, 2);

        let weekly = OAuthQuotaWindow {
            duration_seconds: Some(7 * 24 * 3600),
            reset_at: Some(START + 7 * 24 * 3600),
            ..window(80.0)
        };
        assert_eq!(
            estimate_window(&conn, 1, weekly, NOW)
                .unwrap()
                .unwrap()
                .estimated_total_usd,
            Some(100.0)
        );
    }

    #[test]
    fn quota_estimate_excludes_other_accounts_failed_excluded_and_out_of_window_requests() {
        let conn = database();
        insert(&conn, 1, START, Some(1_000_000_000_000_000));
        for (provider, at) in [(2, NOW), (1, START - 1), (1, NOW + 1)] {
            insert(&conn, provider, at, Some(100_000_000_000_000_000));
        }
        for (status, excluded, error) in [
            (500, 0, None),
            (199, 0, None),
            (300, 0, None),
            (200, 1, None),
            (200, 0, Some("upstream_error")),
        ] {
            conn.execute(
                "INSERT INTO request_logs(final_provider_id, created_at, status, excluded_from_stats, error_code, cost_usd_femto)
                 VALUES (1, ?1, ?2, ?3, ?4, 100000000000000000)",
                params![NOW, status, excluded, error],
            )
            .unwrap();
        }
        let estimate = estimate_window(&conn, 1, window(50.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(estimate.used_usd, Some(1.0));
        assert_eq!(estimate.estimated_total_usd, Some(2.0));
        assert_eq!(estimate.sample_count, 1);
    }

    #[test]
    fn quota_estimate_excludes_prior_login_direct_and_bridge_costs() {
        let conn = database();
        for (provider, at, cost) in [(1, NOW - 1, 10), (2, NOW - 1, 10), (1, NOW, 1), (2, NOW, 2)] {
            insert(&conn, provider, at, Some(cost * 1_000_000_000_000_000));
        }
        conn.execute(
            "UPDATE request_logs SET cost_source_provider_id = 1 WHERE final_provider_id = 2",
            [],
        )
        .unwrap();
        let current = OAuthQuotaWindow {
            cost_start_at: Some(NOW),
            ..window(80.0)
        };
        let estimate = estimate_window(&conn, 1, current, NOW).unwrap().unwrap();
        assert_eq!(estimate.used_usd, Some(3.0));
        assert_eq!(estimate.estimated_total_usd, Some(15.0));
        assert_eq!(estimate.sample_count, 2);
        assert_eq!(estimate.window_start, START);

        let just_logged_in = OAuthQuotaWindow {
            cost_start_at: Some(NOW + 1),
            ..current
        };
        let empty = estimate_window(&conn, 1, just_logged_in, NOW)
            .unwrap()
            .unwrap();
        assert_eq!(empty.sample_count, 0);
        assert!(empty.estimated_total_usd.is_none());
    }

    #[test]
    fn quota_estimate_queries_each_provider_through_an_index() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("quota-index.db")).unwrap();
        let conn = db.open_connection().unwrap();
        let plan: Vec<String> = conn
            .prepare(&format!("EXPLAIN QUERY PLAN {WINDOW_COST_SQL}"))
            .unwrap()
            .query_map(params![1, START, NOW], |row| row.get(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            plan.iter().any(|step| {
                step.contains("SEARCH request_logs") && step.contains("final_provider_id=?")
            }),
            "{plan:?}"
        );
        assert!(
            plan.iter().any(|step| {
                step.contains("idx_request_logs_cost_source_created_at")
                    && step.contains("cost_source_provider_id=?")
            }),
            "{plan:?}"
        );
        assert!(!plan.iter().any(|step| step.contains("SCAN request_logs")));
    }

    #[test]
    fn quota_estimate_uses_priced_samples_when_other_costs_are_missing() {
        let conn = database();
        let empty = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert!(empty.used_usd.is_none());
        assert!(empty.estimated_total_usd.is_none());
        assert!(empty.estimated_remaining_usd.is_none());
        for cost in [None, Some(-1)] {
            insert(&conn, 1, NOW, cost);
        }
        let unpriced = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert!(unpriced.used_usd.is_none());
        assert!(unpriced.estimated_total_usd.is_none());
        assert!(unpriced.estimated_remaining_usd.is_none());
        assert_eq!(unpriced.unpriced_count, 2);
        insert(&conn, 1, NOW, Some(1_000_000_000_000_000));
        let partial = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(partial.used_usd, Some(1.0));
        assert_eq!(partial.sample_count, 3);
        assert_eq!(partial.unpriced_count, 2);
        assert_eq!(partial.estimated_total_usd, Some(5.0));
        assert_eq!(partial.estimated_remaining_usd, Some(4.0));
    }

    #[test]
    fn quota_estimate_keeps_recalculating_after_an_unpriced_request() {
        let conn = database();
        insert(&conn, 1, NOW, Some(20_000_000_000_000_000));
        let initial = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(initial.estimated_total_usd, Some(100.0));
        assert_eq!(initial.unpriced_count, 0);

        insert(&conn, 1, NOW, None);
        let partial = estimate_window(&conn, 1, window(80.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(partial.estimated_total_usd, Some(100.0));
        assert_eq!(partial.estimated_remaining_usd, Some(80.0));
        assert_eq!(partial.unpriced_count, 1);

        insert(&conn, 1, NOW, Some(10_000_000_000_000_000));
        let updated = estimate_window(&conn, 1, window(75.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(updated.used_usd, Some(30.0));
        assert_eq!(updated.estimated_total_usd, Some(120.0));
        assert_eq!(updated.estimated_remaining_usd, Some(90.0));
        assert_eq!(updated.unpriced_count, 1);
    }

    #[test]
    fn quota_estimate_includes_bridge_costs_using_recorded_source_identity_once() {
        let conn = database();
        insert(&conn, 1, NOW, Some(1_000_000_000_000_000));
        conn.execute(
            "UPDATE request_logs SET cost_source_provider_id = 1 WHERE final_provider_id = 1",
            [],
        )
        .unwrap();
        for (bridge, source, recorded_bridge, amount) in
            [(2, 1, 2, 3), (3, 1, 3, 2), (4, 1, 99, 10), (5, 88, 5, 10)]
        {
            insert(&conn, bridge, NOW, Some(amount * 1_000_000_000_000_000));
            let identity = serde_json::json!({
                "type": "cx2cc_cost_basis", "source_provider_id": source,
                "bridge_provider_id": recorded_bridge,
            });
            let settings = serde_json::json!(["not JSON", null, 7, true, identity, identity]);
            conn.execute(
                "UPDATE request_logs SET special_settings_json = ?1, cost_source_provider_id = ?3
                 WHERE final_provider_id = ?2",
                params![
                    settings.to_string(),
                    bridge,
                    crate::request_logs::semantics::resolve_cx2cc_source_provider_id(
                        Some(&settings.to_string()),
                        Some(bridge),
                    ),
                ],
            )
            .unwrap();
        }
        for (provider, malformed) in [
            (6, "not JSON"),
            (7, "null"),
            (8, r#"{"source_provider_id":1}"#),
        ] {
            insert(&conn, provider, NOW, Some(10_000_000_000_000_000));
            conn.execute(
                "UPDATE request_logs SET special_settings_json = ?1 WHERE final_provider_id = ?2",
                params![malformed, provider],
            )
            .unwrap();
        }
        let estimate = estimate_window(&conn, 1, window(50.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(estimate.sample_count, 3);
        assert_eq!(estimate.used_usd, Some(6.0));
        assert_eq!(estimate.estimated_total_usd, Some(12.0));
    }

    #[test]
    fn quota_estimate_handles_full_empty_fractional_and_large_costs() {
        let conn = database();
        insert(&conn, 1, NOW, Some(1_000_000_000_000_000));
        let full = estimate_window(&conn, 1, window(100.0), NOW)
            .unwrap()
            .unwrap();
        assert!(full.estimated_total_usd.is_none());
        let exhausted = estimate_window(&conn, 1, window(0.0), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(exhausted.estimated_total_usd, Some(1.0));
        assert_eq!(exhausted.estimated_remaining_usd, Some(0.0));
        let fractional = estimate_window(&conn, 1, window(99.5), NOW)
            .unwrap()
            .unwrap();
        assert_eq!(fractional.estimated_total_usd, Some(200.0));
        assert_eq!(fractional.estimated_remaining_usd, Some(199.0));
        insert(&conn, 2, NOW, Some(i64::MAX));
        insert(&conn, 2, NOW, Some(i64::MAX));
        let large = estimate_window(&conn, 2, window(50.0), NOW)
            .unwrap()
            .unwrap();
        assert!(large.estimated_total_usd.unwrap() > 30_000.0);
        insert(&conn, 3, NOW, Some(0));
        assert!(estimate_window(&conn, 3, window(50.0), NOW)
            .unwrap()
            .unwrap()
            .estimated_total_usd
            .is_none());
    }

    #[test]
    fn quota_estimate_rejects_missing_invalid_and_expired_windows() {
        let conn = database();
        for remaining in [f64::NAN, f64::INFINITY, -0.1, 100.1] {
            assert!(estimate_window(&conn, 1, window(remaining), NOW)
                .unwrap()
                .is_none());
        }
        for invalid in [
            OAuthQuotaWindow {
                remaining_percent: None,
                ..window(80.0)
            },
            OAuthQuotaWindow {
                reset_at: None,
                ..window(80.0)
            },
            OAuthQuotaWindow {
                reset_at: Some(NOW),
                ..window(80.0)
            },
            OAuthQuotaWindow {
                reset_at: Some(NOW - 1),
                ..window(80.0)
            },
            OAuthQuotaWindow {
                reset_at: Some(NOW + 6 * 3600),
                ..window(80.0)
            },
            OAuthQuotaWindow {
                duration_seconds: None,
                ..window(80.0)
            },
            OAuthQuotaWindow {
                duration_seconds: Some(0),
                ..window(80.0)
            },
            OAuthQuotaWindow {
                duration_seconds: Some(32 * 24 * 3600),
                ..window(80.0)
            },
        ] {
            assert!(estimate_window(&conn, 1, invalid, NOW).unwrap().is_none());
        }
        assert!(estimate_window(&conn, 0, window(80.0), NOW)
            .unwrap()
            .is_none());
        assert!(estimate_window(&conn, 1, window(80.0), -1)
            .unwrap()
            .is_none());
    }

    #[test]
    fn quota_estimate_propagates_database_errors() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(estimate_window(&conn, 1, window(80.0), NOW).is_err());
    }
}
