//! Database CRUD operations for providers.

use super::types::*;
use super::validation::*;
use crate::db;
use crate::shared::error::db_err;
use crate::shared::sqlite::enabled_to_int;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

fn decode_provider_row(
    row: &rusqlite::Row<'_>,
    cli_key: &str,
) -> Result<DecodedProviderRow, rusqlite::Error> {
    let base_url_fallback: String = row.get("base_url")?;
    let base_urls_json: String = row.get("base_urls_json")?;
    let base_url_mode_raw: String = row.get("base_url_mode")?;
    let claude_models_json: String = row.get("claude_models_json")?;
    let daily_reset_mode_raw: String = row.get("daily_reset_mode")?;
    let daily_reset_time_raw: String = row.get("daily_reset_time")?;

    Ok(DecodedProviderRow {
        id: row.get("id")?,
        name: row.get("name")?,
        base_urls: base_urls_from_row(&base_url_fallback, &base_urls_json),
        base_url_mode: ProviderBaseUrlMode::parse(&base_url_mode_raw)
            .unwrap_or(ProviderBaseUrlMode::Order),
        claude_models: if cli_key == "claude" {
            claude_models_from_json(&claude_models_json)
        } else {
            ClaudeModels::default()
        },
        limit_5h_usd: row.get("limit_5h_usd")?,
        limit_daily_usd: row.get("limit_daily_usd")?,
        daily_reset_mode: DailyResetMode::parse(&daily_reset_mode_raw)
            .unwrap_or(DailyResetMode::Fixed),
        daily_reset_time: normalize_reset_time_hms_lossy(&daily_reset_time_raw),
        limit_weekly_usd: row.get("limit_weekly_usd")?,
        limit_monthly_usd: row.get("limit_monthly_usd")?,
        limit_total_usd: row.get("limit_total_usd")?,
        auth_mode: row
            .get::<_, Option<String>>("auth_mode")?
            .unwrap_or_else(|| "api_key".to_string()),
        oauth_provider_type: row.get("oauth_provider_type")?,
        source_provider_id: row.get("source_provider_id")?,
        bridge_type: row.get("bridge_type").unwrap_or(None),
    })
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<ProviderSummary, rusqlite::Error> {
    let cli_key: String = row.get("cli_key")?;
    let tags_json: String = row.get("tags_json")?;
    let decoded = decode_provider_row(row, &cli_key)?;

    Ok(ProviderSummary {
        id: decoded.id,
        cli_key,
        name: decoded.name,
        base_urls: decoded.base_urls,
        base_url_mode: decoded.base_url_mode,
        claude_models: decoded.claude_models,
        enabled: row.get::<_, i64>("enabled")? != 0,
        priority: row.get("priority")?,
        cost_multiplier: row.get("cost_multiplier")?,
        limit_5h_usd: decoded.limit_5h_usd,
        limit_daily_usd: decoded.limit_daily_usd,
        daily_reset_mode: decoded.daily_reset_mode,
        daily_reset_time: decoded.daily_reset_time,
        limit_weekly_usd: decoded.limit_weekly_usd,
        limit_monthly_usd: decoded.limit_monthly_usd,
        limit_total_usd: decoded.limit_total_usd,
        tags: tags_from_json(&tags_json),
        note: row.get("note")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        auth_mode: decoded.auth_mode,
        oauth_provider_type: decoded.oauth_provider_type,
        oauth_email: row.get("oauth_email")?,
        oauth_expires_at: row.get("oauth_expires_at")?,
        oauth_last_error: row.get("oauth_last_error")?,
        source_provider_id: decoded.source_provider_id,
        bridge_type: decoded.bridge_type,
        stream_idle_timeout_seconds: parse_positive_optional_u32(
            row.get("stream_idle_timeout_seconds").unwrap_or(None),
        ),
        api_key_configured: row
            .get::<_, Option<i64>>("api_key_configured")
            .unwrap_or(None)
            .unwrap_or(0)
            != 0,
    })
}

pub(crate) fn get_by_id(
    conn: &Connection,
    provider_id: i64,
) -> crate::shared::error::AppResult<ProviderSummary> {
    conn.query_row(
        r#"
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  tags_json,
  note,
  enabled,
  priority,
  cost_multiplier,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  created_at,
  updated_at,
  auth_mode,
  oauth_provider_type,
  oauth_email,
  oauth_expires_at,
  oauth_last_error,
  source_provider_id,
  bridge_type,
  stream_idle_timeout_seconds,
  CASE WHEN COALESCE(api_key_plaintext, '') = '' THEN 0 ELSE 1 END AS api_key_configured
FROM providers
WHERE id = ?1
"#,
        params![provider_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| db_err!("failed to query provider: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found"))
}

pub(crate) fn claude_terminal_launch_context(
    db: &db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<ClaudeTerminalLaunchContext> {
    type ClaudeLaunchProviderRow = (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<i64>,
        Option<String>,
    );

    if provider_id <= 0 {
        return Err(format!("SEC_INVALID_INPUT: invalid provider_id={provider_id}").into());
    }

    let conn = db.open_connection()?;
    let row: Option<ClaudeLaunchProviderRow> = conn
        .query_row(
            r#"
SELECT
  cli_key,
  base_url,
  base_urls_json,
  api_key_plaintext,
  auth_mode,
  oauth_access_token,
  source_provider_id,
  bridge_type
FROM providers
WHERE id = ?1
"#,
            params![provider_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(|e| db_err!("failed to query provider for launch context: {e}"))?;

    let Some((
        cli_key,
        base_url_fallback,
        base_urls_json,
        api_key_plaintext,
        auth_mode,
        oauth_access_token,
        source_provider_id,
        bridge_type,
    )) = row
    else {
        return Err("DB_NOT_FOUND: provider not found".to_string().into());
    };

    if cli_key != "claude" {
        return Err(format!("SEC_INVALID_INPUT: provider_id={provider_id} is not claude").into());
    }

    let is_cx2cc = is_cx2cc_bridge(source_provider_id, bridge_type.as_deref());

    // For OAuth mode or cx2cc providers, base_url may legitimately be empty
    // (the gateway handles routing via source provider or local Codex gateway).
    // For api_key mode without cx2cc, base_url is still required.
    if auth_mode != "oauth" && !is_cx2cc {
        let base_url = base_urls_from_row(&base_url_fallback, &base_urls_json)
            .into_iter()
            .find(|v| !v.trim().is_empty())
            .ok_or_else(|| "SEC_INVALID_INPUT: provider base_url is empty".to_string())?;

        reqwest::Url::parse(&base_url)
            .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url={base_url}: {e}"))?;
    }

    // Resolve the credential based on auth_mode.
    // For cx2cc providers the gateway uses the source Codex provider's key at runtime,
    // so the cx2cc provider itself may have no api_key.  We use a placeholder token
    // that lets Claude CLI start; the gateway ignores it.
    let effective_credential = if auth_mode == "oauth" {
        let token = oauth_access_token
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider OAuth access token is empty — please re-authenticate"
                    .to_string()
            })?;
        token.to_string()
    } else if is_cx2cc {
        let key = api_key_plaintext.trim().to_string();
        if key.is_empty() {
            format!("cx2cc-{provider_id}")
        } else {
            key
        }
    } else {
        let key = api_key_plaintext.trim().to_string();
        if key.is_empty() {
            return Err("SEC_INVALID_INPUT: provider api_key is empty"
                .to_string()
                .into());
        }
        key
    };

    Ok(ClaudeTerminalLaunchContext {
        api_key_plaintext: effective_credential,
    })
}

/// Returns the raw API key for any provider (not limited to Claude).
pub fn get_api_key_plaintext(
    db: &db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<String> {
    let conn = db.open_connection()?;
    let key: Option<String> = conn
        .query_row(
            "SELECT api_key_plaintext FROM providers WHERE id = ?1",
            rusqlite::params![provider_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query provider api_key: {e}"))?;

    key.ok_or_else(|| "DB_NOT_FOUND: provider not found".to_string().into())
}

pub fn names_by_id(
    db: &db::Db,
    provider_ids: &[i64],
) -> crate::shared::error::AppResult<HashMap<i64, String>> {
    let ids: Vec<i64> = provider_ids
        .iter()
        .copied()
        .filter(|id| *id > 0)
        .collect::<HashSet<i64>>()
        .into_iter()
        .collect();

    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let conn = db.open_connection()?;

    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql = format!("SELECT id, name FROM providers WHERE id IN ({placeholders})");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let mut rows = stmt
        .query(params_from_iter(ids.iter()))
        .map_err(|e| db_err!("failed to query provider names: {e}"))?;

    let mut out: HashMap<i64, String> = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| db_err!("failed to read provider row: {e}"))?
    {
        let id: i64 = row
            .get(0)
            .map_err(|e| db_err!("invalid provider id: {e}"))?;
        let name: String = row
            .get(1)
            .map_err(|e| db_err!("invalid provider name: {e}"))?;
        out.insert(id, name);
    }

    Ok(out)
}

pub(crate) fn cli_key_by_id(
    db: &db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<Option<String>> {
    if provider_id <= 0 {
        return Err(format!("SEC_INVALID_INPUT: invalid provider_id={provider_id}").into());
    }

    let conn = db.open_connection()?;
    let cli_key = conn
        .query_row(
            "SELECT cli_key FROM providers WHERE id = ?1",
            params![provider_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query provider cli_key: {e}"))?;
    Ok(cli_key)
}

pub fn list_by_cli(
    db: &db::Db,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<ProviderSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  tags_json,
  note,
  enabled,
  priority,
  cost_multiplier,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  created_at,
  updated_at,
  auth_mode,
  oauth_provider_type,
  oauth_email,
  oauth_expires_at,
  oauth_last_error,
  source_provider_id,
  bridge_type,
  stream_idle_timeout_seconds,
  CASE WHEN COALESCE(api_key_plaintext, '') = '' THEN 0 ELSE 1 END AS api_key_configured
FROM providers
WHERE cli_key = ?1
ORDER BY sort_order ASC, id DESC
"#,
        )
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| db_err!("failed to list providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read provider row: {e}"))?);
    }

    Ok(items)
}

/// Map a database row to `ProviderForGateway`. Both gateway query functions share
/// the same column set, so this single mapper eliminates duplication.
fn map_gateway_provider_row(
    row: &rusqlite::Row<'_>,
    cli_key: &str,
) -> Result<ProviderForGateway, rusqlite::Error> {
    let decoded = decode_provider_row(row, cli_key)?;

    Ok(ProviderForGateway {
        id: decoded.id,
        name: decoded.name,
        base_urls: decoded.base_urls,
        base_url_mode: decoded.base_url_mode,
        api_key_plaintext: row.get("api_key_plaintext")?,
        claude_models: decoded.claude_models,
        limit_5h_usd: decoded.limit_5h_usd,
        limit_daily_usd: decoded.limit_daily_usd,
        daily_reset_mode: decoded.daily_reset_mode,
        daily_reset_time: decoded.daily_reset_time,
        limit_weekly_usd: decoded.limit_weekly_usd,
        limit_monthly_usd: decoded.limit_monthly_usd,
        limit_total_usd: decoded.limit_total_usd,
        auth_mode: decoded.auth_mode,
        oauth_provider_type: decoded.oauth_provider_type,
        source_provider_id: decoded.source_provider_id,
        bridge_type: decoded.bridge_type,
        stream_idle_timeout_seconds: parse_positive_optional_u32(
            row.get("stream_idle_timeout_seconds").unwrap_or(None),
        ),
    })
}

fn list_enabled_for_gateway_in_sort_mode(
    conn: &Connection,
    cli_key: &str,
    mode_id: i64,
) -> crate::shared::error::AppResult<Vec<ProviderForGateway>> {
    let cli_key_owned = cli_key.to_string();
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  p.id,
  p.name,
  p.base_url,
  p.base_urls_json,
  p.base_url_mode,
  p.api_key_plaintext,
  p.claude_models_json,
  p.limit_5h_usd,
  p.limit_daily_usd,
  p.daily_reset_mode,
  p.daily_reset_time,
  p.limit_weekly_usd,
  p.limit_monthly_usd,
  p.limit_total_usd,
  p.auth_mode,
  p.oauth_provider_type,
  p.source_provider_id,
  p.bridge_type,
  p.stream_idle_timeout_seconds
FROM sort_mode_providers mp
JOIN providers p ON p.id = mp.provider_id
WHERE mp.mode_id = ?1
  AND mp.cli_key = ?2
  AND p.cli_key = ?2
  AND mp.enabled = 1
ORDER BY mp.sort_order ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare gateway sort_mode query: {e}"))?;

    let rows = stmt
        .query_map(params![mode_id, cli_key], |row| {
            map_gateway_provider_row(row, &cli_key_owned)
        })
        .map_err(|e| db_err!("failed to list gateway sort_mode providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read gateway provider row: {e}"))?);
    }
    Ok(items)
}

fn list_enabled_for_gateway_default(
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<ProviderForGateway>> {
    let cli_key_owned = cli_key.to_string();
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  claude_models_json,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  auth_mode,
  oauth_provider_type,
  source_provider_id,
  bridge_type,
  stream_idle_timeout_seconds
FROM providers
WHERE cli_key = ?1
  AND enabled = 1
ORDER BY sort_order ASC, id DESC
"#,
        )
        .map_err(|e| db_err!("failed to prepare gateway provider query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], |row| {
            map_gateway_provider_row(row, &cli_key_owned)
        })
        .map_err(|e| db_err!("failed to list gateway providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read gateway provider row: {e}"))?);
    }
    Ok(items)
}

