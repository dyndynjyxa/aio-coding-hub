//! Usage: Idempotent ensure patches applied after all versioned migrations.
//!
//! These patches add columns, tables, and indexes without bumping user_version.
//! They are safe to run repeatedly and handle missing tables/columns gracefully.

use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};

/// Apply all idempotent ensure patches.
pub(super) fn apply_ensure_patches(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    ensure_workspace_cluster(conn)?;
    ensure_provider_limits(conn)?;
    ensure_provider_oauth_columns(conn)?;
    ensure_sort_mode_providers_enabled(conn)?;
    ensure_usage_indexes(conn)?;
    ensure_provider_tags(conn)?;
    ensure_provider_note(conn)?;
    ensure_provider_source_provider_id(conn)?;
    ensure_provider_bridge_type(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_workspace_cluster (from v29_to_v30.rs)
// ---------------------------------------------------------------------------

fn ensure_workspace_cluster(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    ensure_workspaces_and_active(&tx)?;
    ensure_prompts_scoped_by_workspace(&tx)?;
    ensure_mcp_scoped_by_workspace(&tx)?;
    ensure_skills_scoped_by_workspace(&tx)?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;

    Ok(())
}

fn ensure_workspaces_and_active(conn: &Connection) -> crate::shared::error::AppResult<()> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS workspaces (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, normalized_name)
);
CREATE INDEX IF NOT EXISTS idx_workspaces_cli_key_updated_at ON workspaces(cli_key, updated_at);

CREATE TABLE IF NOT EXISTS workspace_active (
  cli_key TEXT PRIMARY KEY,
  workspace_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_workspace_active_workspace_id ON workspace_active(workspace_id);
"#,
    )
    .map_err(|e| format!("failed to ensure workspaces tables: {e}"))?;

    let now = now_unix_seconds();
    let default_name = "默认";
    let default_normalized = normalize_name(default_name);

    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        conn.execute(
            r#"
INSERT OR IGNORE INTO workspaces(
  cli_key,
  name,
  normalized_name,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5)
"#,
            params![cli_key, default_name, default_normalized, now, now],
        )
        .map_err(|e| format!("failed to seed default workspace for cli_key={cli_key}: {e}"))?;

        let workspace_id: i64 = conn
            .query_row(
                r#"
SELECT id
FROM workspaces
WHERE cli_key = ?1 AND normalized_name = ?2
ORDER BY id DESC
LIMIT 1
"#,
                params![cli_key, default_normalized],
                |row| row.get(0),
            )
            .map_err(|e| format!("failed to query default workspace for cli_key={cli_key}: {e}"))?;

        conn.execute(
            r#"
INSERT OR IGNORE INTO workspace_active(
  cli_key,
  workspace_id,
  updated_at
) VALUES (?1, ?2, ?3)
"#,
            params![cli_key, workspace_id, now],
        )
        .map_err(|e| format!("failed to seed workspace_active for cli_key={cli_key}: {e}"))?;

        // If workspace_active exists but workspace_id is NULL, backfill it.
        let existing: Option<Option<i64>> = conn
            .query_row(
                "SELECT workspace_id FROM workspace_active WHERE cli_key = ?1",
                params![cli_key],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(|e| format!("failed to query workspace_active for cli_key={cli_key}: {e}"))?;
        if existing.flatten().is_none() {
            conn.execute(
                "UPDATE workspace_active SET workspace_id = ?1, updated_at = ?2 WHERE cli_key = ?3",
                params![workspace_id, now, cli_key],
            )
            .map_err(|e| {
                format!("failed to backfill workspace_active for cli_key={cli_key}: {e}")
            })?;
        }
    }

    Ok(())
}
fn ensure_prompts_scoped_by_workspace(conn: &Connection) -> crate::shared::error::AppResult<()> {
    if !column_exists(conn, "prompts", "workspace_id")? {
        conn.execute_batch(
            r#"
DROP TABLE IF EXISTS prompts_next;

CREATE TABLE IF NOT EXISTS prompts_next (
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

INSERT INTO prompts_next(
  id,
  workspace_id,
  name,
  content,
  enabled,
  created_at,
  updated_at
)
SELECT
  p.id,
  COALESCE(
    (SELECT workspace_id FROM workspace_active WHERE cli_key = p.cli_key),
    (SELECT id FROM workspaces WHERE cli_key = p.cli_key ORDER BY id DESC LIMIT 1)
  ) AS workspace_id,
  p.name,
  p.content,
  p.enabled,
  p.created_at,
  p.updated_at
FROM prompts p;

DROP TABLE prompts;
ALTER TABLE prompts_next RENAME TO prompts;
"#,
        )
        .map_err(|e| format!("failed to scope prompts by workspace_id: {e}"))?;
    }

    conn.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_prompts_workspace_id ON prompts(workspace_id);
CREATE INDEX IF NOT EXISTS idx_prompts_workspace_id_updated_at ON prompts(workspace_id, updated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_workspace_single_enabled
  ON prompts(workspace_id)
  WHERE enabled = 1;
"#,
    )
    .map_err(|e| format!("failed to ensure prompts indexes: {e}"))?;

    Ok(())
}
fn ensure_mcp_scoped_by_workspace(conn: &Connection) -> crate::shared::error::AppResult<()> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS workspace_mcp_enabled (
  workspace_id INTEGER NOT NULL,
  server_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, server_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workspace_mcp_enabled_workspace_id
  ON workspace_mcp_enabled(workspace_id);
CREATE INDEX IF NOT EXISTS idx_workspace_mcp_enabled_server_id
  ON workspace_mcp_enabled(server_id);
"#,
    )
    .map_err(|e| format!("failed to ensure workspace_mcp_enabled: {e}"))?;

    // Backfill legacy enabled flags only once by clearing them after migration.
    let now = now_unix_seconds();
    for (cli_key, flag_col) in [
        ("claude", "enabled_claude"),
        ("codex", "enabled_codex"),
        ("gemini", "enabled_gemini"),
    ] {
        if !column_exists(conn, "mcp_servers", flag_col)? {
            continue;
        }

        let sql = format!(
            r#"
INSERT OR IGNORE INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at)
SELECT
  COALESCE(
    (SELECT workspace_id FROM workspace_active WHERE cli_key = '{cli_key}'),
    (SELECT id FROM workspaces WHERE cli_key = '{cli_key}' ORDER BY id DESC LIMIT 1)
  ),
  id,
  ?1,
  ?1
FROM mcp_servers
WHERE {flag_col} = 1
"#
        );

        conn.execute(&sql, params![now]).map_err(|e| {
            format!("failed to migrate mcp enabled flags for cli_key={cli_key}: {e}")
        })?;

        let clear_sql = format!("UPDATE mcp_servers SET {flag_col} = 0 WHERE {flag_col} != 0");
        conn.execute(&clear_sql, [])
            .map_err(|e| format!("failed to clear legacy mcp enabled flag {flag_col}: {e}"))?;
    }

    Ok(())
}
fn ensure_skills_scoped_by_workspace(conn: &Connection) -> crate::shared::error::AppResult<()> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS workspace_skill_enabled (
  workspace_id INTEGER NOT NULL,
  skill_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, skill_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workspace_skill_enabled_workspace_id
  ON workspace_skill_enabled(workspace_id);
CREATE INDEX IF NOT EXISTS idx_workspace_skill_enabled_skill_id
  ON workspace_skill_enabled(skill_id);
"#,
    )
    .map_err(|e| format!("failed to ensure workspace_skill_enabled: {e}"))?;

    // Backfill legacy enabled flags only once by clearing them after migration.
    let now = now_unix_seconds();
    for (cli_key, flag_col) in [
        ("claude", "enabled_claude"),
        ("codex", "enabled_codex"),
        ("gemini", "enabled_gemini"),
    ] {
        if !column_exists(conn, "skills", flag_col)? {
            continue;
        }

        let sql = format!(
            r#"
INSERT OR IGNORE INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
SELECT
  COALESCE(
    (SELECT workspace_id FROM workspace_active WHERE cli_key = '{cli_key}'),
    (SELECT id FROM workspaces WHERE cli_key = '{cli_key}' ORDER BY id DESC LIMIT 1)
  ),
  id,
  ?1,
  ?1
FROM skills
WHERE {flag_col} = 1
"#
        );

        conn.execute(&sql, params![now]).map_err(|e| {
            format!("failed to migrate skill enabled flags for cli_key={cli_key}: {e}")
        })?;

        let clear_sql = format!("UPDATE skills SET {flag_col} = 0 WHERE {flag_col} != 0");
        conn.execute(&clear_sql, [])
            .map_err(|e| format!("failed to clear legacy skill enabled flag {flag_col}: {e}"))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_limits (from v29_to_v30_provider_limits.rs)
// ---------------------------------------------------------------------------
fn ensure_provider_limits(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_providers_table: bool = tx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_providers_table {
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
        return Ok(());
    }

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            existing.insert(name);
        }
    }

    let mut ddl: Vec<&'static str> = Vec::new();

    if !existing.contains("limit_5h_usd") {
        ddl.push("ALTER TABLE providers ADD COLUMN limit_5h_usd REAL;");
    }
    if !existing.contains("limit_daily_usd") {
        ddl.push("ALTER TABLE providers ADD COLUMN limit_daily_usd REAL;");
    }
    if !existing.contains("daily_reset_mode") {
        ddl.push(
            "ALTER TABLE providers ADD COLUMN daily_reset_mode TEXT NOT NULL DEFAULT 'fixed';",
        );
    }
    if !existing.contains("daily_reset_time") {
        ddl.push(
            "ALTER TABLE providers ADD COLUMN daily_reset_time TEXT NOT NULL DEFAULT '00:00:00';",
        );
    }
    if !existing.contains("limit_weekly_usd") {
        ddl.push("ALTER TABLE providers ADD COLUMN limit_weekly_usd REAL;");
    }
    if !existing.contains("limit_monthly_usd") {
        ddl.push("ALTER TABLE providers ADD COLUMN limit_monthly_usd REAL;");
    }
    if !existing.contains("limit_total_usd") {
        ddl.push("ALTER TABLE providers ADD COLUMN limit_total_usd REAL;");
    }
    if !existing.contains("window_5h_start_ts") {
        ddl.push("ALTER TABLE providers ADD COLUMN window_5h_start_ts INTEGER;");
    }

    if !ddl.is_empty() {
        tx.execute_batch(ddl.join("\n").as_str())
            .map_err(|e| format!("failed to ensure providers spend limit columns: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_oauth_columns (from v29_to_v30.rs)
// ---------------------------------------------------------------------------
fn ensure_provider_oauth_columns(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_providers_table: bool = tx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_providers_table {
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
        return Ok(());
    }

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            existing.insert(name);
        }
    }

    let mut ddl: Vec<&'static str> = Vec::new();
    let adding_auth_mode = !existing.contains("auth_mode");
    let adding_oauth_refresh_lead_s = !existing.contains("oauth_refresh_lead_s");

    if adding_auth_mode {
        ddl.push("ALTER TABLE providers ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'api_key';");
    }
    if !existing.contains("oauth_provider_type") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_provider_type TEXT;");
    }
    if !existing.contains("oauth_access_token") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_access_token TEXT;");
    }
    if !existing.contains("oauth_refresh_token") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_refresh_token TEXT;");
    }
    if !existing.contains("oauth_id_token") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_id_token TEXT;");
    }
    if !existing.contains("oauth_token_uri") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_token_uri TEXT;");
    }
    if !existing.contains("oauth_client_id") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_client_id TEXT;");
    }
    if !existing.contains("oauth_client_secret") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_client_secret TEXT;");
    }
    if !existing.contains("oauth_expires_at") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_expires_at INTEGER;");
    }
    if !existing.contains("oauth_email") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_email TEXT;");
    }
    if !existing.contains("oauth_last_refreshed_at") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_last_refreshed_at INTEGER;");
    }
    if !existing.contains("oauth_last_error") {
        ddl.push("ALTER TABLE providers ADD COLUMN oauth_last_error TEXT;");
    }
    if adding_oauth_refresh_lead_s {
        ddl.push(
            "ALTER TABLE providers ADD COLUMN oauth_refresh_lead_s INTEGER NOT NULL DEFAULT 3600;",
        );
    }

    if !ddl.is_empty() {
        tx.execute_batch(ddl.join("\n").as_str())
            .map_err(|e| format!("failed to ensure providers oauth columns: {e}"))?;
    }

    if existing.contains("auth_mode") || adding_auth_mode {
        tx.execute_batch(
            r#"
UPDATE providers
SET auth_mode = 'api_key'
WHERE auth_mode IS NULL OR TRIM(auth_mode) = '';
"#,
        )
        .map_err(|e| format!("failed to backfill providers.auth_mode: {e}"))?;
    }

    if existing.contains("oauth_refresh_lead_s") || adding_oauth_refresh_lead_s {
        tx.execute_batch(
            r#"
UPDATE providers
SET oauth_refresh_lead_s = 3600
WHERE oauth_refresh_lead_s IS NULL OR oauth_refresh_lead_s <= 0;
"#,
        )
        .map_err(|e| format!("failed to backfill providers.oauth_refresh_lead_s: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_sort_mode_providers_enabled (from v29_to_v30_sort_mode_providers_enabled.rs)
// ---------------------------------------------------------------------------
fn ensure_sort_mode_providers_enabled(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_table: bool = tx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sort_mode_providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_table {
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
        return Ok(());
    }

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(sort_mode_providers)")
            .map_err(|e| format!("failed to prepare sort_mode_providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query sort_mode_providers table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read sort_mode_providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read sort_mode_providers column name: {e}"))?;
            existing.insert(name);
        }
    }

    if !existing.contains("enabled") {
        tx.execute_batch(
            "ALTER TABLE sort_mode_providers ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;",
        )
        .map_err(|e| format!("failed to ensure sort_mode_providers.enabled column: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_usage_indexes (from v29_to_v30_usage_indexes.rs)
// ---------------------------------------------------------------------------
fn ensure_usage_indexes(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_request_logs: bool = tx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'request_logs' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_request_logs {
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
        return Ok(());
    }

    // Index 1: Composite index for usage stats summary queries (usage_stats/summary.rs:44-113)
    // Optimizes WHERE cli_key = ? AND created_at >= ? AND created_at < ? AND excluded_from_stats = 0
    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at_excluded
  ON request_logs(cli_key, created_at, excluded_from_stats);
"#,
    )
    .map_err(|e| format!("failed to create idx_request_logs_cli_created_at_excluded: {e}"))?;

    // Index 2: Partial index for provider cost queries (provider_limits.rs)
    // Optimizes queries on successful requests with valid cost data
    tx.execute_batch(
        r#"
CREATE INDEX IF NOT EXISTS idx_request_logs_provider_success_cost
  ON request_logs(final_provider_id, created_at)
  WHERE status >= 200 AND status < 300
    AND error_code IS NULL
    AND cost_usd_femto IS NOT NULL
    AND excluded_from_stats = 0;
"#,
    )
    .map_err(|e| format!("failed to create idx_request_logs_provider_success_cost: {e}"))?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_tags (from ensure_provider_tags.rs)
// ---------------------------------------------------------------------------

fn ensure_provider_tags(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    let has_providers_table: bool = tx
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_providers_table {
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
        return Ok(());
    }

    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            existing.insert(name);
        }
    }

    if !existing.contains("tags_json") {
        tx.execute_batch("ALTER TABLE providers ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]';")
            .map_err(|e| format!("failed to ensure providers tags_json column: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_note
// ---------------------------------------------------------------------------

fn ensure_provider_note(conn: &mut Connection) -> Result<(), String> {
    let has_providers_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_providers_table {
        return Ok(());
    }

    if !column_exists(conn, "providers", "note")? {
        conn.execute_batch("ALTER TABLE providers ADD COLUMN note TEXT NOT NULL DEFAULT '';")
            .map_err(|e| format!("failed to ensure providers note column: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_source_provider_id (CX2CC translation provider)
// ---------------------------------------------------------------------------

fn ensure_provider_source_provider_id(conn: &mut Connection) -> Result<(), String> {
    let has_providers_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("failed to query sqlite_master: {e}"))?
        .unwrap_or(false);

    if !has_providers_table {
        return Ok(());
    }

    if !column_exists(conn, "providers", "source_provider_id")? {
        conn.execute_batch(
            "ALTER TABLE providers ADD COLUMN source_provider_id INTEGER DEFAULT NULL REFERENCES providers(id) ON DELETE SET NULL;",
        )
        .map_err(|e| format!("failed to ensure providers source_provider_id column: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ensure_provider_bridge_type (protocol bridge type identifier)
// ---------------------------------------------------------------------------

fn ensure_provider_bridge_type(conn: &mut Connection) -> Result<(), String> {
    let has_providers_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !has_providers_table {
        return Ok(());
    }

    if !column_exists(conn, "providers", "bridge_type")? {
        conn.execute_batch("ALTER TABLE providers ADD COLUMN bridge_type TEXT DEFAULT NULL;")
            .map_err(|e| format!("failed to ensure providers bridge_type column: {e}"))?;

        // Back-fill existing CX2CC providers.
        conn.execute_batch(
            "UPDATE providers SET bridge_type = 'cx2cc' WHERE source_provider_id IS NOT NULL AND bridge_type IS NULL;",
        )
        .map_err(|e| format!("failed to back-fill bridge_type for cx2cc providers: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

fn column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
) -> crate::shared::error::AppResult<bool> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to prepare {sql}: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("failed to query {sql}: {e}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("failed to read table_info row: {e}"))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| format!("failed to read column name from table_info: {e}"))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}
