//! Usage: SQLite connection setup, schema migrations, and common DB helpers.

use crate::app_paths;
use rusqlite::Connection;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DB_FILE_NAME: &str = "aio-coding-hub.db";
const LATEST_SCHEMA_VERSION: i64 = 26;
const BUSY_TIMEOUT: Duration = Duration::from_millis(2000);

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn sql_placeholders(count: usize) -> String {
    if count == 0 {
        return String::new();
    }

    let mut out = String::with_capacity(count.saturating_mul(2).saturating_sub(1));
    for idx in 0..count {
        if idx > 0 {
            out.push(',');
        }
        out.push('?');
    }
    out
}

pub fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join(DB_FILE_NAME))
}

pub fn open_connection(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = db_path(app)?;
    let path_hint = path.to_string_lossy();
    let conn = Connection::open(&path)
        .map_err(|e| format!("failed to open sqlite db at {path_hint}: {e}"))?;

    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|e| format!("failed to set sqlite busy_timeout for {path_hint}: {e}"))?;

    configure_connection(&conn).map_err(|e| format!("sqlite init failed at {path_hint}: {e}"))?;

    Ok(conn)
}

pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let path = db_path(app)?;
    let path_hint = path.to_string_lossy();
    let mut conn = open_connection(app)?;

    apply_migrations(&mut conn)
        .map_err(|e| format!("sqlite migration failed at {path_hint}: {e}"))?;

    Ok(())
}

fn configure_connection(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
"#,
    )
    .map_err(|e| format!("failed to configure sqlite pragmas: {e}"))?;

    Ok(())
}

