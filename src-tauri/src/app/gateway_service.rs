//! Usage: Gateway shell-side orchestration facade extracted from Tauri IPC commands.

mod circuit;
mod lifecycle;
mod port_check;
mod sessions;

pub(crate) use circuit::{circuit_reset_cli, circuit_reset_provider, circuit_status};
pub(crate) use lifecycle::{start_and_sync, stop_and_restore, sync_cli_proxy_to_gateway};
pub(crate) use port_check::check_port_available;
pub(crate) use sessions::{list_active_sessions, GatewayActiveSessionSummary};
