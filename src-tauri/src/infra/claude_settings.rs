//! Usage: Read / patch Claude Code global `settings.json` (~/.claude/settings.json).

use crate::shared::fs::{read_optional_file, write_file_atomic_if_changed};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

const ENV_KEY_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
const ENV_KEY_MCP_TOOL_TIMEOUT: &str = "MCP_TOOL_TIMEOUT";
const ENV_KEY_DISABLE_ERROR_REPORTING: &str = "DISABLE_ERROR_REPORTING";
const ENV_KEY_DISABLE_TELEMETRY: &str = "DISABLE_TELEMETRY";
const ENV_KEY_DISABLE_BACKGROUND_TASKS: &str = "CLAUDE_CODE_DISABLE_BACKGROUND_TASKS";
const ENV_KEY_DISABLE_TERMINAL_TITLE: &str = "CLAUDE_CODE_DISABLE_TERMINAL_TITLE";
const ENV_KEY_CLAUDE_BASH_NO_LOGIN: &str = "CLAUDE_BASH_NO_LOGIN";
const ENV_KEY_CLAUDE_CODE_ATTRIBUTION_HEADER: &str = "CLAUDE_CODE_ATTRIBUTION_HEADER";
const ENV_KEY_CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE: &str = "CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE";
const ENV_KEY_CLAUDE_AUTOCOMPACT_PCT_OVERRIDE: &str = "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE";
const ENV_KEY_CLAUDE_CODE_MAX_OUTPUT_TOKENS: &str = "CLAUDE_CODE_MAX_OUTPUT_TOKENS";
const ENV_KEY_ENABLE_EXPERIMENTAL_MCP_CLI: &str = "ENABLE_EXPERIMENTAL_MCP_CLI";
const ENV_KEY_ENABLE_TOOL_SEARCH: &str = "ENABLE_TOOL_SEARCH";
const ENV_KEY_MAX_MCP_OUTPUT_TOKENS: &str = "MAX_MCP_OUTPUT_TOKENS";

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeSettingsState {
    pub config_dir: String,
    pub settings_path: String,
    pub exists: bool,

    pub model: Option<String>,
    pub output_style: Option<String>,
    pub language: Option<String>,
    pub always_thinking_enabled: Option<bool>,

    pub show_turn_duration: Option<bool>,
    pub spinner_tips_enabled: Option<bool>,
    pub terminal_progress_bar_enabled: Option<bool>,
    pub respect_gitignore: Option<bool>,

    pub disable_all_hooks: Option<bool>,

    pub permissions_allow: Vec<String>,
    pub permissions_ask: Vec<String>,
    pub permissions_deny: Vec<String>,

    pub env_mcp_timeout_ms: Option<u64>,
    pub env_mcp_tool_timeout_ms: Option<u64>,
    pub env_disable_error_reporting: bool,
    pub env_disable_telemetry: bool,
    pub env_disable_background_tasks: bool,
    pub env_disable_terminal_title: bool,
    pub env_claude_bash_no_login: bool,
    pub env_claude_code_attribution_header_disabled: bool,
    pub env_claude_code_blocking_limit_override: Option<u64>,
    pub env_claude_autocompact_pct_override: Option<u64>,
    pub env_claude_code_max_output_tokens: Option<u64>,
    pub env_enable_experimental_mcp_cli: bool,
    pub env_enable_tool_search: bool,
    pub env_max_mcp_output_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeSettingsPatch {
    pub model: Option<String>,
    pub output_style: Option<String>,
    pub language: Option<String>,
    pub always_thinking_enabled: Option<bool>,

    pub show_turn_duration: Option<bool>,
    pub spinner_tips_enabled: Option<bool>,
    pub terminal_progress_bar_enabled: Option<bool>,
    pub respect_gitignore: Option<bool>,

    // UI semantics: `true` => write `disableAllHooks=true`, `false` => delete the key (do not write `false`).
    pub disable_all_hooks: Option<bool>,

    pub permissions_allow: Option<Vec<String>>,
    pub permissions_ask: Option<Vec<String>>,
    pub permissions_deny: Option<Vec<String>>,

    // Env semantics:
    // - numeric: `0` => delete the key (use default), `>0` => write
    // - bool: `true` => set key, `false` => delete key
    // - bool (zero-toggle): `true` => write "0", `false` => delete key
    pub env_mcp_timeout_ms: Option<u64>,
    pub env_mcp_tool_timeout_ms: Option<u64>,
    pub env_disable_error_reporting: Option<bool>,
    pub env_disable_telemetry: Option<bool>,
    pub env_disable_background_tasks: Option<bool>,
    pub env_disable_terminal_title: Option<bool>,
    pub env_claude_bash_no_login: Option<bool>,
    pub env_claude_code_attribution_header_disabled: Option<bool>,
    pub env_claude_code_blocking_limit_override: Option<u64>,
    pub env_claude_autocompact_pct_override: Option<u64>,
    pub env_claude_code_max_output_tokens: Option<u64>,
    pub env_enable_experimental_mcp_cli: Option<bool>,
    pub env_enable_tool_search: Option<bool>,
    pub env_max_mcp_output_tokens: Option<u64>,
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

fn claude_config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".claude"))
}

