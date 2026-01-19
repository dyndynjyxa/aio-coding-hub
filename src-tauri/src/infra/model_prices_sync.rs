//! Usage: Sync model price data from external sources and persist into sqlite.

use crate::{app_paths, blocking, db};
use reqwest::header::{HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BASELLM_ALL_JSON_URL: &str = "https://basellm.github.io/llm-metadata/api/all.json";

#[derive(Debug, Clone, Serialize)]
pub struct ModelPricesSyncReport {
    pub status: String,
    pub inserted: u32,
    pub updated: u32,
    pub skipped: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct BasellmCacheMeta {
    etag: Option<String>,
    last_modified: Option<String>,
}

#[derive(Debug, Clone)]
struct ModelPriceRow {
    cli_key: String,
    model: String,
    price_json: String,
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn model_prices_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_paths::app_data_dir(app)?.join("model-prices");
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create model-prices dir: {e}"))?;
    Ok(dir)
}

fn basellm_cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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
    let content = match std::fs::read_to_string(&path) {
        Ok(v) => v,
        Err(_) => return BasellmCacheMeta::default(),
    };
    serde_json::from_str::<BasellmCacheMeta>(&content).unwrap_or_default()
}

fn write_json_atomically(path: &Path, json_bytes: Vec<u8>) -> Result<(), String> {
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
        return Err(format!("failed to finalize cache file: {e}"));
    }

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    Ok(())
}

fn write_basellm_cache(app: &tauri::AppHandle, cache: &BasellmCacheMeta) -> Result<(), String> {
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
        _ => None,
    }
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

fn parse_basellm_all_json(root: &Value) -> Result<Vec<ModelPriceRow>, String> {
    let provider_map = root
        .as_object()
        .ok_or_else(|| "SYNC_ERROR: basellm all.json root must be an object".to_string())?;

    let mut rows = Vec::new();

    for (provider_key, provider_value) in provider_map {
        let Some(cli_key) = cli_key_from_basellm_provider(provider_key.as_str()) else {
            continue;
        };

        let models = provider_value
            .get("models")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("SYNC_ERROR: basellm provider {provider_key} missing models"))?;

        for (model_name, model_value) in models {
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

            if let Some(context_over_200k) =
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

            if !has_input && !has_output {
                continue;
            }

            let price_json = serde_json::to_string(&Value::Object(price))
                .map_err(|e| format!("SYNC_ERROR: failed to serialize price_json: {e}"))?;

            rows.push(ModelPriceRow {
                cli_key: cli_key.to_string(),
                model: model_name.to_string(),
                price_json,
            });
        }
    }

    Ok(rows)
}

fn load_existing_price_map(
    tx: &rusqlite::Transaction<'_>,
    cli_key: &str,
) -> Result<HashMap<String, String>, String> {
    let mut stmt = tx
        .prepare("SELECT model, price_json FROM model_prices WHERE cli_key = ?1")
        .map_err(|e| format!("DB_ERROR: failed to prepare existing model_prices query: {e}"))?;

    let mut map = HashMap::new();
    let rows = stmt
        .query_map(params![cli_key], |row| {
            let model: String = row.get(0)?;
            let price_json: String = row.get(1)?;
            Ok((model, price_json))
        })
        .map_err(|e| format!("DB_ERROR: failed to query existing model_prices: {e}"))?;

    for row in rows {
        let (model, raw_price) =
            row.map_err(|e| format!("DB_ERROR: failed to read existing model_price row: {e}"))?;
        let normalized = match serde_json::from_str::<Value>(&raw_price)
            .ok()
            .and_then(|v| serde_json::to_string(&v).ok())
        {
            Some(v) => v,
            None => raw_price,
        };
        map.insert(model, normalized);
    }

    Ok(map)
}

