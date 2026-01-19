use crate::db;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_KEEP_PER_PROVIDER: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeModelValidationRunRow {
    pub id: i64,
    pub provider_id: i64,
    pub created_at: i64,
    pub request_json: String,
    pub result_json: String,
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ensure_provider_is_claude(conn: &rusqlite::Connection, provider_id: i64) -> Result<(), String> {
    if provider_id <= 0 {
        return Err(format!(
            "SEC_INVALID_INPUT: invalid provider_id={provider_id}"
        ));
    }

    let cli_key: Option<String> = conn
        .query_row(
            "SELECT cli_key FROM providers WHERE id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB_ERROR: failed to query provider cli_key: {e}"))?;

    let Some(cli_key) = cli_key else {
        return Err("DB_NOT_FOUND: provider not found".to_string());
    };

    if cli_key != "claude" {
        return Err(format!(
            "SEC_INVALID_INPUT: only cli_key=claude is supported (provider_id={provider_id})"
        ));
    }

    Ok(())
}

fn is_http_success_result_json(result_json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(result_json) else {
        return false;
    };

    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if !ok {
        return false;
    }

    // 过滤早期误插入的“看似 ok 但无响应数据/读流出错”的记录。
    //
    // 新口径：HTTP 2xx + 有响应数据 + 无 stream 读取错误 才算“请求成功”，才应出现在历史列表。
    if let Some(raw_excerpt) = value.get("raw_excerpt").and_then(|v| v.as_str()) {
        if raw_excerpt.trim().is_empty() {
            return false;
        }
    }

    if let Some(stream_read_error) = value
        .get("signals")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("stream_read_error"))
        .and_then(|v| v.as_bool())
    {
        if stream_read_error {
            return false;
        }
    }

    true
}

pub fn insert_run_and_prune(
    app: &tauri::AppHandle,
    provider_id: i64,
    request_json: &str,
    result_json: &str,
    keep: Option<usize>,
) -> Result<i64, String> {
    let keep = keep.unwrap_or(DEFAULT_KEEP_PER_PROVIDER).clamp(1, 500);
    if request_json.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: request_json is required".to_string());
    }
    if result_json.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: result_json is required".to_string());
    }

    let mut conn = db::open_connection(app)?;
    ensure_provider_is_claude(&conn, provider_id)?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let now = now_unix_seconds();
    tx.execute(
        r#"
INSERT INTO claude_model_validation_runs(
  provider_id,
  created_at,
  request_json,
  result_json
) VALUES (?1, ?2, ?3, ?4)
"#,
        params![provider_id, now, request_json, result_json],
    )
    .map_err(|e| format!("DB_ERROR: failed to insert claude_model_validation_run: {e}"))?;

    let inserted_id = tx.last_insert_rowid();

    tx.execute(
        r#"
DELETE FROM claude_model_validation_runs
WHERE provider_id = ?1
  AND id NOT IN (
    SELECT id
    FROM claude_model_validation_runs
    WHERE provider_id = ?1
    ORDER BY id DESC
    LIMIT ?2
  )
"#,
        params![provider_id, keep as i64],
    )
    .map_err(|e| format!("DB_ERROR: failed to prune claude_model_validation_runs: {e}"))?;

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit transaction: {e}"))?;

    Ok(inserted_id)
}

pub fn list_runs(
    app: &tauri::AppHandle,
    provider_id: i64,
    limit: Option<usize>,
) -> Result<Vec<ClaudeModelValidationRunRow>, String> {
    let limit = limit.unwrap_or(DEFAULT_KEEP_PER_PROVIDER).clamp(1, 500);
    let fetch_limit = limit.saturating_mul(3).clamp(limit, 500);

    let conn = db::open_connection(app)?;
    ensure_provider_is_claude(&conn, provider_id)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  provider_id,
  created_at,
  request_json,
  result_json
FROM claude_model_validation_runs
WHERE provider_id = ?1
ORDER BY id DESC
LIMIT ?2
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare history list query: {e}"))?;

    let rows = stmt
        .query_map(params![provider_id, fetch_limit as i64], |row| {
            Ok(ClaudeModelValidationRunRow {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                created_at: row.get(2)?,
                request_json: row.get(3)?,
                result_json: row.get(4)?,
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to list claude_model_validation_runs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        let item = row.map_err(|e| format!("DB_ERROR: failed to read history row: {e}"))?;
        // 历史只返回“请求成功”的结果（ok=true 且有响应数据且无 stream 读取错误），
        // 早期误插入的异常记录（例如 4xx/5xx、空响应、读流错误）在此过滤掉。
        if is_http_success_result_json(&item.result_json) {
            items.push(item);
        }
        if items.len() >= limit {
            break;
        }
    }
    Ok(items)
}

pub fn clear_provider(app: &tauri::AppHandle, provider_id: i64) -> Result<bool, String> {
    let conn = db::open_connection(app)?;
    ensure_provider_is_claude(&conn, provider_id)?;

    conn.execute(
        "DELETE FROM claude_model_validation_runs WHERE provider_id = ?1",
        params![provider_id],
    )
    .map_err(|e| format!("DB_ERROR: failed to clear claude_model_validation_runs: {e}"))?;

    Ok(true)
}