pub(crate) fn list_enabled_for_gateway_using_active_mode(
    db: &db::Db,
    cli_key: &str,
) -> crate::shared::error::AppResult<GatewayProvidersSelection> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let active_mode_id: Option<i64> = conn
        .query_row(
            "SELECT mode_id FROM sort_mode_active WHERE cli_key = ?1",
            params![cli_key],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query sort_mode_active: {e}"))?
        .flatten();

    if let Some(mode_id) = active_mode_id {
        let providers = list_enabled_for_gateway_in_sort_mode(&conn, cli_key, mode_id)?;
        return Ok(GatewayProvidersSelection {
            sort_mode_id: Some(mode_id),
            providers,
        });
    }

    let providers = list_enabled_for_gateway_default(&conn, cli_key)?;
    Ok(GatewayProvidersSelection {
        sort_mode_id: None,
        providers,
    })
}

pub(crate) fn list_enabled_for_gateway_in_mode(
    db: &db::Db,
    cli_key: &str,
    sort_mode_id: Option<i64>,
) -> crate::shared::error::AppResult<Vec<ProviderForGateway>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    match sort_mode_id {
        Some(mode_id) => Ok(list_enabled_for_gateway_in_sort_mode(
            &conn, cli_key, mode_id,
        )?),
        None => Ok(list_enabled_for_gateway_default(&conn, cli_key)?),
    }
}