fn upsert_rows(
    app: &tauri::AppHandle,
    mut rows: Vec<ModelPriceRow>,
) -> Result<ModelPricesSyncReport, String> {
    // De-dup by (cli_key, model) to avoid conflicting writes if basellm contains duplicates.
    // Keep the first occurrence deterministically by stable sort + dedup.
    rows.sort_by(|a, b| {
        (a.cli_key.as_str(), a.model.as_str()).cmp(&(b.cli_key.as_str(), b.model.as_str()))
    });
    rows.dedup_by(|a, b| a.cli_key == b.cli_key && a.model == b.model);

    let mut conn = db::open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start sqlite transaction: {e}"))?;

    let mut cli_keys: HashSet<String> = HashSet::new();
    for row in &rows {
        cli_keys.insert(row.cli_key.clone());
    }

    let mut existing_by_cli: HashMap<String, HashMap<String, String>> = HashMap::new();
    for cli_key in cli_keys {
        existing_by_cli.insert(cli_key.clone(), load_existing_price_map(&tx, &cli_key)?);
    }

    let now = now_unix_seconds();
    let mut inserted: u32 = 0;
    let mut updated: u32 = 0;
    let mut skipped: u32 = 0;

    {
        let mut stmt = tx
            .prepare(
                r#"
INSERT INTO model_prices(cli_key, model, price_json, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
ON CONFLICT(cli_key, model) DO UPDATE SET
  price_json = excluded.price_json,
  updated_at = excluded.updated_at
"#,
            )
            .map_err(|e| format!("DB_ERROR: failed to prepare model_prices upsert: {e}"))?;

        for row in rows {
            let normalized_new = match serde_json::from_str::<Value>(&row.price_json)
                .ok()
                .and_then(|v| serde_json::to_string(&v).ok())
            {
                Some(v) => v,
                None => row.price_json.clone(),
            };

            let existing = existing_by_cli
                .get(&row.cli_key)
                .and_then(|m| m.get(&row.model))
                .map(|s| s.as_str());

            if let Some(existing_price) = existing {
                if existing_price == normalized_new {
                    skipped += 1;
                    continue;
                }
                updated += 1;
            } else {
                inserted += 1;
            }

            stmt.execute(params![row.cli_key, row.model, normalized_new, now])
                .map_err(|e| format!("DB_ERROR: failed to upsert model_price: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit model_prices sync transaction: {e}"))?;

    Ok(ModelPricesSyncReport {
        status: "updated".to_string(),
        inserted,
        updated,
        skipped,
        total: inserted.saturating_add(updated).saturating_add(skipped),
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

pub async fn sync_basellm(
    app: &tauri::AppHandle,
    force: bool,
) -> Result<ModelPricesSyncReport, String> {
    let app_handle = app.clone();
    let cache = if force {
        BasellmCacheMeta::default()
    } else {
        blocking::run("basellm_read_cache", {
            let app_handle = app_handle.clone();
            move || Ok(read_basellm_cache(&app_handle))
        })
        .await?
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("SYNC_ERROR: failed to build http client: {e}"))?;

    let request = client.get(BASELLM_ALL_JSON_URL);
    let request = if force {
        request
    } else {
        request.headers(add_cache_headers(HeaderMap::new(), &cache))
    };

    let resp = request
        .send()
        .await
        .map_err(|e| format!("SYNC_ERROR: basellm request failed: {e}"))?;

    if resp.status() == reqwest::StatusCode::NOT_MODIFIED && !force {
        return Ok(ModelPricesSyncReport {
            status: "not_modified".to_string(),
            inserted: 0,
            updated: 0,
            skipped: 0,
            total: 0,
        });
    }

    if !resp.status().is_success() {
        return Err(format!(
            "SYNC_ERROR: basellm returned http status {}",
            resp.status()
        ));
    }

    let new_cache = headers_to_cache(resp.headers());
    let body = resp
        .text()
        .await
        .map_err(|e| format!("SYNC_ERROR: failed to read basellm response: {e}"))?;

    let rows = blocking::run("basellm_parse_rows", move || {
        let root: Value = serde_json::from_str(&body)
            .map_err(|e| format!("SYNC_ERROR: basellm json parse failed: {e}"))?;
        parse_basellm_all_json(&root)
    })
    .await?;

    let report = blocking::run("basellm_upsert_rows", {
        let app_handle = app_handle.clone();
        move || upsert_rows(&app_handle, rows)
    })
    .await?;

    // Best-effort: cache write should not fail the whole sync after DB is updated.
    if let Err(err) = blocking::run("basellm_write_cache", move || {
        write_basellm_cache(&app_handle, &new_cache)
    })
    .await
    {
        eprintln!("basellm cache write error: {err}");
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_1m_cost_to_per_token_with_plain_number() {
        assert_eq!(
            shift_cost_per_1m_to_per_token("2.5").as_deref(),
            Some("0.0000025")
        );
        assert_eq!(
            shift_cost_per_1m_to_per_token("10").as_deref(),
            Some("0.00001")
        );
    }

    #[test]
    fn shifts_1m_cost_to_per_token_with_existing_exponent() {
        assert_eq!(
            shift_cost_per_1m_to_per_token("1e3").as_deref(),
            Some("0.001")
        );
        assert_eq!(
            shift_cost_per_1m_to_per_token("1e-3").as_deref(),
            Some("0.000000001")
        );
    }

    #[test]
    fn parses_google_context_over_200k_to_above_200k_fields() {
        let root = serde_json::json!({
          "google": {
            "models": {
              "gemini-3-pro-preview": {
                "cost": {
                  "input": 2,
                  "output": 10,
                  "context_over_200k": { "input": 3, "output": 15 }
                }
              }
            }
          }
        });

        let rows = parse_basellm_all_json(&root).expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cli_key, "gemini");
        assert_eq!(rows[0].model, "gemini-3-pro-preview");

        let price: Value = serde_json::from_str(&rows[0].price_json).expect("price json");
        assert_eq!(
            price
                .get("input_cost_per_token")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "0.000002"
        );
        assert_eq!(
            price
                .get("output_cost_per_token")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "0.00001"
        );
        assert_eq!(
            price
                .get("input_cost_per_token_above_200k_tokens")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "0.000003"
        );
        assert_eq!(
            price
                .get("output_cost_per_token_above_200k_tokens")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "0.000015"
        );
    }
}
