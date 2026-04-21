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

fn test_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).expect("prepare table_info");
    let mut rows = stmt.query([]).expect("query table_info");
    while let Some(row) = rows.next().expect("read table_info row") {
        let name: String = row.get(1).expect("read column name");
        if name == column {
            return true;
        }
    }
    false
}

#[test]
fn strict_v29_patch_adds_sort_mode_provider_enabled_column() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE TABLE sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id)
);

PRAGMA user_version = 29;
"#,
    )
    .expect("create legacy sort_mode_providers schema");

    assert!(!test_has_column(&conn, "sort_mode_providers", "enabled"));

    apply_migrations(&mut conn).expect("apply migrations");
    assert!(test_has_column(&conn, "sort_mode_providers", "enabled"));

    // Idempotent: second run should succeed.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn ensure_patch_backfills_oauth_columns_for_legacy_v30_schema() {
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
  cost_multiplier REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

PRAGMA user_version = 30;
"#,
    )
    .expect("create legacy v30 schema without oauth columns");

    conn.execute(
        r#"
INSERT INTO providers(
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
  cost_multiplier
) VALUES ('claude', 'legacy', 'https://example.com', '[]', 'order', '{}', 'sk-test', 1, 100, 1, 1, 0, 1.0)
"#,
        [],
    )
    .expect("insert legacy provider");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 32);

    for column in [
        "auth_mode",
        "oauth_provider_type",
        "oauth_access_token",
        "oauth_refresh_token",
        "oauth_id_token",
        "oauth_token_uri",
        "oauth_client_id",
        "oauth_client_secret",
        "oauth_expires_at",
        "oauth_email",
        "oauth_last_refreshed_at",
        "oauth_last_error",
        "oauth_refresh_lead_s",
    ] {
        assert!(test_has_column(&conn, "providers", column));
    }

    let (auth_mode, oauth_refresh_lead_s): (String, i64) = conn
        .query_row(
            "SELECT auth_mode, oauth_refresh_lead_s FROM providers WHERE name = 'legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read oauth defaults");
    assert_eq!(auth_mode, "api_key");
    assert_eq!(oauth_refresh_lead_s, 3600);

    // Idempotent: second run should succeed.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn strict_v29_patch_migrates_legacy_workspace_cluster_tables() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE TABLE mcp_servers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  server_key TEXT NOT NULL,
  name TEXT NOT NULL,
  transport TEXT NOT NULL,
  command TEXT,
  args_json TEXT NOT NULL DEFAULT '[]',
  env_json TEXT NOT NULL DEFAULT '{}',
  cwd TEXT,
  url TEXT,
  headers_json TEXT NOT NULL DEFAULT '{}',
  enabled_claude INTEGER NOT NULL DEFAULT 0,
  enabled_codex INTEGER NOT NULL DEFAULT 0,
  enabled_gemini INTEGER NOT NULL DEFAULT 0,
  normalized_name TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE TABLE skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  skill_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  source_git_url TEXT NOT NULL,
  source_branch TEXT NOT NULL,
  source_subdir TEXT NOT NULL,
  enabled_claude INTEGER NOT NULL DEFAULT 0,
  enabled_codex INTEGER NOT NULL DEFAULT 0,
  enabled_gemini INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(skill_key)
);

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
  UNIQUE(cli_key, name)
);

