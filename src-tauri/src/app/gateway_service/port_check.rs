//! Usage: Gateway listen-address port availability probes.

use crate::shared::error::AppResult;
use crate::{blocking, gateway, settings, wsl};

pub(crate) async fn check_port_available<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    port: u16,
) -> AppResult<bool> {
    if port < 1024 {
        return Ok(false);
    }

    blocking::run(
        "gateway_check_port_available",
        move || -> AppResult<bool> {
            let cfg = settings::read(&app)?;
            let host = match cfg.gateway_listen_mode {
                settings::GatewayListenMode::Localhost => "127.0.0.1".to_string(),
                settings::GatewayListenMode::Lan => "0.0.0.0".to_string(),
                settings::GatewayListenMode::WslAuto => {
                    wsl::host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
                }
                settings::GatewayListenMode::Custom => {
                    gateway::listen::parse_custom_listen_address(&cfg.gateway_custom_listen_address)
                        .map(|value| value.host)
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                }
            };

            Ok(std::net::TcpListener::bind((host.as_str(), port)).is_ok())
        },
    )
    .await
}