fn claude_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(claude_config_dir(app)?.join("settings.json"))
}

fn is_symlink(path: &Path) -> Result<bool, String> {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))
}

fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> serde_json::Value {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

fn json_to_bytes(value: &serde_json::Value, hint: &str) -> Result<Vec<u8>, String> {
    let mut out =
        serde_json::to_vec_pretty(value).map_err(|e| format!("failed to serialize {hint}: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn ensure_json_object_root(mut root: serde_json::Value) -> serde_json::Value {
    if root.is_object() {
        return root;
    }
    root = serde_json::json!({});
    root
}

fn trimmed_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn parse_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn env_string_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        _ => None,
    }
}

fn env_u64_value(value: &serde_json::Value) -> Option<u64> {
    env_string_value(value).and_then(|s| s.parse::<u64>().ok())
}

fn env_bool_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => n.as_i64().map(|v| v != 0),
        serde_json::Value::String(s) => {
            let trimmed = s.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                return None;
            }
            match trimmed.as_str() {
                "1" | "true" | "yes" | "y" | "on" => Some(true),
                "0" | "false" | "no" | "n" | "off" => Some(false),
                _ => Some(true),
            }
        }
        _ => None,
    }
}

fn env_is_enabled(env: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    env.get(key).and_then(env_bool_value).unwrap_or(false)
}

