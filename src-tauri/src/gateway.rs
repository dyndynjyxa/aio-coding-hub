mod billing_header_rectifier;
mod claude_metadata_user_id_injection;
pub(crate) mod cli_auth;
mod codex_session_id;
pub(crate) mod events;
pub(crate) mod http_client;
pub(crate) mod listen;
mod manager;
pub(crate) mod oauth;
mod proxy;
mod response_fixer;
mod routes;
pub(crate) mod session_manager;
mod streams;
mod thinking_budget_rectifier;
mod thinking_signature_rectifier;
pub(crate) mod util;
mod warmup;

pub use manager::GatewayManager;
pub(crate) use manager::GatewayStartResult;

use crate::settings;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GatewayStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub base_url: Option<String>,
    pub listen_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GatewayProviderCircuitStatus {
    pub provider_id: i64,
    pub state: String,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub open_until: Option<i64>,
    pub cooldown_until: Option<i64>,
}

pub(crate) fn planned_base_url(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<String> {
    manager::planned_base_url(cfg)
}

pub(crate) fn listen_rebind_required(
    previous: &settings::AppSettings,
    next: &settings::AppSettings,
) -> bool {
    manager::listen_rebind_required(previous, next)
}
