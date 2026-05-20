//! Usage: Running gateway instance and router state.

use crate::shared::mutex_ext::MutexExt;
use crate::{circuit_breaker, db, request_logs, session_manager};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use super::background_tasks::GatewayBackgroundTasks;
use super::codex_session_id::CodexSessionIdCache;
use super::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
use super::{GatewayProviderCircuitStatus, GatewayStatus};

pub(in crate::gateway) struct GatewayAppState<R: tauri::Runtime = tauri::Wry> {
    pub(super) app: tauri::AppHandle<R>,
    pub(super) db: db::Db,
    pub(super) log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    pub(super) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(super) session: Arc<session_manager::SessionManager>,
    pub(super) codex_session_cache: Arc<Mutex<CodexSessionIdCache>>,
    pub(super) recent_errors: Arc<Mutex<RecentErrorCache>>,
    pub(super) latency_cache: Arc<Mutex<ProviderBaseUrlPingCache>>,
}

impl<R: tauri::Runtime> Clone for GatewayAppState<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            db: self.db.clone(),
            log_tx: self.log_tx.clone(),
            circuit: self.circuit.clone(),
            session: self.session.clone(),
            codex_session_cache: self.codex_session_cache.clone(),
            recent_errors: self.recent_errors.clone(),
            latency_cache: self.latency_cache.clone(),
        }
    }
}

impl<R: tauri::Runtime> GatewayAppState<R> {
    pub(in crate::gateway) fn client(&self) -> reqwest::Client {
        super::http_client::get()
    }
}

#[cfg(test)]
impl GatewayAppState {
    #[cfg(test)]
    pub(in crate::gateway) fn current_client() -> reqwest::Client {
        super::http_client::get()
    }
}

pub(crate) type GatewayRuntimeHandles = (
    oneshot::Sender<()>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    tokio::sync::watch::Sender<bool>,
    tauri::async_runtime::JoinHandle<()>,
);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GatewayRouteRuntimeClearResult {
    pub(crate) cleared_sessions: usize,
    pub(crate) cleared_recent_errors: usize,
}

pub(super) struct GatewayRuntimeInit {
    pub(super) port: u16,
    pub(super) base_url: String,
    pub(super) listen_addr: String,
    pub(super) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(super) session: Arc<session_manager::SessionManager>,
    pub(super) recent_errors: Arc<Mutex<RecentErrorCache>>,
    pub(super) shutdown: oneshot::Sender<()>,
    pub(super) task: tauri::async_runtime::JoinHandle<()>,
    pub(super) background_tasks: GatewayBackgroundTasks,
}

pub(crate) struct GatewayRuntime {
    port: u16,
    base_url: String,
    listen_addr: String,
    circuit: Arc<circuit_breaker::CircuitBreaker>,
    session: Arc<session_manager::SessionManager>,
    recent_errors: Arc<Mutex<RecentErrorCache>>,
    shutdown: oneshot::Sender<()>,
    task: tauri::async_runtime::JoinHandle<()>,
    background_tasks: GatewayBackgroundTasks,
}

impl GatewayRuntime {
    pub(super) fn new(init: GatewayRuntimeInit) -> Self {
        Self {
            port: init.port,
            base_url: init.base_url,
            listen_addr: init.listen_addr,
            circuit: init.circuit,
            session: init.session,
            recent_errors: init.recent_errors,
            shutdown: init.shutdown,
            task: init.task,
            background_tasks: init.background_tasks,
        }
    }

    pub(crate) fn status(&self) -> GatewayStatus {
        GatewayStatus {
            running: true,
            port: Some(self.port),
            base_url: Some(self.base_url.clone()),
            listen_addr: Some(self.listen_addr.clone()),
        }
    }

    pub(crate) fn active_sessions(
        &self,
        now_unix: i64,
        limit: usize,
    ) -> Vec<session_manager::ActiveSessionSnapshot> {
        self.session.list_active(now_unix, limit)
    }

    pub(crate) fn clear_cli_session_bindings(&self, cli_key: &str) -> usize {
        self.session.clear_cli_bindings(cli_key)
    }

    pub(crate) fn clear_recent_errors(&self) -> usize {
        self.recent_errors.lock_or_recover().clear()
    }

    pub(crate) fn clear_cli_route_runtime_state(
        &self,
        cli_key: &str,
    ) -> GatewayRouteRuntimeClearResult {
        GatewayRouteRuntimeClearResult {
            cleared_sessions: self.clear_cli_session_bindings(cli_key),
            cleared_recent_errors: self.clear_recent_errors(),
        }
    }

    pub(crate) fn circuit_status(
        &self,
        provider_ids: &[i64],
        now_unix: i64,
    ) -> Vec<GatewayProviderCircuitStatus> {
        provider_ids
            .iter()
            .copied()
            .map(|provider_id| {
                let check = self.circuit.should_allow(provider_id, now_unix);
                let snap = check.after;
                GatewayProviderCircuitStatus {
                    provider_id,
                    state: snap.state.as_str().to_string(),
                    failure_count: snap.failure_count,
                    failure_threshold: snap.failure_threshold,
                    open_until: snap.open_until,
                    cooldown_until: snap.cooldown_until,
                }
            })
            .collect()
    }

    pub(crate) fn circuit_reset_provider(&self, provider_id: i64, now_unix: i64) {
        self.circuit.reset(provider_id, now_unix);
        self.clear_recent_errors();
    }

    pub(crate) fn circuit_reset_cli(&self, provider_ids: &[i64], now_unix: i64) {
        for provider_id in provider_ids {
            self.circuit.reset(*provider_id, now_unix);
        }
        self.clear_recent_errors();
    }

    pub(crate) fn update_circuit_config(&self, failure_threshold: u32, open_duration_secs: i64) {
        self.circuit
            .update_config(circuit_breaker::CircuitBreakerConfig {
                failure_threshold,
                open_duration_secs,
            });
    }

    pub(super) fn into_handles(self) -> GatewayRuntimeHandles {
        let (log_task, circuit_task, oauth_refresh_shutdown, oauth_refresh_task) =
            self.background_tasks.into_handles();
        (
            self.shutdown,
            self.task,
            log_task,
            circuit_task,
            oauth_refresh_shutdown,
            oauth_refresh_task,
        )
    }

    #[cfg(test)]
    pub(super) fn for_tests(
        rt: &tokio::runtime::Runtime,
        session: Arc<session_manager::SessionManager>,
        recent_errors: Arc<Mutex<RecentErrorCache>>,
    ) -> Self {
        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            std::collections::HashMap::new(),
            None,
        ));
        let (shutdown, _shutdown_rx) = oneshot::channel();

        Self {
            port: 1,
            base_url: "http://127.0.0.1:1".to_string(),
            listen_addr: "127.0.0.1:1".to_string(),
            circuit,
            session,
            recent_errors,
            shutdown,
            task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            background_tasks: GatewayBackgroundTasks::for_tests(rt),
        }
    }
}
