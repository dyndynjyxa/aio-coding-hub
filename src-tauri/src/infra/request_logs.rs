//! Usage: Request log persistence (sqlite buffered writer and queries).

use crate::shared::error::{db_err, AppResult};
use crate::shared::time::now_unix_seconds;
use crate::{cost, db, model_price_aliases};
use rusqlite::{params, params_from_iter, ErrorCode, OptionalExtension, TransactionBehavior};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};

mod types;
pub use types::{
    RequestLogDetail, RequestLogInsert, RequestLogRouteHop, RequestLogSummary,
    SessionStatsAggregate,
};

mod costing;
use costing::{has_any_cost_usage, is_success_status, usage_for_cost};

mod semantics;

mod queries;
use queries::{final_provider_from_attempts, parse_attempts, validate_cli_key};
pub use queries::{
    get_by_id, get_by_trace_id, list_after_id, list_after_id_all, list_recent, list_recent_all,
};

const WRITE_BUFFER_CAPACITY: usize = 512;
const WRITE_BATCH_MAX: usize = 50;
const WRITE_THROUGH_MAX_CONCURRENT: usize = 4;
const INSERT_RETRY_MAX_ATTEMPTS: u32 = 8;
const INSERT_RETRY_BASE_DELAY_MS: u64 = 20;
const INSERT_RETRY_MAX_DELAY_MS: u64 = 500;

const COST_MULTIPLIER_CACHE_MAX_ENTRIES: usize = 256;
const MODEL_PRICE_CACHE_MAX_ENTRIES: usize = 512;
const CACHE_TTL_SECS: i64 = 5 * 60;
const REQUEST_INTERRUPTED_BY_RESTART_ERROR_CODE: &str = "GW_REQUEST_INTERRUPTED_BY_RESTART";
const REQUEST_INTERRUPTED_BY_GATEWAY_STOP_ERROR_CODE: &str =
    "GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP";
const EFFECTIVE_COST_MULTIPLIER_SQL: &str = r#"
SELECT COALESCE(source.cost_multiplier, bridge.cost_multiplier)
FROM providers bridge
LEFT JOIN providers source ON source.id = bridge.source_provider_id
WHERE bridge.id = ?1
"#;

static WRITE_THROUGH_LIMITER: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestLogReconcileReason {
    StartupRecovery,
    GatewayStop,
}

