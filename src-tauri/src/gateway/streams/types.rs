//! Usage: Stream finalization context for gateway body relays.

use crate::gateway::active_requests::ActiveRequestRegistry;
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use crate::{circuit_breaker, db, request_logs, session_manager};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::events::FailoverAttempt;

const ACTIVITY_FLUSH_INTERVAL_MS: i64 = 30_000;

pub(in crate::gateway) struct StreamActivityTracker {
    trace_id: String,
    cli_key: String,
    created_at_ms: i64,
    last_activity_ms: i64,
    last_flushed_activity_ms: i64,
    chunk_count: i64,
}

impl StreamActivityTracker {
    pub(in crate::gateway) fn new(trace_id: &str, cli_key: &str, created_at_ms: i64) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            cli_key: cli_key.to_string(),
            created_at_ms,
            last_activity_ms: created_at_ms,
            last_flushed_activity_ms: created_at_ms,
            chunk_count: 0,
        }
    }

    pub(in crate::gateway) fn observe_chunk_at(&mut self, now_ms: i64) -> bool {
        self.chunk_count = self.chunk_count.saturating_add(1);
        self.last_activity_ms = now_ms.max(self.last_activity_ms).max(self.created_at_ms);
        if self
            .last_activity_ms
            .saturating_sub(self.last_flushed_activity_ms)
            < ACTIVITY_FLUSH_INTERVAL_MS
        {
            return false;
        }
        self.last_flushed_activity_ms = self.last_activity_ms;
        true
    }

    pub(in crate::gateway) fn last_activity_ms(&self) -> i64 {
        self.last_activity_ms
    }

    pub(in crate::gateway) fn details_json(&self, terminal_signal: Option<&str>) -> Option<String> {
        serde_json::to_string(&serde_json::json!({
            "trace_id": self.trace_id,
            "cli_key": self.cli_key,
            "chunk_count": self.chunk_count,
            "last_activity_ms": self.last_activity_ms,
            "terminal_signal": terminal_signal,
        }))
        .ok()
    }
}

pub(in crate::gateway) struct StreamFinalizeCtx<R: tauri::Runtime = tauri::Wry> {
    pub(in crate::gateway) app: tauri::AppHandle<R>,
    pub(in crate::gateway) db: db::Db,
    pub(in crate::gateway) log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    pub(in crate::gateway) plugin_pipeline: Arc<GatewayPluginPipeline>,
    pub(in crate::gateway) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(in crate::gateway) session: Arc<session_manager::SessionManager>,
    pub(in crate::gateway) session_id: Option<String>,
    pub(in crate::gateway) sort_mode_id: Option<i64>,
    pub(in crate::gateway) trace_id: String,
    pub(in crate::gateway) cli_key: String,
    pub(in crate::gateway) method: String,
    pub(in crate::gateway) path: String,
    pub(in crate::gateway) observe: bool,
    pub(in crate::gateway) query: Option<String>,
    pub(in crate::gateway) excluded_from_stats: bool,
    pub(in crate::gateway) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(in crate::gateway) provider_health_neutral: bool,
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
    pub(in crate::gateway) auth_mode: String,
    pub(in crate::gateway) fake_200_detected: bool,
    pub(in crate::gateway) fake_200_quota_exhausted: bool,
    pub(in crate::gateway) activity: Arc<Mutex<StreamActivityTracker>>,
    pub(in crate::gateway) active_requests: Arc<ActiveRequestRegistry>,
}
