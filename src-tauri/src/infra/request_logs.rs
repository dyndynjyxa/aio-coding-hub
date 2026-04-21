//! Usage: Request log persistence (sqlite buffered writer and queries).

use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::{cost, db, model_price_aliases};
use rusqlite::{params, params_from_iter, ErrorCode, OptionalExtension, TransactionBehavior};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;

mod types;
pub use types::{
    RequestLogDetail, RequestLogInsert, RequestLogRouteHop, RequestLogSummary,
    SessionStatsAggregate,
};

mod costing;
use costing::{has_any_cost_usage, is_success_status, usage_for_cost};

mod queries;
use queries::{final_provider_from_attempts, parse_attempts, validate_cli_key};
pub use queries::{
    get_by_id, get_by_trace_id, list_after_id, list_after_id_all, list_recent, list_recent_all,
};

const WRITE_BUFFER_CAPACITY: usize = 512;
const WRITE_BATCH_MAX: usize = 50;
const INSERT_RETRY_MAX_ATTEMPTS: u32 = 8;
const INSERT_RETRY_BASE_DELAY_MS: u64 = 20;
const INSERT_RETRY_MAX_DELAY_MS: u64 = 500;

const COST_MULTIPLIER_CACHE_MAX_ENTRIES: usize = 256;
const MODEL_PRICE_CACHE_MAX_ENTRIES: usize = 512;
const CACHE_TTL_SECS: i64 = 5 * 60;
const EFFECTIVE_COST_MULTIPLIER_SQL: &str = r#"
SELECT COALESCE(source.cost_multiplier, bridge.cost_multiplier)
FROM providers bridge
LEFT JOIN providers source ON source.id = bridge.source_provider_id
WHERE bridge.id = ?1
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbWriteErrorKind {
    Busy,
    Other,
}

#[derive(Debug)]
struct DbWriteError {
    kind: DbWriteErrorKind,
    message: String,
}

impl DbWriteError {
    fn other(message: String) -> Self {
        Self {
            kind: DbWriteErrorKind::Other,
            message,
        }
    }

    fn from_rusqlite(context: &'static str, err: rusqlite::Error) -> Self {
        let kind = classify_rusqlite_error(&err);
        Self {
            kind,
            message: format!("DB_ERROR: {context}: {err}"),
        }
    }

    fn is_retryable(&self) -> bool {
        self.kind == DbWriteErrorKind::Busy
    }
}

fn classify_rusqlite_error(err: &rusqlite::Error) -> DbWriteErrorKind {
    match err {
        rusqlite::Error::SqliteFailure(e, _) => match e.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => DbWriteErrorKind::Busy,
            _ => DbWriteErrorKind::Other,
        },
        _ => DbWriteErrorKind::Other,
    }
}

fn retry_delay(attempt_index: u32) -> Duration {
    let exp = attempt_index.min(20);
    let raw = INSERT_RETRY_BASE_DELAY_MS.saturating_mul(1u64.checked_shl(exp).unwrap_or(u64::MAX));
    Duration::from_millis(raw.min(INSERT_RETRY_MAX_DELAY_MS))
}

#[derive(Debug, Clone)]
struct CachedValue<T> {
    value: T,
    fetched_at: i64,
}

#[derive(Default)]
struct InsertBatchCache {
    provider_multiplier: HashMap<i64, CachedValue<f64>>,
    model_price_json: HashMap<String, CachedValue<Option<String>>>,
}

impl InsertBatchCache {
    fn get_cost_multiplier(&mut self, provider_id: i64, now: i64) -> Option<f64> {
        let entry = self.provider_multiplier.get(&provider_id)?;
        if now.saturating_sub(entry.fetched_at) > CACHE_TTL_SECS {
            self.provider_multiplier.remove(&provider_id);
            return None;
        }
        Some(entry.value)
    }

    fn put_cost_multiplier(&mut self, provider_id: i64, value: f64, now: i64) {
        if self.provider_multiplier.len() >= COST_MULTIPLIER_CACHE_MAX_ENTRIES {
            self.provider_multiplier.clear();
        }
        self.provider_multiplier.insert(
            provider_id,
            CachedValue {
                value,
                fetched_at: now,
            },
        );
    }

