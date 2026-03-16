//! Usage: Best-effort cleanup hooks for app lifecycle events (exit/restart).

use super::app_state::GatewayState;
use crate::blocking;
use crate::cli_proxy;
#[cfg(windows)]
use crate::infra::wsl;
use crate::shared::mutex_ext::MutexExt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::Notify;

const CLEANUP_STATE_IDLE: u8 = 0;
const CLEANUP_STATE_RUNNING: u8 = 1;
const CLEANUP_STATE_DONE: u8 = 2;

const CLEANUP_WAIT_TIMEOUT: Duration = Duration::from_secs(15);
const CLI_PROXY_RESTORE_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(windows)]
const WSL_RESTORE_TIMEOUT: Duration = Duration::from_secs(5);

static CLEANUP_STATE: AtomicU8 = AtomicU8::new(CLEANUP_STATE_IDLE);
static CLEANUP_NOTIFY: OnceLock<Notify> = OnceLock::new();

fn cleanup_notify() -> &'static Notify {
    CLEANUP_NOTIFY.get_or_init(Notify::new)
}

pub(crate) async fn cleanup_before_exit(app: &tauri::AppHandle) {
    let notify = cleanup_notify();
    match CLEANUP_STATE.compare_exchange(
        CLEANUP_STATE_IDLE,
        CLEANUP_STATE_RUNNING,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => {
            stop_gateway_best_effort(app).await;
            restore_cli_proxy_keep_state_best_effort(
                app,
                "cleanup_cli_proxy_restore_keep_state",
                "退出清理",
                true,
            )
            .await;

            #[cfg(windows)]
            {
                let wsl_restore_app = app.clone();
                let wsl_fut = blocking::run("cleanup_wsl_restore", move || {
                    wsl::restore_wsl_clients(&wsl_restore_app)
                });
                match tokio::time::timeout(WSL_RESTORE_TIMEOUT, wsl_fut).await {
                    Ok(Ok(())) => tracing::info!("WSL config restore completed"),
                    Ok(Err(e)) => tracing::warn!("WSL config restore failed: {e}"),
                    Err(_) => tracing::warn!(
                        "WSL config restore timed out ({}s)",
                        WSL_RESTORE_TIMEOUT.as_secs()
                    ),
                }
            }

            CLEANUP_STATE.store(CLEANUP_STATE_DONE, Ordering::Release);
            notify.notify_waiters();
        }
        Err(state) => {
            if state == CLEANUP_STATE_DONE {
                return;
            }
            wait_for_cleanup_done(notify).await;
        }
    }
}

async fn wait_for_cleanup_done(notify: &Notify) {
    if CLEANUP_STATE.load(Ordering::Acquire) == CLEANUP_STATE_DONE {
        return;
    }

    let wait = async {
        while CLEANUP_STATE.load(Ordering::Acquire) != CLEANUP_STATE_DONE {
            let notified = notify.notified();
            if CLEANUP_STATE.load(Ordering::Acquire) == CLEANUP_STATE_DONE {
                break;
            }
            notified.await;
        }
    };

    if tokio::time::timeout(CLEANUP_WAIT_TIMEOUT, wait)
        .await
        .is_err()
    {
        tracing::warn!(
            "退出清理：等待清理完成超时（{}秒），将继续退出流程",
            CLEANUP_WAIT_TIMEOUT.as_secs()
        );
    }
}

pub(crate) async fn restore_cli_proxy_keep_state_best_effort(
    app: &tauri::AppHandle,
    label: &'static str,
    context: &'static str,
    log_success: bool,
) {
    let app_for_restore = app.clone();
    let fut = blocking::run(label, move || {
        cli_proxy::restore_enabled_keep_state(&app_for_restore)
    });

    match tokio::time::timeout(CLI_PROXY_RESTORE_TIMEOUT, fut).await {
        Ok(Ok(results)) => {
            for result in results {
                if result.ok {
                    if log_success {
                        tracing::info!(
                            cli_key = %result.cli_key,
                            trace_id = %result.trace_id,
                            "{context}: restored cli_proxy direct config (keeping enabled state)"
                        );
                    }
                    continue;
                }

                tracing::warn!(
                    cli_key = %result.cli_key,
                    trace_id = %result.trace_id,
                    error_code = %result.error_code.unwrap_or_default(),
                    "{context}: cli_proxy direct config restore failed: {}",
                    result.message
                );
            }
        }
        Ok(Err(err)) => {
            tracing::warn!(
                "{context}: cli_proxy direct config restore task failed: {}",
                err
            );
        }
        Err(_) => tracing::warn!(
            "{context}: cli_proxy direct config restore task timed out ({}s)",
            CLI_PROXY_RESTORE_TIMEOUT.as_secs()
        ),
    }
}

pub(crate) async fn stop_gateway_best_effort(app: &tauri::AppHandle) {
    let running = {
        let state = app.state::<GatewayState>();
        let mut manager = state.0.lock_or_recover();
        manager.take_running()
    };

    let Some((
        shutdown,
        mut task,
        mut log_task,
        mut circuit_task,
        _oauth_refresh_shutdown,
        mut oauth_refresh_task,
    )) = running
    else {
        return;
    };

    let _ = shutdown.send(());

    let stop_timeout = Duration::from_secs(3);
    let join_all = async {
        let _ = tokio::join!(
            &mut task,
            &mut log_task,
            &mut circuit_task,
            &mut oauth_refresh_task
        );
    };

    if tokio::time::timeout(stop_timeout, join_all).await.is_err() {
        tracing::warn!("exit cleanup: gateway stop timed out, aborting server task");
        task.abort();
        oauth_refresh_task.abort();

        let abort_grace = Duration::from_secs(1);
        let _ = tokio::time::timeout(abort_grace, async {
            let _ = tokio::join!(
                &mut task,
                &mut log_task,
                &mut circuit_task,
                &mut oauth_refresh_task
            );
        })
        .await;
    }
}
