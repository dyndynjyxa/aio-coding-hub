//! Usage: Request attempt logs compatibility layer (derived from request_logs.attempts_json).

use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RequestAttemptLog {
    pub id: i64,
    pub trace_id: String,
    pub cli_key: String,
    pub attempt_index: i64,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub outcome: String,
    pub status: Option<i64>,
    pub attempt_started_ms: i64,
    pub attempt_duration_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AttemptRow {
    provider_id: i64,
    #[serde(default)]
    provider_name: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    outcome: String,
    status: Option<i64>,
    attempt_started_ms: Option<i64>,
    attempt_duration_ms: Option<i64>,
}

fn parse_attempts(attempts_json: &str) -> Vec<AttemptRow> {
    serde_json::from_str(attempts_json).unwrap_or_default()
}

pub fn list_by_trace_id(
    db: &db::Db,
    trace_id: &str,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestAttemptLog>> {
    let trace_id = trace_id.trim();
    if trace_id.is_empty() {
        return Err("SEC_INVALID_INPUT: trace_id is required".into());
    }

    let limit = limit.clamp(1, 200);
    let conn = db.open_connection()?;

    let row = conn
        .query_row(
            r#"
SELECT
  cli_key,
  attempts_json,
  created_at
FROM request_logs
WHERE trace_id = ?1
LIMIT 1
"#,
            params![trace_id],
            |row| {
                let cli_key: String = row.get("cli_key")?;
                let attempts_json: String = row.get("attempts_json")?;
                let created_at: i64 = row.get("created_at")?;
                Ok((cli_key, attempts_json, created_at))
            },
        )
        .optional()
        .map_err(|e| db_err!("failed to query request_logs by trace_id: {e}"))?;

    let Some((cli_key, attempts_json, created_at)) = row else {
        return Ok(Vec::new());
    };

    let attempts = parse_attempts(&attempts_json);
    let mut out = Vec::with_capacity(attempts.len().min(limit));
    for (idx, attempt) in attempts.into_iter().take(limit).enumerate() {
        out.push(RequestAttemptLog {
            id: (idx as i64).saturating_add(1),
            trace_id: trace_id.to_string(),
            cli_key: cli_key.clone(),
            attempt_index: (idx as i64).saturating_add(1),
            provider_id: attempt.provider_id,
            provider_name: attempt.provider_name,
            base_url: attempt.base_url,
            outcome: attempt.outcome,
            status: attempt.status,
            attempt_started_ms: attempt.attempt_started_ms.unwrap_or(0),
            attempt_duration_ms: attempt.attempt_duration_ms.unwrap_or(0),
            created_at,
        });
    }

    Ok(out)
}
