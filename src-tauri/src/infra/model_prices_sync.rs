//! Usage: Sync model price data from external sources and persist into sqlite.

use crate::shared::error::db_err;
use crate::shared::fs::read_file_with_max_len;
use crate::shared::http_body::read_text_with_limit;
use crate::shared::time::now_unix_seconds;
use crate::{app_paths, blocking, db};
use reqwest::header::{HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const BASELLM_ALL_JSON_URL: &str = "https://basellm.github.io/llm-metadata/api/all.json";
const BASELLM_RESPONSE_BODY_LIMIT: usize = 16 * 1024 * 1024;
const BASELLM_CACHE_MAX_BYTES: usize = 256 * 1024;
// Bump whenever parse_basellm_all_json changes what it extracts (providers, tier
// mapping, ...). A stale version drops the cached ETag so a 304 cannot skip
// re-ingesting rows the new parser would produce from unchanged upstream content.
const BASELLM_PARSER_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelPricesSyncStatus {
    Updated,
    NotModified,
    Failed,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ModelPricesSyncReport {
    pub status: ModelPricesSyncStatus,
    pub inserted: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub total: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct BasellmCacheMeta {
    parser_version: u32,
    etag: Option<String>,
    last_modified: Option<String>,
}

#[derive(Debug, Clone)]
struct ModelPriceRow {
    cli_key: &'static str,
    vendor: String,
    model: String,
    price_json: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct UpsertCounts {
    inserted: u32,
    updated: u32,
    unchanged: u32,
}

impl UpsertCounts {
    fn total(self) -> u32 {
        self.inserted
            .saturating_add(self.updated)
            .saturating_add(self.unchanged)
    }
}

#[derive(Debug)]
enum BasellmFetch {
    NotModified,
    Rows {
        rows: Vec<ModelPriceRow>,
        cache: BasellmCacheMeta,
    },
}

fn model_prices_dir(app: &tauri::AppHandle) -> crate::shared::error::AppResult<PathBuf> {
    let dir = app_paths::app_data_dir(app)?.join("model-prices");
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create model-prices dir: {e}"))?;
    Ok(dir)
}

fn basellm_cache_path(app: &tauri::AppHandle) -> crate::shared::error::AppResult<PathBuf> {
    Ok(model_prices_dir(app)?.join("basellm-cache.json"))
}

fn read_basellm_cache(app: &tauri::AppHandle) -> BasellmCacheMeta {
    let path = match basellm_cache_path(app) {
        Ok(v) => v,
        Err(_) => return BasellmCacheMeta::default(),
    };
    if !path.exists() {
        return BasellmCacheMeta::default();
    }
    let bytes = match read_file_with_max_len(&path, BASELLM_CACHE_MAX_BYTES) {
        Ok(v) => v,
        Err(_) => return BasellmCacheMeta::default(),
    };
    let content = match String::from_utf8(bytes) {
        Ok(v) => v,
        Err(_) => return BasellmCacheMeta::default(),
    };
    decode_basellm_cache(&content)
}

fn decode_basellm_cache(content: &str) -> BasellmCacheMeta {
    serde_json::from_str::<BasellmCacheMeta>(content)
        .ok()
        .filter(|cache| cache.parser_version == BASELLM_PARSER_VERSION)
        .unwrap_or_default()
}

fn write_json_atomically(path: &Path, json_bytes: Vec<u8>) -> crate::shared::error::AppResult<()> {
    if json_bytes.len() > BASELLM_CACHE_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: basellm cache file too large (max {BASELLM_CACHE_MAX_BYTES} bytes)"
        )
        .into());
    }

    let tmp_path = path.with_extension("json.tmp");
    let backup_path = path.with_extension("json.bak");

    std::fs::write(&tmp_path, json_bytes)
        .map_err(|e| format!("failed to write temp cache file: {e}"))?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    if path.exists() {
        std::fs::rename(path, &backup_path)
            .map_err(|e| format!("failed to create cache backup: {e}"))?;
    }

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::rename(&backup_path, path);
        return Err(format!("failed to finalize cache file: {e}").into());
    }

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    Ok(())
}

