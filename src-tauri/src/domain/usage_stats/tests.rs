use super::cache_rate_trend_v1::{
    provider_cache_rate_trend_v1_with_conn, ProviderCacheRateTrendQuery,
};
use super::day_detail::{day_detail_v1_with_conn, UsageDayResolvedFolder};
use super::folder_options::folder_options_v1_with_conn;
use super::leaderboard_v2::{
    leaderboard_v2_folder_filtered_with_conn, leaderboard_v2_with_conn,
    leaderboard_v2_with_conn_day_start, FolderFilteredLeaderboardParams,
};
use super::summary::{summary_query, summary_v2_with_conn};
use super::*;
use crate::db;
use rusqlite::{params, Connection};
use tempfile::tempdir;

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
	CREATE TABLE providers (
	  id INTEGER PRIMARY KEY,
	  name TEXT NOT NULL,
	  source_provider_id INTEGER,
	  bridge_type TEXT
	);

	CREATE TABLE request_logs (
	  cli_key TEXT NOT NULL,
	  attempts_json TEXT NOT NULL,
	  final_provider_id INTEGER,
	  requested_model TEXT,
	  status INTEGER,
	  error_code TEXT,
	  duration_ms INTEGER NOT NULL,
	  ttfb_ms INTEGER,
	  input_tokens INTEGER,
	  output_tokens INTEGER,
	  total_tokens INTEGER,
	  cache_read_input_tokens INTEGER,
	  cache_creation_input_tokens INTEGER,
	  cache_creation_5m_input_tokens INTEGER,
	  cache_creation_1h_input_tokens INTEGER,
	  cost_usd_femto INTEGER,
	  usage_json TEXT,
	  excluded_from_stats INTEGER NOT NULL DEFAULT 0,
	  session_id TEXT,
	  created_at INTEGER NOT NULL,
	  created_at_ms INTEGER NOT NULL DEFAULT 0
	);
	"#,
    )
    .expect("create schema");
    conn
}

fn setup_temp_db() -> (tempfile::TempDir, db::Db) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("usage-stats-test.db");
    let db = db::init_for_tests(&path).expect("init test db");
    (dir, db)
}

fn local_day_key(conn: &Connection, ts: i64) -> String {
    conn.query_row(
        "SELECT strftime('%Y-%m-%d', ?1, 'unixepoch', 'localtime')",
        params![ts],
        |row| row.get(0),
    )
    .expect("query local day key")
}

fn local_day_start_ts(conn: &Connection, day: &str) -> i64 {
    conn.query_row(
        "SELECT CAST(strftime('%s', ?1 || ' 00:00:00', 'utc') AS INTEGER)",
        params![day],
        |row| row.get(0),
    )
    .expect("query local day start ts")
}

fn local_usage_day_start_ts(conn: &Connection, day: &str, day_start_hour: i64) -> i64 {
    let time = format!("{day_start_hour:02}:00:00");
    conn.query_row(
        "SELECT CAST(strftime('%s', ?1 || ' ' || ?2, 'utc') AS INTEGER)",
        params![day, time],
        |row| row.get(0),
    )
    .expect("query local usage day start ts")
}

#[derive(Clone)]
struct TestUsageLog<'a> {
    cli_key: &'a str,
    provider_id: i64,
    provider_name: &'a str,
    requested_model: &'a str,
    status: Option<i64>,
    error_code: Option<&'a str>,
    duration_ms: i64,
    ttfb_ms: Option<i64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cost_usd_femto: Option<i64>,
    session_id: Option<&'a str>,
    excluded_from_stats: i64,
    created_at: i64,
}

fn base_usage_log(created_at: i64) -> TestUsageLog<'static> {
    TestUsageLog {
        cli_key: "codex",
        provider_id: 123,
        provider_name: "OpenAI",
        requested_model: "model-test",
        status: Some(200),
        error_code: None,
        duration_ms: 1000,
        ttfb_ms: Some(100),
        input_tokens: Some(100),
        output_tokens: Some(20),
        total_tokens: None,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cost_usd_femto: None,
        session_id: None,
        excluded_from_stats: 0,
        created_at,
    }
}

fn insert_usage_log(conn: &Connection, log: TestUsageLog<'_>) {
    let attempts_json = format!(
        r#"[{{"provider_id":{},"provider_name":"{}","outcome":"success"}}]"#,
        log.provider_id, log.provider_name
    );

    conn.execute(
        r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  session_id,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20);
        "#,
        params![
            log.cli_key,
            attempts_json,
            log.provider_id,
            log.requested_model,
            log.status,
            log.error_code,
            log.duration_ms,
            log.ttfb_ms,
            log.input_tokens,
            log.output_tokens,
            log.total_tokens,
            log.cache_read_input_tokens,
            log.cache_creation_input_tokens,
            0i64,
            0i64,
            log.cost_usd_femto,
            Option::<String>::None,
            log.excluded_from_stats,
            log.session_id,
            log.created_at
        ],
    )
    .expect("insert usage log");
}