/// Resolve a source provider by ID for CX2CC chaining.
pub(crate) fn get_source_provider_for_gateway(
    db: &db::Db,
    source_provider_id: i64,
) -> crate::shared::error::AppResult<(ProviderForGateway, String)> {
    let conn = db.open_connection()?;
    let cli_key_owned = conn
        .query_row(
            "SELECT cli_key FROM providers WHERE id = ?1",
            params![source_provider_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query source provider cli_key: {e}"))?
        .ok_or_else(|| {
            crate::shared::error::AppError::from("DB_NOT_FOUND: source provider not found")
        })?;

    let provider = conn
        .query_row(
            r#"
SELECT
  id,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  claude_models_json,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  auth_mode,
  oauth_provider_type,
  source_provider_id,
  bridge_type,
  stream_idle_timeout_seconds
FROM providers
WHERE id = ?1 AND enabled = 1 AND source_provider_id IS NULL AND cli_key = 'codex'
"#,
            params![source_provider_id],
            |row| map_gateway_provider_row(row, &cli_key_owned),
        )
        .optional()
        .map_err(|e| db_err!("failed to query source provider: {e}"))?
        .ok_or_else(|| {
            crate::shared::error::AppError::from("DB_NOT_FOUND: source provider not found")
        })?;
    Ok((provider, cli_key_owned))
}

fn next_sort_order(conn: &Connection, cli_key: &str) -> crate::shared::error::AppResult<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM providers WHERE cli_key = ?1",
        params![cli_key],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| db_err!("failed to query next sort_order: {e}"))
}

