//! Usage: SQLite schema migrations (user_version + incremental upgrades).

mod v0_to_v1;
mod v10_to_v11;
mod v11_to_v12;
mod v12_to_v13;
mod v13_to_v14;
mod v14_to_v15;
mod v15_to_v16;
mod v16_to_v17;
mod v17_to_v18;
mod v18_to_v19;
mod v19_to_v20;
mod v1_to_v2;
mod v20_to_v21;
mod v21_to_v22;
mod v22_to_v23;
mod v23_to_v24;
mod v24_to_v25;
mod v25_to_v26;
mod v26_to_v27;
mod v27_to_v28;
mod v2_to_v3;
mod v3_to_v4;
mod v4_to_v5;
mod v5_to_v6;
mod v6_to_v7;
mod v7_to_v8;
mod v8_to_v9;
mod v9_to_v10;

use rusqlite::Connection;

const LATEST_SCHEMA_VERSION: i64 = 28;

pub(super) fn apply_migrations(conn: &mut Connection) -> Result<(), String> {
    let mut user_version = read_user_version(conn)?;

    if user_version < 0 {
        return Err(format!(
            "unsupported sqlite schema version: user_version={user_version} (expected 0..={LATEST_SCHEMA_VERSION})"
        ));
    }

    if user_version > LATEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported sqlite schema version: user_version={user_version} (expected 0..={LATEST_SCHEMA_VERSION})"
        ));
    }

    while user_version < LATEST_SCHEMA_VERSION {
        match user_version {
            0 => v0_to_v1::migrate_v0_to_v1(conn)?,
            1 => v1_to_v2::migrate_v1_to_v2(conn)?,
            2 => v2_to_v3::migrate_v2_to_v3(conn)?,
            3 => v3_to_v4::migrate_v3_to_v4(conn)?,
            4 => v4_to_v5::migrate_v4_to_v5(conn)?,
            5 => v5_to_v6::migrate_v5_to_v6(conn)?,
            6 => v6_to_v7::migrate_v6_to_v7(conn)?,
            7 => v7_to_v8::migrate_v7_to_v8(conn)?,
            8 => v8_to_v9::migrate_v8_to_v9(conn)?,
            9 => v9_to_v10::migrate_v9_to_v10(conn)?,
            10 => v10_to_v11::migrate_v10_to_v11(conn)?,
            11 => v11_to_v12::migrate_v11_to_v12(conn)?,
            12 => v12_to_v13::migrate_v12_to_v13(conn)?,
            13 => v13_to_v14::migrate_v13_to_v14(conn)?,
            14 => v14_to_v15::migrate_v14_to_v15(conn)?,
            15 => v15_to_v16::migrate_v15_to_v16(conn)?,
            16 => v16_to_v17::migrate_v16_to_v17(conn)?,
            17 => v17_to_v18::migrate_v17_to_v18(conn)?,
            18 => v18_to_v19::migrate_v18_to_v19(conn)?,
            19 => v19_to_v20::migrate_v19_to_v20(conn)?,
            20 => v20_to_v21::migrate_v20_to_v21(conn)?,
            21 => v21_to_v22::migrate_v21_to_v22(conn)?,
            22 => v22_to_v23::migrate_v22_to_v23(conn)?,
            23 => v23_to_v24::migrate_v23_to_v24(conn)?,
            24 => v24_to_v25::migrate_v24_to_v25(conn)?,
            25 => v25_to_v26::migrate_v25_to_v26(conn)?,
            26 => v26_to_v27::migrate_v26_to_v27(conn)?,
            27 => v27_to_v28::migrate_v27_to_v28(conn)?,
            v => {
                return Err(format!(
                    "unsupported sqlite schema version: user_version={v} (expected 0..={LATEST_SCHEMA_VERSION})"
                ))
            }
        }
        user_version = read_user_version(conn)?;
    }

    Ok(())
}

fn read_user_version(conn: &Connection) -> Result<i64, String> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| format!("failed to read sqlite user_version: {e}"))
}