fn apply_migrations(conn: &mut Connection) -> Result<(), String> {
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
            0 => migrate_v0_to_v1(conn)?,
            1 => migrate_v1_to_v2(conn)?,
            2 => migrate_v2_to_v3(conn)?,
            3 => migrate_v3_to_v4(conn)?,
            4 => migrate_v4_to_v5(conn)?,
            5 => migrate_v5_to_v6(conn)?,
            6 => migrate_v6_to_v7(conn)?,
            7 => migrate_v7_to_v8(conn)?,
            8 => migrate_v8_to_v9(conn)?,
            9 => migrate_v9_to_v10(conn)?,
            10 => migrate_v10_to_v11(conn)?,
            11 => migrate_v11_to_v12(conn)?,
            12 => migrate_v12_to_v13(conn)?,
            13 => migrate_v13_to_v14(conn)?,
            14 => migrate_v14_to_v15(conn)?,
            15 => migrate_v15_to_v16(conn)?,
            16 => migrate_v16_to_v17(conn)?,
            17 => migrate_v17_to_v18(conn)?,
            18 => migrate_v18_to_v19(conn)?,
            19 => migrate_v19_to_v20(conn)?,
            20 => migrate_v20_to_v21(conn)?,
            21 => migrate_v21_to_v22(conn)?,
            22 => migrate_v22_to_v23(conn)?,
            23 => migrate_v23_to_v24(conn)?,
            24 => migrate_v24_to_v25(conn)?,
            25 => migrate_v25_to_v26(conn)?,
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

fn migrate_v0_to_v1(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 1;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to create schema_migrations: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v1_to_v2(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 2;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

 CREATE TABLE IF NOT EXISTS providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE INDEX IF NOT EXISTS idx_providers_cli_key ON providers(cli_key);
"#,
    )
    .map_err(|e| format!("failed to migrate v1->v2: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v2_to_v3(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 3;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE providers ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);
"#,
    )
    .map_err(|e| format!("failed to migrate v2->v3: {e}"))?;

    let mut cli_keys = Vec::new();
    {
        let mut stmt = tx
            .prepare("SELECT DISTINCT cli_key FROM providers")
            .map_err(|e| format!("failed to prepare cli_key query: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("failed to query cli_key list: {e}"))?;
        for row in rows {
            cli_keys.push(row.map_err(|e| format!("failed to read cli_key row: {e}"))?);
        }
    }

    for cli_key in cli_keys {
        let mut stmt = tx
            .prepare("SELECT id FROM providers WHERE cli_key = ?1 ORDER BY id DESC")
            .map_err(|e| format!("failed to prepare providers list for {cli_key}: {e}"))?;
        let rows = stmt
            .query_map([&cli_key], |row| row.get::<_, i64>(0))
            .map_err(|e| format!("failed to query providers list for {cli_key}: {e}"))?;

        for (sort_order, row) in (0_i64..).zip(rows) {
            let provider_id = row.map_err(|e| format!("failed to read provider id row: {e}"))?;
            tx.execute(
                "UPDATE providers SET sort_order = ?1 WHERE id = ?2",
                (sort_order, provider_id),
            )
            .map_err(|e| {
                format!("failed to backfill sort_order for provider {provider_id}: {e}")
            })?;
        }
    }

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v3_to_v4(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 4;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS request_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trace_id TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  query TEXT,
  status INTEGER,
  error_code TEXT,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  attempts_json TEXT NOT NULL DEFAULT '[]',
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at ON request_logs(cli_key, created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_request_logs_trace_id ON request_logs(trace_id);
"#,
    )
    .map_err(|e| format!("failed to migrate v3->v4: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v4_to_v5(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 5;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE request_logs ADD COLUMN input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN output_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN total_tokens INTEGER;

ALTER TABLE request_logs ADD COLUMN cache_read_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_5m_input_tokens INTEGER;
ALTER TABLE request_logs ADD COLUMN cache_creation_1h_input_tokens INTEGER;

ALTER TABLE request_logs ADD COLUMN usage_json TEXT;
"#,
    )
    .map_err(|e| format!("failed to migrate v4->v5: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v5_to_v6(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 6;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE request_logs ADD COLUMN ttfb_ms INTEGER;

UPDATE request_logs
SET ttfb_ms = duration_ms
WHERE ttfb_ms IS NULL;
"#,
    )
    .map_err(|e| format!("failed to migrate v5->v6: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v6_to_v7(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 7;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS request_attempt_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trace_id TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  query TEXT,
  attempt_index INTEGER NOT NULL,
  provider_id INTEGER NOT NULL,
  provider_name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  outcome TEXT NOT NULL,
  status INTEGER,
  attempt_started_ms INTEGER NOT NULL DEFAULT 0,
  attempt_duration_ms INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_request_attempt_logs_trace_attempt
  ON request_attempt_logs(trace_id, attempt_index);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_trace_id
  ON request_attempt_logs(trace_id);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_created_at
  ON request_attempt_logs(created_at);

CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_cli_created_at
  ON request_attempt_logs(cli_key, created_at);
"#,
    )
    .map_err(|e| format!("failed to migrate v6->v7: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v7_to_v8(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 8;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE INDEX IF NOT EXISTS idx_prompts_cli_key ON prompts(cli_key);
CREATE INDEX IF NOT EXISTS idx_prompts_cli_key_updated_at ON prompts(cli_key, updated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_cli_key_single_enabled
  ON prompts(cli_key)
  WHERE enabled = 1;
"#,
    )
    .map_err(|e| format!("failed to migrate v7->v8: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v8_to_v9(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 9;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mcp_servers (
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
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE INDEX IF NOT EXISTS idx_mcp_servers_updated_at ON mcp_servers(updated_at);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled_flags ON mcp_servers(
  enabled_claude,
  enabled_codex,
  enabled_gemini
);
"#,
    )
    .map_err(|e| format!("failed to migrate v8->v9: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v9_to_v10(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 10;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE mcp_servers ADD COLUMN normalized_name TEXT NOT NULL DEFAULT '';

UPDATE mcp_servers
SET normalized_name = LOWER(TRIM(name))
WHERE normalized_name = '' OR normalized_name IS NULL;

CREATE INDEX IF NOT EXISTS idx_mcp_servers_normalized_name ON mcp_servers(normalized_name);
"#,
    )
    .map_err(|e| format!("failed to migrate v9->v10: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v10_to_v11(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 11;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS skill_repos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  git_url TEXT NOT NULL,
  branch TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(git_url, branch)
);

CREATE INDEX IF NOT EXISTS idx_skill_repos_enabled ON skill_repos(enabled);

CREATE TABLE IF NOT EXISTS skills (
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

CREATE INDEX IF NOT EXISTS idx_skills_normalized_name ON skills(normalized_name);
CREATE INDEX IF NOT EXISTS idx_skills_updated_at ON skills(updated_at);
CREATE INDEX IF NOT EXISTS idx_skills_source ON skills(source_git_url, source_branch, source_subdir);
CREATE INDEX IF NOT EXISTS idx_skills_enabled_flags ON skills(
  enabled_claude,
  enabled_codex,
  enabled_gemini
);
"#,
    )
    .map_err(|e| format!("failed to migrate v10->v11: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v11_to_v12(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 12;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v11->v12: {e}"))?;

    let applied_at = now_unix_seconds();
    for (git_url, branch) in [
        ("https://github.com/anthropics/skills", "auto"),
        (
            "https://github.com/ComposioHQ/awesome-claude-skills",
            "auto",
        ),
        (
            "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill",
            "auto",
        ),
    ] {
        tx.execute(
            r#"
INSERT OR IGNORE INTO skill_repos(git_url, branch, enabled, created_at, updated_at)
VALUES (?1, ?2, 1, ?3, ?3)
"#,
            (git_url, branch, applied_at),
        )
        .map_err(|e| format!("failed to seed skill repo {git_url}#{branch}: {e}"))?;
    }

    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v12_to_v13(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 13;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE providers ADD COLUMN cost_multiplier REAL NOT NULL DEFAULT 1.0;

ALTER TABLE request_logs ADD COLUMN requested_model TEXT;
ALTER TABLE request_logs ADD COLUMN cost_usd_femto INTEGER;

CREATE TABLE IF NOT EXISTS model_prices (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  model TEXT NOT NULL,
  price_json TEXT NOT NULL,
  currency TEXT NOT NULL DEFAULT 'USD',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, model)
);

CREATE INDEX IF NOT EXISTS idx_model_prices_cli_key ON model_prices(cli_key);
CREATE INDEX IF NOT EXISTS idx_model_prices_cli_key_model ON model_prices(cli_key, model);
"#,
    )
    .map_err(|e| format!("failed to migrate v12->v13: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v13_to_v14(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 14;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

ALTER TABLE request_logs ADD COLUMN cost_multiplier REAL NOT NULL DEFAULT 1.0;
"#,
    )
    .map_err(|e| format!("failed to migrate v13->v14: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v14_to_v15(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 15;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
"#,
    )
    .map_err(|e| format!("failed to migrate v14->v15: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v15_to_v16(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 16;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v15->v16: {e}"))?;

    let mut has_weight = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "weight" {
                has_weight = true;
                break;
            }
        }
    }

    if has_weight {
        tx.execute_batch(
            r#"
ALTER TABLE providers RENAME TO providers_old;

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  UNIQUE(cli_key, name)
);

INSERT INTO providers(
  id,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier
)
SELECT
  id,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier
FROM providers_old;

ALTER TABLE provider_circuit_breakers RENAME TO provider_circuit_breakers_old;

CREATE TABLE provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

INSERT INTO provider_circuit_breakers(
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
)
SELECT
  provider_id,
  CASE WHEN state = 'OPEN' THEN 'OPEN' ELSE 'CLOSED' END,
  failure_count,
  CASE WHEN state = 'OPEN' THEN open_until ELSE NULL END,
  updated_at
FROM provider_circuit_breakers_old;

DROP TABLE provider_circuit_breakers_old;
DROP TABLE providers_old;

CREATE INDEX IF NOT EXISTS idx_providers_cli_key ON providers(cli_key);
CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
"#,
        )
        .map_err(|e| format!("failed to migrate v15->v16: {e}"))?;
    }

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v16_to_v17(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 17;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v16->v17: {e}"))?;

    tx.execute_batch(
        r#"
ALTER TABLE provider_circuit_breakers RENAME TO provider_circuit_breakers_old;

CREATE TABLE provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

INSERT INTO provider_circuit_breakers(
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
)
SELECT
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
FROM provider_circuit_breakers_old;

DROP TABLE provider_circuit_breakers_old;

CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);
"#,
    )
    .map_err(|e| format!("failed to migrate v16->v17: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v17_to_v18(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 18;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sort_modes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS sort_mode_providers (
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

CREATE TABLE IF NOT EXISTS sort_mode_active (
  cli_key TEXT PRIMARY KEY,
  mode_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_mode_cli_sort_order
  ON sort_mode_providers(mode_id, cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_provider_id
  ON sort_mode_providers(provider_id);
CREATE INDEX IF NOT EXISTS idx_sort_mode_active_mode_id
  ON sort_mode_active(mode_id);
"#,
    )
    .map_err(|e| format!("failed to migrate v17->v18: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v18_to_v19(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 19;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;

    let mut has_excluded_from_stats = false;
    let mut has_special_settings_json = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(request_logs)")
            .map_err(|e| format!("failed to prepare request_logs table_info: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query request_logs table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read request_logs column name: {e}"))?;
            match name.as_str() {
                "excluded_from_stats" => has_excluded_from_stats = true,
                "special_settings_json" => has_special_settings_json = true,
                _ => {}
            }
            if has_excluded_from_stats && has_special_settings_json {
                break;
            }
        }
    }

    if !has_excluded_from_stats {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN excluded_from_stats INTEGER NOT NULL DEFAULT 0;
"#,
        )
        .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;
    }

    if !has_special_settings_json {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN special_settings_json TEXT;
"#,
        )
        .map_err(|e| format!("failed to migrate v18->v19: {e}"))?;
    }

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v19_to_v20(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 20;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;

    let mut has_created_at_ms = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(request_logs)")
            .map_err(|e| format!("failed to prepare request_logs table_info: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query request_logs table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read request_logs column name: {e}"))?;
            if name == "created_at_ms" {
                has_created_at_ms = true;
                break;
            }
        }
    }

    if !has_created_at_ms {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0;
"#,
        )
        .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;
    }

    tx.execute(
        "UPDATE request_logs SET created_at_ms = created_at * 1000 WHERE created_at_ms = 0",
        [],
    )
    .map_err(|e| format!("failed to backfill request_logs.created_at_ms: {e}"))?;

    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at_ms ON request_logs(created_at_ms);
CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at_ms
  ON request_logs(cli_key, created_at_ms);
"#,
    )
    .map_err(|e| format!("failed to migrate v19->v20: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v20_to_v21(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 21;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;

    let mut has_session_id = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(request_logs)")
            .map_err(|e| format!("failed to prepare request_logs table_info: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query request_logs table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read request_logs column name: {e}"))?;
            if name == "session_id" {
                has_session_id = true;
                break;
            }
        }
    }

    if !has_session_id {
        tx.execute_batch(
            r#"
ALTER TABLE request_logs
ADD COLUMN session_id TEXT;
"#,
        )
        .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;
    }

    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_session_id ON request_logs(session_id);
"#,
    )
    .map_err(|e| format!("failed to migrate v20->v21: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v21_to_v22(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 22;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;

    let mut has_base_urls_json = false;
    let mut has_base_url_mode = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "base_urls_json" {
                has_base_urls_json = true;
            }
            if name == "base_url_mode" {
                has_base_url_mode = true;
            }
            if has_base_urls_json && has_base_url_mode {
                break;
            }
        }
    }

    if !has_base_urls_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN base_urls_json TEXT NOT NULL DEFAULT '[]';
"#,
        )
        .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;
    }

    if !has_base_url_mode {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN base_url_mode TEXT NOT NULL DEFAULT 'order';
"#,
        )
        .map_err(|e| format!("failed to migrate v21->v22: {e}"))?;
    }

    // Backfill `base_urls_json` from legacy `base_url` if needed.
    {
        let mut stmt = tx
            .prepare("SELECT id, base_url, base_urls_json FROM providers")
            .map_err(|e| format!("failed to prepare providers backfill query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers for backfill: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers backfill row: {e}"))?
        {
            let id: i64 = row
                .get(0)
                .map_err(|e| format!("failed to read provider id: {e}"))?;
            let base_url: String = row
                .get(1)
                .map_err(|e| format!("failed to read provider base_url: {e}"))?;
            let base_urls_json: String = row
                .get(2)
                .map_err(|e| format!("failed to read provider base_urls_json: {e}"))?;

            let should_backfill = base_urls_json.trim().is_empty() || base_urls_json.trim() == "[]";
            let base_url = base_url.trim();
            if !should_backfill || base_url.is_empty() {
                continue;
            }

            let json = serde_json::to_string(&vec![base_url.to_string()])
                .unwrap_or_else(|_| "[]".to_string());
            tx.execute(
                "UPDATE providers SET base_urls_json = ?1 WHERE id = ?2",
                (json, id),
            )
            .map_err(|e| format!("failed to backfill providers.base_urls_json: {e}"))?;
        }
    }

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v22_to_v23(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 23;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v22->v23: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS claude_model_validation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  request_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_claude_model_validation_runs_provider_id_id
  ON claude_model_validation_runs(provider_id, id);
"#,
    )
    .map_err(|e| format!("failed to migrate v22->v23: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v23_to_v24(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 24;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v23->v24: {e}"))?;

    tx.execute_batch(
        r#"
ALTER TABLE request_logs ADD COLUMN final_provider_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_request_logs_final_provider_id_created_at
  ON request_logs(final_provider_id, created_at);
"#,
    )
    .map_err(|e| format!("failed to migrate v23->v24: {e}"))?;

    // Backfill final_provider_id for existing request logs.
    // We use the same semantics as runtime routing:
    // - Prefer the last success attempt, otherwise fallback to the last attempt.
    #[derive(Debug, Deserialize)]
    struct AttemptRow {
        provider_id: i64,
        outcome: String,
    }

    fn final_provider_id_from_attempts_json(attempts_json: &str) -> Option<i64> {
        let attempts: Vec<AttemptRow> = serde_json::from_str(attempts_json).unwrap_or_default();
        let picked = attempts
            .iter()
            .rev()
            .find(|a| a.outcome == "success")
            .or_else(|| attempts.last());
        picked.map(|a| a.provider_id).filter(|v| *v > 0)
    }

    {
        let mut select_stmt = tx
            .prepare("SELECT id, attempts_json FROM request_logs")
            .map_err(|e| format!("failed to prepare request_logs backfill query: {e}"))?;
        let mut update_stmt = tx
            .prepare("UPDATE request_logs SET final_provider_id = ?1 WHERE id = ?2")
            .map_err(|e| format!("failed to prepare request_logs backfill update: {e}"))?;

        let mut rows = select_stmt
            .query([])
            .map_err(|e| format!("failed to run request_logs backfill query: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read request_logs backfill row: {e}"))?
        {
            let id: i64 = row
                .get("id")
                .map_err(|e| format!("failed to read request_logs.id for backfill: {e}"))?;
            let attempts_json: String = row.get("attempts_json").unwrap_or_default();
            let provider_id = final_provider_id_from_attempts_json(&attempts_json);
            if provider_id.is_none() {
                continue;
            }
            update_stmt
                .execute(rusqlite::params![provider_id, id])
                .map_err(|e| format!("failed to backfill request_logs.final_provider_id: {e}"))?;
        }
    }

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v24_to_v25(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 25;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;

    let mut has_supported_models_json = false;
    let mut has_model_mapping_json = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "supported_models_json" {
                has_supported_models_json = true;
            }
            if name == "model_mapping_json" {
                has_model_mapping_json = true;
            }
            if has_supported_models_json && has_model_mapping_json {
                break;
            }
        }
    }

    if !has_supported_models_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN supported_models_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;
    }

    if !has_model_mapping_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN model_mapping_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;
    }

    // Backfill invalid/empty values to keep JSON parsing stable even if DB gets partially corrupted.
    tx.execute_batch(
        r#"
UPDATE providers
SET supported_models_json = '{}'
WHERE supported_models_json IS NULL OR TRIM(supported_models_json) = '';

UPDATE providers
SET model_mapping_json = '{}'
WHERE model_mapping_json IS NULL OR TRIM(model_mapping_json) = '';
"#,
    )
    .map_err(|e| format!("failed to migrate v24->v25: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}

fn migrate_v25_to_v26(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 26;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;

    let mut has_claude_models_json = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "claude_models_json" {
                has_claude_models_json = true;
                break;
            }
        }
    }

    if !has_claude_models_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN claude_models_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;
    }

    // Backfill invalid/empty values to keep JSON parsing stable even if DB gets partially corrupted.
    tx.execute_batch(
        r#"
UPDATE providers
SET claude_models_json = '{}'
WHERE claude_models_json IS NULL OR TRIM(claude_models_json) = '';
"#,
    )
    .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;

    fn normalize_slot(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.len() > 200 {
            return Some(trimmed[..200].to_string());
        }
        Some(trimmed.to_string())
    }

    fn main_candidate_score(key_lower: &str) -> i64 {
        if key_lower == "*" {
            return 100;
        }
        if key_lower.contains('*') && key_lower.contains("claude") {
            return 90;
        }
        if key_lower.contains('*') {
            return 80;
        }
        if key_lower.contains("default") || key_lower.contains("main") {
            return 75;
        }
        if key_lower.contains("claude") {
            return 70;
        }
        50
    }

    {
        let mut select_stmt = tx
            .prepare(
                r#"
	SELECT id, model_mapping_json, claude_models_json
	FROM providers
	WHERE cli_key = 'claude'
	"#,
            )
            .map_err(|e| format!("failed to prepare providers migration query: {e}"))?;

        let mut update_stmt = tx
            .prepare(
                r#"
	UPDATE providers
	SET claude_models_json = ?1
	WHERE id = ?2
	"#,
            )
            .map_err(|e| format!("failed to prepare providers migration update: {e}"))?;

        let mut rows = select_stmt
            .query([])
            .map_err(|e| format!("failed to run providers migration query: {e}"))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers migration row: {e}"))?
        {
            let id: i64 = row
                .get("id")
                .map_err(|e| format!("failed to read providers.id during migration: {e}"))?;
            let model_mapping_json: String = row.get("model_mapping_json").unwrap_or_default();
            let claude_models_json: String = row.get("claude_models_json").unwrap_or_default();

            // Do not override if user already has a non-empty Claude models config (defensive).
            let claude_models_trimmed = claude_models_json.trim();
            if !claude_models_trimmed.is_empty() && claude_models_trimmed != "{}" {
                continue;
            }

            let mapping: std::collections::HashMap<String, String> =
                serde_json::from_str(&model_mapping_json).unwrap_or_default();

            let mut main_model: Option<String> = None;
            let mut reasoning_model: Option<String> = None;
            let mut haiku_model: Option<String> = None;
            let mut sonnet_model: Option<String> = None;
            let mut opus_model: Option<String> = None;

            let mut main_candidates: Vec<(i64, String, String)> = Vec::new();

            for (raw_key, raw_value) in mapping {
                let key = raw_key.trim();
                let Some(value) = normalize_slot(&raw_value) else {
                    continue;
                };

                let key_lower = key.to_ascii_lowercase();

                if reasoning_model.is_none()
                    && (key_lower.contains("thinking")
                        || key_lower.contains("reasoning")
                        || key_lower.contains("extended"))
                {
                    reasoning_model = Some(value.clone());
                }
                if haiku_model.is_none() && key_lower.contains("haiku") {
                    haiku_model = Some(value.clone());
                }
                if sonnet_model.is_none() && key_lower.contains("sonnet") {
                    sonnet_model = Some(value.clone());
                }
                if opus_model.is_none() && key_lower.contains("opus") {
                    opus_model = Some(value.clone());
                }

                let is_specialized = key_lower.contains("thinking")
                    || key_lower.contains("reasoning")
                    || key_lower.contains("extended")
                    || key_lower.contains("haiku")
                    || key_lower.contains("sonnet")
                    || key_lower.contains("opus");
                if !is_specialized {
                    let score = main_candidate_score(&key_lower);
                    main_candidates.push((score, key.to_string(), value));
                }
            }

            if main_model.is_none() && !main_candidates.is_empty() {
                // Deterministic selection: higher score first, then lexicographic key.
                main_candidates
                    .sort_by(|(sa, ka, _), (sb, kb, _)| sb.cmp(sa).then_with(|| ka.cmp(kb)));
                main_model = Some(main_candidates[0].2.clone());
            }

            let mut obj = serde_json::Map::new();
            if let Some(v) = main_model {
                obj.insert("main_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = reasoning_model {
                obj.insert("reasoning_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = haiku_model {
                obj.insert("haiku_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = sonnet_model {
                obj.insert("sonnet_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = opus_model {
                obj.insert("opus_model".to_string(), serde_json::Value::String(v));
            }

            let next_json = serde_json::to_string(&serde_json::Value::Object(obj))
                .unwrap_or_else(|_| "{}".to_string());

            update_stmt
                .execute(rusqlite::params![next_json, id])
                .map_err(|e| format!("failed to update providers.claude_models_json: {e}"))?;
        }
    }

    tx.execute_batch(
        r#"
UPDATE providers
SET supported_models_json = '{}',
    model_mapping_json = '{}'
WHERE cli_key = 'claude';
"#,
    )
    .map_err(|e| format!("failed to clear legacy provider model config: {e}"))?;

    let applied_at = now_unix_seconds();
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

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

        migrate_v25_to_v26(&mut conn).expect("migrate v25->v26");

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
}