#[test]
fn lifecycle_interruption_rows_are_excluded_from_usage_summary_and_leaderboard() {
    let conn = setup_conn();
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 1,
            provider_name: "Included Provider",
            duration_ms: 1000,
            input_tokens: Some(80),
            output_tokens: Some(20),
            total_tokens: Some(100),
            cost_usd_femto: Some(1_000_000_000_000_000),
            ..base_usage_log(1_000)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 1,
            provider_name: "Included Provider",
            status: Some(500),
            error_code: Some("UPSTREAM_ERROR"),
            duration_ms: 2500,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cost_usd_femto: None,
            ..base_usage_log(1_001)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 2,
            provider_name: "Interrupted Provider",
            status: Some(499),
            error_code: Some("GW_REQUEST_INTERRUPTED_BY_RESTART"),
            duration_ms: 99_000,
            input_tokens: Some(8_000),
            output_tokens: Some(2_000),
            total_tokens: Some(10_000),
            cost_usd_femto: Some(99_000_000_000_000_000),
            excluded_from_stats: 1,
            ..base_usage_log(1_002)
        },
    );

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary");
    assert_eq!(summary.requests_total, 2);
    assert_eq!(summary.requests_failed, 1);
    assert_eq!(summary.total_duration_ms, 3500);
    assert_eq!(summary.total_tokens, 100);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "codex:1");
    assert_eq!(rows[0].requests_total, 2);
    assert_eq!(rows[0].requests_failed, 1);
    assert_eq!(rows[0].total_duration_ms, 3500);
    assert_eq!(rows[0].total_tokens, 100);
}

fn insert_migrated_provider(
    conn: &Connection,
    id: i64,
    cli_key: &str,
    name: &str,
    source_provider_id: Option<i64>,
    bridge_type: Option<&str>,
) {
    conn.execute(
        r#"
INSERT INTO providers (
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json,
  source_provider_id,
  bridge_type
) VALUES (
  ?1,
  ?2,
  ?3,
  'https://example.invalid',
  '[]',
  'order',
  '{}',
  'test-key',
  1,
  100,
  1000,
  1000,
  0,
  1.0,
  '{}',
  '{}',
  ?4,
  ?5
);
        "#,
        params![id, cli_key, name, source_provider_id, bridge_type],
    )
    .expect("insert migrated provider");
}

#[allow(clippy::too_many_arguments)]
fn insert_migrated_usage_log(
    conn: &Connection,
    trace_id: &str,
    cli_key: &str,
    provider_id: i64,
    provider_name: &str,
    input_tokens: i64,
    output_tokens: i64,
    created_at: i64,
    session_id: Option<&str>,
) {
    let attempts_json = format!(
        r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
    );
    conn.execute(
        r#"
INSERT INTO request_logs (
  trace_id,
  cli_key,
  method,
  path,
  status,
  error_code,
  duration_ms,
  attempts_json,
  created_at,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  usage_json,
  ttfb_ms,
  requested_model,
  cost_usd_femto,
  excluded_from_stats,
  session_id,
  final_provider_id
) VALUES (
  ?1,
  ?2,
  'POST',
  '/v1/messages',
  200,
  NULL,
  1000,
  ?3,
  ?4,
  ?5,
  ?6,
  NULL,
  0,
  0,
  0,
  0,
  NULL,
  100,
  'model-test',
  NULL,
  0,
  ?7,
  ?8
);
        "#,
        params![
            trace_id,
            cli_key,
            attempts_json,
            created_at,
            input_tokens,
            output_tokens,
            session_id,
            provider_id
        ],
    )
    .expect("insert migrated usage log");
}

#[test]
fn usage_params_accept_generated_and_legacy_cx2cc_filter_keys() {
    let params: UsageQueryParams = serde_json::from_value(serde_json::json!({
        "period": "daily",
        "startTs": null,
        "endTs": null,
        "cliKey": null,
        "providerId": null,
        "folderKeys": null,
        "dayStartHour": 5,
        "excludeCx2CcGatewayBridge": true
    }))
    .expect("deserialize usage query params");
    assert_eq!(params.exclude_cx2cc_gateway_bridge, Some(true));
    assert_eq!(params.day_start_hour, Some(5));

    let legacy_params: UsageQueryParams = serde_json::from_value(serde_json::json!({
        "period": "daily",
        "startTs": null,
        "endTs": null,
        "cliKey": null,
        "providerId": null,
        "folderKeys": null,
        "excludeCx2ccGatewayBridge": true
    }))
    .expect("deserialize legacy usage query params");
    assert_eq!(legacy_params.exclude_cx2cc_gateway_bridge, Some(true));

    let detail_params: UsageDayDetailParams = serde_json::from_value(serde_json::json!({
        "day": "2026-04-22",
        "cliKey": null,
        "providerId": null,
        "folderLimit": 8,
        "folderKeys": null,
        "dayStartHour": 5,
        "excludeCx2CcGatewayBridge": true
    }))
    .expect("deserialize usage day detail params");
    assert_eq!(detail_params.exclude_cx2cc_gateway_bridge, Some(true));
    assert_eq!(detail_params.day_start_hour, Some(5));

    let legacy_detail_params: UsageDayDetailParams = serde_json::from_value(serde_json::json!({
        "day": "2026-04-22",
        "cliKey": null,
        "providerId": null,
        "folderLimit": 8,
        "folderKeys": null,
        "excludeCx2ccGatewayBridge": true
    }))
    .expect("deserialize legacy usage day detail params");
    assert_eq!(
        legacy_detail_params.exclude_cx2cc_gateway_bridge,
        Some(true)
    );
}