fn set_user_version(tx: &rusqlite::Transaction<'_>, version: i64) -> Result<(), String> {
    tx.pragma_update(None, "user_version", version)
        .map_err(|e| format!("failed to update sqlite user_version: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_v25_to_v26_backfills_claude_models_json_from_legacy_mapping() {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");

        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);
"#,
        )
        .expect("create providers table");

        let legacy_mapping = serde_json::json!({
            "*": "glm-4-plus",
            "claude-*sonnet*": "glm-4-plus-sonnet",
            "claude-*haiku*": "glm-4-plus-haiku",
            "claude-*thinking*": "glm-4-plus-thinking"
        })
        .to_string();

        conn.execute(
            r#"
INSERT INTO providers(
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 100, 1, 1, 0, 1.0, '{}', ?7)
"#,
            rusqlite::params![
                "claude",
                "legacy",
                "https://example.com",
                "[]",
                "order",
                "sk-test",
                legacy_mapping
            ],
        )
        .expect("insert legacy provider");

        v25_to_v26::migrate_v25_to_v26(&mut conn).expect("migrate v25->v26");

        let claude_models_json: String = conn
            .query_row(
                "SELECT claude_models_json FROM providers WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .expect("read claude_models_json");

        let value: serde_json::Value =
            serde_json::from_str(&claude_models_json).expect("claude_models_json valid json");

        assert_eq!(value["main_model"], "glm-4-plus");
        assert_eq!(value["sonnet_model"], "glm-4-plus-sonnet");
        assert_eq!(value["haiku_model"], "glm-4-plus-haiku");
        assert_eq!(value["reasoning_model"], "glm-4-plus-thinking");

        let supported_models_json: String = conn
            .query_row(
                "SELECT supported_models_json FROM providers WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .expect("read supported_models_json");
        assert_eq!(supported_models_json.trim(), "{}");

        let model_mapping_json: String = conn
            .query_row(
                "SELECT model_mapping_json FROM providers WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .expect("read model_mapping_json");
        assert_eq!(model_mapping_json.trim(), "{}");
    }

    #[test]
    fn migrate_v27_to_v28_drops_provider_mode_and_deletes_official_providers() {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign_keys");

        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  provider_mode TEXT NOT NULL DEFAULT 'relay',
  UNIQUE(cli_key, name)
);

CREATE TABLE provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE sort_modes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(name)
);

CREATE TABLE sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id),
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE CASCADE,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE claude_model_validation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  request_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
"#,
        )
        .expect("create v27 schema");

        conn.execute(
            r#"
INSERT INTO providers(
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
  provider_mode
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
"#,
            rusqlite::params![
                1i64,
                "codex",
                "relay",
                "https://relay.example.com/v1",
                "[\"https://relay.example.com/v1\"]",
                "order",
                "{}",
                "sk-relay",
                1i64,
                100i64,
                1i64,
                1i64,
                0i64,
                1.0f64,
                "{}",
                "{}",
                "relay",
            ],
        )
        .expect("insert relay provider");

        conn.execute(
            r#"
INSERT INTO providers(
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
  provider_mode
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
"#,
            rusqlite::params![
                2i64,
                "codex",
                "official",
                "https://api.openai.com/v1",
                "[\"https://api.openai.com/v1\"]",
                "order",
                "{}",
                "",
                1i64,
                100i64,
                1i64,
                1i64,
                1i64,
                1.0f64,
                "{}",
                "{}",
                "official",
            ],
        )
        .expect("insert official provider");

        conn.execute(
            "INSERT INTO provider_circuit_breakers(provider_id, state, failure_count, open_until, updated_at) VALUES (?1, 'CLOSED', 0, NULL, 1)",
            rusqlite::params![1i64],
        )
        .expect("insert relay breaker");
        conn.execute(
            "INSERT INTO provider_circuit_breakers(provider_id, state, failure_count, open_until, updated_at) VALUES (?1, 'CLOSED', 0, NULL, 1)",
            rusqlite::params![2i64],
        )
        .expect("insert official breaker");

        conn.execute(
            "INSERT INTO sort_modes(id, name, created_at, updated_at) VALUES (1, 'mode', 1, 1)",
            [],
        )
        .expect("insert sort mode");
        conn.execute(
            "INSERT INTO sort_mode_providers(mode_id, cli_key, provider_id, sort_order, created_at, updated_at) VALUES (1, 'codex', 1, 0, 1, 1)",
            [],
        )
        .expect("insert relay sort_mode_provider");
        conn.execute(
            "INSERT INTO sort_mode_providers(mode_id, cli_key, provider_id, sort_order, created_at, updated_at) VALUES (1, 'codex', 2, 1, 1, 1)",
            [],
        )
        .expect("insert official sort_mode_provider");

        conn.execute(
            "INSERT INTO claude_model_validation_runs(id, provider_id, created_at, request_json, result_json) VALUES (1, 1, 1, '{}', '{}')",
            [],
        )
        .expect("insert relay validation run");
        conn.execute(
            "INSERT INTO claude_model_validation_runs(id, provider_id, created_at, request_json, result_json) VALUES (2, 2, 1, '{}', '{}')",
            [],
        )
        .expect("insert official validation run");

        v27_to_v28::migrate_v27_to_v28(&mut conn).expect("migrate v27->v28");

        let user_version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read user_version");
        assert_eq!(user_version, 28);

        let provider_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
            .expect("count providers");
        assert_eq!(provider_count, 1);

        let breaker_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_circuit_breakers",
                [],
                |row| row.get(0),
            )
            .expect("count breakers");
        assert_eq!(breaker_count, 1);

        let sort_mode_provider_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sort_mode_providers", [], |row| {
                row.get(0)
            })
            .expect("count sort_mode_providers");
        assert_eq!(sort_mode_provider_count, 1);

        let validation_run_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM claude_model_validation_runs",
                [],
                |row| row.get(0),
            )
            .expect("count validation runs");
        assert_eq!(validation_run_count, 1);

        let remaining_name: String = conn
            .query_row("SELECT name FROM providers WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("read remaining provider name");
        assert_eq!(remaining_name, "relay");

        let mut has_provider_mode = false;
        {
            let mut stmt = conn
                .prepare("PRAGMA table_info(providers)")
                .expect("prepare providers table_info query");
            let mut rows = stmt.query([]).expect("query providers table_info");
            while let Some(row) = rows.next().expect("read table_info row") {
                let name: String = row.get(1).expect("read column name");
                if name == "provider_mode" {
                    has_provider_mode = true;
                    break;
                }
            }
        }
        assert!(!has_provider_mode);
    }
}
