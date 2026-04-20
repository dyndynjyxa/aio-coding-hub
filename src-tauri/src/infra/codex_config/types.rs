//! Types for Codex config.toml read/write operations.

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CodexConfigState {
    pub config_dir: String,
    pub config_path: String,
    pub user_home_default_dir: String,
    pub user_home_default_path: String,
    pub follow_codex_home_dir: String,
    pub follow_codex_home_path: String,
    pub can_open_config_dir: bool,
    pub exists: bool,

    pub model: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub plan_mode_reasoning_effort: Option<String>,
    pub web_search: Option<String>,
    pub personality: Option<String>,
    pub model_context_window: Option<u64>,
    pub model_auto_compact_token_limit: Option<u64>,
    pub service_tier: Option<String>,

    pub sandbox_workspace_write_network_access: Option<bool>,

    pub features_unified_exec: Option<bool>,
    pub features_shell_snapshot: Option<bool>,
    pub features_apply_patch_freeform: Option<bool>,
    pub features_shell_tool: Option<bool>,
    pub features_exec_policy: Option<bool>,
    pub features_remote_compaction: Option<bool>,
    pub features_fast_mode: Option<bool>,
    pub features_responses_websockets_v2: Option<bool>,
    pub features_multi_agent: Option<bool>,
}

#[derive(Debug, Clone)]
pub(super) struct CodexConfigStateMeta {
    pub config_dir: String,
    pub config_path: String,
    pub user_home_default_dir: String,
    pub user_home_default_path: String,
    pub follow_codex_home_dir: String,
    pub follow_codex_home_path: String,
    pub can_open_config_dir: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct CodexConfigPatch {
    pub model: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub plan_mode_reasoning_effort: Option<String>,
    pub web_search: Option<String>,
    pub personality: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_u64_patch")]
    pub model_context_window: Option<Option<u64>>,
    #[serde(default, deserialize_with = "deserialize_nullable_u64_patch")]
    pub model_auto_compact_token_limit: Option<Option<u64>>,
    pub service_tier: Option<String>,

    pub sandbox_workspace_write_network_access: Option<bool>,

    pub features_unified_exec: Option<bool>,
    pub features_shell_snapshot: Option<bool>,
    pub features_apply_patch_freeform: Option<bool>,
    pub features_shell_tool: Option<bool>,
    pub features_exec_policy: Option<bool>,
    pub features_remote_compaction: Option<bool>,
    pub features_fast_mode: Option<bool>,
    pub features_responses_websockets_v2: Option<bool>,
    pub features_multi_agent: Option<bool>,
}

fn deserialize_nullable_u64_patch<'de, D>(deserializer: D) -> Result<Option<Option<u64>>, D::Error>
where
    D: Deserializer<'de>,
{
    // Preserve the difference between "field omitted" and `"field": null` so
    // the patch layer can delete existing TOML keys on explicit clear.
    Option::<u64>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CodexConfigTomlState {
    pub config_path: String,
    pub exists: bool,
    pub toml: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CodexConfigTomlValidationError {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CodexConfigTomlValidationResult {
    pub ok: bool,
    pub error: Option<CodexConfigTomlValidationError>,
}
