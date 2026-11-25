//! Usage: MCP server management (DB persistence + import/export + sync integration).

mod backups;
mod cli_specs;
mod db;
mod import;
mod local_swap;
mod sync;
mod types;
mod validate;

pub(crate) use backups::SingleCliBackup;
pub use db::{delete, list_for_workspace, set_enabled, upsert};
pub use import::{import_servers, import_servers_from_workspace_cli, parse_json};
pub(crate) use local_swap::swap_local_mcp_servers_for_workspace_switch;
pub(crate) use sync::{list_enabled_for_cli, sync_cli_for_workspace, sync_one_cli};
pub use types::{McpImportReport, McpImportServer, McpParseResult, McpServerSummary};
