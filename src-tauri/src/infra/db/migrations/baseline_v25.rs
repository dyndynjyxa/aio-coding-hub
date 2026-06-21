//! Usage: Baseline schema for fresh installs.
//!
//! This file creates the complete current database schema.
//! For existing users (user_version >= 25), this is skipped entirely.
//! Incremental migrations handle upgrades from earlier supported schemas.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn create_baseline_v25(conn: &mut Connection) -> Result<(), String> {
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
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);

CREATE INDEX IF NOT EXISTS idx_providers_cli_key ON providers(cli_key);
CREATE INDEX IF NOT EXISTS idx_providers_cli_key_sort_order ON providers(cli_key, sort_order);

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
  created_at INTEGER NOT NULL,
  input_tokens INTEGER,
  output_tokens INTEGER,
  total_tokens INTEGER,
  cache_read_input_tokens INTEGER,
  cache_creation_input_tokens INTEGER,
  cache_creation_5m_input_tokens INTEGER,
  cache_creation_1h_input_tokens INTEGER,
  usage_json TEXT,
  ttfb_ms INTEGER,
  requested_model TEXT,
  cost_usd_femto INTEGER,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  excluded_from_stats INTEGER NOT NULL DEFAULT 0,
  special_settings_json TEXT,
  created_at_ms INTEGER NOT NULL DEFAULT 0,
  session_id TEXT,
  final_provider_id INTEGER
);

CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at ON request_logs(cli_key, created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_request_logs_trace_id ON request_logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at_ms ON request_logs(created_at_ms);
CREATE INDEX IF NOT EXISTS idx_request_logs_cli_created_at_ms ON request_logs(cli_key, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_request_logs_session_id ON request_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_final_provider_id_created_at ON request_logs(final_provider_id, created_at);

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
CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_cli_key_single_enabled ON prompts(cli_key) WHERE enabled = 1;

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
  normalized_name TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE INDEX IF NOT EXISTS idx_mcp_servers_updated_at ON mcp_servers(updated_at);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled_flags ON mcp_servers(enabled_claude, enabled_codex, enabled_gemini);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_normalized_name ON mcp_servers(normalized_name);

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
  installed_commit TEXT DEFAULT NULL,
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
CREATE INDEX IF NOT EXISTS idx_skills_enabled_flags ON skills(enabled_claude, enabled_codex, enabled_gemini);

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

CREATE TABLE IF NOT EXISTS provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  failure_timestamps_json TEXT NOT NULL DEFAULT '[]',
  half_open_success_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_circuit_breakers_state ON provider_circuit_breakers(state);

CREATE TABLE IF NOT EXISTS provider_pool_order (
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, provider_id),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_pool_order_cli_sort_order ON provider_pool_order(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_provider_pool_order_provider_id ON provider_pool_order(provider_id);

CREATE TABLE IF NOT EXISTS default_route_providers (
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, provider_id),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_default_route_providers_cli_sort_order ON default_route_providers(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_default_route_providers_provider_id ON default_route_providers(provider_id);

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

CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_mode_cli_sort_order ON sort_mode_providers(mode_id, cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_sort_mode_providers_provider_id ON sort_mode_providers(provider_id);

CREATE TABLE IF NOT EXISTS sort_mode_active (
  cli_key TEXT PRIMARY KEY,
  mode_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sort_mode_active_mode_id ON sort_mode_active(mode_id);

CREATE TABLE IF NOT EXISTS claude_model_validation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  request_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_claude_model_validation_runs_provider_id_id ON claude_model_validation_runs(provider_id, id);
"#,
    )
    .map_err(|e| format!("failed to create baseline v25 schema: {e}"))?;

    // Seed default skill repos
    let now = now_unix_seconds();
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
            (git_url, branch, now),
        )
        .map_err(|e| format!("failed to seed skill repo {git_url}#{branch}: {e}"))?;
    }

    // Record baseline in schema_migrations
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (super::LATEST_SCHEMA_VERSION, now),
    )
    .map_err(|e| format!("failed to record baseline migration: {e}"))?;

    super::set_user_version(&tx, super::LATEST_SCHEMA_VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit baseline migration: {e}"))?;

    Ok(())
}