pub fn upsert(
    db: &db::Db,
    input: ProviderUpsertParams,
) -> crate::shared::error::AppResult<ProviderSummary> {
    let ProviderUpsertParams {
        provider_id,
        cli_key,
        name,
        base_urls,
        base_url_mode,
        auth_mode,
        api_key,
        enabled,
        cost_multiplier,
        priority,
        claude_models,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        tags,
        note,
        source_provider_id,
        bridge_type,
        stream_idle_timeout_seconds,
    } = input;
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: provider name is required"
            .to_string()
            .into());
    }

    let requested_auth_mode = auth_mode.unwrap_or(ProviderAuthMode::ApiKey);
    let is_oauth = requested_auth_mode == ProviderAuthMode::Oauth;

    if let Some(ref bt) = bridge_type {
        if bt != CX2CC_BRIDGE_TYPE {
            return Err(format!("SEC_INVALID_INPUT: unsupported bridge_type: {bt}").into());
        }
    }

    let is_cx2cc = is_cx2cc_bridge(source_provider_id, bridge_type.as_deref());
    let bridge_type = if is_cx2cc {
        Some(CX2CC_BRIDGE_TYPE.to_string())
    } else {
        None
    };

    // Validate source_provider_id constraints for CX2CC bridging.
    if let Some(source_id) = source_provider_id {
        if let Some(pid) = provider_id {
            if pid == source_id {
                return Err(
                    "SEC_INVALID_INPUT: source_provider_id cannot reference itself"
                        .to_string()
                        .into(),
                );
            }
        }
        let source_conn = db.open_connection()?;
        let source_row: Option<(String, i64, Option<i64>)> = source_conn
            .query_row(
                "SELECT cli_key, enabled, source_provider_id FROM providers WHERE id = ?1",
                params![source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| db_err!("failed to validate source provider: {e}"))?;

        match source_row {
            None => {
                return Err(
                    "SEC_INVALID_INPUT: source_provider_id references a non-existent provider"
                        .to_string()
                        .into(),
                );
            }
            Some((ref src_cli, enabled, nested_source)) => {
                if src_cli != "codex" {
                    return Err(
                        "SEC_INVALID_INPUT: source provider must belong to codex CLI"
                            .to_string()
                            .into(),
                    );
                }
                if enabled == 0 {
                    return Err("SEC_INVALID_INPUT: source provider must be enabled"
                        .to_string()
                        .into());
                }
                if nested_source.is_some() {
                    return Err(
                        "SEC_INVALID_INPUT: source provider cannot itself be a bridge provider"
                            .to_string()
                            .into(),
                    );
                }
            }
        }
    }

    if is_cx2cc && cli_key != "claude" {
        return Err(
            "SEC_INVALID_INPUT: cx2cc bridge is only supported for claude"
                .to_string()
                .into(),
        );
    }

    let base_urls = if is_oauth || is_cx2cc {
        // OAuth and CX2CC providers don't need base URLs — inherited from source/adapter.
        let filtered: Vec<String> = base_urls
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        filtered
    } else {
        normalize_base_urls(base_urls)?
    };
    let base_url_primary = base_urls.first().cloned().unwrap_or_default();

    let base_urls_json =
        serde_json::to_string(&base_urls).map_err(|e| format!("SYSTEM_ERROR: {e}"))?;
    let stream_idle_timeout_seconds_specified = stream_idle_timeout_seconds.is_some();
    let stream_idle_timeout_seconds =
        normalize_stream_idle_timeout_seconds(stream_idle_timeout_seconds)?;

    let api_key = api_key.as_deref().map(str::trim).filter(|v| !v.is_empty());

    if !cost_multiplier.is_finite() || !(0.0..=1000.0).contains(&cost_multiplier) {
        return Err(
            "SEC_INVALID_INPUT: cost_multiplier must be within [0, 1000]"
                .to_string()
                .into(),
        );
    }

    if let Some(priority) = priority {
        if !(0..=1000).contains(&priority) {
            return Err("SEC_INVALID_INPUT: priority must be within [0, 1000]"
                .to_string()
                .into());
        }
    }

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();

    match provider_id {
        None => {
            let priority = priority.unwrap_or(DEFAULT_PRIORITY);
            let api_key = match is_oauth || is_cx2cc {
                true => api_key.unwrap_or(""),
                false => {
                    api_key.ok_or_else(|| "SEC_INVALID_INPUT: api_key is required".to_string())?
                }
            };
            let sort_order = next_sort_order(&conn, cli_key)?;

            let claude_models = if cli_key == "claude" {
                claude_models.unwrap_or_default().normalized()
            } else {
                ClaudeModels::default()
            };
            let claude_models_json =
                serde_json::to_string(&claude_models).map_err(|e| format!("SYSTEM_ERROR: {e}"))?;

            let limit_5h_usd = validate_limit_usd("limit_5h_usd", limit_5h_usd)?;
            let limit_daily_usd = validate_limit_usd("limit_daily_usd", limit_daily_usd)?;
            let limit_weekly_usd = validate_limit_usd("limit_weekly_usd", limit_weekly_usd)?;
            let limit_monthly_usd = validate_limit_usd("limit_monthly_usd", limit_monthly_usd)?;
            let limit_total_usd = validate_limit_usd("limit_total_usd", limit_total_usd)?;

            let daily_reset_mode = daily_reset_mode.unwrap_or(DailyResetMode::Fixed);
            let daily_reset_time_raw = daily_reset_time.as_deref().unwrap_or("00:00:00");
            let daily_reset_time =
                normalize_reset_time_hms_strict("daily_reset_time", daily_reset_time_raw)?;

            let tags_normalized = normalize_tags(tags.unwrap_or_default());
            let tags_json_value = serde_json::to_string(&tags_normalized)
                .map_err(|e| format!("SYSTEM_ERROR: {e}"))?;
            let note_value = note.as_deref().unwrap_or("").trim().to_string();
            if note_value.len() > 500 {
                return Err("SEC_INVALID_INPUT: note must be at most 500 characters"
                    .to_string()
                    .into());
            }

            conn.execute(
                r#"
INSERT INTO providers(
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  auth_mode,
  claude_models_json,
  supported_models_json,
  model_mapping_json,
  api_key_plaintext,
  sort_order,
  enabled,
  priority,
  cost_multiplier,
  limit_5h_usd,
  limit_daily_usd,
  daily_reset_mode,
  daily_reset_time,
  limit_weekly_usd,
  limit_monthly_usd,
  limit_total_usd,
  tags_json,
  note,
  source_provider_id,
  bridge_type,
  stream_idle_timeout_seconds,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}', '{}', ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
"#,
                params![
                    cli_key,
                    name,
                    base_url_primary,
                    base_urls_json,
                    base_url_mode.as_str(),
                    requested_auth_mode.as_str(),
                    claude_models_json,
                    api_key,
                    sort_order,
                    enabled_to_int(enabled),
                    priority,
                    cost_multiplier,
                    limit_5h_usd,
                    limit_daily_usd,
                    daily_reset_mode.as_str(),
                    daily_reset_time,
                    limit_weekly_usd,
                    limit_monthly_usd,
                    limit_total_usd,
                    tags_json_value,
                    note_value,
                    source_provider_id,
                    bridge_type,
                    stream_idle_timeout_seconds,
                    now,
                    now
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    crate::shared::error::AppError::new("DB_CONSTRAINT", format!(
                        "provider already exists for cli_key={cli_key}, name={name}"
                    ))
                }
                other => db_err!("failed to insert provider: {other}"),
            })?;

            let id = conn.last_insert_rowid();
            Ok(get_by_id(&conn, id)?)
        }
        Some(id) => {
            let tx = conn
                .transaction()
                .map_err(|e| db_err!("failed to start transaction: {e}"))?;

            type ExistingProviderRow = (
                String,
                String,
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<i64>,
            );
            let existing: Option<ExistingProviderRow> = tx
                .query_row(
                    "SELECT cli_key, api_key_plaintext, priority, claude_models_json, auth_mode, daily_reset_mode, daily_reset_time, tags_json, note, stream_idle_timeout_seconds FROM providers WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
                )
                .optional()
                .map_err(|e| db_err!("failed to query provider: {e}"))?;

            let Some((
                existing_cli_key,
                existing_api_key,
                existing_priority,
                existing_claude_models_json,
                existing_auth_mode_raw,
                existing_daily_reset_mode_raw,
                existing_daily_reset_time_raw,
                existing_tags_json,
                existing_note,
                existing_stream_idle_timeout_seconds,
            )) = existing
            else {
                return Err("DB_NOT_FOUND: provider not found".to_string().into());
            };

            if existing_cli_key != cli_key {
                return Err("SEC_INVALID_INPUT: cli_key mismatch".to_string().into());
            }

            // Resolve auth_mode: use requested if provided, else keep existing.
            let next_auth_mode = auth_mode
                .map(ProviderAuthMode::as_str)
                .unwrap_or(existing_auth_mode_raw.as_str());
            let next_is_oauth = next_auth_mode == ProviderAuthMode::Oauth.as_str();

            let next_api_key = api_key.unwrap_or(existing_api_key.as_str());
            if !next_is_oauth && !is_cx2cc && next_api_key.trim().is_empty() {
                return Err("SEC_INVALID_INPUT: api_key is required".to_string().into());
            }
            let next_priority = priority.unwrap_or(existing_priority);

            let existing_claude_models = if cli_key == "claude" {
                claude_models_from_json(&existing_claude_models_json)
            } else {
                ClaudeModels::default()
            };

            let next_claude_models = match claude_models {
                Some(v) if cli_key == "claude" => Some(v.normalized()),
                _ => None,
            };

            let final_claude_models = next_claude_models
                .as_ref()
                .unwrap_or(&existing_claude_models);
            let next_claude_models_json = if cli_key == "claude" {
                serde_json::to_string(final_claude_models)
                    .map_err(|e| format!("SYSTEM_ERROR: {e}"))?
            } else {
                "{}".to_string()
            };

            let next_limit_5h_usd = validate_limit_usd("limit_5h_usd", limit_5h_usd)?;
            let next_limit_daily_usd = validate_limit_usd("limit_daily_usd", limit_daily_usd)?;
            let next_limit_weekly_usd = validate_limit_usd("limit_weekly_usd", limit_weekly_usd)?;
            let next_limit_monthly_usd =
                validate_limit_usd("limit_monthly_usd", limit_monthly_usd)?;
            let next_limit_total_usd = validate_limit_usd("limit_total_usd", limit_total_usd)?;

            let existing_daily_reset_mode = DailyResetMode::parse(&existing_daily_reset_mode_raw)
                .unwrap_or(DailyResetMode::Fixed);
            let existing_daily_reset_time =
                normalize_reset_time_hms_lossy(&existing_daily_reset_time_raw);

            let next_daily_reset_mode = daily_reset_mode.unwrap_or(existing_daily_reset_mode);

            let next_daily_reset_time = match daily_reset_time.as_deref() {
                None => existing_daily_reset_time,
                Some(v) => normalize_reset_time_hms_strict("daily_reset_time", v)?,
            };

            let next_tags = match tags {
                Some(t) => normalize_tags(t),
                None => tags_from_json(&existing_tags_json),
            };
            let next_tags_json =
                serde_json::to_string(&next_tags).map_err(|e| format!("SYSTEM_ERROR: {e}"))?;

            let next_note = match note {
                Some(v) => {
                    let trimmed = v.trim().to_string();
                    if trimmed.len() > 500 {
                        return Err("SEC_INVALID_INPUT: note must be at most 500 characters"
                            .to_string()
                            .into());
                    }
                    trimmed
                }
                None => existing_note,
            };
            let next_stream_idle_timeout_seconds = if stream_idle_timeout_seconds_specified {
                stream_idle_timeout_seconds
            } else {
                parse_positive_optional_u32(existing_stream_idle_timeout_seconds)
            };

            tx.execute(
                r#"
UPDATE providers
SET
  name = ?1,
  base_url = ?2,
  base_urls_json = ?3,
  base_url_mode = ?4,
  auth_mode = ?5,
  claude_models_json = ?6,
  supported_models_json = '{}',
  model_mapping_json = '{}',
  api_key_plaintext = ?7,
  enabled = ?8,
  cost_multiplier = ?9,
  priority = ?10,
  limit_5h_usd = ?11,
  limit_daily_usd = ?12,
  daily_reset_mode = ?13,
  daily_reset_time = ?14,
  limit_weekly_usd = ?15,
  limit_monthly_usd = ?16,
  limit_total_usd = ?17,
  tags_json = ?18,
  note = ?19,
  source_provider_id = ?20,
  bridge_type = ?21,
  stream_idle_timeout_seconds = ?22,
  updated_at = ?23
WHERE id = ?24
"#,
                params![
                    name,
                    base_url_primary,
                    base_urls_json,
                    base_url_mode.as_str(),
                    next_auth_mode,
                    next_claude_models_json,
                    next_api_key,
                    enabled_to_int(enabled),
                    cost_multiplier,
                    next_priority,
                    next_limit_5h_usd,
                    next_limit_daily_usd,
                    next_daily_reset_mode.as_str(),
                    next_daily_reset_time,
                    next_limit_weekly_usd,
                    next_limit_monthly_usd,
                    next_limit_total_usd,
                    next_tags_json,
                    next_note,
                    source_provider_id,
                    bridge_type,
                    next_stream_idle_timeout_seconds,
                    now,
                    id
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    crate::shared::error::AppError::new(
                        "DB_CONSTRAINT",
                        format!("provider name already exists for cli_key={cli_key}, name={name}"),
                    )
                }
                other => db_err!("failed to update provider: {other}"),
            })?;

            tx.commit().map_err(|e| db_err!("failed to commit: {e}"))?;

            get_by_id(&conn, id)
        }
    }
}

