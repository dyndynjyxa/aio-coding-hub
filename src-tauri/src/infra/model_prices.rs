//! Usage: Model price persistence (sqlite CRUD helpers).

use crate::db;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModelPriceSummary {
    pub id: i64,
    pub cli_key: String,
    pub model: String,
    pub currency: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<ModelPriceSummary, rusqlite::Error> {
    Ok(ModelPriceSummary {
        id: row.get("id")?,
        cli_key: row.get("cli_key")?,
        model: row.get("model")?,
        currency: row.get("currency")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_by_cli(db: &db::Db, cli_key: &str) -> Result<Vec<ModelPriceSummary>, String> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  cli_key,
  model,
  currency,
  created_at,
  updated_at
FROM model_prices
WHERE cli_key = ?1
ORDER BY model ASC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare model_prices list: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list model_prices: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read model_price row: {e}"))?);
    }
    Ok(items)
}

pub fn upsert(
    db: &db::Db,
    cli_key: &str,
    model: &str,
    price_json: &str,
) -> Result<ModelPriceSummary, String> {
    validate_cli_key(cli_key)?;

    let model = model.trim();
    if model.is_empty() {
        return Err("SEC_INVALID_INPUT: model is required".to_string());
    }

    let normalized_price = match serde_json::from_str::<serde_json::Value>(price_json) {
        Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        Err(_) => return Err("SEC_INVALID_INPUT: price_json must be valid JSON".to_string()),
    };

    if normalized_price == "{}" {
        return Err("SEC_INVALID_INPUT: price_json is empty".to_string());
    }

    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    conn.execute(
        r#"
INSERT INTO model_prices(cli_key, model, price_json, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
ON CONFLICT(cli_key, model) DO UPDATE SET
  price_json = excluded.price_json,
  updated_at = excluded.updated_at
"#,
        params![cli_key, model, normalized_price, now],
    )
    .map_err(|e| format!("DB_ERROR: failed to upsert model_price: {e}"))?;

    conn.query_row(
        r#"
SELECT
  id,
  cli_key,
  model,
  currency,
  created_at,
  updated_at
FROM model_prices
WHERE cli_key = ?1 AND model = ?2
"#,
        params![cli_key, model],
        row_to_summary,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query model_price: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: model_price not found".to_string())
}
