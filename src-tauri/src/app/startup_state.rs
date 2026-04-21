//! Usage: Startup pipeline state shared between backend bootstrap and frontend status UI.

use crate::shared::mutex_ext::MutexExt;
use std::sync::Mutex;
use tauri::Manager;

pub const APP_STARTUP_STATUS_EVENT_NAME: &str = "app:startup_status";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AppStartupStage {
    Idle,
    InitializingDb,
    ReadingSettings,
    StartingGateway,
    SyncingCliProxy,
    FinalizingWsl,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppStartupStatus {
    pub running: bool,
    pub current_stage: AppStartupStage,
    pub failed_stage: Option<AppStartupStage>,
    pub error_message: Option<String>,
    pub can_retry: bool,
}

impl Default for AppStartupStatus {
    fn default() -> Self {
        Self {
            running: false,
            current_stage: AppStartupStage::Idle,
            failed_stage: None,
            error_message: None,
            can_retry: false,
        }
    }
}

#[derive(Default)]
pub(crate) struct StartupState {
    inner: Mutex<AppStartupStatus>,
}

fn begin_run(status: &mut AppStartupStatus) -> bool {
    if status.running {
        return false;
    }

    status.running = true;
    status.current_stage = AppStartupStage::InitializingDb;
    status.failed_stage = None;
    status.error_message = None;
    status.can_retry = false;
    true
}

fn set_stage(status: &mut AppStartupStatus, stage: AppStartupStage) {
    status.running = true;
    status.current_stage = stage;
    status.failed_stage = None;
    status.error_message = None;
    status.can_retry = false;
}

fn set_failed(status: &mut AppStartupStatus, stage: AppStartupStage, message: String) {
    status.running = false;
    status.current_stage = AppStartupStage::Failed;
    status.failed_stage = Some(stage);
    status.error_message = Some(message);
    status.can_retry = true;
}

fn set_ready(status: &mut AppStartupStatus) {
    status.running = false;
    status.current_stage = AppStartupStage::Ready;
    status.failed_stage = None;
    status.error_message = None;
    status.can_retry = false;
}

fn emit_snapshot<R: tauri::Runtime>(app: &tauri::AppHandle<R>, snapshot: &AppStartupStatus) {
    crate::app::heartbeat_watchdog::gated_emit(
        app,
        APP_STARTUP_STATUS_EVENT_NAME,
        snapshot.clone(),
    );
}

fn update_status<R, F>(app: &tauri::AppHandle<R>, update: F) -> AppStartupStatus
where
    R: tauri::Runtime,
    F: FnOnce(&mut AppStartupStatus),
{
    let state = app.state::<StartupState>();
    let mut guard = state.inner.lock_or_recover();
    update(&mut guard);
    let snapshot = guard.clone();
    drop(guard);
    emit_snapshot(app, &snapshot);
    snapshot
}

pub(crate) fn startup_status_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppStartupStatus {
    let state = app.state::<StartupState>();
    let snapshot = state.inner.lock_or_recover().clone();
    snapshot
}

pub(crate) fn try_begin_startup_run<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let state = app.state::<StartupState>();
    let mut guard = state.inner.lock_or_recover();
    let started = begin_run(&mut guard);
    let snapshot = guard.clone();
    drop(guard);
    if started {
        emit_snapshot(app, &snapshot);
    }
    started
}

pub(crate) fn set_startup_stage<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    stage: AppStartupStage,
) -> AppStartupStatus {
    update_status(app, |status| set_stage(status, stage))
}

pub(crate) fn fail_startup_run<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    stage: AppStartupStage,
    message: impl Into<String>,
) -> AppStartupStatus {
    let message = message.into();
    update_status(app, |status| set_failed(status, stage, message))
}

pub(crate) fn finish_startup_run<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppStartupStatus {
    update_status(app, set_ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_run_resets_failure_and_sets_initial_stage() {
        let mut status = AppStartupStatus {
            running: false,
            current_stage: AppStartupStage::Failed,
            failed_stage: Some(AppStartupStage::StartingGateway),
            error_message: Some("boom".to_string()),
            can_retry: true,
        };

        assert!(begin_run(&mut status));
        assert!(status.running);
        assert_eq!(status.current_stage, AppStartupStage::InitializingDb);
        assert_eq!(status.failed_stage, None);
        assert_eq!(status.error_message, None);
        assert!(!status.can_retry);
    }

    #[test]
    fn begin_run_rejects_parallel_start() {
        let mut status = AppStartupStatus {
            running: true,
            ..AppStartupStatus::default()
        };

        assert!(!begin_run(&mut status));
        assert!(status.running);
    }

    #[test]
    fn set_failed_marks_retryable_failure() {
        let mut status = AppStartupStatus {
            running: true,
            current_stage: AppStartupStage::StartingGateway,
            ..AppStartupStatus::default()
        };

        set_failed(
            &mut status,
            AppStartupStage::StartingGateway,
            "gateway failed".to_string(),
        );

        assert!(!status.running);
        assert_eq!(status.current_stage, AppStartupStage::Failed);
        assert_eq!(status.failed_stage, Some(AppStartupStage::StartingGateway));
        assert_eq!(status.error_message.as_deref(), Some("gateway failed"));
        assert!(status.can_retry);
    }

    #[test]
    fn set_ready_clears_failure_details() {
        let mut status = AppStartupStatus {
            running: true,
            current_stage: AppStartupStage::Failed,
            failed_stage: Some(AppStartupStage::ReadingSettings),
            error_message: Some("bad settings".to_string()),
            can_retry: true,
        };

        set_ready(&mut status);

        assert!(!status.running);
        assert_eq!(status.current_stage, AppStartupStage::Ready);
        assert_eq!(status.failed_stage, None);
        assert_eq!(status.error_message, None);
        assert!(!status.can_retry);
    }
}