fn write_basellm_cache(
    app: &tauri::AppHandle,
    cache: &BasellmCacheMeta,
) -> crate::shared::error::AppResult<()> {
    let path = basellm_cache_path(app)?;
    let content = serde_json::to_vec_pretty(cache)
        .map_err(|e| format!("failed to serialize basellm cache: {e}"))?;
    write_json_atomically(&path, content)
}

fn cli_key_from_basellm_provider(provider: &str) -> Option<&'static str> {
    let provider = provider.trim().to_ascii_lowercase();
    match provider.as_str() {
        "openai" => Some("codex"),
        "anthropic" => Some("claude"),
        // basellm historically used "google"; future-proof in case it switches to "gemini".
        "google" | "gemini" => Some("gemini"),
        "xai" => Some("grok"),
        // Third-party vendors have no owning CLI; store one copy under "claude" (their main
        // consumption path). Cost lookups fall back across cli_key in the price queries.
        "deepseek" | "zai" | "moonshotai" | "minimax" => Some("claude"),
        _ => None,
    }
}

fn supports_text_output(model: &Value) -> bool {
    let Some(output_modalities) = model
        .get("modalities")
        .and_then(|value| value.get("output"))
        .and_then(Value::as_array)
    else {
        return true;
    };

    output_modalities.iter().any(|value| {
        value
            .as_str()
            .is_some_and(|modality| modality.eq_ignore_ascii_case("text"))
    })
}

fn json_scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        }
        _ => None,
    }
}

