//! Usage: Best-effort cleanup hooks for app lifecycle events (exit/restart).

use super::app_state::GatewayState;
use crate::blocking;
use crate::cli_proxy;
use crate::shared::mutex_ext::MutexExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Manager;

static CLEANUP_STARTED: AtomicBool = AtomicBool::new(false);

pub(crate) async fn cleanup_before_exit(app: &tauri::AppHandle) {
    if CLEANUP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    stop_gateway_best_effort(app).await;

    let app_for_restore = app.clone();
    match blocking::run("cleanup_cli_proxy_restore_keep_state", move || {
        cli_proxy::restore_enabled_keep_state(&app_for_restore)
    })
    .await
    {
        Ok(results) => {
            for result in results {
                if result.ok {
                    tracing::info!(
                        cli_key = %result.cli_key,
                        trace_id = %result.trace_id,
                        "退出清理：已恢复 cli_proxy 直连配置（保留启用状态）"
                    );
                } else {
                    tracing::warn!(
                        cli_key = %result.cli_key,
                        trace_id = %result.trace_id,
                        error_code = %result.error_code.unwrap_or_default(),
                        "退出清理：恢复 cli_proxy 直连配置失败: {}",
                        result.message
                    );
                }
            }
        }
        Err(err) => {
            tracing::warn!("退出清理：恢复 cli_proxy 直连配置任务失败: {}", err);
        }
    }
}

pub(crate) async fn stop_gateway_best_effort(app: &tauri::AppHandle) {
    let running = {
        let state = app.state::<GatewayState>();
        let mut manager = state.0.lock_or_recover();
        manager.take_running()
    };

    let Some((shutdown, mut task, mut log_task, mut attempt_log_task, mut circuit_task)) = running
    else {
        return;
    };

    let _ = shutdown.send(());

    let stop_timeout = Duration::from_secs(3);
    let join_all = async {
        let _ = tokio::join!(
            &mut task,
            &mut log_task,
            &mut attempt_log_task,
            &mut circuit_task
        );
    };

    if tokio::time::timeout(stop_timeout, join_all).await.is_err() {
        tracing::warn!("退出清理：网关停止超时，正在中止服务器任务");
        task.abort();

        let abort_grace = Duration::from_secs(1);
        let _ = tokio::time::timeout(abort_grace, async {
            let _ = tokio::join!(
                &mut task,
                &mut log_task,
                &mut attempt_log_task,
                &mut circuit_task
            );
        })
        .await;
    }
}
