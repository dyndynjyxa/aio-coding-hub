//! Usage: Gateway background writers and refresh-loop ownership.

use crate::{circuit_breaker, db, provider_circuit_breakers, request_logs};
use tokio::sync::{mpsc, watch};

pub(super) type GatewayBackgroundTaskHandles = (
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    watch::Sender<bool>,
    tauri::async_runtime::JoinHandle<()>,
);

pub(super) struct GatewayBackgroundTasks {
    log_tx: mpsc::Sender<request_logs::RequestLogInsert>,
    circuit_persist_tx: mpsc::Sender<circuit_breaker::CircuitPersistedState>,
    log_task: tauri::async_runtime::JoinHandle<()>,
    circuit_task: tauri::async_runtime::JoinHandle<()>,
    oauth_refresh_shutdown: watch::Sender<bool>,
    oauth_refresh_task: tauri::async_runtime::JoinHandle<()>,
}

impl GatewayBackgroundTasks {
    pub(super) fn start<R: tauri::Runtime>(app: tauri::AppHandle<R>, db: db::Db) -> Self {
        let (log_tx, log_task) = request_logs::start_buffered_writer(app, db.clone());
        let (circuit_persist_tx, circuit_task) =
            provider_circuit_breakers::start_buffered_writer(db.clone());
        let (oauth_refresh_shutdown, oauth_refresh_rx) = watch::channel(false);
        let oauth_refresh_task = super::oauth::refresh_loop::spawn(db, oauth_refresh_rx);

        Self {
            log_tx,
            circuit_persist_tx,
            log_task,
            circuit_task,
            oauth_refresh_shutdown,
            oauth_refresh_task,
        }
    }

    pub(super) fn log_tx(&self) -> mpsc::Sender<request_logs::RequestLogInsert> {
        self.log_tx.clone()
    }

    pub(super) fn circuit_persist_tx(
        &self,
    ) -> mpsc::Sender<circuit_breaker::CircuitPersistedState> {
        self.circuit_persist_tx.clone()
    }

    pub(super) fn into_handles(self) -> GatewayBackgroundTaskHandles {
        let _ = self.oauth_refresh_shutdown.send(true);
        (
            self.log_task,
            self.circuit_task,
            self.oauth_refresh_shutdown,
            self.oauth_refresh_task,
        )
    }

    #[cfg(test)]
    pub(super) fn for_tests(rt: &tokio::runtime::Runtime) -> Self {
        let (log_tx, _log_rx) = mpsc::channel(1);
        let (circuit_persist_tx, _circuit_rx) = mpsc::channel(1);
        let (oauth_refresh_shutdown, _oauth_refresh_rx) = watch::channel(false);

        Self {
            log_tx,
            circuit_persist_tx,
            log_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            circuit_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            oauth_refresh_shutdown,
            oauth_refresh_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
        }
    }
}
