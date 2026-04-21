//! Usage: Stable data structures for CLI session viewer.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CliSessionsProjectSummary {
    pub source: String,
    pub id: String,
    pub display_path: String,
    pub short_name: String,
    pub session_count: usize,
    pub last_modified: Option<i64>,
    pub model_provider: Option<String>,
    pub wsl_distro: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CliSessionsSessionSummary {
    pub source: String,
    pub session_id: String,
    pub file_path: String,
    pub first_prompt: Option<String>,
    pub message_count: u32,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub git_branch: Option<String>,
    pub project_path: Option<String>,
    pub is_sidechain: Option<bool>,
    pub cwd: Option<String>,
    pub model_provider: Option<String>,
    pub cli_version: Option<String>,
    pub wsl_distro: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CliSessionsFolderLookupEntry {
    pub source: String,
    pub session_id: String,
    pub folder_name: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CliSessionsPaginatedMessages {
    pub messages: Vec<CliSessionsDisplayMessage>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CliSessionsDisplayMessage {
    pub uuid: Option<String>,
    pub role: String,
    pub timestamp: Option<String>,
    pub model: Option<String>,
    pub content: Vec<CliSessionsDisplayContentBlock>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliSessionsDisplayContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Reasoning {
        text: String,
    },
    FunctionCall {
        name: String,
        arguments: String,
        call_id: String,
    },
    FunctionCallOutput {
        call_id: String,
        output: String,
    },
}
