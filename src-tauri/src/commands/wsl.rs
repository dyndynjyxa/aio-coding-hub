//! Usage: Windows WSL related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, gateway, settings, wsl};
#[cfg(windows)]
use tauri::Manager;

const WSL_CONFIG_STATUS_MAX_DISTROS: usize = 64;
const WSL_CONFIG_STATUS_DISTRO_MAX_CHARS: usize = 128;

/// Serializes WSL config writes across manual, startup, and automatic sync.
/// Acquire gateway lifecycle first when both locks are needed.
pub(crate) async fn lock_wsl_sync() -> tokio::sync::OwnedMutexGuard<()> {
    use std::sync::{Arc, OnceLock};
    static LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
        .lock_owned()
        .await
}

async fn detect_wsl_blocking(label: &'static str) -> Result<wsl::WslDetection, String> {
    blocking::run(
        label,
        || -> crate::shared::error::AppResult<wsl::WslDetection> { Ok(wsl::detect()) },
    )
    .await
    .map_err(Into::into)
}

async fn resolve_wsl_host_blocking(
    cfg: settings::AppSettings,
    label: &'static str,
) -> Result<String, String> {
    blocking::run(label, move || -> crate::shared::error::AppResult<String> {
        let host = match cfg.gateway_listen_mode {
            settings::GatewayListenMode::Localhost => "127.0.0.1".to_string(),
            settings::GatewayListenMode::WslAuto | settings::GatewayListenMode::Lan => {
                wsl::resolve_wsl_host(&cfg)
            }
            settings::GatewayListenMode::Custom => {
                let parsed = gateway::listen::parse_custom_listen_address(
                    &cfg.gateway_custom_listen_address,
                )?;
                if gateway::listen::is_wildcard_host(&parsed.host) {
                    wsl::resolve_wsl_host(&cfg)
                } else {
                    parsed.host
                }
            }
        };
        Ok(host)
    })
    .await
    .map_err(Into::into)
}

