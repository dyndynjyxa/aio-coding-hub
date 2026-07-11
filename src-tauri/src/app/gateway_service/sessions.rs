//! Usage: Gateway active-session aggregation for IPC and diagnostics.

use crate::gateway_runtime_access::{
    app_gateway_active_sessions, app_gateway_pin_session_sort_mode,
    app_gateway_unpin_session_sort_mode,
};
use crate::shared::error::AppResult;
use crate::{blocking, db, providers, request_logs, session_pins};

const GATEWAY_SESSIONS_DEFAULT_LIMIT: u32 = 50;
const GATEWAY_SESSIONS_MAX_LIMIT: u32 = 200;
const USD_FEMTO_DIVISOR: f64 = 1_000_000_000_000_000.0;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct GatewayActiveSessionSummary {
    cli_key: String,
    session_id: String,
    session_suffix: String,
    provider_id: i64,
    provider_name: String,
    /// Whether this session has an in-memory (ephemeral) sort_mode pin.
    sort_mode_pinned: bool,
    /// The ephemeral pinned sort_mode id. `null` while `sort_mode_pinned` is true means Default.
    pinned_sort_mode_id: Option<i64>,
    /// Whether this session has a persistent (disk-backed) sort_mode pin.
    persistent_pinned: bool,
    /// The persistent pinned sort_mode id. `null` while `persistent_pinned` is true means Default.
    persistent_pinned_sort_mode_id: Option<i64>,
    expires_at: i64,
    request_count: Option<i64>,
    total_input_tokens: Option<i64>,
    total_output_tokens: Option<i64>,
    total_cost_usd: Option<f64>,
    total_duration_ms: Option<i64>,
}

fn gateway_sessions_limit(limit: Option<u32>) -> usize {
    limit
        .unwrap_or(GATEWAY_SESSIONS_DEFAULT_LIMIT)
        .clamp(1, GATEWAY_SESSIONS_MAX_LIMIT) as usize
}

/// Pin a sort_mode (routing template) to a `(cli_key, session_id)` binding.
///
/// `sort_mode_id == None` pins to Default (always valid). A `Some(id)` must
/// reference an existing sort_mode, so the UI gets immediate feedback instead of
/// a silently-ignored pin. Returns `false` when the gateway is not running.
pub(crate) async fn pin_session_sort_mode<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    cli_key: String,
    session_id: String,
    sort_mode_id: Option<i64>,
) -> AppResult<bool> {
    if session_id.trim().is_empty() {
        return Err(crate::shared::error::AppError::from(
            "SEC_INVALID_INPUT: session_id is empty",
        ));
    }

    if let Some(mode_id) = sort_mode_id {
        let exists = blocking::run("gateway_pin_sort_mode_lookup", move || {
            crate::sort_modes::list_modes(&db).map(|modes| modes.iter().any(|m| m.id == mode_id))
        })
        .await?;

        if !exists {
            return Err(crate::shared::error::AppError::from(
                "SEC_INVALID_INPUT: sort_mode does not exist",
            ));
        }
    }

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    Ok(app_gateway_pin_session_sort_mode(
        &app,
        &cli_key,
        session_id.trim(),
        sort_mode_id,
        now_unix,
    ))
}

/// Clear a session's manual sort_mode pin, reverting it to the auto-inherited
/// active strategy. Returns `false` when the gateway is not running or there is
/// no live binding for this session.
pub(crate) async fn unpin_session_sort_mode<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    cli_key: String,
    session_id: String,
) -> AppResult<bool> {
    if session_id.trim().is_empty() {
        return Err(crate::shared::error::AppError::from(
            "SEC_INVALID_INPUT: session_id is empty",
        ));
    }

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    Ok(app_gateway_unpin_session_sort_mode(
        &app,
        &cli_key,
        session_id.trim(),
        now_unix,
    ))
}

/// Create or update a **persistent** (disk-backed) sort_mode pin for a session.
/// `sort_mode_id == None` pins to Default. Validated against existing sort_modes.
pub(crate) async fn persist_session_sort_mode(
    db: db::Db,
    cli_key: String,
    session_id: String,
    sort_mode_id: Option<i64>,
) -> AppResult<bool> {
    if session_id.trim().is_empty() {
        return Err(crate::shared::error::AppError::from(
            "SEC_INVALID_INPUT: session_id is empty",
        ));
    }
    blocking::run("gateway_persist_session_sort_mode", move || {
        session_pins::upsert_persistent_pin(&db, &cli_key, session_id.trim(), sort_mode_id)
    })
    .await?;
    Ok(true)
}

