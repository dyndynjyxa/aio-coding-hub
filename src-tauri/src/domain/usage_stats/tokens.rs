pub(crate) fn is_bridged_input_semantics(
    source_provider_id: Option<i64>,
    bridge_type: Option<&str>,
) -> bool {
    crate::providers::is_cx2cc_bridge(source_provider_id, bridge_type)
}

fn uses_openai_input_semantics(
    cli_key: &str,
    persisted_openai_semantics: Option<bool>,
    legacy_provider_bridged: bool,
) -> bool {
    if matches!(cli_key, "codex" | "grok") {
        return true;
    }
    if let Some(openai_semantics) = persisted_openai_semantics {
        return openai_semantics;
    }
    legacy_provider_bridged
}

/// Single Rust owner for mutually-exclusive input token buckets.
/// OpenAI input includes cache reads and writes; Gemini only includes reads;
/// Claude and other protocols report cache buckets additively.
pub(crate) fn effective_input_tokens(
    cli_key: &str,
    persisted_openai_semantics: Option<bool>,
    legacy_provider_bridged: bool,
    input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
) -> i64 {
    let input = input_tokens.unwrap_or(0);
    if uses_openai_input_semantics(cli_key, persisted_openai_semantics, legacy_provider_bridged) {
        input
            .saturating_sub(cache_read_input_tokens.unwrap_or(0))
            .saturating_sub(cache_creation_input_tokens.unwrap_or(0))
            .max(0)
    } else if cli_key == "gemini" {
        input
            .saturating_sub(cache_read_input_tokens.unwrap_or(0))
            .max(0)
    } else {
        input
    }
}

/// Display variant of [`effective_input_tokens`]: keeps "usage unknown"
/// (`input_tokens: None`) as `None` instead of collapsing it to 0, so rows and
/// events without usage render as "—" rather than "0". Aggregates keep the
/// COALESCE(..., 0) semantics of the SQL expression.
pub(crate) fn effective_input_tokens_display(
    cli_key: &str,
    persisted_openai_semantics: Option<bool>,
    legacy_provider_bridged: bool,
    input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
) -> Option<i64> {
    input_tokens.is_some().then(|| {
        effective_input_tokens(
            cli_key,
            persisted_openai_semantics,
            legacy_provider_bridged,
            input_tokens,
            cache_read_input_tokens,
            cache_creation_input_tokens,
        )
    })
}

pub(super) fn effective_total_from_buckets(
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_input_tokens: i64,
    cache_read_input_tokens: i64,
) -> i64 {
    input_tokens
        .saturating_add(output_tokens)
        .saturating_add(cache_creation_input_tokens)
        .saturating_add(cache_read_input_tokens)
}

fn sql_column(alias: Option<&str>, column: &str) -> String {
    alias
        .map(|alias| format!("{alias}.{column}"))
        .unwrap_or_else(|| column.to_string())
}

fn build_sql_effective_input_tokens_expr(alias: Option<&str>) -> String {
    let cli_key = sql_column(alias, "cli_key");
    let final_provider_id = sql_column(alias, "final_provider_id");
    let special_settings_json = sql_column(alias, "special_settings_json");
    let input_tokens = sql_column(alias, "input_tokens");
    let cache_read_input_tokens = sql_column(alias, "cache_read_input_tokens");
    let cache_creation_input_tokens = sql_column(alias, "cache_creation_input_tokens");

    let safe_settings_json = format!(
        "CASE WHEN json_valid({special_settings_json}) THEN CASE WHEN json_type({special_settings_json}) = 'array' THEN {special_settings_json} ELSE '[]' END ELSE '[]' END"
    );
    let trimmed_source_cli_key = "TRIM(json_extract(marker.value, '$.source_cli_key'), char(32) || char(9) || char(10) || char(13))";
    let valid_marker_fields = format!(
        "json_extract(marker.value, '$.type') = 'cx2cc_cost_basis' AND json_type(marker.value, '$.source_cli_key') = 'text' AND {trimmed_source_cli_key} != ''"
    );
    let scoped_marker_condition = format!(
        "CASE WHEN marker.type = 'object' THEN {valid_marker_fields} AND json_type(marker.value, '$.bridge_provider_id') = 'integer' AND typeof(json_extract(marker.value, '$.bridge_provider_id')) = 'integer' AND json_extract(marker.value, '$.bridge_provider_id') > 0 ELSE 0 END"
    );
    let legacy_marker_condition = format!(
        "CASE WHEN marker.type = 'object' THEN {valid_marker_fields} AND json_type(marker.value, '$.bridge_provider_id') IS NULL ELSE 0 END"
    );
    let marker_semantics =
        format!("CASE WHEN {trimmed_source_cli_key} = 'codex' THEN 1 ELSE 0 END");
    let packed_marker_semantics = format!("(CAST(marker.key AS INTEGER) * 2 + {marker_semantics})");
    let persisted_marker_semantics = format!(
        "(SELECT CASE WHEN resolved.exact_match IS NOT NULL THEN resolved.exact_match % 2 WHEN resolved.has_scoped = 1 THEN 0 WHEN resolved.legacy_match IS NOT NULL THEN resolved.legacy_match % 2 ELSE NULL END FROM (SELECT MAX(CASE WHEN {scoped_marker_condition} AND json_extract(marker.value, '$.bridge_provider_id') = {final_provider_id} THEN {packed_marker_semantics} END) AS exact_match, MAX(CASE WHEN {scoped_marker_condition} THEN 1 ELSE 0 END) AS has_scoped, MAX(CASE WHEN {legacy_marker_condition} THEN {packed_marker_semantics} END) AS legacy_match FROM json_each({safe_settings_json}) marker) resolved)"
    );
    let legacy_provider_semantics = format!(
        "CASE WHEN EXISTS (SELECT 1 FROM providers p WHERE p.id = {final_provider_id} AND (p.source_provider_id IS NOT NULL OR p.bridge_type = 'cx2cc')) THEN 1 ELSE 0 END"
    );
    let openai_semantics = format!(
        "({cli_key} IN ('codex', 'grok') OR COALESCE({persisted_marker_semantics}, {legacy_provider_semantics}) = 1)"
    );

    format!(
        "CASE WHEN {openai_semantics} THEN MAX(COALESCE({input_tokens}, 0) - COALESCE({cache_read_input_tokens}, 0) - COALESCE({cache_creation_input_tokens}, 0), 0) WHEN {cli_key} = 'gemini' THEN MAX(COALESCE({input_tokens}, 0) - COALESCE({cache_read_input_tokens}, 0), 0) ELSE COALESCE({input_tokens}, 0) END"
    )
}