fn fixture_folder_lookup(keys: &[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder> {
    let requested: std::collections::HashSet<String> = keys
        .iter()
        .map(|key| format!("{}:{}", key.cli_key, key.session_id))
        .collect();
    let fixtures = [
        ("codex", "codex-alpha-1", "alpha", "/work/alpha"),
        ("codex", "codex-alpha-2", "alpha", "/work/alpha"),
        ("claude", "claude-alpha-1", "alpha", "/work/alpha"),
        ("codex", "codex-beta-1", "beta", "/work/beta"),
    ];

    fixtures
        .into_iter()
        .filter(|(cli_key, session_id, _, _)| {
            requested.contains(&format!("{cli_key}:{session_id}"))
        })
        .map(
            |(cli_key, session_id, folder_name, folder_path)| UsageResolvedFolder {
                cli_key: cli_key.to_string(),
                session_id: session_id.to_string(),
                folder_name: folder_name.to_string(),
                folder_path: folder_path.to_string(),
            },
        )
        .collect()
}

#[test]
fn cx2cc_gateway_bridge_filter_covers_overview_and_home_usage_queries() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    let start_ts = compute_start_ts_last_n_days(&conn, 1).expect("today start ts");
    let day = local_day_key(&conn, start_ts);

    insert_migrated_provider(&conn, 100, "claude", "CX2CC Gateway", None, Some("cx2cc"));
    insert_migrated_provider(&conn, 200, "codex", "Codex Inner", None, None);
    insert_migrated_provider(
        &conn,
        300,
        "claude",
        "CX2CC Fixed Source",
        Some(200),
        Some("cx2cc"),
    );

    insert_migrated_usage_log(
        &conn,
        "trace-outer",
        "claude",
        100,
        "CX2CC Gateway",
        1_000,
        100,
        start_ts + 60,
        Some("claude-alpha-1"),
    );
    insert_migrated_usage_log(
        &conn,
        "trace-inner",
        "codex",
        200,
        "Codex Inner",
        2_000,
        200,
        start_ts + 120,
        Some("codex-alpha-1"),
    );
    insert_migrated_usage_log(
        &conn,
        "trace-fixed-source",
        "claude",
        300,
        "CX2CC Fixed Source",
        3_000,
        300,
        start_ts + 180,
        Some("claude-alpha-1"),
    );

    let unfiltered = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        false,
    )
    .expect("unfiltered summary");
    assert_eq!(unfiltered.requests_total, 3);
    assert_eq!(unfiltered.total_tokens, 6_600);

    let filtered = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        true,
    )
    .expect("filtered summary");
    assert_eq!(filtered.requests_total, 2);
    assert_eq!(filtered.total_tokens, 5_500);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        Some(50),
        true,
    )
    .expect("filtered provider leaderboard");
    let keys: std::collections::HashSet<&str> = rows.iter().map(|row| row.key.as_str()).collect();
    assert!(!keys.contains("claude:100"));
    assert!(keys.contains("codex:200"));
    assert!(keys.contains("claude:300"));

    let summary_v2_filtered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered summary v2");
    assert_eq!(summary_v2_filtered.requests_total, 2);
    assert_eq!(summary_v2_filtered.total_tokens, 5_500);

    let summary_v2_unfiltered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(false),
        },
        fixture_folder_lookup,
    )
    .expect("unfiltered summary v2");
    assert_eq!(summary_v2_unfiltered.requests_total, 3);
    assert_eq!(summary_v2_unfiltered.total_tokens, 6_600);

    let folder_options = folder_options_v1_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered folder options");
    let alpha = folder_options
        .iter()
        .find(|row| row.key == "/work/alpha")
        .expect("alpha folder option");
    assert_eq!(alpha.requests_total, 2);
    assert_eq!(alpha.total_tokens, 5_500);

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered day detail");
    assert_eq!(
        detail
            .hours
            .iter()
            .map(|row| row.requests_total)
            .sum::<i64>(),
        2
    );
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].key, "/work/alpha");
    assert_eq!(detail.folders[0].total_tokens, 5_500);

    drop(conn);

    let hourly_rows = hourly_series(&db, 1).expect("hourly series");
    let hourly_total: i64 = hourly_rows.iter().map(|row| row.total_tokens).sum();
    assert_eq!(hourly_total, 5_500);
}

