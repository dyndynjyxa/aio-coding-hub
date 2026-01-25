//! Usage: Shared types for MCP sync operations.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct McpServerForSync {
    pub server_key: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}