PRAGMA user_version = 29;
"#,
    )
    .expect("create legacy v29 tables");

    conn.execute(
        r#"
INSERT INTO prompts(id, cli_key, name, content, enabled, created_at, updated_at)
VALUES (1, 'claude', 'default', 'hello', 1, 1, 1)
"#,
        [],
    )
    .expect("insert prompt");
    conn.execute(
        r#"
INSERT INTO prompts(id, cli_key, name, content, enabled, created_at, updated_at)
VALUES (2, 'codex', 'p2', 'world', 0, 1, 1)
"#,
        [],
    )
    .expect("insert prompt");

    conn.execute(
        r#"
INSERT INTO mcp_servers(
  id,
  server_key,
  name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  normalized_name,
  created_at,
  updated_at
) VALUES (
  1,
  'srv1',
  'S1',
  'stdio',
  'echo',
  '[]',
  '{}',
  NULL,
  NULL,
  '{}',
  1,
  0,
  0,
  's1',
  1,
  1
)
"#,
        [],
    )
    .expect("insert mcp server");
    conn.execute(
        r#"
INSERT INTO mcp_servers(
  id,
  server_key,
  name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  normalized_name,
  created_at,
  updated_at
) VALUES (
  2,
  'srv2',
  'S2',
  'stdio',
  'echo',
  '[]',
  '{}',
  NULL,
  NULL,
  '{}',
  0,
  1,
  0,
  's2',
  1,
  1
)
"#,
        [],
    )
    .expect("insert mcp server");

    conn.execute(
        r#"
INSERT INTO skills(
  id,
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
) VALUES (
  1,
  'sk1',
  'Skill 1',
  'skill-1',
  '',
  'https://example.com',
  'main',
  'skills/skill1',
  0,
  1,
  0,
  1,
  1
)
"#,
        [],
    )
    .expect("insert skill");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 32);

    assert!(test_has_column(&conn, "workspaces", "cli_key"));
    assert!(test_has_column(&conn, "workspace_active", "workspace_id"));

    assert!(test_has_column(&conn, "prompts", "workspace_id"));
    assert!(!test_has_column(&conn, "prompts", "cli_key"));

    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "daily_reset_mode"));
    assert!(test_has_column(&conn, "providers", "daily_reset_time"));
    assert!(test_has_column(&conn, "providers", "limit_weekly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_monthly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_total_usd"));

    let claude_default_ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE cli_key = 'claude' AND name = '默认' ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read default claude workspace id");
    let codex_default_ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE cli_key = 'codex' AND name = '默认' ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read default codex workspace id");

    let p1_cli: String = conn
        .query_row(
            r#"
SELECT w.cli_key
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
WHERE p.id = 1
"#,
            [],
            |row| row.get(0),
        )
        .expect("read migrated prompt cli_key");
    assert_eq!(p1_cli, "claude");

    let p2_cli: String = conn
        .query_row(
            r#"
SELECT w.cli_key
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
WHERE p.id = 2
"#,
            [],
            |row| row.get(0),
        )
        .expect("read migrated prompt cli_key");
    assert_eq!(p2_cli, "codex");

    let claude_enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = ?1 AND server_id = 1",
            [claude_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled mcp for claude");
    assert_eq!(claude_enabled_mcp, 1);

    let codex_enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = ?1 AND server_id = 2",
            [codex_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled mcp for codex");
    assert_eq!(codex_enabled_mcp, 1);

    let legacy_mcp_flags: (i64, i64, i64) = conn
        .query_row(
            "SELECT enabled_claude, enabled_codex, enabled_gemini FROM mcp_servers WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read legacy mcp flags");
    assert_eq!(legacy_mcp_flags, (0, 0, 0));

    let enabled_skill: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_skill_enabled WHERE workspace_id = ?1 AND skill_id = 1",
            [codex_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled skills");
    assert_eq!(enabled_skill, 1);

    let legacy_skill_flags: (i64, i64, i64) = conn
        .query_row(
            "SELECT enabled_claude, enabled_codex, enabled_gemini FROM skills WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read legacy skill flags");
    assert_eq!(legacy_skill_flags, (0, 0, 0));

    // Idempotent: second run should succeed without changing schema.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn baseline_v25_creates_complete_schema_for_fresh_install() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    // Fresh install: user_version = 0
    apply_migrations(&mut conn).expect("apply migrations on fresh db");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 32);

    // Verify all tables exist
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        let rows = stmt.query_map([], |row| row.get(0)).expect("query");
        rows.filter_map(|r| r.ok()).collect()
    };

    // Core tables from baseline
    assert!(tables.contains(&"providers".to_string()));
    assert!(tables.contains(&"request_logs".to_string()));
    assert!(tables.contains(&"prompts".to_string()));
    assert!(tables.contains(&"mcp_servers".to_string()));
    assert!(tables.contains(&"skills".to_string()));
    assert!(tables.contains(&"skill_repos".to_string()));
    assert!(tables.contains(&"model_prices".to_string()));
    assert!(tables.contains(&"sort_modes".to_string()));
    assert!(tables.contains(&"sort_mode_providers".to_string()));
    assert!(tables.contains(&"sort_mode_active".to_string()));
    assert!(tables.contains(&"claude_model_validation_runs".to_string()));
    assert!(tables.contains(&"schema_migrations".to_string()));

    // Tables from ensure patches
    assert!(tables.contains(&"workspaces".to_string()));
    assert!(tables.contains(&"workspace_active".to_string()));
    assert!(tables.contains(&"workspace_mcp_enabled".to_string()));
    assert!(tables.contains(&"workspace_skill_enabled".to_string()));

    // Verify ensure patches ran (provider limit columns)
    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "tags_json"));

    // Verify v25->v26 migration ran (claude_models_json)
    assert!(test_has_column(&conn, "providers", "claude_models_json"));

    // Verify sort_mode_providers.enabled from ensure patch
    assert!(test_has_column(&conn, "sort_mode_providers", "enabled"));

    // Verify prompts was migrated to workspace_id
    assert!(test_has_column(&conn, "prompts", "workspace_id"));
    assert!(!test_has_column(&conn, "prompts", "cli_key"));

    // Idempotent: second run should succeed
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn rejects_unsupported_old_schema_version() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA user_version = 10;")
        .expect("set old version");

    let result = apply_migrations(&mut conn);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("minimum supported: 25"));
}

