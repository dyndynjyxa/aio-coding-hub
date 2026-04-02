use super::cache_rate_trend_v1::provider_cache_rate_trend_v1_with_conn;
use super::leaderboard_v2::leaderboard_v2_with_conn;
use super::summary::summary_query;
use super::*;
use rusqlite::{params, Connection};

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
	CREATE TABLE providers (
	  id INTEGER PRIMARY KEY,
	  name TEXT NOT NULL,
	  source_provider_id INTEGER
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
	  created_at INTEGER NOT NULL
	);
	"#,
    )
    .expect("create schema");
    conn
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

    let summary = summary_query(&conn, None, None, None, None).expect("summary_query");
    assert_eq!(summary.requests_total, 3);
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

    let rows = leaderboard_v2_with_conn(&conn, UsageScopeV2::Cli, None, None, None, None, Some(50))
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

    let rows =
        leaderboard_v2_with_conn(&conn, UsageScopeV2::Model, None, None, None, None, Some(50))
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

    let summary = summary_query(&conn, None, None, None, None).expect("summary_query");
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
        UsagePeriodV2::Daily,
        Some(start_ts_today),
        Some(start_ts_today + 86_400),
        None,
        None,
        None,
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
        UsagePeriodV2::Weekly,
        Some(start_ts_today),
        Some(start_ts_today + 86_400),
        None,
        None,
        None,
    )
    .expect("provider_cache_rate_trend_v1_with_conn day");

    assert_eq!(rows_day.len(), 1);
    assert_eq!(rows_day[0].hour, None);
    assert_eq!(rows_day[0].denom_tokens, 630);
    assert_eq!(rows_day[0].cache_read_input_tokens, 250);
    assert_eq!(rows_day[0].requests_success, 2);
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

    let summary = summary_query(&conn, None, None, None, Some(123)).expect("filtered summary");
    assert_eq!(summary.requests_total, 1);
    assert_eq!(summary.requests_success, 1);
    assert_eq!(summary.input_tokens, 110);

    let cli_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Cli,
        None,
        None,
        None,
        Some(123),
        Some(50),
    )
    .expect("filtered cli leaderboard");
    assert_eq!(cli_rows.len(), 1);
    assert_eq!(cli_rows[0].key, "codex");
    assert_eq!(cli_rows[0].requests_total, 1);

    let cache_rows = provider_cache_rate_trend_v1_with_conn(
        &conn,
        UsagePeriodV2::Daily,
        None,
        None,
        None,
        Some(123),
        None,
    )
    .expect("filtered cache trend");
    assert_eq!(cache_rows.len(), 1);
    assert_eq!(cache_rows[0].key, "codex:123");
}
