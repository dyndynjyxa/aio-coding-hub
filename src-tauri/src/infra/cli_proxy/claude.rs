//! Claude-specific CLI proxy configuration helpers.

use crate::shared::error::AppResult;
use std::path::Path;

use super::{
    read_cli_proxy_file, read_optional_cli_proxy_file, write_cli_proxy_file_atomic, PLACEHOLDER_KEY,
};

pub(super) fn claude_settings_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<std::path::PathBuf> {
    Ok(super::home_dir(app)?.join(".claude").join("settings.json"))
}

/// Patch a JSON object to set `env.ANTHROPIC_BASE_URL` and `env.ANTHROPIC_AUTH_TOKEN`.
pub(super) fn patch_json_set_env_base_url(
    mut root: serde_json::Value,
    base_url: &str,
) -> AppResult<serde_json::Value> {
    let obj = root.as_object_mut().ok_or_else(|| {
        crate::shared::error::AppError::from(
            "CLI_PROXY_INVALID_SETTINGS_JSON: root must be a JSON object",
        )
    })?;

    let env = obj
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| {
            crate::shared::error::AppError::from(
                "CLI_PROXY_INVALID_SETTINGS_JSON: env must be an object",
            )
        })?;

    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        serde_json::Value::String(base_url.to_string()),
    );
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        serde_json::Value::String(PLACEHOLDER_KEY.to_string()),
    );

    Ok(root)
}

pub(super) fn build_claude_settings_json(
    current: Option<Vec<u8>>,
    base_url: &str,
) -> AppResult<Vec<u8>> {
    let root = match current {
        Some(bytes) if bytes.is_empty() => serde_json::json!({}),
        Some(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "cli_proxy: existing settings.json has invalid JSON ({e}), \
                     preserving original as .invalid-backup and starting fresh"
                );
                return Err(
                    format!("CLI_PROXY_INVALID_SETTINGS_JSON: failed to parse JSON: {e}").into(),
                );
            }
        },
        None => serde_json::json!({}),
    };

    let patched = patch_json_set_env_base_url(root, base_url)?;
    let mut out = serde_json::to_vec_pretty(&patched)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

/// Merge-restore Claude `settings.json`: only revert the two proxy-managed env
/// keys (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`) while preserving every
/// other change the user may have made while the proxy was enabled.
pub(super) fn merge_restore_claude_settings_json(
    target_path: &Path,
    backup_path: &Path,
) -> AppResult<()> {
    const PROXY_ENV_KEYS: &[&str] = &["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"];

    let current_bytes = read_optional_cli_proxy_file(target_path)?;
    let backup_bytes = read_cli_proxy_file(backup_path)?;

    let mut current: serde_json::Value = match current_bytes {
        Some(b) if !b.is_empty() => {
            serde_json::from_slice(&b).unwrap_or_else(|_| serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };

    let backup: serde_json::Value =
        serde_json::from_slice(&backup_bytes).unwrap_or_else(|_| serde_json::json!({}));

    let backup_env = backup.get("env").and_then(|v| v.as_object());

    if let Some(obj) = current.as_object_mut() {
        if let Some(env) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
            for key in PROXY_ENV_KEYS {
                if let Some(original) = backup_env.and_then(|e| e.get(*key)) {
                    env.insert(key.to_string(), original.clone());
                } else {
                    env.remove(*key);
                }
            }
            if env.is_empty() {
                obj.remove("env");
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&current)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    bytes.push(b'\n');
    write_cli_proxy_file_atomic(target_path, &bytes)?;
    Ok(())
}

/// Check whether Claude proxy config is currently applied.
pub(super) fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> bool {
    let path = match claude_settings_path(app) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let bytes = match read_cli_proxy_file(&path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let Some(env) = value.get("env").and_then(|v| v.as_object()) else {
        return false;
    };
    let Some(base) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) else {
        return false;
    };
    base == format!("{base_origin}/claude")
}