fn normalize_wsl_config_status_distros(
    distros: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, String> {
    let Some(distros) = distros else {
        return Ok(None);
    };

    if distros.len() > WSL_CONFIG_STATUS_MAX_DISTROS {
        return Err(format!(
            "SEC_INVALID_INPUT: WSL distro list must contain at most {WSL_CONFIG_STATUS_MAX_DISTROS} entries"
        ));
    }

    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for raw in distros {
        let distro = raw.trim();
        if distro.is_empty() {
            continue;
        }
        if distro.chars().any(char::is_control) {
            return Err(
                "SEC_INVALID_INPUT: WSL distro name contains control characters".to_string(),
            );
        }
        if distro.chars().count() > WSL_CONFIG_STATUS_DISTRO_MAX_CHARS {
            return Err(format!(
                "SEC_INVALID_INPUT: WSL distro name is too long (max {WSL_CONFIG_STATUS_DISTRO_MAX_CHARS} chars)"
            ));
        }
        if seen.insert(distro.to_string()) {
            normalized.push(distro.to_string());
        }
    }

    Ok(Some(normalized))
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn wsl_detect() -> wsl::WslDetection {
    detect_wsl_blocking("wsl_detect")
        .await
        .unwrap_or(wsl::WslDetection {
            detected: false,
            distros: Vec::new(),
        })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn wsl_host_address_get() -> Option<String> {
    blocking::run(
        "wsl_host_address_get",
        move || -> crate::shared::error::AppResult<Option<String>> {
            Ok(wsl::host_ipv4_best_effort())
        },
    )
    .await
    .unwrap_or(None)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn wsl_config_status_get(
    distros: Option<Vec<String>>,
) -> Vec<wsl::WslDistroConfigStatus> {
    let distros = match normalize_wsl_config_status_distros(distros) {
        Ok(distros) => distros,
        Err(err) => {
            tracing::warn!("invalid WSL config-status distro filters: {err}");
            return Vec::new();
        }
    };

    blocking::run(
        "wsl_config_status_get",
        move || -> crate::shared::error::AppResult<Vec<wsl::WslDistroConfigStatus>> {
            let distros = match distros {
                Some(v) if v.is_empty() => return Ok(Vec::new()),
                Some(v) if !v.is_empty() => v,
                _ => {
                    let detection = wsl::detect();
                    if !detection.detected || detection.distros.is_empty() {
                        return Ok(Vec::new());
                    }
                    detection.distros
                }
            };

            Ok(wsl::get_config_status(&distros))
        },
    )
    .await
    .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn wsl_configure_clients(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<wsl::WslConfigureReport, String> {
    if !cfg!(windows) {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "WSL configuration is only available on Windows".to_string(),
            distros: Vec::new(),
        });
    }

    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    let _gateway_lifecycle = crate::app::gateway_lifecycle_lock::lock().await;
    let sync_guard = lock_wsl_sync().await;
    let cfg = blocking::run("wsl_configure_clients_read_settings", {
        let app = app.clone();
        move || settings::read(&app)
    })
    .await?;

    if cfg.gateway_listen_mode == settings::GatewayListenMode::Localhost {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "监听模式为“仅本地(127.0.0.1)”时，WSL 无法访问网关。请先切换到：WSL 自动检测 / 局域网 / 自定义地址。".to_string(),
            distros: Vec::new(),
        });
    }

    let detection = detect_wsl_blocking("wsl_configure_clients_detect").await?;
    if !detection.detected || detection.distros.is_empty() {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "WSL not detected".to_string(),
            distros: Vec::new(),
        });
    }

    let preferred_port = cfg.preferred_port;
    let status = crate::app::gateway_service::ensure_running_and_sync_unlocked(
        &app,
        db.clone(),
        Some(preferred_port),
    )
    .await?;

    let port = status
        .port
        .ok_or_else(|| "gateway_start returned no port".to_string())?;

    let host =
        match resolve_wsl_host_blocking(cfg.clone(), "wsl_configure_clients_resolve_host").await {
            Ok(host) => host,
            Err(err) if err.starts_with("SEC_INVALID_INPUT:") => {
                return Ok(wsl::WslConfigureReport {
                    ok: false,
                    message: format!("自定义监听地址无效：{err}"),
                    distros: Vec::new(),
                });
            }
            Err(err) => return Err(err),
        };

    let proxy_origin = format!("http://{}", gateway::listen::format_host_port(&host, port));
    let distros = detection.distros;
    let targets = cfg.wsl_target_cli;

    // Gather MCP, Prompt, and Skills sync data from DB/SSOT
    let (mcp_data, prompt_data, skills_data) = blocking::run("wsl_configure_gather_sync_data", {
        let app = app.clone();
        let db = db.clone();
        move || -> crate::shared::error::AppResult<(
            wsl::WslMcpSyncData,
            wsl::WslPromptSyncData,
            wsl::WslSkillsSyncData,
        )> {
            let conn = db.open_connection()?;
            let mcp = wsl::gather_mcp_sync_data(&conn)?;
            let prompts = wsl::gather_prompt_sync_data(&conn)?;
            let skills = wsl::gather_skills_sync_data(&app, &conn)?;
            Ok((mcp, prompts, skills))
        }
    })
    .await?;

    let app_for_sync = app.clone();
    let (report, _sync_guard) = blocking::run(
        "wsl_configure_clients",
        move || -> crate::shared::error::AppResult<_> {
            let report = wsl::configure_clients(
                &app_for_sync,
                &distros,
                &targets,
                &proxy_origin,
                Some(&mcp_data),
                Some(&prompt_data),
                Some(&skills_data),
            );
            Ok((report, sync_guard))
        },
    )
    .await?;

    Ok(report)
}

/// Core WSL auto-sync logic shared by settings-change sync and MCP/Prompt/Skills-change sync.
/// Checks preconditions (wsl_auto_config enabled, listen mode != Localhost),
/// detects WSL, resolves host, gathers sync data, and configures CLI clients.
#[cfg(windows)]
pub(crate) async fn wsl_auto_sync_core(app: &tauri::AppHandle) -> Result<(), String> {
    wsl_auto_sync(app, false).await
}