#[test]
fn strict_v29_patch_accepts_dev_schema_and_normalizes_user_version_to_29() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE workspaces (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, normalized_name)
);

CREATE TABLE workspace_active (
  cli_key TEXT PRIMARY KEY,
  workspace_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  UNIQUE(workspace_id, name)
);

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
  UNIQUE(cli_key, name)
);

CREATE TABLE mcp_servers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  server_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL DEFAULT '',
  transport TEXT NOT NULL,
  command TEXT,
  args_json TEXT NOT NULL DEFAULT '[]',
  env_json TEXT NOT NULL DEFAULT '{}',
  cwd TEXT,
  url TEXT,
  headers_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE TABLE workspace_mcp_enabled (
  workspace_id INTEGER NOT NULL,
  server_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, server_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

CREATE TABLE skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  skill_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  source_git_url TEXT NOT NULL,
  source_branch TEXT NOT NULL,
  source_subdir TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(skill_key)
);

CREATE TABLE workspace_skill_enabled (
  workspace_id INTEGER NOT NULL,
  skill_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, skill_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

PRAGMA user_version = 33;
"#,
    )
    .expect("create dev schema");

    conn.execute(
        "INSERT INTO workspaces(id, cli_key, name, normalized_name, created_at, updated_at) VALUES (1, 'claude', 'Dev', 'dev', 1, 1)",
        [],
    )
    .expect("insert workspace");
    conn.execute(
        "INSERT INTO workspace_active(cli_key, workspace_id, updated_at) VALUES ('claude', 1, 1)",
        [],
    )
    .expect("insert workspace_active");
    conn.execute(
        "INSERT INTO prompts(id, workspace_id, name, content, enabled, created_at, updated_at) VALUES (1, 1, 'default', 'hello', 1, 1, 1)",
        [],
    )
    .expect("insert prompt");
    conn.execute(
        "INSERT INTO mcp_servers(id, server_key, name, normalized_name, transport, command, args_json, env_json, cwd, url, headers_json, created_at, updated_at) VALUES (1, 'srv1', 'S1', 's1', 'stdio', 'echo', '[]', '{}', NULL, NULL, '{}', 1, 1)",
        [],
    )
    .expect("insert mcp server");
    conn.execute(
        "INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at) VALUES (1, 1, 1, 1)",
        [],
    )
    .expect("insert mcp enabled");
    conn.execute(
        "INSERT INTO skills(id, skill_key, name, normalized_name, description, source_git_url, source_branch, source_subdir, created_at, updated_at) VALUES (1, 'sk1', 'Skill 1', 'skill-1', '', 'https://example.com', 'main', 'skills/skill1', 1, 1)",
        [],
    )
    .expect("insert skill");
    conn.execute(
        "INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at) VALUES (1, 1, 1, 1)",
        [],
    )
    .expect("insert skill enabled");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 32);

    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "daily_reset_mode"));
    assert!(test_has_column(&conn, "providers", "daily_reset_time"));
    assert!(test_has_column(&conn, "providers", "limit_weekly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_monthly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_total_usd"));

    let active_id: i64 = conn
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = 'claude'",
            [],
            |row| row.get(0),
        )
        .expect("read active workspace");
    assert_eq!(active_id, 1);

    let enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = 1 AND server_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("count enabled mcp");
    assert_eq!(enabled_mcp, 1);

    apply_migrations(&mut conn).expect("apply migrations twice");
}