impl RequestLogReconcileReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::StartupRecovery => "startup_recovery",
            Self::GatewayStop => "gateway_stop",
        }
    }

    const fn error_code(self) -> &'static str {
        match self {
            Self::StartupRecovery => REQUEST_INTERRUPTED_BY_RESTART_ERROR_CODE,
            Self::GatewayStop => REQUEST_INTERRUPTED_BY_GATEWAY_STOP_ERROR_CODE,
        }
    }
}

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
        prune_expired_cached_values(&mut self.provider_multiplier, now);
        if self.provider_multiplier.len() >= COST_MULTIPLIER_CACHE_MAX_ENTRIES
            && !self.provider_multiplier.contains_key(&provider_id)
        {
            evict_oldest_cached_value(&mut self.provider_multiplier);
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

        prune_expired_cached_values(&mut self.model_price_json, now);
        if self.model_price_json.len() >= MODEL_PRICE_CACHE_MAX_ENTRIES
            && !self.model_price_json.contains_key(&key)
        {
            evict_oldest_cached_value(&mut self.model_price_json);
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

fn prune_expired_cached_values<K, T>(map: &mut HashMap<K, CachedValue<T>>, now: i64)
where
    K: Eq + Hash,
{
    map.retain(|_, entry| now.saturating_sub(entry.fetched_at) <= CACHE_TTL_SECS);
}

fn evict_oldest_cached_value<K, T>(map: &mut HashMap<K, CachedValue<T>>)
where
    K: Clone + Eq + Hash,
{
    let Some(oldest_key) = map
        .iter()
        .min_by_key(|(_, entry)| entry.fetched_at)
        .map(|(key, _)| key.clone())
    else {
        return;
    };
    map.remove(&oldest_key);
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
pub(crate) struct EffectiveCostBasis {
    pub(crate) cli_key: String,
    pub(crate) model: String,
}

/// Parse `effectivePriority` from `codex_service_tier_result` special setting.
pub(crate) fn parse_effective_priority(special_settings_json: Option<&str>) -> bool {
    let raw = match special_settings_json {
        Some(s) => s.trim(),
        None => return false,
    };
    if raw.is_empty() {
        return false;
    }

    let settings: Vec<Value> = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return false,
    };

    for setting in settings.iter().rev() {
        let Some(obj) = setting.as_object() else {
            continue;
        };
        if obj.get("type").and_then(Value::as_str) != Some("codex_service_tier_result") {
            continue;
        }

        // Check effectivePriority field
        if let Some(effective) = obj.get("effectivePriority").and_then(Value::as_bool) {
            return effective;
        }

        // Legacy compatibility: if no effectivePriority, check actualServiceTier
        if obj.get("billingSourcePreference").is_none() && obj.get("resolvedFrom").is_none() {
            if let Some(actual) = obj.get("actualServiceTier").and_then(Value::as_str) {
                return actual == "priority";
            }
        }
    }

    false
}

pub(crate) fn parse_cx2cc_cost_basis(
    special_settings_json: Option<&str>,
    final_provider_id: Option<i64>,
) -> Option<(String, String)> {
    let semantics::Cx2ccCostBasisResolution::Matched(basis) =
        semantics::resolve_cx2cc_cost_basis(special_settings_json, final_provider_id)
    else {
        return None;
    };
    Some((basis.source_cli_key, basis.priced_model?))
}

#[cfg(test)]
pub(crate) fn cx2cc_openai_input_semantics_override(
    special_settings_json: Option<&str>,
    final_provider_id: Option<i64>,
) -> Option<bool> {
    semantics::resolve_cx2cc_cost_basis(special_settings_json, final_provider_id)
        .openai_input_semantics_override()
}

pub(crate) fn effective_cost_basis(
    cli_key: &str,
    requested_model: Option<&str>,
    special_settings_json: Option<&str>,
    final_provider_id: Option<i64>,
) -> Option<EffectiveCostBasis> {
    if let Some((cli_key, model)) = parse_cx2cc_cost_basis(special_settings_json, final_provider_id)
    {
        return Some(EffectiveCostBasis { cli_key, model });
    }

    let model = requested_model
        .map(str::trim)
        .filter(|v| !v.is_empty())?
        .to_string();

    Some(EffectiveCostBasis {
        cli_key: cli_key.to_string(),
        model,
    })
}

fn write_through_limiter() -> Arc<Semaphore> {
    WRITE_THROUGH_LIMITER
        .get_or_init(|| Arc::new(Semaphore::new(WRITE_THROUGH_MAX_CONCURRENT)))
        .clone()
}

fn try_acquire_write_through_permit(limiter: Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    limiter.try_acquire_owned().ok()
}

pub fn start_buffered_writer<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
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

pub fn spawn_write_through<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    item: RequestLogInsert,
) -> bool {
    let trace_id = item.trace_id.clone();
    let cli_key = item.cli_key.clone();
    let Some(permit) = try_acquire_write_through_permit(write_through_limiter()) else {
        tracing::warn!(
            trace_id = %trace_id,
            cli = %cli_key,
            max_concurrent = WRITE_THROUGH_MAX_CONCURRENT,
            "request log write-through fallback saturated; dropping log"
        );
        return false;
    };

    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut cache = InsertBatchCache::default();
        let items = [item];
        if let Err(err) = insert_batch_with_retries(&app, &db, &items, &mut cache) {
            tracing::error!(error = %err.message, "request log write-through insert failed");
        }
    });
    true
}

pub(crate) fn touch_activity(
    db: &db::Db,
    trace_id: &str,
    cli_key: &str,
    last_activity_ms: i64,
    details: Option<String>,
) -> AppResult<usize> {
    validate_cli_key(cli_key).map_err(crate::shared::error::AppError::from)?;
    let last_activity_ms = last_activity_ms.max(0);
    let conn = db.open_connection()?;
    conn.execute(
        r#"
UPDATE request_logs
SET
  last_activity_ms = CASE
    WHEN last_activity_ms IS NULL OR ?3 > last_activity_ms THEN ?3
    ELSE last_activity_ms
  END,
  activity_details_json = CASE
    WHEN last_activity_ms IS NULL OR ?3 >= last_activity_ms THEN COALESCE(?4, activity_details_json)
    ELSE activity_details_json
  END
WHERE trace_id = ?1
  AND cli_key = ?2
  AND status IS NULL
  AND error_code IS NULL
"#,
        params![trace_id, cli_key, last_activity_ms, details],
    )
    .map_err(|e| db_err!("failed to touch request log activity: {e}"))
}