#[test]
fn v2_cache_rate_denominator_aligns_across_clis() {
    let conn = setup_conn();

    // Codex/Gemini: cache_read_input_tokens is a subset of input_tokens.
    conn.execute(
        r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "codex",
            r#"[{"provider_id":123,"provider_name":"OpenAI","outcome":"success"}]"#,
            123,
            "gpt-test",
            200,
            Option::<String>::None,
            1000,
            100,
            100,
            10,
            999,
            30,
            0,
            0,
            0,
            1_000_000_000_000_000i64,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert codex");

    conn.execute(
        r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "gemini",
            r#"[{"provider_id":456,"provider_name":"GeminiUpstream","outcome":"success"}]"#,
            456,
            "gemini-test",
            200,
            Option::<String>::None,
            1000,
            100,
            200,
            20,
            0,
            50,
            0,
            0,
            0,
            2_000_000_000_000_000i64,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert gemini");

    // Claude: cache_read/cache_creation are additional buckets (not a subset of input_tokens).
    conn.execute(
        r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "claude",
            r#"[{"provider_id":789,"provider_name":"ClaudeUpstream","outcome":"success"}]"#,
            789,
            "claude-test",
            200,
            Option::<String>::None,
            1000,
            100,
            300,
            30,
            Option::<i64>::None,
            40,
            25,
            0,
            0,
            Option::<i64>::None,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert claude");

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary_query");
    assert_eq!(summary.requests_total, 3);
    assert_eq!(summary.cost_covered_success, 2);
    assert_eq!(summary.input_tokens, 520);
    assert_eq!(summary.output_tokens, 60);
    assert_eq!(summary.io_total_tokens, 580);
    assert_eq!(summary.cache_read_input_tokens, 120);
    assert_eq!(summary.cache_creation_input_tokens, 25);
    assert_eq!(summary.total_tokens, 725);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn");
    assert_eq!(rows.len(), 3);

    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();

    let codex = by_key.get("codex:123").expect("codex row");
    assert_eq!(codex.input_tokens, 70);
    assert_eq!(codex.output_tokens, 10);
    assert_eq!(codex.io_total_tokens, 80);
    assert_eq!(codex.cache_read_input_tokens, 30);
    assert_eq!(codex.cache_creation_input_tokens, 0);
    assert_eq!(codex.total_tokens, 110);
    assert_eq!(codex.cost_usd, Some(1.0));

    let gemini = by_key.get("gemini:456").expect("gemini row");
    assert_eq!(gemini.input_tokens, 150);
    assert_eq!(gemini.output_tokens, 20);
    assert_eq!(gemini.io_total_tokens, 170);
    assert_eq!(gemini.cache_read_input_tokens, 50);
    assert_eq!(gemini.cache_creation_input_tokens, 0);
    assert_eq!(gemini.total_tokens, 220);
    assert_eq!(gemini.cost_usd, Some(2.0));

    let claude = by_key.get("claude:789").expect("claude row");
    assert_eq!(claude.input_tokens, 300);
    assert_eq!(claude.output_tokens, 30);
    assert_eq!(claude.io_total_tokens, 330);
    assert_eq!(claude.cache_read_input_tokens, 40);
    assert_eq!(claude.cache_creation_input_tokens, 25);
    assert_eq!(claude.total_tokens, 395);
    assert_eq!(claude.cost_usd, None);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Cli,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn cli");
    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();
    assert_eq!(
        by_key.get("codex").expect("codex cli row").cost_usd,
        Some(1.0)
    );
    assert_eq!(
        by_key.get("gemini").expect("gemini cli row").cost_usd,
        Some(2.0)
    );
    assert_eq!(by_key.get("claude").expect("claude cli row").cost_usd, None);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Model,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn model");
    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();
    assert_eq!(
        by_key.get("gpt-test").expect("gpt-test model row").cost_usd,
        Some(1.0)
    );
    assert_eq!(
        by_key
            .get("gemini-test")
            .expect("gemini-test model row")
            .cost_usd,
        Some(2.0)
    );
    assert_eq!(
        by_key
            .get("claude-test")
            .expect("claude-test model row")
            .cost_usd,
        None
    );
}

#[test]
fn v2_cache_rate_denominator_treats_cx2cc_like_cached_input_subtract() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id) VALUES (?1, ?2, ?3);"#,
        params![900, "Bridge CX2CC", 42],
    )
    .expect("insert provider");

    conn.execute(
        r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
        "#,
        params![
            "claude",
            r#"[{"provider_id":900,"provider_name":"Bridge CX2CC","outcome":"success"}]"#,
            900,
            "claude-through-cx2cc",
            200,
            Option::<String>::None,
            1000,
            100,
            100,
            10,
            Option::<i64>::None,
            30,
            0,
            0,
            0,
            Option::<i64>::None,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert cx2cc request");

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary_query");
    assert_eq!(summary.cost_covered_success, 0);
    assert_eq!(summary.input_tokens, 70);
    assert_eq!(summary.cache_read_input_tokens, 30);
    assert_eq!(summary.total_tokens, 110);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn");
    let row = rows
        .iter()
        .find(|row| row.key == "claude:900")
        .expect("cx2cc provider row");
    assert_eq!(row.input_tokens, 70);
    assert_eq!(row.cache_read_input_tokens, 30);
    assert_eq!(row.total_tokens, 110);
}

#[test]
fn v2_provider_leaderboard_dedupes_by_provider_id() {
    let conn = setup_conn();

    for (provider_name, created_at) in [("OpenAI", 1000i64), ("OpenAI ", 1001i64)] {
        let attempts_json = format!(
            r#"[{{"provider_id":123,"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  status,
  error_code,
  duration_ms,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
        "#,
            params![
                "codex",
                attempts_json,
                123,
                200,
                Option::<String>::None,
                1000,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn provider");

    let keys: std::collections::HashSet<&str> = rows.iter().map(|row| row.key.as_str()).collect();
    assert_eq!(keys.len(), rows.len());

    let row = rows
        .iter()
        .find(|row| row.key == "codex:123")
        .expect("codex provider row");
    assert_eq!(row.name, "codex/OpenAI");
    assert_eq!(row.requests_total, 2);
    assert_eq!(row.requests_success, 2);
    assert_eq!(row.requests_failed, 0);
}

#[test]
fn v1_provider_cache_rate_trend_uses_effective_denom_and_bucket() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name) VALUES (?1, ?2);"#,
        params![123, "OpenAI"],
    )
    .expect("insert provider");

    let start_ts_today = compute_start_ts(&conn, UsageRange::Today)
        .expect("compute_start_ts today")
        .expect("start ts exists");

    for (created_at, input_tokens, cache_read_input_tokens, cache_creation_input_tokens) in [
        (start_ts_today + 3600, 500i64, 200i64, 20i64),
        (start_ts_today + 7200, 100i64, 50i64, 10i64),
    ] {
        conn.execute(
            r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  status,
  error_code,
  duration_ms,
  input_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);
            "#,
            params![
                "codex",
                r#"[{"provider_id":123,"provider_name":"OpenAI","outcome":"success"}]"#,
                123,
                200,
                Option::<String>::None,
                1000,
                input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let rows_hour = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            period: UsagePeriodV2::Daily,
            start_ts: Some(start_ts_today),
            end_ts: Some(start_ts_today + 86_400),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider_cache_rate_trend_v1_with_conn hour");

    assert_eq!(rows_hour.len(), 2);
    assert_eq!(rows_hour[0].name, "codex/OpenAI");
    assert_eq!(rows_hour[0].hour, Some(1));
    assert_eq!(rows_hour[0].denom_tokens, 520);
    assert_eq!(rows_hour[0].cache_read_input_tokens, 200);

    assert_eq!(rows_hour[1].hour, Some(2));
    assert_eq!(rows_hour[1].denom_tokens, 110);
    assert_eq!(rows_hour[1].cache_read_input_tokens, 50);

    // Weekly bucket is day-based and aggregates both rows into a single point.
    let rows_day = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            period: UsagePeriodV2::Weekly,
            start_ts: Some(start_ts_today),
            end_ts: Some(start_ts_today + 86_400),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider_cache_rate_trend_v1_with_conn day");

    assert_eq!(rows_day.len(), 1);
    assert_eq!(rows_day[0].hour, None);
    assert_eq!(rows_day[0].denom_tokens, 630);
    assert_eq!(rows_day[0].cache_read_input_tokens, 250);
    assert_eq!(rows_day[0].requests_success, 2);
}

#[test]
fn provider_cache_rate_trend_excludes_cx2cc_gateway_bridge_when_requested() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![123, "OpenAI", Option::<i64>::None, Option::<String>::None],
    )
    .expect("insert normal provider");
    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![900, "Bridge CX2CC", Option::<i64>::None, "cx2cc"],
    )
    .expect("insert cx2cc provider");

    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(120),
            cache_read_input_tokens: Some(20),
            created_at: 1000,
            ..base_usage_log(1000)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            provider_id: 900,
            provider_name: "Bridge CX2CC",
            input_tokens: Some(240),
            cache_read_input_tokens: Some(80),
            created_at: 1010,
            ..base_usage_log(1010)
        },
    );

    let rows_with_bridge = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            period: UsagePeriodV2::Daily,
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("cache trend with bridge");
    assert_eq!(rows_with_bridge.len(), 2);

    let rows_without_bridge = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            period: UsagePeriodV2::Daily,
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: true,
        },
    )
    .expect("cache trend without bridge");
    assert_eq!(rows_without_bridge.len(), 1);
    assert_eq!(rows_without_bridge[0].key, "codex:123");
}

