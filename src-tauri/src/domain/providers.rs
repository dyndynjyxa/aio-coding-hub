//! Usage: Provider configuration persistence and gateway selection helpers.

use crate::db;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_PRIORITY: i64 = 100;
const MAX_MODEL_NAME_LEN: usize = 200;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeModels {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,
}

fn normalize_model_slot(raw: Option<String>) -> Option<String> {
    let value = raw.map(|v| v.trim().to_string());
    let value = value.as_deref().unwrap_or("");
    if value.is_empty() {
        return None;
    }
    if value.len() > MAX_MODEL_NAME_LEN {
        return Some(value[..MAX_MODEL_NAME_LEN].to_string());
    }
    Some(value.to_string())
}

impl ClaudeModels {
    fn normalized(self) -> Self {
        Self {
            main_model: normalize_model_slot(self.main_model),
            reasoning_model: normalize_model_slot(self.reasoning_model),
            haiku_model: normalize_model_slot(self.haiku_model),
            sonnet_model: normalize_model_slot(self.sonnet_model),
            opus_model: normalize_model_slot(self.opus_model),
        }
    }

    pub(crate) fn has_any(&self) -> bool {
        self.main_model.is_some()
            || self.reasoning_model.is_some()
            || self.haiku_model.is_some()
            || self.sonnet_model.is_some()
            || self.opus_model.is_some()
    }

    pub(crate) fn map_model(&self, original_model: &str, has_thinking: bool) -> String {
        let model_lower = original_model.to_ascii_lowercase();

        // 1) thinking 模式优先使用推理模型
        if has_thinking {
            if let Some(model) = self.reasoning_model.as_deref() {
                return model.to_string();
            }
        }

        // 2) 按模型类型匹配（子串）
        if model_lower.contains("haiku") {
            if let Some(model) = self.haiku_model.as_deref() {
                return model.to_string();
            }
        }
        if model_lower.contains("opus") {
            if let Some(model) = self.opus_model.as_deref() {
                return model.to_string();
            }
        }
        if model_lower.contains("sonnet") {
            if let Some(model) = self.sonnet_model.as_deref() {
                return model.to_string();
            }
        }

        // 3) 主模型兜底
        if let Some(model) = self.main_model.as_deref() {
            return model.to_string();
        }

        // 4) 无映射：保持原样
        original_model.to_string()
    }
}

