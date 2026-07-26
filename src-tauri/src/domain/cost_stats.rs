//! Usage: Cost backfill jobs backed by sqlite.

use crate::cost;
use crate::db;
use crate::model_price_aliases;
use crate::request_logs;
use crate::shared::error::db_err;
use rusqlite::{params, OptionalExtension};

fn has_any_cost_usage(usage: &cost::CostUsage) -> bool {
    usage.input_tokens > 0
        || usage.output_tokens > 0
        || usage.cache_read_input_tokens > 0
        || usage.cache_creation_input_tokens > 0
        || usage.cache_creation_5m_input_tokens > 0
        || usage.cache_creation_1h_input_tokens > 0
}

pub fn backfill_missing_for_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    cli_key: &str,
    max_rows: usize,
) -> crate::shared::error::AppResult<()> {
    let price_aliases = model_price_aliases::read_fail_open(app);
    backfill_missing_for_cli_with_aliases(db, cli_key, max_rows, &price_aliases)
}

fn backfill_missing_for_cli_with_aliases(
    db: &db::Db,
    cli_key: &str,
    max_rows: usize,
    price_aliases: &model_price_aliases::ModelPriceAliasesV1,
) -> crate::shared::error::AppResult<()> {
    let mut conn = db.open_connection()?;
    crate::shared::cli_key::validate_cli_key(cli_key)?;
    let max_rows = max_rows.clamp(1, 10_000) as i64;

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start sqlite transaction: {e}"))?;

    {
        let mut stmt_candidates = tx
            .prepare(
                r#"
SELECT
  id,
  cli_key,
  requested_model,
  special_settings_json,
  final_provider_id,
  cost_multiplier,
  input_tokens,
  output_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens
FROM request_logs
WHERE excluded_from_stats = 0
AND status >= 200 AND status < 300 AND error_code IS NULL
AND cost_usd_femto IS NULL
AND cli_key = ?1
ORDER BY created_at_ms DESC, id DESC
LIMIT ?2
"#,
            )
            .map_err(|e| db_err!("failed to prepare backfill candidates query: {e}"))?;

        let mut stmt_price = tx
            .prepare_cached("SELECT price_json FROM model_prices WHERE cli_key = ?1 AND model = ?2")
            .map_err(|e| db_err!("failed to prepare model_prices query: {e}"))?;

        let mut stmt_update = tx
            .prepare_cached(
                "UPDATE request_logs SET cost_usd_femto = ?1 WHERE id = ?2 AND cost_usd_femto IS NULL",
            )
            .map_err(|e| db_err!("failed to prepare backfill update: {e}"))?;

        let rows = stmt_candidates
            .query_map(params![cli_key, max_rows], |row| {
                Ok((
                    row.get::<_, i64>("id")?,
                    row.get::<_, String>("cli_key")?,
                    row.get::<_, Option<String>>("requested_model")?,
                    row.get::<_, f64>("cost_multiplier")?,
                    row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                    row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                    row.get::<_, Option<i64>>("cache_read_input_tokens")?
                        .unwrap_or(0),
                    row.get::<_, Option<i64>>("cache_creation_input_tokens")?
                        .unwrap_or(0),
                    row.get::<_, Option<i64>>("cache_creation_5m_input_tokens")?
                        .unwrap_or(0),
                    row.get::<_, Option<i64>>("cache_creation_1h_input_tokens")?
                        .unwrap_or(0),
                    row.get::<_, Option<String>>("special_settings_json")?,
                    row.get::<_, Option<i64>>("final_provider_id")?,
                ))
            })
            .map_err(|e| db_err!("failed to run backfill candidates query: {e}"))?;

        for row in rows {
            let (
                id,
                cli_key,
                requested_model,
                cost_multiplier,
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                cache_creation_5m_input_tokens,
                cache_creation_1h_input_tokens,
                special_settings_json,
                final_provider_id,
            ) = row.map_err(|e| db_err!("failed to read backfill candidate row: {e}"))?;

            let Some(cost_basis) = request_logs::effective_cost_basis(
                &cli_key,
                requested_model.as_deref(),
                special_settings_json.as_deref(),
                final_provider_id,
            ) else {
                continue;
            };
            let effective_cli_key = cost_basis.cli_key;
            let mut model = cost_basis.model;

            let usage = cost::CostUsage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                cache_creation_5m_input_tokens,
                cache_creation_1h_input_tokens,
            };

            if !has_any_cost_usage(&usage) {
                continue;
            }

            let multiplier = if cost_multiplier.is_finite() && cost_multiplier >= 0.0 {
                cost_multiplier
            } else {
                1.0
            };

            if multiplier == 0.0 {
                stmt_update
                    .execute(params![0_i64, id])
                    .map_err(|e| db_err!("failed to update zero cost_usd_femto: {e}"))?;
                continue;
            }

            let mut price_json: Option<String> = stmt_price
                .query_row(params![&effective_cli_key, &model], |row| {
                    row.get::<_, String>(0)
                })
                .optional()
                .unwrap_or(None);
            if price_json.is_none() {
                if let Some(target_model) =
                    price_aliases.resolve_target_model(&effective_cli_key, &model)
                {
                    if target_model != model {
                        model = target_model.to_string();
                        price_json = stmt_price
                            .query_row(params![&effective_cli_key, &model], |row| {
                                row.get::<_, String>(0)
                            })
                            .optional()
                            .unwrap_or(None);
                    }
                }
            }

            let Some(price_json) = price_json else {
                continue;
            };

            let options = cost::CostCalculationOptions {
                priority_service_tier_applied: request_logs::parse_effective_priority(
                    special_settings_json.as_deref(),
                ),
            };
            let cost_usd_femto = cost::calculate_cost_usd_femto_with_options(
                &usage,
                &price_json,
                multiplier,
                &effective_cli_key,
                &model,
                &options,
            );
            let Some(cost_usd_femto) = cost_usd_femto else {
                continue;
            };

            stmt_update
                .execute(params![cost_usd_femto, id])
                .map_err(|e| db_err!("failed to update cost_usd_femto: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| db_err!("failed to commit backfill transaction: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_FEMTO_USD: i64 = 1_000_000_000_000_000;

    fn insert_backfill_candidate(conn: &rusqlite::Connection, trace_id: &str, cli_key: &str) {
        conn.execute(
            r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id, requested_model, input_tokens
) VALUES (
  ?1, ?2, 'POST', '/v1/chat/completions', 200, NULL, 10,
  '[]', 1000, 1000000, NULL, 0, 1, 'gpt-test', 100
)
"#,
            params![trace_id, cli_key],
        )
        .expect("insert backfill candidate");
    }

    #[test]
    fn backfill_missing_for_cli_updates_only_matching_cli() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("cost-backfill-test.db")).expect("init db");
        crate::model_prices::upsert(&db, "codex", "gpt-test", r#"{"input_cost_per_token":0.01}"#)
            .expect("insert model price");
        crate::model_prices::upsert(
            &db,
            "claude",
            "gpt-test",
            r#"{"input_cost_per_token":0.01}"#,
        )
        .expect("insert model price");

        let conn = db.open_connection().expect("open db");
        insert_backfill_candidate(&conn, "backfill-codex-cost", "codex");
        insert_backfill_candidate(&conn, "backfill-claude-cost", "claude");
        drop(conn);

        backfill_missing_for_cli_with_aliases(
            &db,
            "codex",
            5000,
            &model_price_aliases::ModelPriceAliasesV1::default(),
        )
        .expect("backfill missing cost");

        let conn = db.open_connection().expect("reopen db");
        let codex_cost = conn
            .query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = 'backfill-codex-cost'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read backfilled cost");
        let claude_cost = conn
            .query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = 'backfill-claude-cost'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read untouched cost");

        assert_eq!(codex_cost, Some(TEST_FEMTO_USD));
        assert_eq!(claude_cost, None);
    }

    #[test]
    fn backfill_uses_default_grok_build_price_alias() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            db::init_for_tests(&dir.path().join("cost-backfill-grok-alias.db")).expect("init db");
        crate::model_prices::upsert(
            &db,
            "grok",
            "grok-build-0.1",
            r#"{"input_cost_per_token":0.01}"#,
        )
        .expect("insert Grok model price");

        let conn = db.open_connection().expect("open db");
        insert_backfill_candidate(&conn, "backfill-grok-build", "grok");
        conn.execute(
            "UPDATE request_logs SET requested_model = 'grok-build' WHERE trace_id = 'backfill-grok-build'",
            [],
        )
        .expect("set Grok request model");
        drop(conn);

        backfill_missing_for_cli_with_aliases(
            &db,
            "grok",
            5000,
            &model_price_aliases::ModelPriceAliasesV1::default(),
        )
        .expect("backfill Grok cost");

        let conn = db.open_connection().expect("reopen db");
        let cost = conn
            .query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = 'backfill-grok-build'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read Grok backfilled cost");
        assert_eq!(cost, Some(TEST_FEMTO_USD));
    }

    #[test]
    fn backfill_zero_multiplier_does_not_require_a_model_price() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("cost-backfill-free.db")).expect("init db");
        let conn = db.open_connection().expect("open db");
        insert_backfill_candidate(&conn, "backfill-free-unpriced", "codex");
        conn.execute(
            "UPDATE request_logs SET requested_model = 'unpriced-model', cost_multiplier = 0 WHERE trace_id = 'backfill-free-unpriced'",
            [],
        )
        .expect("mark backfill candidate as free");
        drop(conn);

        backfill_missing_for_cli_with_aliases(
            &db,
            "codex",
            5000,
            &model_price_aliases::ModelPriceAliasesV1::default(),
        )
        .expect("backfill free request cost");

        let conn = db.open_connection().expect("reopen db");
        let cost = conn
            .query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = 'backfill-free-unpriced'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read free request cost");
        assert_eq!(cost, Some(0));
    }

    #[test]
    fn backfill_uses_the_same_complete_cost_basis_model_as_online_costing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("cost-backfill-model-identity.db"))
            .expect("init db");
        let prefix_model = "m".repeat(200);
        let exact_long_model = format!("{prefix_model}x");
        let missing_long_model = format!("{prefix_model}y");

        crate::model_prices::upsert(
            &db,
            "codex",
            &prefix_model,
            r#"{"input_cost_per_token":0.01}"#,
        )
        .expect("insert prefix model price");
        crate::model_prices::upsert(
            &db,
            "codex",
            &exact_long_model,
            r#"{"input_cost_per_token":0.002}"#,
        )
        .expect("insert exact long model price");
        crate::model_prices::upsert(
            &db,
            "codex",
            "gpt-priced",
            r#"{"input_cost_per_token":0.003}"#,
        )
        .expect("insert ASCII model price");

        let conn = db.open_connection().expect("open db");
        for (trace_id, model, special_settings_json) in [
            ("backfill-long-exact", exact_long_model.as_str(), None),
            ("backfill-long-missing", missing_long_model.as_str(), None),
            (
                "backfill-non-json-whitespace",
                "client-model",
                Some(
                    serde_json::json!([{
                        "type": "cx2cc_cost_basis",
                        "source_cli_key": "codex",
                        "priced_model": "\u{00a0}gpt-priced\u{00a0}",
                    }])
                    .to_string(),
                ),
            ),
        ] {
            conn.execute(
                r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id, requested_model, input_tokens,
  special_settings_json
) VALUES (?1, 'codex', 'POST', '/v1/responses', 200, NULL, 10, '[]', 1000,
  1000000, NULL, 0, 1, ?2, 100, ?3)
"#,
                params![trace_id, model, special_settings_json],
            )
            .expect("insert model identity backfill candidate");
        }
        drop(conn);

        backfill_missing_for_cli_with_aliases(
            &db,
            "codex",
            5000,
            &model_price_aliases::ModelPriceAliasesV1::default(),
        )
        .expect("backfill missing cost");

        let conn = db.open_connection().expect("reopen db");
        let read_cost = |trace_id: &str| {
            conn.query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = ?1",
                [trace_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read backfilled cost")
        };
        assert_eq!(
            read_cost("backfill-long-exact"),
            Some(200_000_000_000_000),
            "the complete long model must win over its priced prefix"
        );
        assert_eq!(read_cost("backfill-long-missing"), None);
        assert_eq!(read_cost("backfill-non-json-whitespace"), None);
    }

    #[test]
    fn backfill_scopes_cx2cc_cost_basis_to_final_provider() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("cost-backfill-cx2cc.db")).expect("init db");
        crate::model_prices::upsert(
            &db,
            "codex",
            "gpt-failed",
            r#"{"input_cost_per_token":0.01}"#,
        )
        .expect("insert Codex model price");
        crate::model_prices::upsert(
            &db,
            "claude",
            "claude-final",
            r#"{"input_cost_per_token":0.001}"#,
        )
        .expect("insert Claude model price");

        let marker = r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":12,"source_cli_key":"codex","priced_model":"gpt-failed"}]"#;
        let conn = db.open_connection().expect("open db");
        for (trace_id, final_provider_id) in [
            ("backfill-cx2cc-match", 12_i64),
            ("backfill-cx2cc-mismatch", 13_i64),
        ] {
            conn.execute(
                r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id, requested_model, input_tokens,
  special_settings_json
) VALUES (?1, 'claude', 'POST', '/v1/messages', 200, NULL, 10, '[]', 1000,
  1000000, NULL, 0, ?2, 'claude-final', 100, ?3)
"#,
                params![trace_id, final_provider_id, marker],
            )
            .expect("insert CX2CC backfill candidate");
        }
        drop(conn);

        backfill_missing_for_cli_with_aliases(
            &db,
            "claude",
            5000,
            &model_price_aliases::ModelPriceAliasesV1::default(),
        )
        .expect("backfill missing cost");

        let conn = db.open_connection().expect("reopen db");
        let read_cost = |trace_id: &str| {
            conn.query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = ?1",
                [trace_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read backfilled cost")
        };
        assert_eq!(read_cost("backfill-cx2cc-match"), Some(TEST_FEMTO_USD));
        assert_eq!(
            read_cost("backfill-cx2cc-mismatch"),
            Some(100_000_000_000_000),
            "a failed bridge marker must not override the final Claude provider"
        );
    }

    #[test]
    fn backfill_applies_model_alias_and_priority_options_like_online_costing() {
        let aliases = model_price_aliases::ModelPriceAliasesV1 {
            version: 1,
            rules: vec![model_price_aliases::ModelPriceAliasRuleV1 {
                cli_key: "codex".to_string(),
                match_type: model_price_aliases::ModelPriceAliasMatchTypeV1::Exact,
                pattern: "gpt-alias".to_string(),
                target_model: "gpt-priced".to_string(),
                enabled: true,
            }],
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("cost-backfill-alias-priority.db"))
            .expect("init db");
        crate::model_prices::upsert(
            &db,
            "codex",
            "gpt-priced",
            r#"{"input_cost_per_token":0.001,"input_cost_per_token_priority":0.003}"#,
        )
        .expect("insert aliased model price");

        let special_settings_json = serde_json::json!([{
            "type": "codex_service_tier_result",
            "effectivePriority": true,
        }])
        .to_string();
        let conn = db.open_connection().expect("open db");
        conn.execute(
            r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id, requested_model, input_tokens,
  special_settings_json
) VALUES ('backfill-alias-priority', 'codex', 'POST', '/v1/responses', 200, NULL,
  10, '[]', 1000, 1000000, NULL, 0, 1, 'gpt-alias', 100, ?1)
"#,
            [special_settings_json],
        )
        .expect("insert alias/priority backfill candidate");
        drop(conn);

        backfill_missing_for_cli_with_aliases(&db, "codex", 5000, &aliases)
            .expect("backfill missing cost");

        let conn = db.open_connection().expect("reopen db");
        let cost = conn
            .query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = 'backfill-alias-priority'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read backfilled cost");
        assert_eq!(cost, Some(300_000_000_000_000));
    }
}
