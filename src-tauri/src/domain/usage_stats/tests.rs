use super::*;
use rusqlite::{params, Connection};

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
CREATE TABLE request_logs (
  cli_key TEXT NOT NULL,
  attempts_json TEXT NOT NULL,
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
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17);
"#,
        params![
            "codex",
            r#"[{"provider_id":123,"provider_name":"OpenAI","outcome":"success"}]"#,
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
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17);
"#,
        params![
            "gemini",
            r#"[{"provider_id":456,"provider_name":"GeminiUpstream","outcome":"success"}]"#,
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
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17);
"#,
        params![
            "claude",
            r#"[{"provider_id":789,"provider_name":"ClaudeUpstream","outcome":"success"}]"#,
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
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert claude");

    let summary = summary_query(&conn, None, None, None).expect("summary_query");
    assert_eq!(summary.requests_total, 3);
    assert_eq!(summary.input_tokens, 520);
    assert_eq!(summary.output_tokens, 60);
    assert_eq!(summary.io_total_tokens, 580);
    assert_eq!(summary.cache_read_input_tokens, 120);
    assert_eq!(summary.cache_creation_input_tokens, 25);
    assert_eq!(summary.total_tokens, 725);

    let rows = leaderboard_v2_with_conn(&conn, UsageScopeV2::Provider, None, None, None, 50)
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

    let gemini = by_key.get("gemini:456").expect("gemini row");
    assert_eq!(gemini.input_tokens, 150);
    assert_eq!(gemini.output_tokens, 20);
    assert_eq!(gemini.io_total_tokens, 170);
    assert_eq!(gemini.cache_read_input_tokens, 50);
    assert_eq!(gemini.cache_creation_input_tokens, 0);
    assert_eq!(gemini.total_tokens, 220);

    let claude = by_key.get("claude:789").expect("claude row");
    assert_eq!(claude.input_tokens, 300);
    assert_eq!(claude.output_tokens, 30);
    assert_eq!(claude.io_total_tokens, 330);
    assert_eq!(claude.cache_read_input_tokens, 40);
    assert_eq!(claude.cache_creation_input_tokens, 25);
    assert_eq!(claude.total_tokens, 395);
}