/// Remove a session's **persistent** pin. Returns `true` if a row was deleted.
pub(crate) async fn unpersist_session_sort_mode(
    db: db::Db,
    cli_key: String,
    session_id: String,
) -> AppResult<bool> {
    if session_id.trim().is_empty() {
        return Err(crate::shared::error::AppError::from(
            "SEC_INVALID_INPUT: session_id is empty",
        ));
    }
    blocking::run("gateway_unpersist_session_sort_mode", move || {
        session_pins::delete_persistent_pin(&db, &cli_key, session_id.trim())
    })
    .await
}

/// List all persistent pins (for management / cross-UI visibility).
pub(crate) async fn list_persistent_pins(
    db: db::Db,
) -> AppResult<Vec<session_pins::PersistentPinRow>> {
    blocking::run("gateway_list_persistent_pins", move || {
        session_pins::list_persistent_pins(&db)
    })
    .await
}

pub(crate) async fn list_active_sessions<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    limit: Option<u32>,
) -> AppResult<Vec<GatewayActiveSessionSummary>> {
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    let sessions = app_gateway_active_sessions(&app, now_unix, gateway_sessions_limit(limit));
    if sessions.is_empty() {
        return Ok(Vec::new());
    }

    let provider_ids: Vec<i64> = sessions.iter().map(|session| session.provider_id).collect();
    let session_ids: Vec<String> = sessions
        .iter()
        .map(|session| session.session_id.clone())
        .collect();

    let db_for_names = db.clone();
    let provider_names = blocking::run("providers_names_by_id", move || {
        providers::names_by_id(&db_for_names, &provider_ids)
    })
    .await?;

    let db_for_agg = db.clone();
    let session_stats = blocking::run("request_logs_aggregate_by_session_ids", move || {
        request_logs::aggregate_by_session_ids(&db_for_agg, &session_ids)
    })
    .await?;

    // Persistent pins: fetch all once, index by (cli_key, session_id).
    let db_for_pins = db.clone();
    let persistent_pins = blocking::run("gateway_list_persistent_pins", move || {
        session_pins::list_persistent_pins(&db_for_pins)
    })
    .await
    .unwrap_or_default();
    let persistent_by_key: std::collections::HashMap<(String, String), Option<i64>> =
        persistent_pins
            .into_iter()
            .map(|row| ((row.cli_key, row.session_id), row.sort_mode_id))
            .collect();

    Ok(sessions
        .into_iter()
        .map(|session| {
            let cli_key = session.cli_key;
            let session_id = session.session_id;
            let session_suffix = session.session_suffix;
            let provider_id = session.provider_id;
            let (sort_mode_pinned, pinned_sort_mode_id) = match session.pinned_sort_mode_id {
                Some(mode) => (true, mode),
                None => (false, None),
            };
            let (persistent_pinned, persistent_pinned_sort_mode_id) =
                match persistent_by_key.get(&(cli_key.clone(), session_id.clone())) {
                    Some(mode) => (true, *mode),
                    None => (false, None),
                };
            let expires_at = session.expires_at;

            let provider_name = provider_names
                .get(&provider_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let stats = session_stats.get(&(cli_key.clone(), session_id.clone()));

            GatewayActiveSessionSummary {
                cli_key,
                session_id,
                session_suffix,
                provider_id,
                provider_name,
                sort_mode_pinned,
                pinned_sort_mode_id,
                persistent_pinned,
                persistent_pinned_sort_mode_id,
                expires_at,
                request_count: stats
                    .map(|row| row.request_count)
                    .filter(|value| *value > 0),
                total_input_tokens: stats
                    .map(|row| row.total_input_tokens)
                    .filter(|value| *value > 0),
                total_output_tokens: stats
                    .map(|row| row.total_output_tokens)
                    .filter(|value| *value > 0),
                total_cost_usd: stats
                    .map(|row| row.total_cost_usd_femto)
                    .filter(|value| *value > 0)
                    .map(|value| value as f64 / USD_FEMTO_DIVISOR),
                total_duration_ms: stats
                    .map(|row| row.total_duration_ms)
                    .filter(|value| *value > 0),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::gateway_sessions_limit;

    #[test]
    fn gateway_sessions_limit_uses_default_and_clamps() {
        assert_eq!(gateway_sessions_limit(None), 50);
        assert_eq!(gateway_sessions_limit(Some(0)), 1);
        assert_eq!(gateway_sessions_limit(Some(999)), 200);
        assert_eq!(gateway_sessions_limit(Some(88)), 88);
    }
}