#[test]
fn v2_queries_apply_provider_filter() {
    let conn = setup_conn();

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    for (provider_id, cli_key, provider_name, input_tokens, created_at) in [
        (123, "codex", "OpenAI", 120, 1000i64),
        (456, "gemini", "Gemini Upstream", 240, 1010i64),
    ] {
        let attempts_json = format!(
            r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
            "#,
            params![
                cli_key,
                attempts_json,
                provider_id,
                "model-test",
                200,
                Option::<String>::None,
                1000,
                100,
                input_tokens,
                20,
                Option::<i64>::None,
                10,
                0,
                0,
                0,
                Option::<i64>::None,
                Option::<String>::None,
                0,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let summary =
        summary_query(&conn, None, None, None, Some(123), false).expect("filtered summary");
    assert_eq!(summary.requests_total, 1);
    assert_eq!(summary.requests_success, 1);
    assert_eq!(summary.cost_covered_success, 0);
    assert_eq!(summary.input_tokens, 110);

    let cli_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Cli,
        None,
        None,
        None,
        Some(123),
        Some(50),
        false,
    )
    .expect("filtered cli leaderboard");
    assert_eq!(cli_rows.len(), 1);
    assert_eq!(cli_rows[0].key, "codex");
    assert_eq!(cli_rows[0].requests_total, 1);

    let cache_rows = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            period: UsagePeriodV2::Daily,
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: Some(123),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("filtered cache trend");
    assert_eq!(cache_rows.len(), 1);
    assert_eq!(cache_rows[0].key, "codex:123");
}

#[test]
fn v2_day_leaderboard_groups_by_local_day_and_applies_filters() {
    let conn = setup_conn();

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    let day_one_ts = 1_704_108_800i64;
    let day_two_ts = day_one_ts + 86_400;
    let end_ts = day_one_ts + 172_800;

    for (
        cli_key,
        provider_id,
        provider_name,
        requested_model,
        input_tokens,
        output_tokens,
        cache_read_input_tokens,
        cost_usd_femto,
        created_at,
    ) in [
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            100i64,
            50i64,
            10i64,
            1_000_000_000_000_000i64,
            day_one_ts,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            200i64,
            40i64,
            20i64,
            2_000_000_000_000_000i64,
            day_one_ts + 3600,
        ),
        (
            "gemini",
            456,
            "Gemini Upstream",
            "gemini-test",
            300i64,
            30i64,
            30i64,
            3_000_000_000_000_000i64,
            day_two_ts,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            999i64,
            1i64,
            0i64,
            4_000_000_000_000_000i64,
            end_ts,
        ),
    ] {
        let attempts_json = format!(
            r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO request_logs (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
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
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
            "#,
            params![
                cli_key,
                attempts_json,
                provider_id,
                requested_model,
                200,
                Option::<String>::None,
                1000,
                100,
                input_tokens,
                output_tokens,
                Option::<i64>::None,
                cache_read_input_tokens,
                0,
                0,
                0,
                cost_usd_femto,
                Option::<String>::None,
                0,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let day_one = local_day_key(&conn, day_one_ts);
    let day_two = local_day_key(&conn, day_two_ts);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        None,
        None,
        Some(50),
        false,
    )
    .expect("day leaderboard");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].key, day_two);
    assert_eq!(rows[0].name, day_two);
    assert_eq!(rows[0].requests_total, 1);
    assert_eq!(rows[0].input_tokens, 270);
    assert_eq!(rows[0].output_tokens, 30);
    assert_eq!(rows[0].total_tokens, 330);
    assert_eq!(rows[0].cost_usd, Some(3.0));
    assert_eq!(rows[0].first_request_created_at_ms, Some(day_two_ts * 1000));
    assert_eq!(rows[0].last_request_created_at_ms, Some(day_two_ts * 1000));

    assert_eq!(rows[1].key, day_one);
    assert_eq!(rows[1].name, day_one);
    assert_eq!(rows[1].requests_total, 2);
    assert_eq!(rows[1].input_tokens, 270);
    assert_eq!(rows[1].output_tokens, 90);
    assert_eq!(rows[1].total_tokens, 390);
    assert_eq!(rows[1].cost_usd, Some(3.0));
    assert_eq!(rows[1].first_request_created_at_ms, Some(day_one_ts * 1000));
    assert_eq!(
        rows[1].last_request_created_at_ms,
        Some((day_one_ts + 3600) * 1000)
    );

    let cli_filtered = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        Some("codex"),
        None,
        Some(50),
        false,
    )
    .expect("day leaderboard cli filter");
    assert_eq!(cli_filtered.len(), 1);
    assert_eq!(cli_filtered[0].key, day_one);
    assert_eq!(cli_filtered[0].requests_total, 2);
    assert_eq!(
        cli_filtered[0].last_request_created_at_ms,
        Some((day_one_ts + 3600) * 1000)
    );

    let provider_filtered = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        None,
        Some(456),
        Some(50),
        false,
    )
    .expect("day leaderboard provider filter");
    assert_eq!(provider_filtered.len(), 1);
    assert_eq!(provider_filtered[0].key, day_two);
    assert_eq!(provider_filtered[0].requests_total, 1);

    let model_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Model,
        Some(day_one_ts),
        Some(end_ts),
        None,
        None,
        Some(50),
        false,
    )
    .expect("model leaderboard");
    assert!(model_rows
        .iter()
        .all(|row| row.first_request_created_at_ms.is_none()
            && row.last_request_created_at_ms.is_none()));
}