pub(super) fn sql_effective_input_tokens_expr() -> String {
    build_sql_effective_input_tokens_expr(None)
}

pub(super) fn sql_effective_input_tokens_expr_with_alias(alias: &str) -> String {
    build_sql_effective_input_tokens_expr(Some(alias))
}

pub(super) fn sql_effective_total_tokens_expr() -> String {
    let effective_input_expr = sql_effective_input_tokens_expr();
    format!(
        "({effective_input_expr}) + COALESCE(output_tokens, 0) + COALESCE(cache_creation_input_tokens, 0) + COALESCE(cache_read_input_tokens, 0)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_input_tokens_uses_protocol_specific_buckets_and_preserves_unknown() {
        assert_eq!(
            effective_input_tokens_display("claude", None, false, None, None, None),
            None
        );
        assert_eq!(
            effective_input_tokens_display("codex", None, false, None, Some(100), Some(200)),
            None
        );
        assert_eq!(
            effective_input_tokens_display("codex", None, false, Some(1000), Some(100), Some(200)),
            Some(700)
        );
        assert_eq!(
            effective_input_tokens_display("grok", None, false, Some(1000), Some(100), Some(200)),
            Some(700)
        );
        assert_eq!(
            effective_input_tokens_display(
                "claude",
                Some(true),
                false,
                Some(1000),
                Some(100),
                Some(200)
            ),
            Some(700)
        );
        assert_eq!(
            effective_input_tokens_display("gemini", None, false, Some(1000), Some(100), Some(200)),
            Some(900)
        );
        assert_eq!(
            effective_input_tokens_display("claude", None, false, Some(1000), Some(100), Some(200)),
            Some(1000)
        );
        assert_eq!(
            effective_input_tokens_display(
                "claude",
                Some(false),
                true,
                Some(1000),
                Some(100),
                Some(200)
            ),
            Some(1000),
            "a persisted non-Codex marker must block provider fallback"
        );
        assert_eq!(
            effective_input_tokens_display("codex", None, false, Some(100), Some(80), Some(50)),
            Some(0)
        );
    }

    #[test]
    fn effective_input_tokens_matches_plain_and_aliased_sql_expressions() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (id INTEGER PRIMARY KEY, source_provider_id INTEGER, bridge_type TEXT);
INSERT INTO providers VALUES (1, NULL, NULL);
INSERT INTO providers VALUES (2, 99, NULL);
INSERT INTO providers VALUES (3, NULL, 'cx2cc');
INSERT INTO providers VALUES (4, 99, 'cx2cc');
CREATE TABLE r (
  cli_key TEXT,
  final_provider_id INTEGER,
  special_settings_json TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cache_read_input_tokens INTEGER,
  cache_creation_input_tokens INTEGER
);
"#,
        )
        .expect("create schema");

        let provider_cases: [(i64, Option<i64>, Option<&str>); 5] = [
            (1, None, None),
            (2, Some(99), None),
            (3, None, Some("cx2cc")),
            (4, Some(99), Some("cx2cc")),
            // provider id 5 does not exist in the providers table at all
            (5, None, None),
        ];
        let marker_cases: [Option<&str>; 14] = [
            None,
            Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex"}]"#),
            Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"claude"}]"#),
            Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"\tcodex\t"}]"#),
            Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"\r\ncodex\n"}]"#),
            Some(
                r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":2,"source_cli_key":"codex"}]"#,
            ),
            Some(
                r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":99,"source_cli_key":"codex"}]"#,
            ),
            Some("not-json"),
            Some(r#"[1,"text",false,null]"#),
            Some(r#"[{"type":"cx2cc_cost_basis"}]"#),
            Some(r#"[1,"text",{"type":"cx2cc_cost_basis","source_cli_key":"codex"},false]"#),
            Some(
                r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex"},{"type":"cx2cc_cost_basis","source_cli_key":""}]"#,
            ),
            Some(
                r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":null,"source_cli_key":"codex"}]"#,
            ),
            Some(
                r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":9223372036854775808,"source_cli_key":"codex"}]"#,
            ),
        ];
        let token_cases: [(Option<i64>, Option<i64>, Option<i64>); 5] = [
            (None, None, None),
            (Some(1200), None, None),
            (Some(1000), Some(100), Some(200)),
            (Some(100), Some(80), Some(50)),
            (Some(0), Some(0), Some(0)),
        ];

        let plain_sql = format!("SELECT {} FROM r", sql_effective_input_tokens_expr());
        let aliased_sql = format!(
            "SELECT {} FROM r",
            sql_effective_input_tokens_expr_with_alias("r")
        );
        for cli_key in ["claude", "codex", "gemini", "grok"] {
            for (provider_id, source_provider_id, bridge_type) in provider_cases {
                for special_settings_json in marker_cases {
                    for (input, cache_read, cache_creation) in token_cases {
                        conn.execute("DELETE FROM r", []).expect("clear r");
                        conn.execute(
                            "INSERT INTO r VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
                            rusqlite::params![
                                cli_key,
                                provider_id,
                                special_settings_json,
                                input,
                                cache_read,
                                cache_creation
                            ],
                        )
                        .expect("insert row");

                        let plain_value: i64 = conn
                            .query_row(&plain_sql, [], |row| row.get(0))
                            .expect("evaluate plain SQL expression");
                        let aliased_value: i64 = conn
                            .query_row(&aliased_sql, [], |row| row.get(0))
                            .expect("evaluate aliased SQL expression");
                        let bridged = is_bridged_input_semantics(source_provider_id, bridge_type);
                        let persisted_openai_semantics =
                            crate::request_logs::cx2cc_openai_input_semantics_override(
                                special_settings_json,
                                Some(provider_id),
                            );
                        let rust_value = effective_input_tokens(
                            cli_key,
                            persisted_openai_semantics,
                            bridged,
                            input,
                            cache_read,
                            cache_creation,
                        );

                        assert_eq!(plain_value, rust_value, "plain SQL: cli_key={cli_key} provider={provider_id} marker={special_settings_json:?} input={input:?} read={cache_read:?} creation={cache_creation:?}");
                        assert_eq!(aliased_value, rust_value, "aliased SQL: cli_key={cli_key} provider={provider_id} marker={special_settings_json:?} input={input:?} read={cache_read:?} creation={cache_creation:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn effective_total_tokens_preserves_openai_token_conservation() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (id INTEGER PRIMARY KEY, source_provider_id INTEGER, bridge_type TEXT);
CREATE TABLE r (
  cli_key TEXT,
  final_provider_id INTEGER,
  special_settings_json TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cache_read_input_tokens INTEGER,
  cache_creation_input_tokens INTEGER
);
INSERT INTO r VALUES ('codex', 1, NULL, 1000, 50, 100, 200);
"#,
        )
        .expect("create token conservation fixture");

        let sql = format!("SELECT {} FROM r", sql_effective_total_tokens_expr());
        let total: i64 = conn
            .query_row(&sql, [], |row| row.get(0))
            .expect("evaluate effective total");
        assert_eq!(total, 1050);
    }

    #[test]
    fn effective_input_sql_uses_one_unsorted_marker_scan() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (id INTEGER PRIMARY KEY, source_provider_id INTEGER, bridge_type TEXT);
CREATE TABLE r (
  cli_key TEXT,
  final_provider_id INTEGER,
  special_settings_json TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cache_read_input_tokens INTEGER,
  cache_creation_input_tokens INTEGER
);
"#,
        )
        .expect("create explain schema");

        for expression in [
            sql_effective_input_tokens_expr(),
            sql_effective_input_tokens_expr_with_alias("r"),
        ] {
            let sql = format!("EXPLAIN QUERY PLAN SELECT {expression} FROM r");
            let mut stmt = conn.prepare(&sql).expect("prepare query plan");
            let details = stmt
                .query_map([], |row| row.get::<_, String>(3))
                .expect("query plan")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("read query plan");
            let marker_scans = details
                .iter()
                .filter(|detail| detail.contains("SCAN marker VIRTUAL TABLE"))
                .count();

            assert_eq!(marker_scans, 1, "query plan: {details:?}");
            assert!(
                details
                    .iter()
                    .all(|detail| !detail.contains("USE TEMP B-TREE FOR ORDER BY")),
                "query plan: {details:?}"
            );
        }
    }
}
