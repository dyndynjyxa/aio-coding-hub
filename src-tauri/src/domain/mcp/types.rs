//! Usage: MCP server management types.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct McpServerSummary {
    pub id: i64,
    pub server_key: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl McpServerSummary {
    pub fn env_keys(&self) -> Vec<String> {
        self.env.keys().cloned().collect()
    }

    pub fn header_keys(&self) -> Vec<String> {
        self.headers.keys().cloned().collect()
    }
}

impl Serialize for McpServerSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("McpServerSummary", 13)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("server_key", &self.server_key)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("transport", &self.transport)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("args", &self.args)?;
        state.serialize_field("env_keys", &self.env_keys())?;
        state.serialize_field("cwd", &self.cwd)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("header_keys", &self.header_keys())?;
        state.serialize_field("enabled", &self.enabled)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct McpImportServer {
    pub server_key: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct McpParseResult {
    pub servers: Vec<McpImportServer>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct McpImportReport {
    pub inserted: u32,
    pub updated: u32,
    pub skipped: Vec<McpImportSkip>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct McpImportSkip {
    pub name: String,
    pub reason: String,
}