pub fn claude_settings_get(app: &tauri::AppHandle) -> Result<ClaudeSettingsState, String> {
    let config_dir = claude_config_dir(app)?;
    let settings_path = claude_settings_path(app)?;
    let exists = settings_path.exists();

    let root = json_root_from_bytes(read_optional_file(&settings_path)?);
    let root = ensure_json_object_root(root);
    let obj = root.as_object().expect("root must be object");

    let permissions = obj.get("permissions").and_then(|v| v.as_object());
    let permissions_allow = parse_string_list(permissions.and_then(|p| p.get("allow")));
    let permissions_ask = parse_string_list(permissions.and_then(|p| p.get("ask")));
    let permissions_deny = parse_string_list(permissions.and_then(|p| p.get("deny")));

    let env = obj.get("env").and_then(|v| v.as_object());
    let env_mcp_timeout_ms = env
        .and_then(|e| e.get(ENV_KEY_MCP_TIMEOUT))
        .and_then(env_u64_value);
    let env_mcp_tool_timeout_ms = env
        .and_then(|e| e.get(ENV_KEY_MCP_TOOL_TIMEOUT))
        .and_then(env_u64_value);

    let env_disable_error_reporting = env
        .map(|e| env_is_enabled(e, ENV_KEY_DISABLE_ERROR_REPORTING))
        .unwrap_or(false);
    let env_disable_telemetry = env
        .map(|e| env_is_enabled(e, ENV_KEY_DISABLE_TELEMETRY))
        .unwrap_or(false);
    let env_disable_background_tasks = env
        .map(|e| env_is_enabled(e, ENV_KEY_DISABLE_BACKGROUND_TASKS))
        .unwrap_or(false);
    let env_disable_terminal_title = env
        .map(|e| env_is_enabled(e, ENV_KEY_DISABLE_TERMINAL_TITLE))
        .unwrap_or(false);
    let env_claude_bash_no_login = env
        .map(|e| env_is_enabled(e, ENV_KEY_CLAUDE_BASH_NO_LOGIN))
        .unwrap_or(false);
    let env_claude_code_attribution_header_disabled = env
        .and_then(|e| e.get(ENV_KEY_CLAUDE_CODE_ATTRIBUTION_HEADER))
        .and_then(env_bool_value)
        .map(|v| !v)
        .unwrap_or(false);
    let env_claude_code_blocking_limit_override = env
        .and_then(|e| e.get(ENV_KEY_CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE))
        .and_then(env_u64_value);
    let env_claude_autocompact_pct_override = env
        .and_then(|e| e.get(ENV_KEY_CLAUDE_AUTOCOMPACT_PCT_OVERRIDE))
        .and_then(env_u64_value);
    let env_claude_code_max_output_tokens = env
        .and_then(|e| e.get(ENV_KEY_CLAUDE_CODE_MAX_OUTPUT_TOKENS))
        .and_then(env_u64_value);
    let env_enable_experimental_mcp_cli = env
        .map(|e| env_is_enabled(e, ENV_KEY_ENABLE_EXPERIMENTAL_MCP_CLI))
        .unwrap_or(false);
    let env_enable_tool_search = env
        .map(|e| env_is_enabled(e, ENV_KEY_ENABLE_TOOL_SEARCH))
        .unwrap_or(false);
    let env_max_mcp_output_tokens = env
        .and_then(|e| e.get(ENV_KEY_MAX_MCP_OUTPUT_TOKENS))
        .and_then(env_u64_value);

    Ok(ClaudeSettingsState {
        config_dir: config_dir.to_string_lossy().to_string(),
        settings_path: settings_path.to_string_lossy().to_string(),
        exists,

        model: obj.get("model").and_then(trimmed_string),
        output_style: obj.get("outputStyle").and_then(trimmed_string),
        language: obj.get("language").and_then(trimmed_string),
        always_thinking_enabled: obj.get("alwaysThinkingEnabled").and_then(|v| v.as_bool()),

        show_turn_duration: obj.get("showTurnDuration").and_then(|v| v.as_bool()),
        spinner_tips_enabled: obj.get("spinnerTipsEnabled").and_then(|v| v.as_bool()),
        terminal_progress_bar_enabled: obj
            .get("terminalProgressBarEnabled")
            .and_then(|v| v.as_bool()),
        respect_gitignore: obj.get("respectGitignore").and_then(|v| v.as_bool()),

        disable_all_hooks: obj.get("disableAllHooks").and_then(|v| v.as_bool()),

        permissions_allow,
        permissions_ask,
        permissions_deny,

        env_mcp_timeout_ms,
        env_mcp_tool_timeout_ms,
        env_disable_error_reporting,
        env_disable_telemetry,
        env_disable_background_tasks,
        env_disable_terminal_title,
        env_claude_bash_no_login,
        env_claude_code_attribution_header_disabled,
        env_claude_code_blocking_limit_override,
        env_claude_autocompact_pct_override,
        env_claude_code_max_output_tokens,
        env_enable_experimental_mcp_cli,
        env_enable_tool_search,
        env_max_mcp_output_tokens,
    })
}

fn sanitize_lines(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn patch_string_key(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str, raw: String) {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        obj.remove(key);
    } else {
        obj.insert(key.to_string(), serde_json::Value::String(trimmed));
    }
}

fn patch_env_u64(env: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: u64) {
    match value {
        0 => {
            env.remove(key);
        }
        v => {
            env.insert(key.to_string(), serde_json::Value::String(v.to_string()));
        }
    }
}

