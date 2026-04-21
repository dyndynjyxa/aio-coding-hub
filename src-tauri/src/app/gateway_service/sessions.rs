//! Usage: Gateway active-session aggregation for IPC and diagnostics.

use crate::gateway_runtime_access::app_gateway_active_sessions;
use crate::shared::error::AppResult;
use crate::{blocking, db, providers, request_logs};

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

    Ok(sessions
        .into_iter()
        .map(|session| {
            let cli_key = session.cli_key;
            let session_id = session.session_id;
            let session_suffix = session.session_suffix;
            let provider_id = session.provider_id;
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
