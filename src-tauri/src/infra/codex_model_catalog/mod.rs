//! Read the model capability catalog exposed by the installed Codex CLI.

mod protocol;

use crate::{cli_manager, codex_paths};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexModelCatalogStatus {
    Ready,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexModelCatalogIssue {
    CliNotFound,
    AppServerUnavailable,
    Timeout,
    ProtocolError,
    EmptyCatalog,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct CodexReasoningEffortOption {
    pub reasoning_effort: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct CodexModelCapability {
    pub id: String,
    pub model: String,
    pub display_name: String,
    pub hidden: bool,
    pub is_default: bool,
    pub supported_reasoning_efforts: Option<Vec<CodexReasoningEffortOption>>,
    pub default_reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct CodexModelCatalogSnapshot {
    pub config_path: String,
    pub executable_path: Option<String>,
    pub cli_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct CodexModelCatalogState {
    pub status: CodexModelCatalogStatus,
    pub issue: Option<CodexModelCatalogIssue>,
    pub snapshot: CodexModelCatalogSnapshot,
    pub models: Vec<CodexModelCapability>,
}

pub fn codex_model_catalog_get(
    app: &tauri::AppHandle,
) -> crate::shared::error::AppResult<CodexModelCatalogState> {
    let config_path = codex_paths::codex_config_toml_path(app)?;
    let mut snapshot = CodexModelCatalogSnapshot {
        config_path: config_path.to_string_lossy().to_string(),
        executable_path: None,
        cli_version: None,
    };

    let Some(launch) = cli_manager::codex_launch_spec(app)? else {
        return Ok(CodexModelCatalogState {
            status: CodexModelCatalogStatus::Unavailable,
            issue: Some(CodexModelCatalogIssue::CliNotFound),
            snapshot,
            models: Vec::new(),
        });
    };

    snapshot.executable_path = Some(launch.executable.to_string_lossy().to_string());
    snapshot.cli_version = launch.version.clone();

    let codex_home = codex_paths::codex_home_dir(app)?;
    match protocol::fetch_model_catalog(&launch, &codex_home) {
        Ok(models) if models.is_empty() => Ok(CodexModelCatalogState {
            status: CodexModelCatalogStatus::Degraded,
            issue: Some(CodexModelCatalogIssue::EmptyCatalog),
            snapshot,
            models,
        }),
        Ok(models) => Ok(CodexModelCatalogState {
            status: CodexModelCatalogStatus::Ready,
            issue: None,
            snapshot,
            models,
        }),
        Err(protocol::ProtocolError::Timeout) => Ok(CodexModelCatalogState {
            status: CodexModelCatalogStatus::Degraded,
            issue: Some(CodexModelCatalogIssue::Timeout),
            snapshot,
            models: Vec::new(),
        }),
        Err(protocol::ProtocolError::Spawn) => Ok(CodexModelCatalogState {
            status: CodexModelCatalogStatus::Degraded,
            issue: Some(CodexModelCatalogIssue::AppServerUnavailable),
            snapshot,
            models: Vec::new(),
        }),
        Err(protocol::ProtocolError::Malformed | protocol::ProtocolError::JsonRpc) => {
            Ok(CodexModelCatalogState {
                status: CodexModelCatalogStatus::Degraded,
                issue: Some(CodexModelCatalogIssue::ProtocolError),
                snapshot,
                models: Vec::new(),
            })
        }
    }
}
