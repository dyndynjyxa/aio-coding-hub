//! Usage: Stream finalization context for gateway body relays.

use crate::{circuit_breaker, db, request_logs, session_manager};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::events::FailoverAttempt;

pub(in crate::gateway) struct StreamFinalizeCtx {
    pub(in crate::gateway) app: tauri::AppHandle,
    pub(in crate::gateway) db: db::Db,
    pub(in crate::gateway) log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    pub(in crate::gateway) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(in crate::gateway) session: Arc<session_manager::SessionManager>,
    pub(in crate::gateway) session_id: Option<String>,
    pub(in crate::gateway) sort_mode_id: Option<i64>,
    pub(in crate::gateway) trace_id: String,
    pub(in crate::gateway) cli_key: String,
    pub(in crate::gateway) method: String,
    pub(in crate::gateway) path: String,
    pub(in crate::gateway) query: Option<String>,
    pub(in crate::gateway) excluded_from_stats: bool,
    pub(in crate::gateway) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(in crate::gateway) status: u16,
    pub(in crate::gateway) error_category: Option<&'static str>,
    pub(in crate::gateway) error_code: Option<&'static str>,
    pub(in crate::gateway) started: Instant,
    pub(in crate::gateway) attempts: Vec<FailoverAttempt>,
    pub(in crate::gateway) attempts_json: String,
    pub(in crate::gateway) requested_model: Option<String>,
    pub(in crate::gateway) created_at_ms: i64,
    pub(in crate::gateway) created_at: i64,
    pub(in crate::gateway) provider_cooldown_secs: i64,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) provider_name: String,
    pub(in crate::gateway) base_url: String,
}