const RETENTION_PURGE_BATCH_SIZE: usize = 1000;
const RETENTION_PURGE_BATCH_PAUSE_MS: u64 = 50;
const RETENTION_TASK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Deletes request logs older than `retention_days`, in small batches so the
/// write lock is never held long (WAL-friendly). `retention_days == 0` means
/// retention is disabled (keep forever) and nothing is deleted.
pub fn purge_expired(db: &db::Db, retention_days: u32, now_unix: i64) -> AppResult<u64> {
    if retention_days == 0 {
        return Ok(0);
    }
    let cutoff = now_unix.saturating_sub(i64::from(retention_days).saturating_mul(24 * 60 * 60));
    let mut total: u64 = 0;
    loop {
        // Re-acquire per batch so the pooled connection (pool max is small) is
        // not held across the inter-batch pauses of a long purge.
        let conn = db.open_connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM request_logs WHERE id IN (
                   SELECT id FROM request_logs WHERE created_at < ?1 LIMIT ?2
                 )",
                params![cutoff, RETENTION_PURGE_BATCH_SIZE as i64],
            )
            .map_err(|e| db_err!("failed to purge expired request_logs: {e}"))?;
        drop(conn);
        total = total.saturating_add(deleted as u64);
        if deleted < RETENTION_PURGE_BATCH_SIZE {
            break;
        }
        std::thread::sleep(Duration::from_millis(RETENTION_PURGE_BATCH_PAUSE_MS));
    }
    Ok(total)
}

/// Spawns the daily request-log retention job (idempotent). Reads the setting
/// fresh on each tick — fail-open to disabled — so changes apply without a
/// restart. Lives at app level, not in the gateway: retention must not depend
/// on the gateway running.
pub(crate) fn spawn_retention_task(app: tauri::AppHandle, db: db::Db) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        run_retention_once(&app, &db).await;

        let mut interval = tokio::time::interval(RETENTION_TASK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // First tick is immediate; skip it so we don't run twice at startup.
        interval.tick().await;
        loop {
            interval.tick().await;
            run_retention_once(&app, &db).await;
        }
    });
}

async fn run_retention_once(app: &tauri::AppHandle, db: &db::Db) {
    let app = app.clone();
    let db = db.clone();
    let result = crate::blocking::run("request_log_retention", move || {
        let retention_days = crate::settings::request_log_retention_days_fail_open(&app);
        if retention_days == 0 {
            return Ok::<u64, crate::shared::error::AppError>(0);
        }
        let deleted = purge_expired(&db, retention_days, now_unix_seconds())?;
        if deleted > 0 {
            tracing::info!(retention_days, deleted, "purged expired request logs");
        }
        Ok(deleted)
    })
    .await;

    if let Err(err) = result {
        tracing::warn!("request-log retention task failed: {}", err);
    }
}

pub(crate) fn reconcile_unresolved_pending(
    db: &db::Db,
    reason: RequestLogReconcileReason,
    now_ms: i64,
) -> AppResult<usize> {
    let now_ms = now_ms.max(0);
    let conn = db.open_connection()?;
    let pending_age_expr =
        "CASE WHEN created_at_ms > 0 AND ?1 > created_at_ms THEN ?1 - created_at_ms ELSE 0 END";
    let sql = format!(
        r#"
UPDATE request_logs
SET
  status = 499,
  error_code = ?2,
  duration_ms = CASE
    WHEN COALESCE(duration_ms, 0) > {pending_age_expr} THEN COALESCE(duration_ms, 0)
    ELSE {pending_age_expr}
  END,
  excluded_from_stats = 1,
  error_details_json = json_object(
    'reason', ?3,
    'reconciled_at_ms', ?1,
    'pending_age_ms', {pending_age_expr}
  )
WHERE status IS NULL
  AND error_code IS NULL
"#
    );
    let affected = conn
        .execute(&sql, params![now_ms, reason.error_code(), reason.as_str()])
        .map_err(|e| db_err!("failed to reconcile pending request_logs: {e}"))?;
    Ok(affected)
}

