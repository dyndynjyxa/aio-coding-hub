//! Usage: Persist provider circuit breaker state to sqlite (buffered writer + load helpers).

use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::{circuit_breaker, db};
use rusqlite::{params, params_from_iter, ErrorCode, TransactionBehavior};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;

const WRITE_BUFFER_CAPACITY: usize = 512;
const WRITE_BATCH_MAX: usize = 200;
const INSERT_RETRY_MAX_ATTEMPTS: u32 = 6;
const INSERT_RETRY_BASE_DELAY_MS: u64 = 20;
const INSERT_RETRY_MAX_DELAY_MS: u64 = 400;
const FAILURE_TIMESTAMPS_JSON_MAX_BYTES: usize = circuit_breaker::MAX_FAILURE_TIMESTAMPS * 24 + 2;

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

fn bounded_failure_timestamp_slice(timestamps: &[u64]) -> &[u64] {
    let start = timestamps
        .len()
        .saturating_sub(circuit_breaker::MAX_FAILURE_TIMESTAMPS);
    &timestamps[start..]
}

fn serialize_failure_timestamps(timestamps: &[u64]) -> String {
    serde_json::to_string(bounded_failure_timestamp_slice(timestamps))
        .unwrap_or_else(|_| "[]".to_string())
}

fn is_inert_closed_state(item: &circuit_breaker::CircuitPersistedState) -> bool {
    item.provider_id > 0
        && item.state == circuit_breaker::CircuitState::Closed
        && item.failure_timestamps.is_empty()
        && item.half_open_success_count == 0
        && item.open_until.is_none()
}

fn deserialize_failure_timestamps(raw: &str) -> Vec<u64> {
    if raw.len() > FAILURE_TIMESTAMPS_JSON_MAX_BYTES {
        tracing::warn!(
            bytes = raw.len(),
            max_bytes = FAILURE_TIMESTAMPS_JSON_MAX_BYTES,
            "ignoring oversized circuit breaker failure timestamp history"
        );
        return Vec::new();
    }

    let mut timestamps: Vec<u64> = serde_json::from_str(raw).unwrap_or_default();
    if timestamps.len() > circuit_breaker::MAX_FAILURE_TIMESTAMPS {
        let excess = timestamps.len() - circuit_breaker::MAX_FAILURE_TIMESTAMPS;
        timestamps.drain(..excess);
    }
    timestamps
}

pub fn start_buffered_writer(
    db: db::Db,
) -> (
    mpsc::Sender<circuit_breaker::CircuitPersistedState>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel::<circuit_breaker::CircuitPersistedState>(WRITE_BUFFER_CAPACITY);
    let task = tauri::async_runtime::spawn_blocking(move || {
        writer_loop(db, rx);
    });
    (tx, task)
}

fn writer_loop(db: db::Db, mut rx: mpsc::Receiver<circuit_breaker::CircuitPersistedState>) {
    let mut buffer: Vec<circuit_breaker::CircuitPersistedState> =
        Vec::with_capacity(WRITE_BATCH_MAX);

    while let Some(item) = rx.blocking_recv() {
        buffer.push(item);

        while buffer.len() < WRITE_BATCH_MAX {
            match rx.try_recv() {
                Ok(next) => buffer.push(next),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        if let Err(err) = insert_batch_with_retries(&db, &buffer) {
            tracing::error!(error = %err.message, "circuit breaker state batch insert failed");
        }
        buffer.clear();
    }

    if !buffer.is_empty() {
        if let Err(err) = insert_batch_with_retries(&db, &buffer) {
            tracing::error!(error = %err.message, "circuit breaker state final batch insert failed");
        }
    }
}

fn insert_batch_with_retries(
    db: &db::Db,
    items: &[circuit_breaker::CircuitPersistedState],
) -> Result<(), DbWriteError> {
    if items.is_empty() {
        return Ok(());
    }

    let mut attempt: u32 = 0;
    loop {
        match insert_batch_once(db, items) {
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
                    "sqlite busy/locked; retrying provider_circuit_breakers insert"
                );
                std::thread::sleep(delay);
            }
        }
    }
}