pub fn set_enabled(
    db: &db::Db,
    provider_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<ProviderSummary> {
    let conn = db.open_connection()?;
    let now = now_unix_seconds();
    let changed = conn
        .execute(
            "UPDATE providers SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled_to_int(enabled), now, provider_id],
        )
        .map_err(|e| db_err!("failed to update provider: {e}"))?;

    if changed == 0 {
        return Err("DB_NOT_FOUND: provider not found".to_string().into());
    }

    get_by_id(&conn, provider_id)
}

pub fn delete(db: &db::Db, provider_id: i64) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let changed = conn
        .execute("DELETE FROM providers WHERE id = ?1", params![provider_id])
        .map_err(|e| db_err!("failed to delete provider: {e}"))?;

    if changed == 0 {
        return Err("DB_NOT_FOUND: provider not found".to_string().into());
    }

    Ok(())
}

pub fn reorder(
    db: &db::Db,
    cli_key: &str,
    ordered_provider_ids: Vec<i64>,
) -> crate::shared::error::AppResult<Vec<ProviderSummary>> {
    validate_cli_key(cli_key)?;

    let mut seen = HashSet::new();
    for id in &ordered_provider_ids {
        if !seen.insert(*id) {
            return Err(format!("SEC_INVALID_INPUT: duplicate provider_id={id}").into());
        }
    }

    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let mut existing_ids = Vec::new();
    {
        let mut stmt = tx
            .prepare_cached(
                "SELECT id FROM providers WHERE cli_key = ?1 ORDER BY sort_order ASC, id DESC",
            )
            .map_err(|e| db_err!("failed to prepare existing id list: {e}"))?;
        let rows = stmt
            .query_map(params![cli_key], |row| row.get::<_, i64>(0))
            .map_err(|e| db_err!("failed to query existing id list: {e}"))?;
        for row in rows {
            existing_ids.push(row.map_err(|e| db_err!("failed to read existing id: {e}"))?);
        }
    }

    let existing_set: HashSet<i64> = existing_ids.iter().copied().collect();
    for id in &ordered_provider_ids {
        if !existing_set.contains(id) {
            return Err(format!(
                "SEC_INVALID_INPUT: provider_id does not belong to cli_key={cli_key}: {id}"
            )
            .into());
        }
    }

    let mut final_ids = Vec::with_capacity(existing_ids.len());
    final_ids.extend(ordered_provider_ids);
    for id in existing_ids {
        if !seen.contains(&id) {
            final_ids.push(id);
        }
    }

    let now = now_unix_seconds();
    for (idx, id) in final_ids.iter().enumerate() {
        let sort_order = idx as i64;
        tx.execute(
            "UPDATE providers SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![sort_order, now, id],
        )
        .map_err(|e| db_err!("failed to update sort_order for provider {id}: {e}"))?;
    }

    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;

    list_by_cli(db, cli_key)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_oauth_tokens(
    db: &crate::db::Db,
    provider_id: i64,
    auth_mode: &str,
    oauth_provider_type: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    id_token: Option<&str>,
    token_uri: &str,
    client_id: &str,
    client_secret: Option<&str>,
    expires_at: Option<i64>,
    email: Option<&str>,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    conn.execute(
        r#"
UPDATE providers SET
  auth_mode = ?1,
  oauth_provider_type = ?2,
  oauth_access_token = ?3,
  oauth_refresh_token = ?4,
  oauth_id_token = ?5,
  oauth_token_uri = ?6,
  oauth_client_id = ?7,
  oauth_client_secret = ?8,
  oauth_expires_at = ?9,
  oauth_email = ?10,
  oauth_last_refreshed_at = ?11,
  oauth_last_error = NULL,
  updated_at = ?11
WHERE id = ?12
"#,
        rusqlite::params![
            auth_mode,
            oauth_provider_type,
            access_token,
            refresh_token,
            id_token,
            token_uri,
            client_id,
            client_secret,
            expires_at,
            email,
            now,
            provider_id,
        ],
    )
    .map_err(|e| crate::shared::error::db_err!("failed to update OAuth tokens: {e}"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_oauth_tokens_if_last_refreshed_matches(
    db: &crate::db::Db,
    provider_id: i64,
    auth_mode: &str,
    oauth_provider_type: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    id_token: Option<&str>,
    token_uri: &str,
    client_id: &str,
    client_secret: Option<&str>,
    expires_at: Option<i64>,
    email: Option<&str>,
    expected_last_refreshed_at: Option<i64>,
) -> crate::shared::error::AppResult<bool> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    // Ensure CAS token advances even if two updates happen in the same second.
    let refreshed_at = match expected_last_refreshed_at {
        Some(expected) if expected >= now => expected.saturating_add(1),
        _ => now,
    };

    let rows = conn
        .execute(
            r#"
UPDATE providers SET
  auth_mode = ?1,
  oauth_provider_type = ?2,
  oauth_access_token = ?3,
  oauth_refresh_token = ?4,
  oauth_id_token = ?5,
  oauth_token_uri = ?6,
  oauth_client_id = ?7,
  oauth_client_secret = ?8,
  oauth_expires_at = ?9,
  oauth_email = ?10,
  oauth_last_refreshed_at = ?11,
  oauth_last_error = NULL,
  updated_at = ?11
WHERE id = ?12
  AND auth_mode = 'oauth'
  AND (
    (?13 IS NULL AND oauth_last_refreshed_at IS NULL)
    OR oauth_last_refreshed_at = ?13
  )
"#,
            rusqlite::params![
                auth_mode,
                oauth_provider_type,
                access_token,
                refresh_token,
                id_token,
                token_uri,
                client_id,
                client_secret,
                expires_at,
                email,
                refreshed_at,
                provider_id,
                expected_last_refreshed_at,
            ],
        )
        .map_err(|e| crate::shared::error::db_err!("failed to CAS-update OAuth tokens: {e}"))?;
    Ok(rows == 1)
}

pub(crate) fn clear_oauth(
    db: &crate::db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    conn.execute(
        r#"
UPDATE providers SET
  auth_mode = 'api_key',
  oauth_provider_type = NULL,
  oauth_access_token = NULL,
  oauth_refresh_token = NULL,
  oauth_id_token = NULL,
  oauth_token_uri = NULL,
  oauth_client_id = NULL,
  oauth_client_secret = NULL,
  oauth_expires_at = NULL,
  oauth_email = NULL,
  oauth_last_refreshed_at = NULL,
  oauth_last_error = NULL,
  updated_at = ?1
WHERE id = ?2
"#,
        rusqlite::params![now, provider_id],
    )
    .map_err(|e| crate::shared::error::db_err!("failed to clear OAuth: {e}"))?;
    Ok(())
}

fn map_oauth_details_row(row: &rusqlite::Row) -> rusqlite::Result<ProviderOAuthDetails> {
    Ok(ProviderOAuthDetails {
        id: row.get(0)?,
        cli_key: row.get(1)?,
        oauth_provider_type: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        oauth_access_token: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        oauth_refresh_token: row.get(4)?,
        oauth_id_token: row.get(5)?,
        oauth_token_uri: row.get(6)?,
        oauth_client_id: row.get(7)?,
        oauth_client_secret: row.get(8)?,
        oauth_expires_at: row.get(9)?,
        oauth_email: row.get(10)?,
        oauth_refresh_lead_s: row.get(11)?,
        oauth_last_refreshed_at: row.get(12)?,
    })
}

pub(crate) fn get_oauth_details(
    db: &crate::db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<ProviderOAuthDetails> {
    let conn = db.open_connection()?;
    conn.query_row(
        r#"
SELECT id, cli_key, oauth_provider_type, oauth_access_token, oauth_refresh_token,
       oauth_id_token, oauth_token_uri, oauth_client_id, oauth_client_secret,
       oauth_expires_at, oauth_email, oauth_refresh_lead_s, oauth_last_refreshed_at
FROM providers WHERE id = ?1 AND auth_mode = 'oauth'
"#,
        rusqlite::params![provider_id],
        map_oauth_details_row,
    )
    .optional()
    .map_err(|e| crate::shared::error::db_err!("failed to query OAuth details: {e}"))?
    .ok_or_else(|| {
        crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found or not OAuth")
    })
}

/// Lists all OAuth providers whose tokens are approaching or past expiry.
///
/// Returns providers where `auth_mode = 'oauth'` AND the refresh token is present
/// AND `oauth_expires_at` is within `oauth_refresh_lead_s` seconds from now (or already expired).
/// Also includes providers with `oauth_expires_at IS NULL` that have refresh tokens
/// (they might need a proactive refresh to populate expiry).
pub(crate) fn list_oauth_providers_needing_refresh(
    db: &crate::db::Db,
) -> crate::shared::error::AppResult<Vec<ProviderOAuthDetails>> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    let mut stmt = conn
        .prepare(
            r#"
SELECT id, cli_key, oauth_provider_type, oauth_access_token, oauth_refresh_token,
       oauth_id_token, oauth_token_uri, oauth_client_id, oauth_client_secret,
       oauth_expires_at, oauth_email, oauth_refresh_lead_s, oauth_last_refreshed_at
FROM providers
WHERE auth_mode = 'oauth'
  AND oauth_refresh_token IS NOT NULL
  AND oauth_refresh_token != ''
  AND enabled = 1
  AND (
    oauth_expires_at IS NULL
    OR oauth_expires_at <= (?1 + oauth_refresh_lead_s)
  )
"#,
        )
        .map_err(|e| db_err!("prepare list_oauth_providers_needing_refresh: {e}"))?;

    let rows = stmt
        .query_map(rusqlite::params![now], map_oauth_details_row)
        .map_err(|e| db_err!("query list_oauth_providers_needing_refresh: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("read oauth provider row: {e}"))?);
    }
    Ok(out)
}

/// Update the `oauth_last_error` column for a provider (e.g. when background refresh fails).
pub(crate) fn set_oauth_last_error(
    db: &crate::db::Db,
    provider_id: i64,
    error_msg: &str,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    conn.execute(
        "UPDATE providers SET oauth_last_error = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![error_msg, now, provider_id],
    )
    .map_err(|e| db_err!("failed to set oauth_last_error: {e}"))?;
    Ok(())
}