#[test]
fn v2_day_leaderboard_respects_usage_day_start_hour() {
    let conn = setup_conn();
    let day_one = "2026-04-16";
    let day_two = "2026-04-17";
    let day_start_hour = 5;
    let usage_day_one_start = local_usage_day_start_ts(&conn, day_one, day_start_hour);
    let usage_day_two_start = local_usage_day_start_ts(&conn, day_two, day_start_hour);
    let query_end = local_usage_day_start_ts(&conn, "2026-04-18", day_start_hour);

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    for (cli_key, provider_id, provider_name, session_id, created_at, input_tokens) in [
        (
            "codex",
            123,
            "OpenAI",
            "codex-alpha-1",
            usage_day_one_start + 4 * 3600,
            100i64,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "codex-alpha-2",
            usage_day_one_start + 21 * 3600,
            200i64,
        ),
        (
            "codex",
            456,
            "Gemini Upstream",
            "codex-beta-1",
            usage_day_two_start + 4 * 3600,
            300i64,
        ),
        (
            "claude",
            123,
            "OpenAI",
            "claude-alpha-1",
            usage_day_two_start + 15 * 3600,
            400i64,
        ),
    ] {
        insert_usage_log(
            &conn,
            TestUsageLog {
                cli_key,
                provider_id,
                provider_name,
                requested_model: "model-test",
                input_tokens: Some(input_tokens),
                output_tokens: Some(10),
                session_id: Some(session_id),
                created_at,
                ..base_usage_log(created_at)
            },
        );
    }

    let usage_day_rows = leaderboard_v2_with_conn_day_start(
        &conn,
        UsageScopeV2::Day,
        Some(usage_day_one_start),
        Some(query_end),
        None,
        None,
        Some(50),
        false,
        day_start_hour,
    )
    .expect("usage day leaderboard");
    assert_eq!(usage_day_rows.len(), 2);
    assert_eq!(usage_day_rows[0].key, day_two);
    assert_eq!(usage_day_rows[0].requests_total, 2);
    assert_eq!(
        usage_day_rows[0].first_request_created_at_ms,
        Some((usage_day_two_start + 4 * 3600) * 1000)
    );
    assert_eq!(
        usage_day_rows[0].last_request_created_at_ms,
        Some((usage_day_two_start + 15 * 3600) * 1000)
    );
    assert_eq!(usage_day_rows[1].key, day_one);
    assert_eq!(usage_day_rows[1].requests_total, 2);
    assert_eq!(
        usage_day_rows[1].first_request_created_at_ms,
        Some((usage_day_one_start + 4 * 3600) * 1000)
    );
    assert_eq!(
        usage_day_rows[1].last_request_created_at_ms,
        Some((usage_day_one_start + 21 * 3600) * 1000)
    );

    let natural_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(usage_day_one_start),
        Some(query_end),
        None,
        None,
        Some(50),
        false,
    )
    .expect("natural day leaderboard");
    assert_eq!(natural_rows.len(), 2);
    assert_eq!(natural_rows[0].key, day_two);
    assert_eq!(natural_rows[0].requests_total, 3);
    assert_eq!(
        natural_rows[0].first_request_created_at_ms,
        Some((usage_day_one_start + 21 * 3600) * 1000)
    );
    assert_eq!(
        natural_rows[0].last_request_created_at_ms,
        Some((usage_day_two_start + 15 * 3600) * 1000)
    );
    assert_eq!(natural_rows[1].key, day_one);
    assert_eq!(natural_rows[1].requests_total, 1);

    let folder_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Day,
            start_ts: Some(usage_day_one_start),
            end_ts: Some(query_end),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour,
        },
        fixture_folder_lookup,
    )
    .expect("folder filtered usage day leaderboard");
    assert_eq!(folder_rows.len(), 2);
    assert_eq!(folder_rows[0].key, day_two);
    assert_eq!(folder_rows[0].requests_total, 1);
    assert_eq!(folder_rows[1].key, day_one);
    assert_eq!(folder_rows[1].requests_total, 2);

    let day_one_detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day_one.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: Some(day_start_hour),
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("usage day detail");
    assert_eq!(
        day_one_detail
            .hours
            .iter()
            .map(|row| row.requests_total)
            .sum::<i64>(),
        2
    );
    assert_eq!(day_one_detail.hours[2].requests_total, 1);
    assert_eq!(day_one_detail.hours[9].requests_total, 1);
    assert_eq!(day_one_detail.folders.len(), 1);
    assert_eq!(day_one_detail.folders[0].key, "/work/alpha");
    assert_eq!(day_one_detail.folders[0].requests_total, 2);
}