fn shift_cost_per_1m_to_per_token(cost_per_1m: &str) -> Option<String> {
    let s = cost_per_1m.trim();
    if s.is_empty() {
        return None;
    }

    let (sign, rest) = if let Some(tail) = s.strip_prefix('-') {
        (-1, tail)
    } else if let Some(tail) = s.strip_prefix('+') {
        (1, tail)
    } else {
        (1, s)
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let (mantissa, exp10) = match rest.split_once(['e', 'E']) {
        Some((m, e)) => (m.trim(), e.trim().parse::<i64>().ok()?),
        None => (rest, 0),
    };
    if mantissa.is_empty() {
        return None;
    }

    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((a, b)) => (a.trim(), b.trim()),
        None => (mantissa.trim(), ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let mut digits = String::with_capacity(int_part.len() + frac_part.len());
    digits.push_str(int_part);
    digits.push_str(frac_part);
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some("0".to_string());
    }

    let digits = digits.to_string();
    let frac_places = frac_part.len() as i64;
    let exp10 = exp10.saturating_sub(6);

    // value = digits * 10^(exp10 - frac_places)
    let exp_total = exp10 - frac_places;
    let len = digits.len() as i64;
    let decimal_index = len + exp_total;

    let mut out = String::new();
    if sign < 0 {
        out.push('-');
    }

    if decimal_index <= 0 {
        out.push_str("0.");
        for _ in 0..(-decimal_index) {
            out.push('0');
        }
        out.push_str(&digits);
    } else if decimal_index >= len {
        out.push_str(&digits);
        for _ in 0..(decimal_index - len) {
            out.push('0');
        }
    } else {
        let idx = decimal_index as usize;
        out.push_str(&digits[..idx]);
        out.push('.');
        out.push_str(&digits[idx..]);
    }

    if let Some((head, tail)) = out.split_once('.') {
        let trimmed_tail = tail.trim_end_matches('0');
        if trimmed_tail.is_empty() {
            out = head.to_string();
        } else {
            out = format!("{head}.{trimmed_tail}");
        }
    }

    if out == "-0" {
        out = "0".to_string();
    }
    Some(out)
}

fn set_price_field(
    out: &mut serde_json::Map<String, Value>,
    key: &str,
    cost_per_1m: Option<String>,
) -> bool {
    let Some(cost_per_1m) = cost_per_1m else {
        return false;
    };
    let Some(per_token) = shift_cost_per_1m_to_per_token(&cost_per_1m) else {
        return false;
    };
    out.insert(key.to_string(), Value::String(per_token));
    true
}

/// A single `{"tier": {"type": "context", "size": N}, ...}` entry from basellm `cost.tiers`.
fn context_tier(
    cost: &serde_json::Map<String, Value>,
) -> Option<(i64, &serde_json::Map<String, Value>)> {
    let tiers = cost.get("tiers").and_then(Value::as_array)?;
    if tiers.len() != 1 {
        return None;
    }
    let tier = tiers[0].as_object()?;
    let meta = tier.get("tier").and_then(Value::as_object)?;
    if meta.get("type").and_then(Value::as_str) != Some("context") {
        return None;
    }
    let size = meta
        .get("size")
        .and_then(Value::as_i64)
        .filter(|s| *s > 0)?;
    Some((size, tier))
}

fn parse_basellm_all_json(root: &Value) -> crate::shared::error::AppResult<Vec<ModelPriceRow>> {
    let provider_map = root
        .as_object()
        .ok_or_else(|| "SYNC_ERROR: basellm all.json root must be an object".to_string())?;

    let mut rows = Vec::new();

    for (provider_key, provider_value) in provider_map {
        let Some(cli_key) = cli_key_from_basellm_provider(provider_key) else {
            continue;
        };

        let models = provider_value
            .get("models")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("SYNC_ERROR: basellm provider {provider_key} missing models"))?;

        for (model_name, model_value) in models {
            if !supports_text_output(model_value) {
                continue;
            }
            let Some(cost) = model_value.get("cost").and_then(|v| v.as_object()) else {
                continue;
            };

            let mut price = serde_json::Map::new();

            let has_input = set_price_field(
                &mut price,
                "input_cost_per_token",
                cost.get("input").and_then(json_scalar_to_string),
            );
            let has_output = set_price_field(
                &mut price,
                "output_cost_per_token",
                cost.get("output").and_then(json_scalar_to_string),
            );

            let _ = set_price_field(
                &mut price,
                "cache_read_input_token_cost",
                cost.get("cache_read").and_then(json_scalar_to_string),
            );
            let has_cache_write = set_price_field(
                &mut price,
                "cache_creation_input_token_cost",
                cost.get("cache_write").and_then(json_scalar_to_string),
            );
            if has_cache_write {
                if let Some(v) = price.get("cache_creation_input_token_cost").cloned() {
                    price.insert("cache_creation_input_token_cost_above_1hr".to_string(), v);
                }
            }

            // Context-tiered pricing: for non-200k thresholds (e.g. MiniMax-M3's 512k tier)
            // basellm mirrors the tier into `context_over_200k`, whose 200k semantics would be
            // wrong — map those to the custom threshold fields instead. 200k tiers keep the
            // legacy `*_above_200k_tokens` split-pricing fields.
            let custom_tier = context_tier(cost).filter(|(size, _)| *size != 200_000);
            if let Some((size, tier)) = custom_tier {
                price.insert(
                    "context_tier_threshold_tokens".to_string(),
                    Value::Number(size.into()),
                );
                let _ = set_price_field(
                    &mut price,
                    "input_cost_per_token_above_threshold",
                    tier.get("input").and_then(json_scalar_to_string),
                );
                let _ = set_price_field(
                    &mut price,
                    "output_cost_per_token_above_threshold",
                    tier.get("output").and_then(json_scalar_to_string),
                );
                let _ = set_price_field(
                    &mut price,
                    "cache_read_input_token_cost_above_threshold",
                    tier.get("cache_read").and_then(json_scalar_to_string),
                );
            } else if let Some(context_over_200k) =
                cost.get("context_over_200k").and_then(|v| v.as_object())
            {
                let _ = set_price_field(
                    &mut price,
                    "input_cost_per_token_above_200k_tokens",
                    context_over_200k
                        .get("input")
                        .and_then(json_scalar_to_string),
                );
                let _ = set_price_field(
                    &mut price,
                    "output_cost_per_token_above_200k_tokens",
                    context_over_200k
                        .get("output")
                        .and_then(json_scalar_to_string),
                );
            }

            if !has_input || !has_output {
                continue;
            }

            let price_json = serde_json::to_string(&Value::Object(price))
                .map_err(|e| format!("SYNC_ERROR: failed to serialize price_json: {e}"))?;

            rows.push(ModelPriceRow {
                cli_key,
                vendor: provider_key.to_string(),
                model: model_name.to_string(),
                price_json,
            });
        }
    }

    Ok(rows)
}

