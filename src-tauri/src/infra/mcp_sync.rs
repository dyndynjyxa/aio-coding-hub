//! Usage: Sync/backup/restore MCP configuration files across supported CLIs (infra adapter).

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const LEGACY_APP_DOTDIR_NAMES: &[&str] = &[".aio-gateway", ".aio_gateway"];

mod claude_json;
mod codex_toml;
mod fs;
mod gemini_json;
mod json_patch;
mod legacy;
mod manifest;
mod paths;
mod sync;
mod types;

pub(crate) use types::McpServerForSync;

pub use fs::{read_target_bytes, restore_target_bytes};
pub use manifest::{read_manifest_bytes, restore_manifest_bytes};
pub use sync::sync_cli;