#[test]
fn day_detail_v1_filters_by_local_day_and_returns_hour_buckets() {
    let conn = setup_conn();
    let day = "2026-04-16";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(120),
            output_tokens: Some(30),
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            session_id: Some("codex-hour-2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(50),
            output_tokens: Some(10),
            cache_read_input_tokens: Some(5),
            cache_creation_input_tokens: Some(5),
            session_id: Some("claude-hour-2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 456,
            provider_name: "Gemini Upstream",
            input_tokens: Some(70),
            output_tokens: Some(20),
            cache_read_input_tokens: Some(10),
            session_id: Some("codex-hour-5"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            input_tokens: Some(999),
            output_tokens: Some(1),
            created_at: start_ts - 1,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            input_tokens: Some(999),
            output_tokens: Some(1),
            created_at: start_ts + 86_400,
            ..base_usage_log(start_ts)
        },
    );

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("day detail");

    assert_eq!(detail.day, day);
    assert_eq!(detail.hours.len(), 24);
    assert_eq!(detail.hours[0].hour, 0);
    assert_eq!(detail.hours[0].requests_total, 0);
    assert_eq!(detail.hours[0].total_tokens, 0);
    assert_eq!(detail.hours[2].requests_total, 2);
    assert_eq!(detail.hours[2].total_tokens, 230);
    assert_eq!(detail.hours[2].io_total_tokens, 190);
    assert_eq!(detail.hours[5].requests_total, 1);
    assert_eq!(detail.hours[5].total_tokens, 90);
    assert_eq!(detail.hours[23].hour, 23);
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].name, "未知文件夹");
    assert_eq!(detail.folders[0].requests_total, 3);
    assert_eq!(detail.folders[0].total_tokens, 320);

    let provider_filtered = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: Some(456),
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("provider filtered day detail");
    assert_eq!(provider_filtered.hours[2].requests_total, 0);
    assert_eq!(provider_filtered.hours[5].requests_total, 1);
    assert_eq!(provider_filtered.hours[5].total_tokens, 90);
    assert_eq!(provider_filtered.folders[0].requests_total, 1);

    let cli_filtered = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: Some("claude".to_string()),
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("cli filtered day detail");
    assert_eq!(cli_filtered.hours[2].requests_total, 1);
    assert_eq!(cli_filtered.hours[2].total_tokens, 70);
    assert_eq!(cli_filtered.hours[5].requests_total, 0);
    assert_eq!(cli_filtered.folders[0].input_tokens, 50);
}

#[test]
fn day_detail_v1_groups_resolved_folders_and_unknown_sessions() {
    let conn = setup_conn();
    let day = "2026-04-17";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(120),
            output_tokens: Some(30),
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            session_id: Some("codex-s1"),
            created_at: start_ts + 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(80),
            output_tokens: Some(20),
            session_id: Some("codex-s2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            status: Some(500),
            input_tokens: Some(60),
            output_tokens: Some(10),
            session_id: Some("claude-s3"),
            created_at: start_ts + 3 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(40),
            output_tokens: Some(5),
            session_id: Some("missing-folder"),
            created_at: start_ts + 4 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            input_tokens: Some(50),
            output_tokens: Some(5),
            cache_read_input_tokens: Some(10),
            session_id: Some("gemini-s1"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: None,
            created_at: start_ts + 6 * 3600,
            ..base_usage_log(start_ts)
        },
    );

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |keys| {
            let mut pairs: Vec<String> = keys
                .iter()
                .map(|key| format!("{}:{}", key.cli_key, key.session_id))
                .collect();
            pairs.sort();
            assert_eq!(
                pairs,
                vec![
                    "claude:claude-s3".to_string(),
                    "codex:codex-s1".to_string(),
                    "codex:codex-s2".to_string(),
                    "codex:missing-folder".to_string(),
                ]
            );

            vec![
                UsageDayResolvedFolder {
                    cli_key: "codex".to_string(),
                    session_id: "codex-s1".to_string(),
                    folder_name: "alpha".to_string(),
                    folder_path: "/work/alpha".to_string(),
                },
                UsageDayResolvedFolder {
                    cli_key: "codex".to_string(),
                    session_id: "codex-s2".to_string(),
                    folder_name: "alpha".to_string(),
                    folder_path: "/work/alpha".to_string(),
                },
                UsageDayResolvedFolder {
                    cli_key: "claude".to_string(),
                    session_id: "claude-s3".to_string(),
                    folder_name: "beta".to_string(),
                    folder_path: "/work/beta".to_string(),
                },
            ]
        },
    )
    .expect("day detail");

    let by_key: std::collections::HashMap<String, UsageDayFolderRow> = detail
        .folders
        .into_iter()
        .map(|row| (row.key.clone(), row))
        .collect();

    let alpha = by_key.get("/work/alpha").expect("alpha folder row");
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.folder_path.as_deref(), Some("/work/alpha"));
    assert_eq!(alpha.requests_total, 2);
    assert_eq!(alpha.requests_success, 2);
    assert_eq!(alpha.total_tokens, 260);
    assert_eq!(alpha.io_total_tokens, 230);

    let beta = by_key.get("/work/beta").expect("beta folder row");
    assert_eq!(beta.name, "beta");
    assert_eq!(beta.requests_total, 1);
    assert_eq!(beta.requests_success, 0);
    assert_eq!(beta.requests_failed, 1);
    assert_eq!(beta.total_tokens, 70);

    let unknown = by_key.get("__unknown__").expect("unknown folder row");
    assert_eq!(unknown.name, "未知文件夹");
    assert_eq!(unknown.folder_path, None);
    assert_eq!(unknown.requests_total, 3);
    assert_eq!(unknown.total_tokens, 135);
}