fn writer_loop<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    mut rx: mpsc::Receiver<RequestLogInsert>,
) {
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
    app: &tauri::AppHandle<impl tauri::Runtime>,
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
    app: &tauri::AppHandle<impl tauri::Runtime>,
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
		  last_activity_ms,
		  activity_details_json,
		  created_at,
		  final_provider_id,
		  provider_chain_json,
		  error_details_json
		) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31)
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
		  last_activity_ms = CASE
		    WHEN request_logs.last_activity_ms IS NULL THEN excluded.last_activity_ms
		    WHEN excluded.last_activity_ms > request_logs.last_activity_ms THEN excluded.last_activity_ms
		    ELSE request_logs.last_activity_ms
		  END,
		  activity_details_json = COALESCE(request_logs.activity_details_json, excluded.activity_details_json),
		  created_at = CASE WHEN request_logs.created_at = 0 THEN excluded.created_at ELSE request_logs.created_at END,
		  final_provider_id = excluded.final_provider_id,
		  provider_chain_json = excluded.provider_chain_json,
		  error_details_json = excluded.error_details_json
		WHERE NOT (
		  (request_logs.status IS NOT NULL OR request_logs.error_code IS NOT NULL)
		  AND excluded.status IS NULL
		  AND excluded.error_code IS NULL
		)
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
                match effective_cost_basis(
                    &item.cli_key,
                    item.requested_model.as_deref(),
                    item.special_settings_json.as_deref(),
                    final_provider_id_db,
                ) {
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
                                Some(price_json) => {
                                    let priority_applied = parse_effective_priority(
                                        item.special_settings_json.as_deref(),
                                    );
                                    let options = cost::CostCalculationOptions {
                                        priority_service_tier_applied: priority_applied,
                                    };
                                    cost::calculate_cost_usd_femto_with_options(
                                        &usage,
                                        &price_json,
                                        cost_multiplier,
                                        priced_cli_key,
                                        priced_model,
                                        &options,
                                    )
                                }
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
                item.last_activity_ms.unwrap_or(item.created_at_ms),
                item.activity_details_json,
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
    use super::{
        insert_batch_once, parse_cx2cc_cost_basis, purge_expired, reconcile_unresolved_pending,
        touch_activity, try_acquire_write_through_permit, writer_loop, InsertBatchCache,
        RequestLogInsert, RequestLogReconcileReason, COST_MULTIPLIER_CACHE_MAX_ENTRIES,
        EFFECTIVE_COST_MULTIPLIER_SQL, MODEL_PRICE_CACHE_MAX_ENTRIES, WRITE_BATCH_MAX,
    };
    use rusqlite::{params, Connection};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::{mpsc, Semaphore};

    fn request_log_insert(trace_id: &str) -> RequestLogInsert {
        RequestLogInsert {
            trace_id: trace_id.to_string(),
            cli_key: "claude".to_string(),
            session_id: None,
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            query: None,
            excluded_from_stats: false,
            special_settings_json: None,
            status: Some(200),
            error_code: None,
            duration_ms: 10,
            ttfb_ms: Some(5),
            attempts_json: "[]".to_string(),
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            cache_creation_5m_input_tokens: None,
            cache_creation_1h_input_tokens: None,
            usage_json: None,
            requested_model: None,
            provider_chain_json: None,
            error_details_json: None,
            created_at_ms: 1_770_000_000_000,
            last_activity_ms: None,
            activity_details_json: None,
            created_at: 1_770_000_000,
        }
    }

    fn init_test_db() -> (tauri::App<tauri::test::MockRuntime>, crate::db::Db, TempDir) {
        let app = tauri::test::mock_app();
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("request-log-writer.sqlite");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        (app, db, dir)
    }

    fn count_request_logs(db: &crate::db::Db) -> i64 {
        let conn = db.open_connection().expect("open connection");
        conn.query_row("SELECT COUNT(1) FROM request_logs", [], |row| row.get(0))
            .expect("count request logs")
    }

    fn insert_request_log_row(
        db: &crate::db::Db,
        trace_id: &str,
        status: Option<i64>,
        error_code: Option<&str>,
        duration_ms: i64,
        created_at_ms: i64,
    ) {
        let conn = db.open_connection().expect("open connection");
        conn.execute(
            r#"
INSERT INTO request_logs (
  trace_id, cli_key, method, path, status, error_code, duration_ms, attempts_json,
  created_at, created_at_ms, excluded_from_stats
) VALUES (?1, 'claude', 'POST', '/v1/messages', ?2, ?3, ?4, '[]', ?5, ?6, 0)
"#,
            params![
                trace_id,
                status,
                error_code,
                duration_ms,
                created_at_ms.saturating_div(1000),
                created_at_ms,
            ],
        )
        .expect("insert request log row");
    }

    fn fetch_lifecycle_row(
        db: &crate::db::Db,
        trace_id: &str,
    ) -> (Option<i64>, Option<String>, i64, i64, Option<String>) {
        let conn = db.open_connection().expect("open connection");
        conn.query_row(
            r#"
SELECT status, error_code, duration_ms, excluded_from_stats, error_details_json
FROM request_logs
WHERE trace_id = ?1
"#,
            params![trace_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("fetch lifecycle row")
    }

    #[test]
    fn writer_loop_flushes_partial_batch_when_sender_closes() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let db_for_writer = db.clone();
        let (tx, rx) = mpsc::channel(WRITE_BATCH_MAX);

        let writer = std::thread::spawn(move || {
            writer_loop(app_handle, db_for_writer, rx);
        });

        for idx in 0..3 {
            tx.blocking_send(request_log_insert(&format!("trace-partial-{idx}")))
                .expect("send log");
        }
        drop(tx);

        writer.join().expect("writer joins");

        assert_eq!(count_request_logs(&db), 3);
    }

    #[test]
    fn writer_loop_flushes_final_partial_batch_after_full_batches() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let db_for_writer = db.clone();
        let total = WRITE_BATCH_MAX + 3;
        let (tx, rx) = mpsc::channel(total);

        let writer = std::thread::spawn(move || {
            writer_loop(app_handle, db_for_writer, rx);
        });

        for idx in 0..total {
            tx.blocking_send(request_log_insert(&format!("trace-multi-{idx}")))
                .expect("send log");
        }
        drop(tx);

        writer.join().expect("writer joins");

        assert_eq!(count_request_logs(&db), total as i64);
    }

    #[test]
    fn write_through_permit_helper_rejects_when_limiter_full() {
        let limiter = Arc::new(Semaphore::new(2));
        let first = try_acquire_write_through_permit(limiter.clone()).expect("first permit");
        let second = try_acquire_write_through_permit(limiter.clone()).expect("second permit");

        assert!(try_acquire_write_through_permit(limiter.clone()).is_none());

        drop(first);
        assert!(try_acquire_write_through_permit(limiter).is_some());
        drop(second);
    }

    #[test]
    fn purge_expired_deletes_only_rows_older_than_retention() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        let now_unix = 1_770_000_000_i64;
        let day_secs = 24 * 60 * 60;

        insert_batch_once(
            &app_handle,
            &db,
            &[
                RequestLogInsert {
                    created_at: now_unix - 10 * day_secs,
                    created_at_ms: (now_unix - 10 * day_secs) * 1000,
                    ..request_log_insert("trace-purge-old")
                },
                RequestLogInsert {
                    created_at: now_unix - day_secs / 2,
                    created_at_ms: (now_unix - day_secs / 2) * 1000,
                    ..request_log_insert("trace-purge-recent")
                },
            ],
            &mut cache,
        )
        .expect("insert rows");

        let deleted = purge_expired(&db, 7, now_unix).expect("purge");
        assert_eq!(deleted, 1);
        assert_eq!(count_request_logs(&db), 1);

        let conn = db.open_connection().expect("open connection");
        let remaining: String = conn
            .query_row("SELECT trace_id FROM request_logs", [], |row| row.get(0))
            .expect("remaining row");
        assert_eq!(remaining, "trace-purge-recent");
    }

    #[test]
    fn purge_expired_is_disabled_when_retention_is_zero() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        let now_unix = 1_770_000_000_i64;

        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                created_at: now_unix - 400 * 24 * 60 * 60,
                created_at_ms: (now_unix - 400 * 24 * 60 * 60) * 1000,
                ..request_log_insert("trace-purge-disabled")
            }],
            &mut cache,
        )
        .expect("insert row");

        let deleted = purge_expired(&db, 0, now_unix).expect("purge disabled");
        assert_eq!(deleted, 0);
        assert_eq!(count_request_logs(&db), 1);
    }

    #[test]
    fn purge_expired_drains_multiple_batches() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        let now_unix = 1_770_000_000_i64;
        let old_created_at = now_unix - 30 * 24 * 60 * 60;

        // More rows than one purge batch (batch size 1000) to cover the loop.
        let rows: Vec<RequestLogInsert> = (0..1100)
            .map(|index| RequestLogInsert {
                created_at: old_created_at,
                created_at_ms: old_created_at * 1000,
                ..request_log_insert(&format!("trace-purge-batch-{index}"))
            })
            .collect();
        for chunk in rows.chunks(WRITE_BATCH_MAX) {
            insert_batch_once(&app_handle, &db, chunk, &mut cache).expect("insert chunk");
        }

        let deleted = purge_expired(&db, 7, now_unix).expect("purge batches");
        assert_eq!(deleted, 1100);
        assert_eq!(count_request_logs(&db), 0);
    }

    #[test]
    fn request_log_insert_initializes_last_activity_from_created_at() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                status: None,
                error_code: None,
                ..request_log_insert("trace-activity-init")
            }],
            &mut cache,
        )
        .expect("insert placeholder");

        let conn = db.open_connection().expect("open connection");
        let value: i64 = conn
            .query_row(
                "SELECT last_activity_ms FROM request_logs WHERE trace_id = ?1",
                ["trace-activity-init"],
                |row| row.get(0),
            )
            .expect("read last activity");
        assert_eq!(value, 1_770_000_000_000);
    }

    #[test]
    fn touch_activity_only_updates_pending_rows_and_never_moves_backwards() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                status: None,
                error_code: None,
                ..request_log_insert("trace-touch")
            }],
            &mut cache,
        )
        .expect("insert pending");

        touch_activity(
            &db,
            "trace-touch",
            "claude",
            1_770_000_030_000,
            Some(r#"{"chunk_count":1}"#.to_string()),
        )
        .expect("touch newer");
        touch_activity(
            &db,
            "trace-touch",
            "claude",
            1_770_000_010_000,
            Some(r#"{"chunk_count":0}"#.to_string()),
        )
        .expect("older touch ignored");

        let conn = db.open_connection().expect("open connection");
        let row: (i64, Option<String>) = conn
            .query_row(
                "SELECT last_activity_ms, activity_details_json FROM request_logs WHERE trace_id = ?1",
                ["trace-touch"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read activity");
        assert_eq!(row.0, 1_770_000_030_000);
        assert_eq!(row.1.as_deref(), Some(r#"{"chunk_count":1}"#));
        drop(conn);

        insert_batch_once(
            &app_handle,
            &db,
            &[request_log_insert("trace-touch")],
            &mut cache,
        )
        .expect("finalize");
        let changed = touch_activity(&db, "trace-touch", "claude", 1_770_000_060_000, None)
            .expect("touch completed row");
        assert_eq!(changed, 0);
    }

    #[test]
    fn request_log_finalize_preserves_newer_last_activity_from_insert_payload() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();
        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                status: None,
                error_code: None,
                ..request_log_insert("trace-final-activity")
            }],
            &mut cache,
        )
        .expect("insert pending");

        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                last_activity_ms: Some(1_770_000_090_000),
                activity_details_json: Some(r#"{"terminal_signal":"completed"}"#.to_string()),
                ..request_log_insert("trace-final-activity")
            }],
            &mut cache,
        )
        .expect("finalize");

        let conn = db.open_connection().expect("open connection");
        let row: (i64, Option<String>) = conn
            .query_row(
                "SELECT last_activity_ms, activity_details_json FROM request_logs WHERE trace_id = ?1",
                ["trace-final-activity"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read activity");
        assert_eq!(row.0, 1_770_000_090_000);
        assert_eq!(row.1.as_deref(), Some(r#"{"terminal_signal":"completed"}"#));
    }

    #[test]
    fn late_placeholder_does_not_downgrade_terminal_request_log() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let mut cache = InsertBatchCache::default();

        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                attempts_json: r#"[{"outcome":"success"}]"#.to_string(),
                input_tokens: Some(12),
                output_tokens: Some(34),
                total_tokens: Some(46),
                usage_json: Some(r#"{"input_tokens":12,"output_tokens":34}"#.to_string()),
                requested_model: Some("claude-sonnet-4".to_string()),
                provider_chain_json: Some(r#"[{"provider":"anthropic"}]"#.to_string()),
                ..request_log_insert("trace-late-placeholder")
            }],
            &mut cache,
        )
        .expect("insert terminal");

        insert_batch_once(
            &app_handle,
            &db,
            &[RequestLogInsert {
                status: None,
                error_code: None,
                duration_ms: 0,
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                usage_json: None,
                requested_model: None,
                provider_chain_json: None,
                error_details_json: None,
                ..request_log_insert("trace-late-placeholder")
            }],
            &mut cache,
        )
        .expect("insert late placeholder");

        struct TerminalRow {
            status: Option<i64>,
            error_code: Option<String>,
            duration_ms: i64,
            ttfb_ms: Option<i64>,
            input_tokens: Option<i64>,
            output_tokens: Option<i64>,
            total_tokens: Option<i64>,
            attempts_json: String,
            usage_json: Option<String>,
            requested_model: Option<String>,
            provider_chain_json: Option<String>,
        }

        let conn = db.open_connection().expect("open connection");
        let row = conn
            .query_row(
                r#"
SELECT
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  attempts_json,
  usage_json,
  requested_model,
  provider_chain_json
FROM request_logs
WHERE trace_id = ?1
"#,
                ["trace-late-placeholder"],
                |row| {
                    Ok(TerminalRow {
                        status: row.get(0)?,
                        error_code: row.get(1)?,
                        duration_ms: row.get(2)?,
                        ttfb_ms: row.get(3)?,
                        input_tokens: row.get(4)?,
                        output_tokens: row.get(5)?,
                        total_tokens: row.get(6)?,
                        attempts_json: row.get(7)?,
                        usage_json: row.get(8)?,
                        requested_model: row.get(9)?,
                        provider_chain_json: row.get(10)?,
                    })
                },
            )
            .expect("read request log");

        assert_eq!(row.status, Some(200));
        assert_eq!(row.error_code, None);
        assert_eq!(row.duration_ms, 10);
        assert_eq!(row.ttfb_ms, Some(5));
        assert_eq!(row.input_tokens, Some(12));
        assert_eq!(row.output_tokens, Some(34));
        assert_eq!(row.total_tokens, Some(46));
        assert_eq!(row.attempts_json, r#"[{"outcome":"success"}]"#);
        assert_eq!(
            row.usage_json.as_deref(),
            Some(r#"{"input_tokens":12,"output_tokens":34}"#)
        );
        assert_eq!(row.requested_model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(
            row.provider_chain_json.as_deref(),
            Some(r#"[{"provider":"anthropic"}]"#)
        );
    }

    #[test]
    fn reconcile_unresolved_pending_marks_only_pending_rows() {
        let (_app, db, _dir) = init_test_db();
        insert_request_log_row(&db, "trace-pending", None, None, 10, 1_000);
        insert_request_log_row(&db, "trace-success", Some(200), None, 20, 2_000);
        insert_request_log_row(
            &db,
            "trace-failed",
            None,
            Some("GW_UPSTREAM_TIMEOUT"),
            30,
            3_000,
        );

        let affected =
            reconcile_unresolved_pending(&db, RequestLogReconcileReason::StartupRecovery, 11_000)
                .expect("reconcile pending rows");

        assert_eq!(affected, 1);
        assert_eq!(
            fetch_lifecycle_row(&db, "trace-pending"),
            (
                Some(499),
                Some("GW_REQUEST_INTERRUPTED_BY_RESTART".to_string()),
                10_000,
                1,
                Some(
                    r#"{"reason":"startup_recovery","reconciled_at_ms":11000,"pending_age_ms":10000}"#
                        .to_string()
                )
            )
        );
        assert_eq!(
            fetch_lifecycle_row(&db, "trace-success"),
            (Some(200), None, 20, 0, None)
        );
        assert_eq!(
            fetch_lifecycle_row(&db, "trace-failed"),
            (None, Some("GW_UPSTREAM_TIMEOUT".to_string()), 30, 0, None)
        );
    }

    #[test]
    fn reconcile_unresolved_pending_uses_gateway_stop_code_and_never_decreases_duration() {
        let (_app, db, _dir) = init_test_db();
        insert_request_log_row(&db, "trace-longer-duration", None, None, 20_000, 10_000);

        let affected =
            reconcile_unresolved_pending(&db, RequestLogReconcileReason::GatewayStop, 15_000)
                .expect("reconcile pending rows");

        assert_eq!(affected, 1);
        assert_eq!(
            fetch_lifecycle_row(&db, "trace-longer-duration"),
            (
                Some(499),
                Some("GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP".to_string()),
                20_000,
                1,
                Some(
                    r#"{"reason":"gateway_stop","reconciled_at_ms":15000,"pending_age_ms":5000}"#
                        .to_string()
                )
            )
        );
    }

    #[test]
    fn reconcile_unresolved_pending_does_not_use_duration_as_terminal_predicate() {
        let (_app, db, _dir) = init_test_db();
        insert_request_log_row(&db, "trace-nonzero-duration", None, None, 123, 0);

        let affected =
            reconcile_unresolved_pending(&db, RequestLogReconcileReason::StartupRecovery, 20_000)
                .expect("reconcile pending rows");

        assert_eq!(affected, 1);
        assert_eq!(
            fetch_lifecycle_row(&db, "trace-nonzero-duration"),
            (
                Some(499),
                Some("GW_REQUEST_INTERRUPTED_BY_RESTART".to_string()),
                123,
                1,
                Some(
                    r#"{"reason":"startup_recovery","reconciled_at_ms":20000,"pending_age_ms":0}"#
                        .to_string()
                )
            )
        );
    }

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
                "bridge_provider_id": 12,
                "source_cli_key": "codex",
                "source_provider_id": 42,
                "priced_model": "gpt-5.4"
            }
        ])
        .to_string();

        assert_eq!(
            parse_cx2cc_cost_basis(Some(&special_settings_json), Some(12)),
            Some(("codex".to_string(), "gpt-5.4".to_string()))
        );
    }

    #[test]
    fn cx2cc_cost_basis_uses_codex_cache_creation_buckets_when_persisting_cost() {
        let (app, db, _dir) = init_test_db();
        let app_handle = app.handle().clone();
        let conn = db.open_connection().expect("open connection");
        conn.execute(
            r#"
INSERT INTO model_prices (cli_key, model, price_json, created_at, updated_at)
VALUES ('codex', 'gpt-explicit', ?1, 1, 1)
"#,
            [r#"{
              "input_cost_per_token": 0.004,
              "output_cost_per_token": 0.02,
              "cache_read_input_token_cost": 0.001,
              "cache_creation_input_token_cost": 0.006
            }"#],
        )
        .expect("insert explicit model price");
        conn.execute(
            r#"
INSERT INTO model_prices (cli_key, model, price_json, created_at, updated_at)
VALUES ('codex', 'gpt-fallback', ?1, 1, 1)
"#,
            [r#"{
              "input_cost_per_token": 0.004,
              "output_cost_per_token": 0.02,
              "cache_read_input_token_cost": 0.001
            }"#],
        )
        .expect("insert fallback model price");
        conn.execute(
            r#"
INSERT INTO model_prices (cli_key, model, price_json, created_at, updated_at)
VALUES ('claude', 'claude-client-model', '{"input_cost_per_token":0.001}', 1, 1)
"#,
            [],
        )
        .expect("insert Claude model price");
        drop(conn);

        let marker = |priced_model: &str| {
            serde_json::json!([{
                "type": "cx2cc_cost_basis",
                "source_cli_key": "codex",
                "priced_model": priced_model,
            }])
            .to_string()
        };
        let items = [
            RequestLogInsert {
                special_settings_json: Some(marker("gpt-explicit")),
                requested_model: Some("claude-client-model".to_string()),
                input_tokens: Some(1_000),
                output_tokens: Some(50),
                total_tokens: Some(1_050),
                cache_read_input_tokens: Some(100),
                cache_creation_input_tokens: Some(200),
                ..request_log_insert("trace-cx2cc-explicit-cost")
            },
            RequestLogInsert {
                special_settings_json: Some(marker("gpt-fallback")),
                requested_model: Some("claude-client-model".to_string()),
                input_tokens: Some(1_000),
                output_tokens: Some(50),
                total_tokens: Some(1_050),
                cache_read_input_tokens: Some(100),
                cache_creation_input_tokens: Some(200),
                ..request_log_insert("trace-cx2cc-fallback-cost")
            },
            RequestLogInsert {
                special_settings_json: Some(
                    serde_json::json!([{
                        "type": "cx2cc_cost_basis",
                        "bridge_provider_id": 12,
                        "source_cli_key": "codex",
                        "priced_model": "gpt-explicit",
                    }])
                    .to_string(),
                ),
                requested_model: Some("claude-client-model".to_string()),
                attempts_json: serde_json::json!([
                    {
                        "provider_id": 12,
                        "provider_name": "Failed CX2CC",
                        "outcome": "failed",
                        "status": 502
                    },
                    {
                        "provider_id": 13,
                        "provider_name": "Plain Claude",
                        "outcome": "success",
                        "status": 200
                    }
                ])
                .to_string(),
                input_tokens: Some(100),
                total_tokens: Some(100),
                ..request_log_insert("trace-cx2cc-failover-plain-claude-cost")
            },
        ];

        insert_batch_once(&app_handle, &db, &items, &mut InsertBatchCache::default())
            .expect("insert CX2CC request costs");

        let conn = db.open_connection().expect("open connection");
        let read_cost = |trace_id: &str| {
            conn.query_row(
                "SELECT cost_usd_femto FROM request_logs WHERE trace_id = ?1",
                [trace_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("read request cost")
            .expect("request cost should be present")
        };

        assert_eq!(
            read_cost("trace-cx2cc-explicit-cost"),
            5_100_000_000_000_000
        );
        assert_eq!(
            read_cost("trace-cx2cc-fallback-cost"),
            4_900_000_000_000_000
        );
        assert_eq!(
            read_cost("trace-cx2cc-failover-plain-claude-cost"),
            100_000_000_000_000,
            "a failed CX2CC attempt must not price the final plain Claude response"
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
    fn model_price_cache_evicts_oldest_entry_instead_of_clearing_all() {
        let mut cache = InsertBatchCache::default();
        let now = 1_770_000_001;
        let value = Some(r#"{"input_cost_per_token":"0.000005"}"#.to_string());

        for index in 0..MODEL_PRICE_CACHE_MAX_ENTRIES {
            let fetched_at = if index == 0 { now } else { now + 1 };
            cache.put_model_price_json(format!("claude\nmodel-{index}"), value.clone(), fetched_at);
        }

        cache.put_model_price_json("claude\nnew-model".to_string(), value.clone(), now + 2);

        assert_eq!(cache.model_price_json.len(), MODEL_PRICE_CACHE_MAX_ENTRIES);
        assert_eq!(cache.get_model_price_json("claude\nmodel-0", now + 2), None);
        assert_eq!(
            cache.get_model_price_json("claude\nmodel-1", now + 2),
            Some(value.clone())
        );
        assert_eq!(
            cache.get_model_price_json("claude\nnew-model", now + 2),
            Some(value)
        );
    }

    #[test]
    fn cost_multiplier_cache_prunes_expired_entries_before_capacity_eviction() {
        let mut cache = InsertBatchCache::default();
        let expired_now = 10_000;
        cache.put_cost_multiplier(1, 1.1, expired_now - super::CACHE_TTL_SECS - 1);

        for index in 2..=COST_MULTIPLIER_CACHE_MAX_ENTRIES as i64 {
            cache.put_cost_multiplier(index, 1.2, expired_now);
        }

        cache.put_cost_multiplier(99_999, 1.3, expired_now);

        assert!(cache.provider_multiplier.len() <= COST_MULTIPLIER_CACHE_MAX_ENTRIES);
        assert_eq!(cache.get_cost_multiplier(1, expired_now), None);
        assert_eq!(cache.get_cost_multiplier(99_999, expired_now), Some(1.3));
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