fn claude_models_from_json(raw: &str) -> ClaudeModels {
    serde_json::from_str::<ClaudeModels>(raw)
        .ok()
        .unwrap_or_default()
        .normalized()
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderBaseUrlMode {
    Order,
    Ping,
}

impl ProviderBaseUrlMode {
    fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "order" => Some(Self::Order),
            "ping" => Some(Self::Ping),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Order => "order",
            Self::Ping => "ping",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub id: i64,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub claude_models: ClaudeModels,
    pub enabled: bool,
    pub priority: i64,
    pub cost_multiplier: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderForGateway {
    pub id: i64,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: ProviderBaseUrlMode,
    pub api_key_plaintext: String,
    pub claude_models: ClaudeModels,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayProvidersSelection {
    pub sort_mode_id: Option<i64>,
    pub providers: Vec<ProviderForGateway>,
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn enabled_to_int(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

fn normalize_base_urls(base_urls: Vec<String>) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::with_capacity(base_urls.len().max(1));
    let mut seen: HashSet<String> = HashSet::with_capacity(base_urls.len());

    for raw in base_urls {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !seen.insert(trimmed.to_string()) {
            continue;
        }

        // Validate URL early to avoid runtime proxy errors.
        reqwest::Url::parse(trimmed)
            .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url={trimmed}: {e}"))?;

        out.push(trimmed.to_string());
    }

    if out.is_empty() {
        return Err("SEC_INVALID_INPUT: base_urls is required".to_string());
    }

    Ok(out)
}

fn base_urls_from_row(base_url_fallback: &str, base_urls_json: &str) -> Vec<String> {
    let mut parsed: Vec<String> = serde_json::from_str::<Vec<String>>(base_urls_json)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    // De-dup while preserving order.
    let mut seen: HashSet<String> = HashSet::with_capacity(parsed.len());
    parsed.retain(|v| seen.insert(v.clone()));

    if parsed.is_empty() {
        let fallback = base_url_fallback.trim();
        if fallback.is_empty() {
            return vec![String::new()];
        }
        return vec![fallback.to_string()];
    }

    parsed
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<ProviderSummary, rusqlite::Error> {
    let cli_key: String = row.get("cli_key")?;
    let base_url_fallback: String = row.get("base_url")?;
    let base_urls_json: String = row.get("base_urls_json")?;
    let claude_models_json: String = row.get("claude_models_json")?;
    let base_url_mode_raw: String = row.get("base_url_mode")?;
    let base_url_mode =
        ProviderBaseUrlMode::parse(&base_url_mode_raw).unwrap_or(ProviderBaseUrlMode::Order);

    Ok(ProviderSummary {
        id: row.get("id")?,
        cli_key: cli_key.clone(),
        name: row.get("name")?,
        base_urls: base_urls_from_row(&base_url_fallback, &base_urls_json),
        base_url_mode,
        claude_models: if cli_key == "claude" {
            claude_models_from_json(&claude_models_json)
        } else {
            ClaudeModels::default()
        },
        enabled: row.get::<_, i64>("enabled")? != 0,
        priority: row.get("priority")?,
        cost_multiplier: row.get("cost_multiplier")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

impl ProviderForGateway {
    pub(crate) fn get_effective_claude_model(
        &self,
        requested_model: &str,
        has_thinking: bool,
    ) -> String {
        self.claude_models.map_model(requested_model, has_thinking)
    }
}

fn get_by_id(conn: &Connection, provider_id: i64) -> Result<ProviderSummary, String> {
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
  enabled,
  priority,
  cost_multiplier,
  created_at,
  updated_at
FROM providers
WHERE id = ?1
"#,
        params![provider_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query provider: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: provider not found".to_string())
}

pub fn names_by_id(
    app: &tauri::AppHandle,
    provider_ids: &[i64],
) -> Result<HashMap<i64, String>, String> {
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

    let conn = db::open_connection(app)?;

    let placeholders = db::sql_placeholders(ids.len());
    let sql = format!("SELECT id, name FROM providers WHERE id IN ({placeholders})");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let mut rows = stmt
        .query(params_from_iter(ids.iter()))
        .map_err(|e| format!("DB_ERROR: failed to query provider names: {e}"))?;

    let mut out: HashMap<i64, String> = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("DB_ERROR: failed to read provider row: {e}"))?
    {
        let id: i64 = row
            .get(0)
            .map_err(|e| format!("DB_ERROR: invalid provider id: {e}"))?;
        let name: String = row
            .get(1)
            .map_err(|e| format!("DB_ERROR: invalid provider name: {e}"))?;
        out.insert(id, name);
    }

    Ok(out)
}

pub fn list_by_cli(app: &tauri::AppHandle, cli_key: &str) -> Result<Vec<ProviderSummary>, String> {
    validate_cli_key(cli_key)?;
    let conn = db::open_connection(app)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  enabled,
  priority,
  cost_multiplier,
  created_at,
  updated_at
FROM providers
WHERE cli_key = ?1
ORDER BY sort_order ASC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read provider row: {e}"))?);
    }

    Ok(items)
}

fn list_enabled_for_gateway_in_sort_mode(
    conn: &Connection,
    cli_key: &str,
    mode_id: i64,
) -> Result<Vec<ProviderForGateway>, String> {
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  p.id,
  p.name,
  p.base_url,
  p.base_urls_json,
  p.base_url_mode,
  p.api_key_plaintext,
  p.claude_models_json
FROM sort_mode_providers mp
JOIN providers p ON p.id = mp.provider_id
WHERE mp.mode_id = ?1
  AND mp.cli_key = ?2
  AND p.cli_key = ?2
  AND p.enabled = 1
ORDER BY mp.sort_order ASC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare gateway sort_mode query: {e}"))?;

    let rows = stmt
        .query_map(params![mode_id, cli_key], |row| {
            let base_url_fallback: String = row.get("base_url")?;
            let base_urls_json: String = row.get("base_urls_json")?;
            let base_url_mode_raw: String = row.get("base_url_mode")?;
            let claude_models_json: String = row.get("claude_models_json")?;
            let base_url_mode = ProviderBaseUrlMode::parse(&base_url_mode_raw)
                .unwrap_or(ProviderBaseUrlMode::Order);
            Ok(ProviderForGateway {
                id: row.get("id")?,
                name: row.get("name")?,
                base_urls: base_urls_from_row(&base_url_fallback, &base_urls_json),
                base_url_mode,
                api_key_plaintext: row.get("api_key_plaintext")?,
                claude_models: if cli_key == "claude" {
                    claude_models_from_json(&claude_models_json)
                } else {
                    ClaudeModels::default()
                },
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to list gateway sort_mode providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read gateway provider row: {e}"))?);
    }
    Ok(items)
}

fn list_enabled_for_gateway_default(
    conn: &Connection,
    cli_key: &str,
) -> Result<Vec<ProviderForGateway>, String> {
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  claude_models_json
FROM providers
WHERE cli_key = ?1
  AND enabled = 1
ORDER BY sort_order ASC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare gateway provider query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], |row| {
            let base_url_fallback: String = row.get("base_url")?;
            let base_urls_json: String = row.get("base_urls_json")?;
            let base_url_mode_raw: String = row.get("base_url_mode")?;
            let claude_models_json: String = row.get("claude_models_json")?;
            let base_url_mode = ProviderBaseUrlMode::parse(&base_url_mode_raw)
                .unwrap_or(ProviderBaseUrlMode::Order);
            Ok(ProviderForGateway {
                id: row.get("id")?,
                name: row.get("name")?,
                base_urls: base_urls_from_row(&base_url_fallback, &base_urls_json),
                base_url_mode,
                api_key_plaintext: row.get("api_key_plaintext")?,
                claude_models: if cli_key == "claude" {
                    claude_models_from_json(&claude_models_json)
                } else {
                    ClaudeModels::default()
                },
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to list gateway providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read gateway provider row: {e}"))?);
    }
    Ok(items)
}

pub(crate) fn list_enabled_for_gateway_using_active_mode(
    app: &tauri::AppHandle,
    cli_key: &str,
) -> Result<GatewayProvidersSelection, String> {
    validate_cli_key(cli_key)?;
    let conn = db::open_connection(app)?;

    let active_mode_id: Option<i64> = conn
        .query_row(
            "SELECT mode_id FROM sort_mode_active WHERE cli_key = ?1",
            params![cli_key],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query sort_mode_active: {e}"))?
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
    app: &tauri::AppHandle,
    cli_key: &str,
    sort_mode_id: Option<i64>,
) -> Result<Vec<ProviderForGateway>, String> {
    validate_cli_key(cli_key)?;
    let conn = db::open_connection(app)?;

    match sort_mode_id {
        Some(mode_id) => list_enabled_for_gateway_in_sort_mode(&conn, cli_key, mode_id),
        None => list_enabled_for_gateway_default(&conn, cli_key),
    }
}

fn next_sort_order(conn: &Connection, cli_key: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM providers WHERE cli_key = ?1",
        params![cli_key],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| format!("DB_ERROR: failed to query next sort_order: {e}"))
}

#[allow(clippy::too_many_arguments)]
pub fn upsert(
    app: &tauri::AppHandle,
    provider_id: Option<i64>,
    cli_key: &str,
    name: &str,
    base_urls: Vec<String>,
    base_url_mode: &str,
    api_key: Option<&str>,
    enabled: bool,
    cost_multiplier: f64,
    priority: Option<i64>,
    claude_models: Option<ClaudeModels>,
) -> Result<ProviderSummary, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: provider name is required".to_string());
    }

    let base_urls = normalize_base_urls(base_urls)?;
    let base_url_primary = base_urls.first().cloned().unwrap_or_default();

    let base_url_mode = ProviderBaseUrlMode::parse(base_url_mode)
        .ok_or_else(|| "SEC_INVALID_INPUT: base_url_mode must be 'order' or 'ping'".to_string())?;
    let base_urls_json =
        serde_json::to_string(&base_urls).map_err(|e| format!("SYSTEM_ERROR: {e}"))?;

    let api_key = api_key.map(str::trim).filter(|v| !v.is_empty());

    if !cost_multiplier.is_finite() || cost_multiplier <= 0.0 || cost_multiplier > 1000.0 {
        return Err("SEC_INVALID_INPUT: cost_multiplier must be within (0, 1000]".to_string());
    }

    if let Some(priority) = priority {
        if !(0..=1000).contains(&priority) {
            return Err("SEC_INVALID_INPUT: priority must be within [0, 1000]".to_string());
        }
    }

    let mut conn = db::open_connection(app)?;
    let now = now_unix_seconds();

    match provider_id {
        None => {
            let priority = priority.unwrap_or(DEFAULT_PRIORITY);
            let api_key =
                api_key.ok_or_else(|| "SEC_INVALID_INPUT: api_key is required".to_string())?;
            let sort_order = next_sort_order(&conn, cli_key)?;

            let claude_models = if cli_key == "claude" {
                claude_models.unwrap_or_default().normalized()
            } else {
                ClaudeModels::default()
            };
            let claude_models_json =
                serde_json::to_string(&claude_models).map_err(|e| format!("SYSTEM_ERROR: {e}"))?;

            conn.execute(
                r#"
INSERT INTO providers(
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  supported_models_json,
  model_mapping_json,
  api_key_plaintext,
  sort_order,
  enabled,
  priority,
  cost_multiplier,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', '{}', ?7, ?8, ?9, ?10, ?11, ?12, ?13)
"#,
                params![
                    cli_key,
                    name,
                    base_url_primary,
                    base_urls_json,
                    base_url_mode.as_str(),
                    claude_models_json,
                    api_key,
                    sort_order,
                    enabled_to_int(enabled),
                    priority,
                    cost_multiplier,
                    now,
                    now
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    format!(
                        "DB_CONSTRAINT: provider already exists for cli_key={cli_key}, name={name}"
                    )
                }
                other => format!("DB_ERROR: failed to insert provider: {other}"),
            })?;

            let id = conn.last_insert_rowid();
            get_by_id(&conn, id)
        }
        Some(id) => {
            let tx = conn
                .transaction()
                .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

            let existing: Option<(String, String, i64, String)> = tx
                .query_row(
                    "SELECT cli_key, api_key_plaintext, priority, claude_models_json FROM providers WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|e| format!("DB_ERROR: failed to query provider: {e}"))?;

            let Some((
                existing_cli_key,
                existing_api_key,
                existing_priority,
                existing_claude_models_json,
            )) = existing
            else {
                return Err("DB_NOT_FOUND: provider not found".to_string());
            };

            if existing_cli_key != cli_key {
                return Err("SEC_INVALID_INPUT: cli_key mismatch".to_string());
            }

            let next_api_key = api_key.unwrap_or(existing_api_key.as_str());
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

            tx.execute(
                r#"
UPDATE providers
SET
  name = ?1,
  base_url = ?2,
  base_urls_json = ?3,
  base_url_mode = ?4,
  claude_models_json = ?5,
  supported_models_json = '{}',
  model_mapping_json = '{}',
  api_key_plaintext = ?6,
  enabled = ?7,
  cost_multiplier = ?8,
  priority = ?9,
  updated_at = ?10
WHERE id = ?11
"#,
                params![
                    name,
                    base_url_primary,
                    base_urls_json,
                    base_url_mode.as_str(),
                    next_claude_models_json,
                    next_api_key,
                    enabled_to_int(enabled),
                    cost_multiplier,
                    next_priority,
                    now,
                    id
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                    format!("DB_CONSTRAINT: provider name already exists for cli_key={cli_key}, name={name}")
                }
                other => format!("DB_ERROR: failed to update provider: {other}"),
            })?;

            tx.commit()
                .map_err(|e| format!("DB_ERROR: failed to commit: {e}"))?;

            get_by_id(&conn, id)
        }
    }
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    provider_id: i64,
    enabled: bool,
) -> Result<ProviderSummary, String> {
    let conn = db::open_connection(app)?;
    let now = now_unix_seconds();
    let changed = conn
        .execute(
            "UPDATE providers SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled_to_int(enabled), now, provider_id],
        )
        .map_err(|e| format!("DB_ERROR: failed to update provider: {e}"))?;

    if changed == 0 {
        return Err("DB_NOT_FOUND: provider not found".to_string());
    }

    get_by_id(&conn, provider_id)
}

pub fn delete(app: &tauri::AppHandle, provider_id: i64) -> Result<(), String> {
    let conn = db::open_connection(app)?;
    let changed = conn
        .execute("DELETE FROM providers WHERE id = ?1", params![provider_id])
        .map_err(|e| format!("DB_ERROR: failed to delete provider: {e}"))?;

    if changed == 0 {
        return Err("DB_NOT_FOUND: provider not found".to_string());
    }

    Ok(())
}

pub fn reorder(
    app: &tauri::AppHandle,
    cli_key: &str,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<ProviderSummary>, String> {
    validate_cli_key(cli_key)?;

    let mut seen = HashSet::new();
    for id in &ordered_provider_ids {
        if !seen.insert(*id) {
            return Err(format!("SEC_INVALID_INPUT: duplicate provider_id={id}"));
        }
    }

    let mut conn = db::open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let mut existing_ids = Vec::new();
    {
        let mut stmt = tx
            .prepare("SELECT id FROM providers WHERE cli_key = ?1 ORDER BY sort_order ASC, id DESC")
            .map_err(|e| format!("DB_ERROR: failed to prepare existing id list: {e}"))?;
        let rows = stmt
            .query_map(params![cli_key], |row| row.get::<_, i64>(0))
            .map_err(|e| format!("DB_ERROR: failed to query existing id list: {e}"))?;
        for row in rows {
            existing_ids
                .push(row.map_err(|e| format!("DB_ERROR: failed to read existing id: {e}"))?);
        }
    }

    let existing_set: HashSet<i64> = existing_ids.iter().copied().collect();
    for id in &ordered_provider_ids {
        if !existing_set.contains(id) {
            return Err(format!(
                "SEC_INVALID_INPUT: provider_id does not belong to cli_key={cli_key}: {id}"
            ));
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
        .map_err(|e| format!("DB_ERROR: failed to update sort_order for provider {id}: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit transaction: {e}"))?;

    list_by_cli(app, cli_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_models_no_config_keeps_original() {
        let models = ClaudeModels::default();
        assert_eq!(
            models.map_model("claude-sonnet-4", false),
            "claude-sonnet-4"
        );
    }

    #[test]
    fn claude_models_thinking_prefers_reasoning_model() {
        let models = ClaudeModels {
            main_model: Some("glm-main".to_string()),
            reasoning_model: Some("glm-thinking".to_string()),
            haiku_model: Some("glm-haiku".to_string()),
            sonnet_model: Some("glm-sonnet".to_string()),
            opus_model: Some("glm-opus".to_string()),
        }
        .normalized();

        assert_eq!(models.map_model("claude-sonnet-4", true), "glm-thinking");
    }

    #[test]
    fn claude_models_type_slot_selected_by_substring() {
        let models = ClaudeModels {
            main_model: Some("glm-main".to_string()),
            haiku_model: Some("glm-haiku".to_string()),
            sonnet_model: Some("glm-sonnet".to_string()),
            opus_model: Some("glm-opus".to_string()),
            ..Default::default()
        }
        .normalized();

        assert_eq!(models.map_model("claude-haiku-4", false), "glm-haiku");
        assert_eq!(models.map_model("claude-sonnet-4", false), "glm-sonnet");
        assert_eq!(models.map_model("claude-opus-4", false), "glm-opus");
    }

    #[test]
    fn claude_models_falls_back_to_main_model() {
        let models = ClaudeModels {
            main_model: Some("glm-main".to_string()),
            ..Default::default()
        }
        .normalized();

        assert_eq!(models.map_model("some-unknown-model", false), "glm-main");
    }
}
