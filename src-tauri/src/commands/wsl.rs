//! Usage: Windows WSL related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::shared::mutex_ext::MutexExt;
use crate::{blocking, gateway, settings, wsl};
use tauri::Manager;

#[tauri::command]
pub(crate) async fn wsl_detect() -> wsl::WslDetection {
    blocking::run("wsl_detect", move || Ok(wsl::detect()))
        .await
        .unwrap_or(wsl::WslDetection {
            detected: false,
            distros: Vec::new(),
        })
}

#[tauri::command]
pub(crate) async fn wsl_host_address_get() -> Option<String> {
    blocking::run("wsl_host_address_get", move || {
        Ok(wsl::host_ipv4_best_effort())
    })
    .await
    .unwrap_or(None)
}

#[tauri::command]
pub(crate) async fn wsl_config_status_get(
    distros: Option<Vec<String>>,
) -> Vec<wsl::WslDistroConfigStatus> {
    blocking::run("wsl_config_status_get", move || {
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
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub(crate) async fn wsl_configure_clients(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    targets: settings::WslTargetCli,
) -> Result<wsl::WslConfigureReport, String> {
    if !cfg!(windows) {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "WSL configuration is only available on Windows".to_string(),
            distros: Vec::new(),
        });
    }

    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    let cfg = blocking::run("wsl_configure_clients_read_settings", {
        let app = app.clone();
        move || Ok(settings::read(&app).unwrap_or_default())
    })
    .await?;

    if cfg.gateway_listen_mode == settings::GatewayListenMode::Localhost {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "监听模式为“仅本地(127.0.0.1)”时，WSL 无法访问网关。请先切换到：WSL 自动检测 / 局域网 / 自定义地址。".to_string(),
            distros: Vec::new(),
        });
    }

    let detection = wsl::detect();
    if !detection.detected || detection.distros.is_empty() {
        return Ok(wsl::WslConfigureReport {
            ok: false,
            message: "WSL not detected".to_string(),
            distros: Vec::new(),
        });
    }

    let preferred_port = cfg.preferred_port;
    let status = blocking::run("wsl_configure_clients_ensure_gateway", {
        let app = app.clone();
        let db = db.clone();
        move || {
            let state = app.state::<GatewayState>();
            let mut manager = state.0.lock_or_recover();
            manager.start(&app, db, Some(preferred_port))
        }
    })
    .await?;

    let port = status
        .port
        .ok_or_else(|| "gateway_start returned no port".to_string())?;

    let host = match cfg.gateway_listen_mode {
        settings::GatewayListenMode::Localhost => "127.0.0.1".to_string(),
        settings::GatewayListenMode::WslAuto | settings::GatewayListenMode::Lan => {
            wsl::host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
        }
        settings::GatewayListenMode::Custom => {
            let parsed = match gateway::listen::parse_custom_listen_address(
                &cfg.gateway_custom_listen_address,
            ) {
                Ok(v) => v,
                Err(err) => {
                    return Ok(wsl::WslConfigureReport {
                        ok: false,
                        message: format!("自定义监听地址无效：{err}"),
                        distros: Vec::new(),
                    });
                }
            };
            if gateway::listen::is_wildcard_host(&parsed.host) {
                wsl::host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
            } else {
                parsed.host
            }
        }
    };

    let proxy_origin = format!("http://{}", gateway::listen::format_host_port(&host, port));
    let distros = detection.distros;
    let report = blocking::run("wsl_configure_clients", move || {
        Ok(wsl::configure_clients(&distros, &targets, &proxy_origin))
    })
    .await?;

    Ok(report)
}