#[cfg(windows)]
async fn wsl_auto_sync(app: &tauri::AppHandle, prompt_localhost: bool) -> Result<(), String> {
    use crate::app_state::{ensure_db_ready, DbInitState};
    use crate::gateway_runtime_access::app_gateway_status;

    let sync_guard = lock_wsl_sync().await;
    // Read after acquiring the lock: queued operations must not carry stale settings.
    let cfg = blocking::run("wsl_core_read_settings", {
        let app = app.clone();
        move || settings::read(&app)
    })
    .await
    .map_err(|e| e.to_string())?;

    if !cfg.wsl_auto_config {
        tracing::debug!("WSL auto-sync core: wsl_auto_config disabled, skipping");
        return Ok(());
    }

    if cfg.gateway_listen_mode == settings::GatewayListenMode::Localhost {
        if prompt_localhost {
            let detection = detect_wsl_blocking("wsl_startup_detect").await?;
            if detection.detected && !detection.distros.is_empty() {
                crate::app::heartbeat_watchdog::gated_emit(app, "wsl:localhost_switch_prompt", ());
            }
        }
        tracing::debug!("WSL auto-sync core: listen mode is localhost, skipping");
        return Ok(());
    }

    // 2. Get gateway port
    let status = app_gateway_status(app);
    if !status.running {
        tracing::debug!("WSL auto-sync core: gateway not running, skipping");
        return Ok(());
    }
    let port = match status.port {
        Some(port) => port,
        None => {
            tracing::debug!("WSL auto-sync core: gateway not running, skipping");
            return Ok(());
        }
    };

    // 3. Detect WSL
    let detection = detect_wsl_blocking("wsl_core_detect").await?;

    if !detection.detected || detection.distros.is_empty() {
        tracing::debug!("WSL auto-sync core: no WSL environment detected, skipping");
        return Ok(());
    }

    // 4. Resolve host
    let host = resolve_wsl_host_blocking(cfg.clone(), "wsl_core_resolve_host").await?;

    let proxy_origin = format!("http://{}", gateway::listen::format_host_port(&host, port));
    let targets = cfg.wsl_target_cli;
    let distros = detection.distros;

    // 5. Gather MCP, Prompt, and Skills sync data
    let db_state = app.state::<DbInitState>();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    let (mcp_data, prompt_data, skills_data) = blocking::run("wsl_core_gather_sync_data", {
        let app = app.clone();
        let db = db.clone();
        move || -> crate::shared::error::AppResult<(
            wsl::WslMcpSyncData,
            wsl::WslPromptSyncData,
            wsl::WslSkillsSyncData,
        )> {
            let conn = db.open_connection()?;
            let mcp = wsl::gather_mcp_sync_data(&conn)?;
            let prompts = wsl::gather_prompt_sync_data(&conn)?;
            let skills = wsl::gather_skills_sync_data(&app, &conn)?;
            Ok((mcp, prompts, skills))
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    // 6. Configure clients
    let app_for_sync = app.clone();
    let (report, _sync_guard) = blocking::run(
        "wsl_core_configure",
        move || -> crate::shared::error::AppResult<_> {
            let report = wsl::configure_clients(
                &app_for_sync,
                &distros,
                &targets,
                &proxy_origin,
                Some(&mcp_data),
                Some(&prompt_data),
                Some(&skills_data),
            );
            Ok((report, sync_guard))
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!(
        ok = report.ok,
        message = %report.message,
        "WSL auto-sync core completed"
    );

    crate::app::heartbeat_watchdog::gated_emit(app, "wsl:auto_config_result", &report);

    wsl_sync_report_result(&report)
}

#[cfg(any(windows, test))]
fn wsl_sync_report_result(report: &wsl::WslConfigureReport) -> Result<(), String> {
    if report.ok && report.distros.iter().all(|distro| distro.ok) {
        Ok(())
    } else {
        Err(report.message.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_sync_failed_or_partial_reports_are_not_success() {
        let mut report = wsl::WslConfigureReport {
            ok: false,
            message: "sync failed".to_string(),
            distros: Vec::new(),
        };
        assert!(wsl_sync_report_result(&report).is_err());
        report.ok = true;
        report.distros.push(wsl::WslConfigureDistroReport {
            distro: "Ubuntu".to_string(),
            ok: false,
            results: Vec::new(),
        });
        assert!(wsl_sync_report_result(&report).is_err());
        report.distros[0].ok = true;
        assert!(wsl_sync_report_result(&report).is_ok());
    }

    #[tokio::test]
    async fn wsl_sync_lock_orders_latest_state_after_cancelled_slow_writer() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        };
        use std::time::Duration;
        let setting = Arc::new(AtomicBool::new(true));
        let writes = Arc::new(Mutex::new(Vec::new()));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let first_guard = lock_wsl_sync().await;
        let first_value = setting.load(Ordering::SeqCst);
        let first_writes = writes.clone();
        let first = tokio::spawn(async move {
            blocking::run(
                "wsl_slow_writer_test",
                move || -> crate::shared::error::AppResult<()> {
                    let _sync_guard = first_guard;
                    started_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                    first_writes.lock().unwrap().push(first_value);
                    Ok(())
                },
            )
            .await
        });
        started_rx.await.unwrap();
        setting.store(false, Ordering::SeqCst);
        first.abort();
        let _ = first.await;
        assert!(
            tokio::time::timeout(Duration::from_millis(20), lock_wsl_sync())
                .await
                .is_err()
        );
        finish_tx.send(()).unwrap();
        let _next_guard = tokio::time::timeout(Duration::from_secs(2), lock_wsl_sync())
            .await
            .unwrap();
        let latest_value = setting.load(Ordering::SeqCst);
        writes.lock().unwrap().push(latest_value);
        assert_eq!(*writes.lock().unwrap(), [true, false]);
    }

    #[test]
    fn normalize_wsl_config_status_distros_keeps_none_for_auto_detect() {
        assert_eq!(normalize_wsl_config_status_distros(None).unwrap(), None);
    }

    #[test]
    fn normalize_wsl_config_status_distros_trims_dedupes_and_drops_empty_items() {
        assert_eq!(
            normalize_wsl_config_status_distros(Some(vec![
                " Ubuntu ".to_string(),
                "\t".to_string(),
                "Ubuntu".to_string(),
                "Debian".to_string(),
            ]))
            .unwrap(),
            Some(vec!["Ubuntu".to_string(), "Debian".to_string()])
        );
    }

    #[test]
    fn normalize_wsl_config_status_distros_preserves_explicit_empty_filter() {
        assert_eq!(
            normalize_wsl_config_status_distros(Some(vec![" ".to_string()])).unwrap(),
            Some(Vec::new())
        );
    }

    #[test]
    fn normalize_wsl_config_status_distros_rejects_oversized_lists() {
        let err = normalize_wsl_config_status_distros(Some(vec![
            "Ubuntu".to_string();
            WSL_CONFIG_STATUS_MAX_DISTROS + 1
        ]))
        .expect_err("oversized distro list");

        assert_eq!(
            err,
            "SEC_INVALID_INPUT: WSL distro list must contain at most 64 entries"
        );
    }

    #[test]
    fn normalize_wsl_config_status_distros_rejects_control_characters() {
        let err = normalize_wsl_config_status_distros(Some(vec!["Ubu\nntu".to_string()]))
            .expect_err("control character");

        assert_eq!(
            err,
            "SEC_INVALID_INPUT: WSL distro name contains control characters"
        );
    }

    #[test]
    fn normalize_wsl_config_status_distros_rejects_oversized_names() {
        let err = normalize_wsl_config_status_distros(Some(vec![
            ":".repeat(WSL_CONFIG_STATUS_DISTRO_MAX_CHARS + 1)
        ]))
        .expect_err("oversized distro name");

        assert_eq!(
            err,
            "SEC_INVALID_INPUT: WSL distro name is too long (max 128 chars)"
        );
    }
}

/// Debounced WSL sync trigger for MCP/Prompt/Skills changes.
/// Uses a background task with 500ms debounce window to coalesce rapid changes.
#[cfg(windows)]
pub(crate) mod wsl_sync_trigger {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;
    use std::time::Duration;
    use tokio::sync::Notify;

    static TRIGGER_NOTIFY: OnceLock<Notify> = OnceLock::new();
    static TASK_SPAWNED: AtomicBool = AtomicBool::new(false);

    fn trigger_notify() -> &'static Notify {
        TRIGGER_NOTIFY.get_or_init(Notify::new)
    }

    /// Fire-and-forget trigger. Notifies the background debounce task to schedule a WSL sync.
    /// If the background task hasn't been spawned yet, it will be spawned on first call.
    pub(crate) fn trigger(app: tauri::AppHandle) {
        if !TASK_SPAWNED.swap(true, Ordering::SeqCst) {
            tauri::async_runtime::spawn(debounce_loop(app));
        }
        trigger_notify().notify_one();
    }

    async fn debounce_loop(app: tauri::AppHandle) {
        const DEBOUNCE: Duration = Duration::from_millis(500);
        let notify = trigger_notify();

        loop {
            // Wait for initial trigger
            notify.notified().await;

            // Debounce: keep resetting while new notifications arrive within the window
            while tokio::time::timeout(DEBOUNCE, notify.notified())
                .await
                .is_ok()
            {}

            // Execute sync
            if let Err(err) = super::wsl_auto_sync_core(&app).await {
                tracing::warn!("WSL debounced sync failed: {}", err);
            }
        }
    }
}

/// WSL startup auto-configure: detect WSL environment and configure all CLI clients.
/// If the current listen mode is localhost, emit an event to prompt the user to switch.
#[cfg(windows)]
pub(crate) async fn wsl_auto_configure_on_startup(app: &tauri::AppHandle) -> Result<(), String> {
    wsl_auto_sync(app, true).await
}
