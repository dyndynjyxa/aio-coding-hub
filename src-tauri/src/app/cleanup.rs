//! Usage: Best-effort cleanup hooks for app lifecycle events (exit/restart).

use super::gateway_control::app_take_running_gateway;
use crate::blocking;
use crate::cli_proxy;
use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
#[cfg(windows)]
use crate::infra::wsl;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Notify;

const CLEANUP_STATE_IDLE: u8 = 0;
const CLEANUP_STATE_RUNNING: u8 = 1;
const CLEANUP_STATE_DONE: u8 = 2;

const CLEANUP_WAIT_TIMEOUT: Duration = Duration::from_secs(15);
const CLI_PROXY_RESTORE_TIMEOUT: Duration = Duration::from_secs(3);
const GATEWAY_SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(3);
const GATEWAY_BACKGROUND_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);
const GATEWAY_OAUTH_STOP_TIMEOUT: Duration = Duration::from_secs(1);
const GATEWAY_TASK_ABORT_GRACE: Duration = Duration::from_secs(1);
#[cfg(windows)]
const WSL_RESTORE_TIMEOUT: Duration = Duration::from_secs(5);

static CLEANUP_STATE: AtomicU8 = AtomicU8::new(CLEANUP_STATE_IDLE);
static CLEANUP_NOTIFY: OnceLock<Notify> = OnceLock::new();

#[derive(Clone, Copy)]
struct GatewayStopTimeouts {
    server_stop: Duration,
    background_drain: Duration,
    oauth_stop: Duration,
    abort_grace: Duration,
}

const GATEWAY_STOP_TIMEOUTS: GatewayStopTimeouts = GatewayStopTimeouts {
    server_stop: GATEWAY_SERVER_STOP_TIMEOUT,
    background_drain: GATEWAY_BACKGROUND_DRAIN_TIMEOUT,
    oauth_stop: GATEWAY_OAUTH_STOP_TIMEOUT,
    abort_grace: GATEWAY_TASK_ABORT_GRACE,
};

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

pub(crate) async fn stop_gateway_best_effort<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let _gateway_lifecycle = super::gateway_lifecycle_lock::lock().await;
    stop_gateway_best_effort_unlocked(app).await;
}

pub(crate) async fn stop_gateway_best_effort_unlocked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) {
    let running = app_take_running_gateway(app);

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

    // Emit stopped status event so the frontend updates immediately
    let stopped_status = crate::gateway::GatewayStatus {
        running: false,
        port: None,
        base_url: None,
        listen_addr: None,
    };
    crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_STATUS_EVENT_NAME, stopped_status);

    stop_gateway_tasks_best_effort(
        &mut task,
        &mut log_task,
        &mut circuit_task,
        &mut oauth_refresh_task,
        GATEWAY_STOP_TIMEOUTS,
    )
    .await;
}

async fn stop_gateway_tasks_best_effort(
    server_task: &mut tauri::async_runtime::JoinHandle<()>,
    log_task: &mut tauri::async_runtime::JoinHandle<()>,
    circuit_task: &mut tauri::async_runtime::JoinHandle<()>,
    oauth_refresh_task: &mut tauri::async_runtime::JoinHandle<()>,
    timeouts: GatewayStopTimeouts,
) {
    if !join_task_with_timeout(server_task, timeouts.server_stop).await {
        tracing::warn!("exit cleanup: gateway server stop timed out, aborting gateway server task");
        abort_task_and_wait(server_task, timeouts.abort_grace).await;
    }

    if !join_task_with_timeout(log_task, timeouts.background_drain).await {
        tracing::warn!(
            "exit cleanup: gateway request-log writer drain timed out, aborting writer task"
        );
        abort_task_and_wait(log_task, timeouts.abort_grace).await;
    }

    if !join_task_with_timeout(circuit_task, timeouts.background_drain).await {
        tracing::warn!(
            "exit cleanup: gateway circuit writer drain timed out, aborting writer task"
        );
        abort_task_and_wait(circuit_task, timeouts.abort_grace).await;
    }

    if !join_task_with_timeout(oauth_refresh_task, timeouts.oauth_stop).await {
        tracing::warn!("exit cleanup: gateway OAuth refresh task stop timed out, aborting task");
        abort_task_and_wait(oauth_refresh_task, timeouts.abort_grace).await;
    }
}

async fn join_task_with_timeout(
    task: &mut tauri::async_runtime::JoinHandle<()>,
    timeout: Duration,
) -> bool {
    tokio::time::timeout(timeout, task).await.is_ok()
}

async fn abort_task_and_wait(task: &mut tauri::async_runtime::JoinHandle<()>, grace: Duration) {
    task.abort();
    let _ = join_task_with_timeout(task, grace).await;
}

#[cfg(test)]
mod tests {
    use super::{stop_gateway_tasks_best_effort, GatewayStopTimeouts};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    #[tokio::test]
    async fn gateway_stop_drains_writers_after_server_abort_drops_route_senders() {
        let (held_tx, mut rx) = mpsc::channel::<()>(1);
        let (drained_tx, drained_rx) = oneshot::channel::<()>();
        let mut server_task = tauri::async_runtime::spawn(async move {
            let _held_tx = held_tx;
            std::future::pending::<()>().await;
        });
        let mut log_task = tauri::async_runtime::spawn(async move {
            while rx.recv().await.is_some() {}
            let _ = drained_tx.send(());
        });
        let mut circuit_task = tauri::async_runtime::spawn(async {});
        let mut oauth_refresh_task = tauri::async_runtime::spawn(async {});

        stop_gateway_tasks_best_effort(
            &mut server_task,
            &mut log_task,
            &mut circuit_task,
            &mut oauth_refresh_task,
            GatewayStopTimeouts {
                server_stop: Duration::from_millis(10),
                background_drain: Duration::from_millis(100),
                oauth_stop: Duration::from_millis(100),
                abort_grace: Duration::from_millis(100),
            },
        )
        .await;

        tokio::time::timeout(Duration::from_millis(100), drained_rx)
            .await
            .expect("writer should drain after server abort drops route sender")
            .expect("writer should report drain");
    }
}
