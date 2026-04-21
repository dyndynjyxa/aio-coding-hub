//! Usage: Gateway listen-address planning and socket binding helpers.

use crate::{settings, wsl};

pub(crate) fn planned_base_url(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<String> {
    let binding = resolve_gateway_binding(cfg)?;
    let port = binding.fixed_port.unwrap_or(cfg.preferred_port);
    Ok(format!(
        "http://{}",
        super::listen::format_host_port(&binding.base_host, port)
    ))
}

pub(crate) fn listen_rebind_required(
    previous: &settings::AppSettings,
    next: &settings::AppSettings,
) -> bool {
    if previous.preferred_port != next.preferred_port
        || previous.gateway_listen_mode != next.gateway_listen_mode
        || previous.gateway_custom_listen_address != next.gateway_custom_listen_address
    {
        return true;
    }

    if next.gateway_listen_mode == settings::GatewayListenMode::WslAuto
        && (previous.wsl_host_address_mode != next.wsl_host_address_mode
            || previous.wsl_custom_host_address != next.wsl_custom_host_address)
    {
        return true;
    }

    false
}

pub(super) struct ResolvedGatewayBinding {
    pub(super) bind_host: String,
    pub(super) base_host: String,
    pub(super) fixed_port: Option<u16>,
}

pub(super) fn resolve_gateway_binding(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<ResolvedGatewayBinding> {
    let (bind_host, fixed_port) = match cfg.gateway_listen_mode {
        settings::GatewayListenMode::Localhost => ("127.0.0.1".to_string(), None),
        settings::GatewayListenMode::Lan => ("0.0.0.0".to_string(), None),
        settings::GatewayListenMode::WslAuto => (wsl::resolve_wsl_host(cfg), None),
        settings::GatewayListenMode::Custom => {
            let parsed =
                super::listen::parse_custom_listen_address(&cfg.gateway_custom_listen_address)?;
            (parsed.host, parsed.port)
        }
    };

    let base_host = match cfg.gateway_listen_mode {
        settings::GatewayListenMode::Lan => "127.0.0.1".to_string(),
        settings::GatewayListenMode::Custom if super::listen::is_wildcard_host(&bind_host) => {
            "127.0.0.1".to_string()
        }
        _ => bind_host.clone(),
    };

    Ok(ResolvedGatewayBinding {
        bind_host,
        base_host,
        fixed_port,
    })
}

pub(super) fn bind_exact(
    bind_host: &str,
    port: u16,
) -> crate::shared::error::AppResult<std::net::TcpListener> {
    bind_host_port(bind_host, port)
        .ok_or_else(|| format!("failed to bind {bind_host}:{port}").into())
}

pub(super) fn bind_first_available(
    bind_host: &str,
    preferred: Option<u16>,
) -> crate::shared::error::AppResult<(u16, std::net::TcpListener)> {
    for port in port_candidates(preferred) {
        if let Some(std_listener) = bind_host_port(bind_host, port) {
            return Ok((port, std_listener));
        }
    }

    Err(format!(
        "no available port in range {}..{} for host {bind_host}",
        settings::DEFAULT_GATEWAY_PORT,
        settings::MAX_GATEWAY_PORT
    )
    .into())
}

fn port_candidates(preferred: Option<u16>) -> impl Iterator<Item = u16> {
    let mut candidates = Vec::with_capacity(
        (settings::MAX_GATEWAY_PORT - settings::DEFAULT_GATEWAY_PORT + 2) as usize,
    );

    if let Some(port) = preferred {
        if port > 0 {
            candidates.push(port);
        }
    }

    for port in settings::DEFAULT_GATEWAY_PORT..=settings::MAX_GATEWAY_PORT {
        if candidates.first().copied() == Some(port) {
            continue;
        }
        candidates.push(port);
    }

    candidates.into_iter()
}

fn bind_host_port(bind_host: &str, port: u16) -> Option<std::net::TcpListener> {
    let std_listener = std::net::TcpListener::bind((bind_host, port)).ok()?;
    std_listener.set_nonblocking(true).ok()?;
    Some(std_listener)
}
