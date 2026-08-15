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

/// Whether `settings.json` currently looks like a file we wrote, regardless of
/// which gateway port `ANTHROPIC_BASE_URL` points at. Unlike
/// [`is_proxy_config_applied`] (which requires an exact port match), this stays
/// true across a gateway port change, so it can be used to tell "still under
/// our management" apart from "reverted to the user's direct config" (e.g. the
/// file was restored on exit, or hand-edited while the app was closed).
///
/// Either marker is enough. The token alone would miss a file whose
/// `ANTHROPIC_AUTH_TOKEN` was hand-edited while the proxy was running (a user
/// swapping the placeholder for their real key): that file still points at the
/// gateway, and treating it as a direct config would snapshot the gateway
/// address as the user's "direct" backup and lose their real one for good.
pub(super) fn is_proxy_managed<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
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
    let token_is_ours =
        env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) == Some(PLACEHOLDER_KEY);
    let url_is_ours = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .is_some_and(is_local_gateway_claude_url);

    token_is_ours || url_is_ours
}

/// Whether a URL is one of our own gateway endpoints. The gateway always
/// publishes `http://127.0.0.1:<port>` (see `settings_service`), so a loopback
/// `/claude` URL cannot be a real upstream.
fn is_local_gateway_claude_url(url: &str) -> bool {
    url.starts_with("http://127.0.0.1:") && url.ends_with("/claude")
}