fn patch_env_toggle(
    env: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    enabled: bool,
) {
    if enabled {
        env.insert(key.to_string(), serde_json::Value::String("1".to_string()));
    } else {
        env.remove(key);
    }
}

fn patch_env_zero_toggle(
    env: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    disabled: bool,
) {
    if disabled {
        env.insert(key.to_string(), serde_json::Value::String("0".to_string()));
    } else {
        env.remove(key);
    }
}

fn patch_env_bool_toggle(
    env: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    enabled: bool,
) {
    if enabled {
        env.insert(
            key.to_string(),
            serde_json::Value::String("true".to_string()),
        );
    } else {
        env.remove(key);
    }
}

fn patch_claude_settings(
    mut root: serde_json::Value,
    patch: ClaudeSettingsPatch,
) -> Result<serde_json::Value, String> {
    root = ensure_json_object_root(root);
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "settings.json root must be a JSON object".to_string())?;

    if let Some(raw) = patch.model {
        patch_string_key(obj, "model", raw);
    }
    if let Some(raw) = patch.output_style {
        patch_string_key(obj, "outputStyle", raw);
    }
    if let Some(raw) = patch.language {
        patch_string_key(obj, "language", raw);
    }
    if let Some(v) = patch.always_thinking_enabled {
        obj.insert(
            "alwaysThinkingEnabled".to_string(),
            serde_json::Value::Bool(v),
        );
    }

    if let Some(v) = patch.show_turn_duration {
        obj.insert("showTurnDuration".to_string(), serde_json::Value::Bool(v));
    }
    if let Some(v) = patch.spinner_tips_enabled {
        obj.insert("spinnerTipsEnabled".to_string(), serde_json::Value::Bool(v));
    }
    if let Some(v) = patch.terminal_progress_bar_enabled {
        obj.insert(
            "terminalProgressBarEnabled".to_string(),
            serde_json::Value::Bool(v),
        );
    }
    if let Some(v) = patch.respect_gitignore {
        obj.insert("respectGitignore".to_string(), serde_json::Value::Bool(v));
    }

    if let Some(v) = patch.disable_all_hooks {
        if v {
            obj.insert("disableAllHooks".to_string(), serde_json::Value::Bool(true));
        } else {
            obj.remove("disableAllHooks");
        }
    }

    let has_permission_patch = patch.permissions_allow.is_some()
        || patch.permissions_ask.is_some()
        || patch.permissions_deny.is_some();
    if has_permission_patch {
        let entry = obj
            .entry("permissions".to_string())
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(Default::default());
        }

        let should_remove_permissions = {
            let perms = entry
                .as_object_mut()
                .ok_or_else(|| "settings.json permissions must be an object".to_string())?;

            if let Some(lines) = patch.permissions_allow {
                let cleaned = sanitize_lines(lines);
                if cleaned.is_empty() {
                    perms.remove("allow");
                } else {
                    perms.insert(
                        "allow".to_string(),
                        serde_json::Value::Array(
                            cleaned.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }
            }
            if let Some(lines) = patch.permissions_ask {
                let cleaned = sanitize_lines(lines);
                if cleaned.is_empty() {
                    perms.remove("ask");
                } else {
                    perms.insert(
                        "ask".to_string(),
                        serde_json::Value::Array(
                            cleaned.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }
            }
            if let Some(lines) = patch.permissions_deny {
                let cleaned = sanitize_lines(lines);
                if cleaned.is_empty() {
                    perms.remove("deny");
                } else {
                    perms.insert(
                        "deny".to_string(),
                        serde_json::Value::Array(
                            cleaned.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }
            }

            perms.is_empty()
        };

        if should_remove_permissions {
            obj.remove("permissions");
        }
    }

    let has_env_patch = patch.env_mcp_timeout_ms.is_some()
        || patch.env_mcp_tool_timeout_ms.is_some()
        || patch.env_disable_error_reporting.is_some()
        || patch.env_disable_telemetry.is_some()
        || patch.env_disable_background_tasks.is_some()
        || patch.env_disable_terminal_title.is_some()
        || patch.env_claude_bash_no_login.is_some()
        || patch.env_claude_code_attribution_header_disabled.is_some()
        || patch.env_claude_code_blocking_limit_override.is_some()
        || patch.env_claude_autocompact_pct_override.is_some()
        || patch.env_claude_code_max_output_tokens.is_some()
        || patch.env_enable_experimental_mcp_cli.is_some()
        || patch.env_enable_tool_search.is_some()
        || patch.env_max_mcp_output_tokens.is_some();
    if has_env_patch {
        let entry = obj
            .entry("env".to_string())
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(Default::default());
        }

        let should_remove_env = {
            let env = entry
                .as_object_mut()
                .ok_or_else(|| "settings.json env must be an object".to_string())?;

            if let Some(v) = patch.env_mcp_timeout_ms {
                patch_env_u64(env, ENV_KEY_MCP_TIMEOUT, v);
            }
            if let Some(v) = patch.env_mcp_tool_timeout_ms {
                patch_env_u64(env, ENV_KEY_MCP_TOOL_TIMEOUT, v);
            }
            if let Some(v) = patch.env_disable_error_reporting {
                patch_env_toggle(env, ENV_KEY_DISABLE_ERROR_REPORTING, v);
            }
            if let Some(v) = patch.env_disable_telemetry {
                patch_env_toggle(env, ENV_KEY_DISABLE_TELEMETRY, v);
            }
            if let Some(v) = patch.env_disable_background_tasks {
                patch_env_toggle(env, ENV_KEY_DISABLE_BACKGROUND_TASKS, v);
            }
            if let Some(v) = patch.env_disable_terminal_title {
                patch_env_toggle(env, ENV_KEY_DISABLE_TERMINAL_TITLE, v);
            }
            if let Some(v) = patch.env_claude_bash_no_login {
                patch_env_toggle(env, ENV_KEY_CLAUDE_BASH_NO_LOGIN, v);
            }
            if let Some(v) = patch.env_claude_code_attribution_header_disabled {
                patch_env_zero_toggle(env, ENV_KEY_CLAUDE_CODE_ATTRIBUTION_HEADER, v);
            }
            if let Some(v) = patch.env_claude_code_blocking_limit_override {
                patch_env_u64(env, ENV_KEY_CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE, v);
            }
            if let Some(v) = patch.env_claude_autocompact_pct_override {
                patch_env_u64(env, ENV_KEY_CLAUDE_AUTOCOMPACT_PCT_OVERRIDE, v);
            }
            if let Some(v) = patch.env_claude_code_max_output_tokens {
                patch_env_u64(env, ENV_KEY_CLAUDE_CODE_MAX_OUTPUT_TOKENS, v);
            }
            if let Some(v) = patch.env_enable_experimental_mcp_cli {
                patch_env_bool_toggle(env, ENV_KEY_ENABLE_EXPERIMENTAL_MCP_CLI, v);
            }
            if let Some(v) = patch.env_enable_tool_search {
                patch_env_bool_toggle(env, ENV_KEY_ENABLE_TOOL_SEARCH, v);
            }
            if let Some(v) = patch.env_max_mcp_output_tokens {
                patch_env_u64(env, ENV_KEY_MAX_MCP_OUTPUT_TOKENS, v);
            }

            env.is_empty()
        };

        if should_remove_env {
            obj.remove("env");
        }
    }

    Ok(root)
}

pub fn claude_settings_set(
    app: &tauri::AppHandle,
    patch: ClaudeSettingsPatch,
) -> Result<ClaudeSettingsState, String> {
    let path = claude_settings_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        ));
    }

    let current = read_optional_file(&path)?;
    let root = json_root_from_bytes(current);
    let patched = patch_claude_settings(root, patch)?;
    let bytes = json_to_bytes(&patched, "claude/settings.json")?;
    let _ = write_file_atomic_if_changed(&path, &bytes)?;
    claude_settings_get(app)
}

#[cfg(test)]
mod tests;
