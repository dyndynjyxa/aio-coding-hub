mod claude_metadata_user_id_injection;
mod codex_session_id;
mod events;
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
mod util;
mod warmup;

pub use manager::GatewayManager;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GatewayStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub base_url: Option<String>,
    pub listen_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayProviderCircuitStatus {
    pub provider_id: i64,
    pub state: String,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub open_until: Option<i64>,
    pub cooldown_until: Option<i64>,
}