    fn get_model_price_json(&mut self, key: &str, now: i64) -> Option<Option<String>> {
        let entry = self.model_price_json.get(key)?;
        if now.saturating_sub(entry.fetched_at) > CACHE_TTL_SECS {
            self.model_price_json.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    fn put_model_price_json(&mut self, key: String, value: Option<String>, now: i64) {
        let Some(value) = value else {
            return;
        };

        if self.model_price_json.len() >= MODEL_PRICE_CACHE_MAX_ENTRIES {
            self.model_price_json.clear();
        }
        self.model_price_json.insert(
            key,
            CachedValue {
                value: Some(value),
                fetched_at: now,
            },
        );
    }
}

fn fetch_model_price_json(
    stmt_price_json: &mut rusqlite::Statement<'_>,
    cache: &mut InsertBatchCache,
    batch_price_json: &mut HashMap<String, Option<String>>,
    now_unix: i64,
    cli_key: &str,
    model: &str,
) -> Option<String> {
    let price_key = format!("{cli_key}\n{model}");
    if let Some(v) = batch_price_json.get(&price_key) {
        return v.clone();
    }

    let cached = cache.get_model_price_json(&price_key, now_unix);
    let queried = cached.unwrap_or_else(|| {
        let value = stmt_price_json
            .query_row(params![cli_key, model], |row| row.get::<_, String>(0))
            .optional()
            .unwrap_or(None);
        cache.put_model_price_json(price_key.clone(), value.clone(), now_unix);
        value
    });

    batch_price_json.insert(price_key, queried.clone());
    queried
}

#[derive(Debug, Clone)]
struct EffectiveCostBasis {
    cli_key: String,
    model: String,
}

pub(crate) fn parse_cx2cc_cost_basis(
    special_settings_json: Option<&str>,
) -> Option<(String, String)> {
    let raw = special_settings_json?.trim();
    if raw.is_empty() {
        return None;
    }

    let settings: Vec<Value> = serde_json::from_str(raw).ok()?;
    for setting in settings.iter().rev() {
        let Some(obj) = setting.as_object() else {
            continue;
        };
        if obj.get("type").and_then(Value::as_str) != Some("cx2cc_cost_basis") {
            continue;
        }

        let Some(cli_key) = obj
            .get("source_cli_key")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(model) = obj
            .get("priced_model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        return Some((cli_key.to_string(), model.to_string()));
    }

    None
}

fn effective_cost_basis(item: &RequestLogInsert) -> Option<EffectiveCostBasis> {
    if let Some((cli_key, model)) = parse_cx2cc_cost_basis(item.special_settings_json.as_deref()) {
        return Some(EffectiveCostBasis { cli_key, model });
    }

    let model = item
        .requested_model
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())?
        .to_string();

    Some(EffectiveCostBasis {
        cli_key: item.cli_key.clone(),
        model,
    })
}

pub fn start_buffered_writer(
    app: tauri::AppHandle,
    db: db::Db,
) -> (
    mpsc::Sender<RequestLogInsert>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel::<RequestLogInsert>(WRITE_BUFFER_CAPACITY);
    let task = tauri::async_runtime::spawn_blocking(move || {
        writer_loop(app, db, rx);
    });
    (tx, task)
}

pub fn spawn_write_through(app: tauri::AppHandle, db: db::Db, item: RequestLogInsert) {
    tauri::async_runtime::spawn_blocking(move || {
        let mut cache = InsertBatchCache::default();
        let items = [item];
        if let Err(err) = insert_batch_with_retries(&app, &db, &items, &mut cache) {
            tracing::error!(error = %err.message, "request log write-through insert failed");
        }
    });
}

fn writer_loop(app: tauri::AppHandle, db: db::Db, mut rx: mpsc::Receiver<RequestLogInsert>) {
    let mut buffer: Vec<RequestLogInsert> = Vec::with_capacity(WRITE_BATCH_MAX);
    let mut cache = InsertBatchCache::default();

    while let Some(item) = rx.blocking_recv() {
        buffer.push(item);

        while buffer.len() < WRITE_BATCH_MAX {
            match rx.try_recv() {
                Ok(next) => buffer.push(next),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        if let Err(err) = insert_batch_with_retries(&app, &db, &buffer, &mut cache) {
            tracing::error!(error = %err.message, "request log batch insert failed");
        }
        buffer.clear();
    }

    if !buffer.is_empty() {
        if let Err(err) = insert_batch_with_retries(&app, &db, &buffer, &mut cache) {
            tracing::error!(error = %err.message, "request log final batch insert failed");
        }
    }
}

fn insert_batch_with_retries(
    app: &tauri::AppHandle,
    db: &db::Db,
    items: &[RequestLogInsert],
    cache: &mut InsertBatchCache,
) -> Result<(), DbWriteError> {
    let mut attempt: u32 = 0;
    loop {
        match insert_batch_once(app, db, items, cache) {
            Ok(()) => return Ok(()),
            Err(err) => {
                attempt = attempt.saturating_add(1);
                if !err.is_retryable() || attempt >= INSERT_RETRY_MAX_ATTEMPTS {
                    return Err(err);
                }
                let delay = retry_delay(attempt.saturating_sub(1));
                tracing::debug!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    error = %err.message,
                    "sqlite busy/locked; retrying request_logs insert"
                );
                std::thread::sleep(delay);
            }
        }
    }
}

fn insert_batch_once(
    app: &tauri::AppHandle,
    db: &db::Db,
    items: &[RequestLogInsert],
    cache: &mut InsertBatchCache,
) -> Result<(), DbWriteError> {
    if items.is_empty() {
        return Ok(());
    }

    let now_unix = now_unix_seconds();
    let price_aliases = model_price_aliases::read_fail_open(app);
    let mut conn = db
        .open_connection()
        .map_err(|e| DbWriteError::other(e.to_string()))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| DbWriteError::from_rusqlite("failed to start transaction", e))?;

    {
        let mut stmt_multiplier =
            tx.prepare_cached(EFFECTIVE_COST_MULTIPLIER_SQL)
                .map_err(|e| {
                    DbWriteError::from_rusqlite("failed to prepare cost_multiplier query", e)
                })?;
        let mut stmt_price_json = tx
            .prepare_cached("SELECT price_json FROM model_prices WHERE cli_key = ?1 AND model = ?2")
            .map_err(|e| DbWriteError::from_rusqlite("failed to prepare model_price query", e))?;

        let mut stmt = tx
            .prepare_cached(
                r#"
		INSERT INTO request_logs (
		  trace_id,
		  cli_key,
		  session_id,
		  method,
		  path,
		  query,
		  excluded_from_stats,
		  special_settings_json,
		  status,
		  error_code,
		  duration_ms,
		  ttfb_ms,
		  attempts_json,
		  input_tokens,
		  output_tokens,
		  total_tokens,
		  cache_read_input_tokens,
		  cache_creation_input_tokens,
		  cache_creation_5m_input_tokens,
		  cache_creation_1h_input_tokens,
		  usage_json,
		  requested_model,
		  cost_usd_femto,
		  cost_multiplier,
		  created_at_ms,
		  created_at,
		  final_provider_id,
		  provider_chain_json,
		  error_details_json
		) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)
		ON CONFLICT(trace_id) DO UPDATE SET
		  method = excluded.method,
		  path = excluded.path,
		  query = excluded.query,
		  excluded_from_stats = excluded.excluded_from_stats,
	  special_settings_json = excluded.special_settings_json,
	  status = excluded.status,
	  error_code = excluded.error_code,
	  duration_ms = excluded.duration_ms,
	  ttfb_ms = excluded.ttfb_ms,
	  attempts_json = excluded.attempts_json,
	  input_tokens = excluded.input_tokens,
	  output_tokens = excluded.output_tokens,
	  total_tokens = excluded.total_tokens,
	  cache_read_input_tokens = excluded.cache_read_input_tokens,
	  cache_creation_input_tokens = excluded.cache_creation_input_tokens,
	  cache_creation_5m_input_tokens = excluded.cache_creation_5m_input_tokens,
	  cache_creation_1h_input_tokens = excluded.cache_creation_1h_input_tokens,
		  usage_json = excluded.usage_json,
		  requested_model = excluded.requested_model,
		  cost_usd_femto = excluded.cost_usd_femto,
		  cost_multiplier = excluded.cost_multiplier,
		  session_id = excluded.session_id,
		  created_at_ms = CASE
		    WHEN request_logs.created_at_ms = 0 THEN excluded.created_at_ms
		    ELSE request_logs.created_at_ms
		  END,
		  created_at = CASE WHEN request_logs.created_at = 0 THEN excluded.created_at ELSE request_logs.created_at END,
		  final_provider_id = excluded.final_provider_id,
		  provider_chain_json = excluded.provider_chain_json,
		  error_details_json = excluded.error_details_json
		"#,
            )
            .map_err(|e| DbWriteError::from_rusqlite("failed to prepare insert", e))?;

        let mut batch_multiplier: HashMap<i64, f64> = HashMap::new();
        let mut batch_price_json: HashMap<String, Option<String>> = HashMap::new();

        for item in items {
            validate_cli_key(&item.cli_key).map_err(DbWriteError::other)?;

            let attempts = parse_attempts(&item.attempts_json);
            let (final_provider_id, _) = final_provider_from_attempts(&attempts);
            let final_provider_id_db = (final_provider_id > 0).then_some(final_provider_id);

            let cost_multiplier = if final_provider_id > 0 {
                if let Some(v) = batch_multiplier.get(&final_provider_id) {
                    *v
                } else {
                    let cached = cache.get_cost_multiplier(final_provider_id, now_unix);
                    let queried = cached.unwrap_or_else(|| {
                        let value = stmt_multiplier
                            .query_row(params![final_provider_id], |row| row.get::<_, f64>(0))
                            .optional()
                            .unwrap_or(None)
                            .filter(|v| v.is_finite() && *v >= 0.0)
                            .unwrap_or(1.0);
                        cache.put_cost_multiplier(final_provider_id, value, now_unix);
                        value
                    });
                    batch_multiplier.insert(final_provider_id, queried);
                    queried
                }
            } else {
                1.0
            };

            let cost_usd_femto = if is_success_status(item.status, item.error_code.as_deref()) {
                match effective_cost_basis(item) {
                    Some(cost_basis) => {
                        let usage = usage_for_cost(item);
                        if !has_any_cost_usage(&usage) {
                            None
                        } else if cost_multiplier == 0.0 {
                            Some(0)
                        } else {
                            let mut priced_model = cost_basis.model.as_str();
                            let priced_cli_key = cost_basis.cli_key.as_str();
                            let mut price_json = fetch_model_price_json(
                                &mut stmt_price_json,
                                cache,
                                &mut batch_price_json,
                                now_unix,
                                priced_cli_key,
                                priced_model,
                            );

                            if price_json.is_none() {
                                if let Some(target_model) =
                                    price_aliases.resolve_target_model(priced_cli_key, priced_model)
                                {
                                    if target_model != priced_model {
                                        priced_model = target_model;
                                        price_json = fetch_model_price_json(
                                            &mut stmt_price_json,
                                            cache,
                                            &mut batch_price_json,
                                            now_unix,
                                            priced_cli_key,
                                            target_model,
                                        );
                                    }
                                }
                            }

                            match price_json {
                                Some(price_json) => cost::calculate_cost_usd_femto(
                                    &usage,
                                    &price_json,
                                    cost_multiplier,
                                    priced_cli_key,
                                    priced_model,
                                ),
                                None => None,
                            }
                        }
                    }
                    None => None,
                }
            } else {
                None
            };

            stmt.execute(params![
                item.trace_id,
                item.cli_key,
                item.session_id,
                item.method,
                item.path,
                item.query,
                if item.excluded_from_stats { 1i64 } else { 0i64 },
                item.special_settings_json,
                item.status,
                item.error_code,
                item.duration_ms,
                item.ttfb_ms,
                item.attempts_json,
                item.input_tokens,
                item.output_tokens,
                item.total_tokens,
                item.cache_read_input_tokens,
                item.cache_creation_input_tokens,
                item.cache_creation_5m_input_tokens,
                item.cache_creation_1h_input_tokens,
                item.usage_json,
                item.requested_model,
                cost_usd_femto,
                cost_multiplier,
                item.created_at_ms,
                item.created_at,
                final_provider_id_db,
                item.provider_chain_json,
                item.error_details_json,
            ])
            .map_err(|e| DbWriteError::from_rusqlite("failed to insert request_log", e))?;
        }
    }

    tx.commit()
        .map_err(|e| DbWriteError::from_rusqlite("failed to commit transaction", e))?;

    Ok(())
}

pub fn aggregate_by_session_ids(
    db: &db::Db,
    session_ids: &[String],
) -> crate::shared::error::AppResult<HashMap<(String, String), SessionStatsAggregate>> {
    let ids: Vec<String> = session_ids
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .take(5000)
        .map(str::to_string)
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();

    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql = format!(
        r#"
SELECT
  cli_key,
  session_id,
  COUNT(1) AS request_count,
  SUM(COALESCE(input_tokens, 0)) AS total_input_tokens,
  SUM(COALESCE(output_tokens, 0)) AS total_output_tokens,
  SUM(COALESCE(cost_usd_femto, 0)) AS total_cost_usd_femto,
  SUM(duration_ms) AS total_duration_ms
FROM request_logs
WHERE session_id IN ({placeholders})
  AND excluded_from_stats = 0
GROUP BY cli_key, session_id
"#
    );

    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare session aggregate query: {e}"))?;

    let mut rows = stmt
        .query(params_from_iter(ids.iter()))
        .map_err(|e| db_err!("failed to query session aggregates: {e}"))?;

    let mut out: HashMap<(String, String), SessionStatsAggregate> = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| db_err!("failed to read session aggregate row: {e}"))?
    {
        let cli_key: String = row
            .get("cli_key")
            .map_err(|e| db_err!("invalid session aggregate cli_key: {e}"))?;
        let session_id: String = row
            .get("session_id")
            .map_err(|e| db_err!("invalid session aggregate session_id: {e}"))?;
        let request_count: i64 = row
            .get("request_count")
            .map_err(|e| db_err!("invalid session aggregate request_count: {e}"))?;
        let total_input_tokens: i64 = row
            .get("total_input_tokens")
            .map_err(|e| db_err!("invalid session aggregate total_input_tokens: {e}"))?;
        let total_output_tokens: i64 = row
            .get("total_output_tokens")
            .map_err(|e| db_err!("invalid session aggregate total_output_tokens: {e}"))?;
        let total_cost_usd_femto: i64 = row
            .get("total_cost_usd_femto")
            .map_err(|e| db_err!("invalid session aggregate total_cost_usd_femto: {e}"))?;
        let total_duration_ms: i64 = row
            .get("total_duration_ms")
            .map_err(|e| db_err!("invalid session aggregate total_duration_ms: {e}"))?;

        out.insert(
            (cli_key, session_id),
            SessionStatsAggregate {
                request_count: request_count.max(0),
                total_input_tokens: total_input_tokens.max(0),
                total_output_tokens: total_output_tokens.max(0),
                total_cost_usd_femto: total_cost_usd_femto.max(0),
                total_duration_ms: total_duration_ms.max(0),
            },
        );
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{parse_cx2cc_cost_basis, InsertBatchCache, EFFECTIVE_COST_MULTIPLIER_SQL};
    use rusqlite::{params, Connection};

    #[test]
    fn parses_cx2cc_cost_basis_from_special_settings() {
        let special_settings_json = serde_json::json!([
            {
                "type": "provider_lock",
                "scope": "request",
                "providerId": 1
            },
            {
                "type": "cx2cc_cost_basis",
                "scope": "request",
                "source_cli_key": "codex",
                "source_provider_id": 42,
                "priced_model": "gpt-5.4"
            }
        ])
        .to_string();

        assert_eq!(
            parse_cx2cc_cost_basis(Some(&special_settings_json)),
            Some(("codex".to_string(), "gpt-5.4".to_string()))
        );
    }

    #[test]
    fn model_price_cache_does_not_store_none_miss() {
        let mut cache = InsertBatchCache::default();
        let now = 1_770_000_000;
        let key = "claude\\nclaude-opus-4-6".to_string();

        cache.put_model_price_json(key.clone(), None, now);
        assert_eq!(cache.get_model_price_json(&key, now), None);
    }

    #[test]
    fn model_price_cache_stores_hit_value() {
        let mut cache = InsertBatchCache::default();
        let now = 1_770_000_001;
        let key = "claude\\nclaude-opus-4-6".to_string();
        let value = Some(r#"{"input_cost_per_token":"0.000005"}"#.to_string());

        cache.put_model_price_json(key.clone(), value.clone(), now);
        assert_eq!(cache.get_model_price_json(&key, now), Some(value));
    }

    #[test]
    fn effective_cost_multiplier_prefers_cx2cc_source_provider() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  source_provider_id INTEGER
);
INSERT INTO providers (id, cost_multiplier, source_provider_id) VALUES (7, 1.8, NULL);
INSERT INTO providers (id, cost_multiplier, source_provider_id) VALUES (12, 1.2, 7);
            "#,
        )
        .expect("seed providers");

        let multiplier: f64 = conn
            .query_row(EFFECTIVE_COST_MULTIPLIER_SQL, params![12], |row| row.get(0))
            .expect("query multiplier");

        assert_eq!(multiplier, 1.8);
    }

    #[test]
    fn effective_cost_multiplier_falls_back_when_source_deleted() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  source_provider_id INTEGER
);
INSERT INTO providers (id, cost_multiplier, source_provider_id) VALUES (12, 1.2, 999);
            "#,
        )
        .expect("seed providers");

        let multiplier: f64 = conn
            .query_row(EFFECTIVE_COST_MULTIPLIER_SQL, params![12], |row| row.get(0))
            .expect("query multiplier");

        assert_eq!(multiplier, 1.2);
    }

    #[test]
    fn effective_cost_multiplier_returns_own_when_no_source() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  source_provider_id INTEGER
);
INSERT INTO providers (id, cost_multiplier, source_provider_id) VALUES (5, 2.5, NULL);
            "#,
        )
        .expect("seed providers");

        let multiplier: f64 = conn
            .query_row(EFFECTIVE_COST_MULTIPLIER_SQL, params![5], |row| row.get(0))
            .expect("query multiplier");

        assert_eq!(multiplier, 2.5);
    }
}
