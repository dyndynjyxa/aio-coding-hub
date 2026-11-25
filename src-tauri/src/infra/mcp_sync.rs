//! Usage: Sync/backup/restore MCP configuration files across supported CLIs (infra adapter).

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const LEGACY_APP_DOTDIR_NAMES: &[&str] = &[".aio-gateway", ".aio_gateway"];
const MCP_SYNC_TARGET_MAX_BYTES: usize = 1024 * 1024;
const MCP_SYNC_MANIFEST_MAX_BYTES: usize = 256 * 1024;

mod claude_json;
mod codex_toml;
mod fs;
mod gemini_json;
mod grok_toml;
mod json_patch;
mod legacy;
mod manifest;
mod paths;
mod sync;
mod types;

pub(crate) use gemini_json::{
    merge_antigravity_mcp, parse_mcp_root as parse_antigravity_mcp_root, release_legacy_entries,
};
pub(crate) use types::McpServerForSync;

pub use fs::{read_target_bytes, restore_target_bytes};
pub(crate) use manifest::{antigravity_owned_keys, relinquish_antigravity_keys};
pub use manifest::{read_manifest_bytes, restore_manifest_bytes};
pub use sync::sync_cli;
pub(crate) use sync::{
    antigravity_rebind_needed, build_next_bytes, legacy_antigravity_snapshot,
    swap_grok_local_servers_for_workspace, warn_antigravity_mcp,
};