fn insert_batch_once(
    db: &db::Db,
    items: &[circuit_breaker::CircuitPersistedState],
) -> Result<(), DbWriteError> {
    let mut latest_by_provider: HashMap<i64, circuit_breaker::CircuitPersistedState> =
        HashMap::with_capacity(items.len().min(WRITE_BATCH_MAX));
    for item in items {
        latest_by_provider.insert(item.provider_id, item.clone());
    }

    let mut conn = db
        .open_connection()
        .map_err(|e| DbWriteError::other(e.to_string()))?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| DbWriteError::from_rusqlite("failed to start transaction", e))?;

    {
        let mut stmt = tx
            .prepare_cached("DELETE FROM provider_circuit_breakers WHERE provider_id = ?1")
            .map_err(|e| {
                DbWriteError::from_rusqlite("failed to prepare circuit breaker delete", e)
            })?;

        for item in latest_by_provider.values() {
            if !is_inert_closed_state(item) {
                continue;
            }
            stmt.execute(params![item.provider_id]).map_err(|e| {
                DbWriteError::from_rusqlite("failed to delete inert provider_circuit_breaker", e)
            })?;
        }
    }

    {
        let mut stmt = tx
            .prepare_cached(
                r#"
INSERT INTO provider_circuit_breakers (
  provider_id,
  state,
  failure_count,
  failure_timestamps_json,
  half_open_success_count,
  open_until,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
ON CONFLICT(provider_id) DO UPDATE SET
  state = excluded.state,
  failure_count = excluded.failure_count,
  failure_timestamps_json = excluded.failure_timestamps_json,
  half_open_success_count = excluded.half_open_success_count,
  open_until = excluded.open_until,
  updated_at = excluded.updated_at
"#,
            )
            .map_err(|e| {
                DbWriteError::from_rusqlite("failed to prepare circuit breaker upsert", e)
            })?;

        for item in latest_by_provider.values() {
            if is_inert_closed_state(item) {
                continue;
            }

            let updated_at = if item.updated_at > 0 {
                item.updated_at
            } else {
                now_unix_seconds()
            };

            let bounded_timestamps = bounded_failure_timestamp_slice(&item.failure_timestamps);
            let timestamps_json = serialize_failure_timestamps(bounded_timestamps);
            let failure_count = bounded_timestamps.len().min(u32::MAX as usize) as i64;

            stmt.execute(params![
                item.provider_id,
                item.state.as_str(),
                failure_count,
                timestamps_json,
                item.half_open_success_count as i64,
                item.open_until,
                updated_at
            ])
            .map_err(|e| {
                DbWriteError::from_rusqlite("failed to upsert provider_circuit_breaker", e)
            })?;
        }
    }

    tx.commit()
        .map_err(|e| DbWriteError::from_rusqlite("failed to commit transaction", e))?;

    Ok(())
}