#[test]
fn folder_options_v1_groups_resolved_folders_and_keeps_unknown_selectable() {
    let conn = setup_conn();
    let day = "2026-04-18";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(100),
            output_tokens: Some(20),
            session_id: Some("codex-alpha-1"),
            created_at: start_ts + 60,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(40),
            output_tokens: Some(10),
            session_id: Some("codex-alpha-2"),
            created_at: start_ts + 120,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: Some("gemini-unknown"),
            created_at: start_ts + 180,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(20),
            output_tokens: Some(5),
            session_id: Some("missing-folder"),
            created_at: start_ts + 240,
            ..base_usage_log(start_ts)
        },
    );

    let options = folder_options_v1_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("folder options");

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].key, "/work/alpha");
    assert_eq!(options[0].name, "alpha");
    assert_eq!(options[0].folder_path.as_deref(), Some("/work/alpha"));
    assert_eq!(options[0].requests_total, 2);
    assert_eq!(options[0].total_tokens, 170);
    assert_eq!(options[1].key, "__unknown__");
    assert_eq!(options[1].name, "未知文件夹");
    assert_eq!(options[1].folder_path, None);
    assert_eq!(options[1].requests_total, 2);
    assert_eq!(options[1].total_tokens, 60);
}

#[test]
fn folder_keys_filter_summary_leaderboard_and_day_detail() {
    let conn = setup_conn();
    let day = "2026-04-19";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 123,
            requested_model: "gpt-alpha",
            input_tokens: Some(100),
            output_tokens: Some(20),
            cost_usd_femto: Some(0),
            session_id: Some("codex-alpha-1"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 456,
            requested_model: "gpt-beta",
            input_tokens: Some(70),
            output_tokens: Some(10),
            session_id: Some("codex-beta-1"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            provider_id: 456,
            requested_model: "gemini-unknown",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: Some("gemini-unknown"),
            created_at: start_ts + 7 * 3600,
            ..base_usage_log(start_ts)
        },
    );

    let alpha_params = UsageQueryParams {
        period: "custom".to_string(),
        start_ts: Some(start_ts),
        end_ts: Some(start_ts + 86_400),
        cli_key: None,
        provider_id: None,
        folder_keys: Some(vec!["/work/alpha".to_string()]),
        day_start_hour: None,
        exclude_cx2cc_gateway_bridge: None,
    };
    let unfiltered_summary = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            folder_keys: None,
            ..alpha_params.clone()
        },
        fixture_folder_lookup,
    )
    .expect("unfiltered summary");
    assert_eq!(unfiltered_summary.cost_covered_success, 1);

    let alpha_summary =
        summary_v2_with_conn(&conn, &alpha_params, fixture_folder_lookup).expect("summary");
    assert_eq!(alpha_summary.requests_total, 1);
    assert_eq!(alpha_summary.cost_covered_success, 1);
    assert_eq!(alpha_summary.total_tokens, 120);
    assert_eq!(alpha_summary.io_total_tokens, 120);

    let alpha_day_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Day,
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour: 0,
        },
        fixture_folder_lookup,
    )
    .expect("day leaderboard");
    assert_eq!(alpha_day_rows.len(), 1);
    assert_eq!(alpha_day_rows[0].key, day);
    assert_eq!(alpha_day_rows[0].total_tokens, 120);
    assert_eq!(
        alpha_day_rows[0].first_request_created_at_ms,
        Some((start_ts + 2 * 3600) * 1000)
    );
    assert_eq!(
        alpha_day_rows[0].last_request_created_at_ms,
        Some((start_ts + 2 * 3600) * 1000)
    );

    let alpha_model_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Model,
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour: 0,
        },
        fixture_folder_lookup,
    )
    .expect("model leaderboard");
    assert_eq!(alpha_model_rows.len(), 1);
    assert_eq!(alpha_model_rows[0].key, "gpt-alpha");
    assert_eq!(alpha_model_rows[0].first_request_created_at_ms, None);
    assert_eq!(alpha_model_rows[0].last_request_created_at_ms, None);

    let unknown_summary = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            folder_keys: Some(vec!["__unknown__".to_string()]),
            exclude_cx2cc_gateway_bridge: None,
            ..alpha_params.clone()
        },
        fixture_folder_lookup,
    )
    .expect("unknown summary");
    assert_eq!(unknown_summary.requests_total, 1);
    assert_eq!(unknown_summary.total_tokens, 35);

    let provider_filtered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            provider_id: Some(456),
            folder_keys: Some(vec!["/work/beta".to_string()]),
            exclude_cx2cc_gateway_bridge: None,
            ..alpha_params
        },
        fixture_folder_lookup,
    )
    .expect("provider plus folder summary");
    assert_eq!(provider_filtered.requests_total, 1);
    assert_eq!(provider_filtered.total_tokens, 80);

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("day detail");
    assert_eq!(detail.hours.len(), 24);
    assert_eq!(detail.hours[2].requests_total, 1);
    assert_eq!(detail.hours[2].total_tokens, 120);
    assert_eq!(detail.hours[5].requests_total, 0);
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].key, "/work/alpha");
    assert_eq!(detail.folders[0].total_tokens, 120);
}