fn load_existing_price_map(
    tx: &rusqlite::Transaction<'_>,
) -> crate::shared::error::AppResult<HashMap<(String, String), (String, String)>> {
    let mut stmt = tx
        .prepare_cached("SELECT cli_key, model, price_json, vendor FROM model_prices")
        .map_err(|e| db_err!("failed to prepare existing model_prices query: {e}"))?;

    let mut map = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            let cli_key: String = row.get(0)?;
            let model: String = row.get(1)?;
            let price_json: String = row.get(2)?;
            let vendor: String = row.get(3)?;
            Ok((cli_key, model, price_json, vendor))
        })
        .map_err(|e| db_err!("failed to query existing model_prices: {e}"))?;

    for row in rows {
        let (cli_key, model, raw_price, vendor) =
            row.map_err(|e| db_err!("failed to read existing model_price row: {e}"))?;
        let normalized = match serde_json::from_str::<Value>(&raw_price)
            .ok()
            .and_then(|v| serde_json::to_string(&v).ok())
        {
            Some(v) => v,
            None => raw_price,
        };
        map.insert((cli_key, model), (normalized, vendor));
    }

    Ok(map)
}

fn upsert_rows(
    db: &db::Db,
    mut rows: Vec<ModelPriceRow>,
) -> crate::shared::error::AppResult<UpsertCounts> {
    rows.sort_by(|a, b| (a.cli_key, &a.model).cmp(&(b.cli_key, &b.model)));
    rows.dedup_by(|a, b| a.cli_key == b.cli_key && a.model == b.model);

    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start sqlite transaction: {e}"))?;

    let existing = load_existing_price_map(&tx)?;

    let now = now_unix_seconds();
    let mut inserted: u32 = 0;
    let mut updated: u32 = 0;
    let mut unchanged: u32 = 0;

    {
        let mut stmt = tx
            .prepare_cached(
                r#"
        INSERT INTO model_prices(cli_key, vendor, model, price_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(cli_key, model) DO UPDATE SET
          vendor = excluded.vendor,
          price_json = excluded.price_json,
          updated_at = excluded.updated_at
        "#,
            )
            .map_err(|e| db_err!("failed to prepare model_prices upsert: {e}"))?;

        for row in rows {
            let normalized_new = match serde_json::from_str::<Value>(&row.price_json)
                .ok()
                .and_then(|v| serde_json::to_string(&v).ok())
            {
                Some(v) => v,
                None => row.price_json.clone(),
            };

            let existing_row = existing.get(&(row.cli_key.to_string(), row.model.clone()));

            if let Some((existing_price, existing_vendor)) = existing_row {
                if *existing_price == normalized_new && *existing_vendor == row.vendor {
                    unchanged += 1;
                    continue;
                }
                updated += 1;
            } else {
                inserted += 1;
            }

            stmt.execute(params![
                row.cli_key,
                &row.vendor,
                &row.model,
                &normalized_new,
                now
            ])
            .map_err(|e| db_err!("failed to upsert model_price: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| db_err!("failed to commit model_prices sync transaction: {e}"))?;

    Ok(UpsertCounts {
        inserted,
        updated,
        unchanged,
    })
}

fn headers_to_cache(headers: &HeaderMap) -> BasellmCacheMeta {
    let etag = headers
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = headers
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    BasellmCacheMeta {
        parser_version: BASELLM_PARSER_VERSION,
        etag,
        last_modified,
    }
}

fn add_cache_headers(mut headers: HeaderMap, cache: &BasellmCacheMeta) -> HeaderMap {
    if let Some(etag) = cache.etag.as_deref() {
        if let Ok(v) = HeaderValue::from_str(etag) {
            headers.insert(IF_NONE_MATCH, v);
        }
    }
    if let Some(last_modified) = cache.last_modified.as_deref() {
        if let Ok(v) = HeaderValue::from_str(last_modified) {
            headers.insert(IF_MODIFIED_SINCE, v);
        }
    }
    headers
}

async fn fetch_basellm(
    client: &reqwest::Client,
    cache: BasellmCacheMeta,
) -> Result<BasellmFetch, String> {
    let resp = client
        .get(BASELLM_ALL_JSON_URL)
        .headers(add_cache_headers(HeaderMap::new(), &cache))
        .send()
        .await
        .map_err(|error| format!("BaseLLM request failed: {error}"))?;

    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(BasellmFetch::NotModified);
    }

    if !resp.status().is_success() {
        return Err(format!("BaseLLM returned HTTP {}", resp.status()));
    }

    let new_cache = headers_to_cache(resp.headers());
    let body = read_text_with_limit(resp, BASELLM_RESPONSE_BODY_LIMIT, "basellm response")
        .await
        .map_err(|error| format!("failed to read BaseLLM response: {error}"))?;
    let rows = blocking::run(
        "basellm_parse_prices",
        move || -> crate::shared::error::AppResult<Vec<ModelPriceRow>> {
            let root: Value = serde_json::from_str(&body)
                .map_err(|error| format!("BaseLLM JSON parse failed: {error}"))?;
            parse_basellm_all_json(&root)
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    if rows.is_empty() {
        return Err("BaseLLM contains no supported text model prices".to_string());
    }
    Ok(BasellmFetch::Rows {
        rows,
        cache: new_cache,
    })
}

fn failed_report(error: String) -> ModelPricesSyncReport {
    tracing::warn!(error = %error, "model price sync failed");
    ModelPricesSyncReport {
        status: ModelPricesSyncStatus::Failed,
        inserted: 0,
        updated: 0,
        unchanged: 0,
        total: 0,
        error: Some(error),
    }
}

pub async fn sync(
    app: &tauri::AppHandle,
    db: db::Db,
) -> crate::shared::error::AppResult<ModelPricesSyncReport> {
    tracing::info!("model prices sync started");
    let app_handle = app.clone();
    let cache = blocking::run("basellm_read_cache", {
        let app_handle = app_handle.clone();
        move || -> crate::shared::error::AppResult<BasellmCacheMeta> {
            Ok(read_basellm_cache(&app_handle))
        }
    })
    .await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("SYNC_ERROR: failed to build HTTP client: {error}"))?;

    let report = match fetch_basellm(&client, cache).await {
        Ok(BasellmFetch::NotModified) => ModelPricesSyncReport {
            status: ModelPricesSyncStatus::NotModified,
            inserted: 0,
            updated: 0,
            unchanged: 0,
            total: 0,
            error: None,
        },
        Ok(BasellmFetch::Rows { rows, cache }) => {
            let upsert_db = db.clone();
            match blocking::run("model_prices_sync_upsert", move || {
                upsert_rows(&upsert_db, rows)
            })
            .await
            {
                Ok(counts) => {
                    if let Err(error) = blocking::run("basellm_write_cache", move || {
                        write_basellm_cache(&app_handle, &cache)
                    })
                    .await
                    {
                        tracing::warn!("BaseLLM cache write failed: {error}");
                    }
                    ModelPricesSyncReport {
                        status: if counts.inserted > 0 || counts.updated > 0 {
                            ModelPricesSyncStatus::Updated
                        } else {
                            ModelPricesSyncStatus::NotModified
                        },
                        inserted: counts.inserted,
                        updated: counts.updated,
                        unchanged: counts.unchanged,
                        total: counts.total(),
                        error: None,
                    }
                }
                Err(error) => failed_report(error.to_string()),
            }
        }
        Err(error) => failed_report(error),
    };

    tracing::info!(
        status = ?report.status,
        inserted = report.inserted,
        updated = report.updated,
        unchanged = report.unchanged,
        total = report.total,
        "model prices sync completed"
    );
    Ok(report)
}

#[cfg(test)]
mod tests;
