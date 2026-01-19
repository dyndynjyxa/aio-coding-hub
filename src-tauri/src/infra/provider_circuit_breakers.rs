//! Usage: Persist provider circuit breaker state to sqlite (buffered writer + load helpers).

use crate::{circuit_breaker, db};
use rusqlite::{params, params_from_iter};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

const WRITE_BUFFER_CAPACITY: usize = 512;
const WRITE_BATCH_MAX: usize = 200;

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn u32_from_i64(value: i64) -> u32 {
    if value <= 0 {
        return 0;
    }
    if value > u32::MAX as i64 {
        return u32::MAX;
    }
    value as u32
}

pub fn start_buffered_writer(
    app: tauri::AppHandle,
) -> (
    mpsc::Sender<circuit_breaker::CircuitPersistedState>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel::<circuit_breaker::CircuitPersistedState>(WRITE_BUFFER_CAPACITY);
    let task = tauri::async_runtime::spawn_blocking(move || {
        writer_loop(app, rx);
    });
    (tx, task)
}

fn writer_loop(
    app: tauri::AppHandle,
    mut rx: mpsc::Receiver<circuit_breaker::CircuitPersistedState>,
) {
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

        if let Err(err) = insert_batch(&app, &buffer) {
            eprintln!("provider_circuit_breakers insert_batch error: {err}");
        }
        buffer.clear();
    }

    if !buffer.is_empty() {
        if let Err(err) = insert_batch(&app, &buffer) {
            eprintln!("provider_circuit_breakers final insert_batch error: {err}");
        }
    }
}

fn insert_batch(
    app: &tauri::AppHandle,
    items: &[circuit_breaker::CircuitPersistedState],
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let mut latest_by_provider: HashMap<i64, circuit_breaker::CircuitPersistedState> =
        HashMap::with_capacity(items.len().min(WRITE_BATCH_MAX));
    for item in items {
        latest_by_provider.insert(item.provider_id, item.clone());
    }

    let mut conn = db::open_connection(app)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    {
        let mut stmt = tx
            .prepare(
                r#"
INSERT INTO provider_circuit_breakers (
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(provider_id) DO UPDATE SET
  state = excluded.state,
  failure_count = excluded.failure_count,
  open_until = excluded.open_until,
  updated_at = excluded.updated_at
"#,
            )
            .map_err(|e| format!("DB_ERROR: failed to prepare circuit breaker upsert: {e}"))?;

        for item in latest_by_provider.values() {
            let updated_at = if item.updated_at > 0 {
                item.updated_at
            } else {
                now_unix_seconds()
            };

            stmt.execute(params![
                item.provider_id,
                item.state.as_str(),
                item.failure_count as i64,
                item.open_until,
                updated_at
            ])
            .map_err(|e| format!("DB_ERROR: failed to upsert provider_circuit_breaker: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("DB_ERROR: failed to commit transaction: {e}"))?;

    Ok(())
}

pub fn load_all(
    app: &tauri::AppHandle,
) -> Result<HashMap<i64, circuit_breaker::CircuitPersistedState>, String> {
    let conn = db::open_connection(app)?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT
  provider_id,
  state,
  failure_count,
  open_until,
  updated_at
FROM provider_circuit_breakers
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare circuit breaker load query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let raw_state: String = row.get("state")?;
            let open_until: Option<i64> = row.get("open_until")?;
            Ok(circuit_breaker::CircuitPersistedState {
                provider_id: row.get("provider_id")?,
                state: circuit_breaker::CircuitState::from_str(&raw_state),
                failure_count: u32_from_i64(row.get::<_, i64>("failure_count")?),
                open_until,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to query circuit breaker states: {e}"))?;

    let mut items = HashMap::new();
    for row in rows {
        let item =
            row.map_err(|e| format!("DB_ERROR: failed to read circuit breaker state: {e}"))?;
        items.insert(item.provider_id, item);
    }

    Ok(items)
}

pub fn delete_by_provider_id(app: &tauri::AppHandle, provider_id: i64) -> Result<usize, String> {
    if provider_id <= 0 {
        return Ok(0);
    }
    let conn = db::open_connection(app)?;
    conn.execute(
        "DELETE FROM provider_circuit_breakers WHERE provider_id = ?1",
        params![provider_id],
    )
    .map_err(|e| format!("DB_ERROR: failed to delete circuit breaker state: {e}"))
}

pub fn delete_by_provider_ids(
    app: &tauri::AppHandle,
    provider_ids: &[i64],
) -> Result<usize, String> {
    let ids: Vec<i64> = provider_ids.iter().copied().filter(|id| *id > 0).collect();

    if ids.is_empty() {
        return Ok(0);
    }

    let placeholders = db::sql_placeholders(ids.len());
    let sql =
        format!("DELETE FROM provider_circuit_breakers WHERE provider_id IN ({placeholders})");

    let conn = db::open_connection(app)?;
    conn.execute(&sql, params_from_iter(ids.iter()))
        .map_err(|e| format!("DB_ERROR: failed to delete circuit breaker states: {e}"))
}