pub fn load_all(
    db: &db::Db,
) -> crate::shared::error::AppResult<HashMap<i64, circuit_breaker::CircuitPersistedState>> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      provider_id,
      state,
      failure_timestamps_json,
      half_open_success_count,
      open_until,
      updated_at
    FROM provider_circuit_breakers
    "#,
        )
        .map_err(|e| db_err!("failed to prepare circuit breaker load query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let raw_state: String = row.get("state")?;
            let open_until: Option<i64> = row.get("open_until")?;
            let timestamps_json: String = row
                .get::<_, String>("failure_timestamps_json")
                .unwrap_or_else(|_| "[]".to_string());
            let half_open_success_count: i64 =
                row.get::<_, i64>("half_open_success_count").unwrap_or(0);
            Ok(circuit_breaker::CircuitPersistedState {
                provider_id: row.get("provider_id")?,
                state: circuit_breaker::CircuitState::from_str(&raw_state),
                failure_timestamps: deserialize_failure_timestamps(&timestamps_json),
                half_open_success_count: half_open_success_count.max(0).min(u32::MAX as i64) as u32,
                open_until,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| db_err!("failed to query circuit breaker states: {e}"))?;

    let mut items = HashMap::new();
    for row in rows {
        let item = row.map_err(|e| db_err!("failed to read circuit breaker state: {e}"))?;
        if is_inert_closed_state(&item) {
            continue;
        }
        items.insert(item.provider_id, item);
    }

    Ok(items)
}

pub fn delete_by_provider_id(
    db: &db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<usize> {
    if provider_id <= 0 {
        return Ok(0);
    }
    let conn = db.open_connection()?;
    conn.execute(
        "DELETE FROM provider_circuit_breakers WHERE provider_id = ?1",
        params![provider_id],
    )
    .map_err(|e| db_err!("failed to delete circuit breaker state: {e}"))
}

pub fn delete_by_provider_ids(
    db: &db::Db,
    provider_ids: &[i64],
) -> crate::shared::error::AppResult<usize> {
    let ids: Vec<i64> = provider_ids.iter().copied().filter(|id| *id > 0).collect();

    if ids.is_empty() {
        return Ok(0);
    }

    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql =
        format!("DELETE FROM provider_circuit_breakers WHERE provider_id IN ({placeholders})");

    let conn = db.open_connection()?;
    conn.execute(&sql, params_from_iter(ids.iter()))
        .map_err(|e| db_err!("failed to delete circuit breaker states: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_db() -> (tempfile::TempDir, db::Db) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("provider_circuit_breakers.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        (dir, db)
    }

    fn insert_test_provider(db: &db::Db, provider_id: i64) {
        let conn = db.open_connection().expect("open db");
        conn.execute(
            r#"
            INSERT INTO providers(
              id,
              cli_key,
              name,
              base_url,
              api_key_plaintext,
              enabled,
              priority,
              created_at,
              updated_at,
              sort_order,
              cost_multiplier,
              base_urls_json,
              base_url_mode,
              supported_models_json,
              model_mapping_json
            ) VALUES (?1, 'test-cli', ?2, 'https://example.test', '', 1, 100, 1, 1, 0, 1.0, '[]', 'order', '{}', '{}')
            "#,
            params![provider_id, format!("provider-{provider_id}")],
        )
        .expect("insert provider");
    }

    fn oversized_timestamps() -> Vec<u64> {
        (0..(circuit_breaker::MAX_FAILURE_TIMESTAMPS + 3))
            .map(|value| value as u64)
            .collect()
    }

    #[test]
    fn deserialize_failure_timestamps_keeps_most_recent_capped_entries() {
        let raw = serde_json::to_string(&oversized_timestamps()).expect("serialize timestamps");

        let timestamps = deserialize_failure_timestamps(&raw);

        assert_eq!(timestamps.len(), circuit_breaker::MAX_FAILURE_TIMESTAMPS);
        assert_eq!(timestamps.first().copied(), Some(3));
        assert_eq!(
            timestamps.last().copied(),
            Some((circuit_breaker::MAX_FAILURE_TIMESTAMPS + 2) as u64)
        );
    }

    #[test]
    fn deserialize_failure_timestamps_rejects_oversized_json_before_parse() {
        let raw = format!("[{}]", "1,".repeat(FAILURE_TIMESTAMPS_JSON_MAX_BYTES));
        assert!(raw.len() > FAILURE_TIMESTAMPS_JSON_MAX_BYTES);

        let timestamps = deserialize_failure_timestamps(&raw);

        assert!(timestamps.is_empty());
    }

    #[test]
    fn serialize_failure_timestamps_writes_only_capped_entries() {
        let raw = serialize_failure_timestamps(&oversized_timestamps());
        let timestamps: Vec<u64> = serde_json::from_str(&raw).expect("parse serialized timestamps");

        assert_eq!(timestamps.len(), circuit_breaker::MAX_FAILURE_TIMESTAMPS);
        assert_eq!(timestamps.first().copied(), Some(3));
    }

    #[test]
    fn load_all_skips_legacy_inert_closed_rows() {
        let (_dir, db) = init_test_db();
        insert_test_provider(&db, 1);
        insert_test_provider(&db, 2);

        let conn = db.open_connection().expect("open db");
        conn.execute(
            r#"
            INSERT INTO provider_circuit_breakers(
              provider_id,
              state,
              failure_count,
              failure_timestamps_json,
              half_open_success_count,
              open_until,
              updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                1_i64,
                "CLOSED",
                0_i64,
                "[]",
                0_i64,
                Option::<i64>::None,
                1_i64
            ],
        )
        .expect("insert inert closed row");
        conn.execute(
            r#"
            INSERT INTO provider_circuit_breakers(
              provider_id,
              state,
              failure_count,
              failure_timestamps_json,
              half_open_success_count,
              open_until,
              updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                2_i64,
                "CLOSED",
                1_i64,
                "[42]",
                0_i64,
                Option::<i64>::None,
                1_i64
            ],
        )
        .expect("insert active closed row");
        drop(conn);

        let loaded = load_all(&db).expect("load circuit states");

        assert!(!loaded.contains_key(&1));
        assert!(loaded.contains_key(&2));
    }

    #[test]
    fn insert_batch_deletes_inert_closed_rows() {
        let (_dir, db) = init_test_db();
        insert_test_provider(&db, 7);

        let open = circuit_breaker::CircuitPersistedState {
            provider_id: 7,
            state: circuit_breaker::CircuitState::Open,
            failure_timestamps: vec![10],
            half_open_success_count: 0,
            open_until: Some(100),
            updated_at: 10,
        };
        insert_batch_once(&db, &[open]).expect("insert open state");
        assert!(load_all(&db).expect("load open state").contains_key(&7));

        let inert_closed = circuit_breaker::CircuitPersistedState {
            provider_id: 7,
            state: circuit_breaker::CircuitState::Closed,
            failure_timestamps: Vec::new(),
            half_open_success_count: 0,
            open_until: None,
            updated_at: 20,
        };
        insert_batch_once(&db, &[inert_closed]).expect("delete inert closed state");

        let loaded = load_all(&db).expect("load after delete");
        assert!(!loaded.contains_key(&7));

        let conn = db.open_connection().expect("open db");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_circuit_breakers WHERE provider_id = ?1",
                params![7_i64],
                |row| row.get(0),
            )
            .expect("count circuit rows");
        assert_eq!(count, 0);
    }
}
