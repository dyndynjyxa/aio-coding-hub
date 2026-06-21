//! Usage: Async startup task pipeline extracted from bootstrap setup.

use super::app_state::{ensure_db_ready, DbInitState};
use super::startup_state::{
    fail_startup_run, finish_startup_run, set_startup_stage, try_begin_startup_run, AppStartupStage,
};
use tauri::Manager;

pub(crate) fn spawn(app_handle: tauri::AppHandle) -> bool {
    if !try_begin_startup_run(&app_handle) {
        return false;
    }

    tauri::async_runtime::spawn(async move {
        run(app_handle).await;
    });
    true
}

async fn run(app_handle: tauri::AppHandle) {
    let db_state = app_handle.state::<DbInitState>();
    let db = match ensure_db_ready(app_handle.clone(), db_state.inner()).await {
        Ok(db) => db,
        Err(err) => {
            tracing::error!("database initialization failed: {}", err);
            fail_startup_run(
                &app_handle,
                AppStartupStage::InitializingDb,
                format!("数据库初始化失败：{err}"),
            );
            return;
        }
    };

    match crate::request_logs::reconcile_unresolved_pending(
        &db,
        crate::request_logs::RequestLogReconcileReason::StartupRecovery,
        crate::shared::time::now_unix_millis(),
    ) {
        Ok(count) => {
            if count > 0 {
                tracing::info!(
                    reconciled_count = count,
                    "startup reconciled previous-process pending request logs"
                );
            }
        }
        Err(err) => {
            tracing::error!("startup request-log reconciliation failed: {}", err);
            fail_startup_run(
                &app_handle,
                AppStartupStage::InitializingDb,
                format!("请求日志恢复失败：{err}"),
            );
            return;
        }
    }

    set_startup_stage(&app_handle, AppStartupStage::ReadingSettings);
    let settings = match crate::app::startup_settings::read(&app_handle).await {
        Ok(settings) => settings,
        Err(err) => {
            fail_startup_run(&app_handle, AppStartupStage::ReadingSettings, err);
            return;
        }
    };

    crate::app::startup_settings::apply_window_state(&app_handle, &settings);

    set_startup_stage(&app_handle, AppStartupStage::StartingGateway);
    let status = match crate::app::startup_gateway::start(&app_handle, db.clone(), &settings).await
    {
        Ok(status) => status,
        Err(err) => {
            fail_startup_run(&app_handle, AppStartupStage::StartingGateway, err);
            return;
        }
    };

    set_startup_stage(&app_handle, AppStartupStage::SyncingCliProxy);
    crate::app::startup_gateway::sync_cli_proxy_after_autostart(&app_handle, &status).await;

    set_startup_stage(&app_handle, AppStartupStage::FinalizingWsl);
    crate::app::startup_wsl::finalize(&app_handle, db, status.port, settings).await;
    finish_startup_run(&app_handle);
}
