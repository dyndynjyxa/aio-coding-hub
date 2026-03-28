//! Usage: Read / patch Gemini CLI `settings.json` (~/.gemini/settings.json).

use crate::shared::fs::{is_symlink, read_optional_file, write_file_atomic_if_changed};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiConfigState {
    pub config_dir: String,
    pub config_path: String,
    pub exists: bool,
    pub model_name: Option<String>,
    pub model_max_session_turns: Option<i64>,
    pub model_compression_threshold: Option<f64>,
    pub default_approval_mode: Option<String>,
    pub enable_auto_update: Option<bool>,
    pub enable_notifications: Option<bool>,
    pub vim_mode: Option<bool>,
    pub retry_fetch_errors: Option<bool>,
    pub max_attempts: Option<u32>,
    pub ui_theme: Option<String>,
    pub ui_hide_banner: Option<bool>,
    pub ui_hide_tips: Option<bool>,
    pub ui_show_line_numbers: Option<bool>,
    pub ui_inline_thinking_mode: Option<String>,
    pub usage_statistics_enabled: Option<bool>,
    pub session_retention_enabled: Option<bool>,
    pub session_retention_max_age: Option<String>,
    pub plan_model_routing: Option<bool>,
    pub security_auth_selected_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiConfigPatch {
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_i64_patch")]
    pub model_max_session_turns: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_nullable_f64_patch")]
    pub model_compression_threshold: Option<Option<f64>>,
    pub default_approval_mode: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub enable_auto_update: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub enable_notifications: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub vim_mode: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub retry_fetch_errors: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_u32_patch")]
    pub max_attempts: Option<Option<u32>>,
    pub ui_theme: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub ui_hide_banner: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub ui_hide_tips: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub ui_show_line_numbers: Option<Option<bool>>,
    pub ui_inline_thinking_mode: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub usage_statistics_enabled: Option<Option<bool>>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub session_retention_enabled: Option<Option<bool>>,
    pub session_retention_max_age: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_bool_patch")]
    pub plan_model_routing: Option<Option<bool>>,
    pub security_auth_selected_type: Option<String>,
}

fn deserialize_nullable_i64_patch<'de, D>(deserializer: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Some(None)),
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(|value| Some(Some(value)))
            .ok_or_else(|| serde::de::Error::custom("expected integer for Gemini config patch")),
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(Some(None));
            }
            trimmed
                .parse::<i64>()
                .map(|value| Some(Some(value)))
                .map_err(|_| serde::de::Error::custom("expected integer for Gemini config patch"))
        }
        _ => Err(serde::de::Error::custom(
            "expected integer, string, or null for Gemini config patch",
        )),
    }
}

fn deserialize_nullable_u32_patch<'de, D>(deserializer: D) -> Result<Option<Option<u32>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Some(None)),
        serde_json::Value::Number(number) => {
            let value = number.as_u64().ok_or_else(|| {
                serde::de::Error::custom("expected unsigned integer for Gemini config patch")
            })?;
            let value = u32::try_from(value).map_err(|_| {
                serde::de::Error::custom("Gemini config patch integer out of range for u32")
            })?;
            Ok(Some(Some(value)))
        }
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(Some(None));
            }
            trimmed
                .parse::<u32>()
                .map(|value| Some(Some(value)))
                .map_err(|_| {
                    serde::de::Error::custom("expected unsigned integer for Gemini config patch")
                })
        }
        _ => Err(serde::de::Error::custom(
            "expected unsigned integer, string, or null for Gemini config patch",
        )),
    }
}

fn deserialize_nullable_f64_patch<'de, D>(deserializer: D) -> Result<Option<Option<f64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Some(None)),
        serde_json::Value::Number(number) => number
            .as_f64()
            .map(|value| Some(Some(value)))
            .ok_or_else(|| serde::de::Error::custom("expected number for Gemini config patch")),
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(Some(None));
            }
            trimmed
                .parse::<f64>()
                .map(|value| Some(Some(value)))
                .map_err(|_| serde::de::Error::custom("expected number for Gemini config patch"))
        }
        _ => Err(serde::de::Error::custom(
            "expected number, string, or null for Gemini config patch",
        )),
    }
}

fn deserialize_nullable_bool_patch<'de, D>(
    deserializer: D,
) -> Result<Option<Option<bool>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Some(None)),
        serde_json::Value::Bool(value) => Ok(Some(Some(value))),
        serde_json::Value::Number(number) => {
            let value = number.as_i64().ok_or_else(|| {
                serde::de::Error::custom("expected boolean for Gemini config patch")
            })?;
            Ok(Some(Some(value != 0)))
        }
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                return Ok(Some(None));
            }
            let value = match trimmed.as_str() {
                "1" | "true" | "yes" | "y" | "on" => true,
                "0" | "false" | "no" | "n" | "off" => false,
                _ => {
                    return Err(serde::de::Error::custom(
                        "expected boolean-compatible string for Gemini config patch",
                    ))
                }
            };
            Ok(Some(Some(value)))
        }
        _ => Err(serde::de::Error::custom(
            "expected boolean, string, number, or null for Gemini config patch",
        )),
    }
}

fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

fn gemini_config_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(home_dir(app)?.join(".gemini"))
}

fn gemini_config_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(gemini_config_dir(app)?.join("settings.json"))
}

fn json_to_bytes(
    value: &serde_json::Value,
    hint: &str,
) -> crate::shared::error::AppResult<Vec<u8>> {
    let mut out =
        serde_json::to_vec_pretty(value).map_err(|e| format!("failed to serialize {hint}: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn parse_json_root(bytes: Option<Vec<u8>>) -> crate::shared::error::AppResult<serde_json::Value> {
    match bytes {
        Some(bytes) => {
            let root: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| format!("failed to parse gemini settings.json: {e}"))?;
            if !root.is_object() {
                return Err("gemini settings.json root must be a JSON object".into());
            }
            Ok(root)
        }
        None => Ok(serde_json::json!({})),
    }
}

fn ensure_json_object_root(root: &mut serde_json::Value) -> crate::shared::error::AppResult<()> {
    if !root.is_object() {
        *root = serde_json::Value::Object(Default::default());
    }
    root.as_object()
        .map(|_| ())
        .ok_or_else(|| "gemini settings.json root must be a JSON object".into())
}

fn string_value(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn bool_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(value) => Some(*value),
        serde_json::Value::Number(number) => number.as_i64().map(|value| value != 0),
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim().to_ascii_lowercase();
            match trimmed.as_str() {
                "" => None,
                "1" | "true" | "yes" | "y" | "on" => Some(true),
                "0" | "false" | "no" | "n" | "off" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

fn i64_value(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(raw) => raw.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn u32_value(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Number(number) => {
            number.as_u64().and_then(|value| u32::try_from(value).ok())
        }
        serde_json::Value::String(raw) => raw.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn f64_value(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(raw) => raw.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn pointer<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    root.pointer(path)
}

fn make_state(
    config_dir: String,
    config_path: String,
    exists: bool,
    root: &serde_json::Value,
) -> GeminiConfigState {
    GeminiConfigState {
        config_dir,
        config_path,
        exists,
        model_name: pointer(root, "/model/name").and_then(string_value),
        model_max_session_turns: pointer(root, "/model/maxSessionTurns").and_then(i64_value),
        model_compression_threshold: pointer(root, "/model/compressionThreshold")
            .and_then(f64_value),
        default_approval_mode: pointer(root, "/general/defaultApprovalMode").and_then(string_value),
        enable_auto_update: pointer(root, "/general/enableAutoUpdate").and_then(bool_value),
        enable_notifications: pointer(root, "/general/enableNotifications").and_then(bool_value),
        vim_mode: pointer(root, "/general/vimMode").and_then(bool_value),
        retry_fetch_errors: pointer(root, "/general/retryFetchErrors").and_then(bool_value),
        max_attempts: pointer(root, "/general/maxAttempts").and_then(u32_value),
        ui_theme: pointer(root, "/ui/theme").and_then(string_value),
        ui_hide_banner: pointer(root, "/ui/hideBanner").and_then(bool_value),
        ui_hide_tips: pointer(root, "/ui/hideTips").and_then(bool_value),
        ui_show_line_numbers: pointer(root, "/ui/showLineNumbers").and_then(bool_value),
        ui_inline_thinking_mode: pointer(root, "/ui/inlineThinkingMode").and_then(string_value),
        usage_statistics_enabled: pointer(root, "/privacy/usageStatisticsEnabled")
            .and_then(bool_value),
        session_retention_enabled: pointer(root, "/general/sessionRetention/enabled")
            .and_then(bool_value),
        session_retention_max_age: pointer(root, "/general/sessionRetention/maxAge")
            .and_then(string_value),
        plan_model_routing: pointer(root, "/general/plan/modelRouting").and_then(bool_value),
        security_auth_selected_type: pointer(root, "/security/auth/selectedType")
            .and_then(string_value),
    }
}

fn set_nested_value(
    root: &mut serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) -> crate::shared::error::AppResult<()> {
    ensure_json_object_root(root)?;
    let mut current = root;

    for segment in &path[..path.len().saturating_sub(1)] {
        let object = current
            .as_object_mut()
            .ok_or_else(|| "gemini settings.json path parent must be an object".to_string())?;
        let entry = object
            .entry((*segment).to_string())
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(Default::default());
        }
        current = entry;
    }

    let object = current
        .as_object_mut()
        .ok_or_else(|| "gemini settings.json path parent must be an object".to_string())?;
    object.insert(path[path.len() - 1].to_string(), value);
    Ok(())
}

fn remove_nested_value_internal(current: &mut serde_json::Value, path: &[&str]) -> bool {
    let Some(object) = current.as_object_mut() else {
        return false;
    };

    if path.len() == 1 {
        object.remove(path[0]);
        return object.is_empty();
    }

    let should_remove_child = object
        .get_mut(path[0])
        .map(|child| remove_nested_value_internal(child, &path[1..]))
        .unwrap_or(false);

    if should_remove_child {
        object.remove(path[0]);
    }

    object.is_empty()
}

fn remove_nested_value(root: &mut serde_json::Value, path: &[&str]) {
    if path.is_empty() || !root.is_object() {
        return;
    }
    let _ = remove_nested_value_internal(root, path);
}

fn patch_string_field(
    root: &mut serde_json::Value,
    path: &[&str],
    patch: Option<String>,
) -> crate::shared::error::AppResult<()> {
    let Some(raw) = patch else {
        return Ok(());
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        remove_nested_value(root, path);
    } else {
        set_nested_value(root, path, serde_json::Value::String(trimmed.to_string()))?;
    }
    Ok(())
}

fn patch_nullable_field(
    root: &mut serde_json::Value,
    path: &[&str],
    patch: Option<serde_json::Value>,
) -> crate::shared::error::AppResult<()> {
    let Some(value) = patch else {
        return Ok(());
    };
    set_nested_value(root, path, value)
}

fn patch_optional_nullable_field(
    root: &mut serde_json::Value,
    path: &[&str],
    patch: Option<Option<serde_json::Value>>,
) -> crate::shared::error::AppResult<()> {
    let Some(value) = patch else {
        return Ok(());
    };

    match value {
        Some(value) => patch_nullable_field(root, path, Some(value)),
        None => {
            remove_nested_value(root, path);
            Ok(())
        }
    }
}

fn patch_gemini_config(
    mut root: serde_json::Value,
    patch: GeminiConfigPatch,
) -> crate::shared::error::AppResult<serde_json::Value> {
    ensure_json_object_root(&mut root)?;

    patch_string_field(&mut root, &["model", "name"], patch.model_name)?;
    patch_optional_nullable_field(
        &mut root,
        &["model", "maxSessionTurns"],
        patch
            .model_max_session_turns
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["model", "compressionThreshold"],
        patch
            .model_compression_threshold
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_string_field(
        &mut root,
        &["general", "defaultApprovalMode"],
        patch.default_approval_mode,
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "enableAutoUpdate"],
        patch
            .enable_auto_update
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "enableNotifications"],
        patch
            .enable_notifications
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "vimMode"],
        patch
            .vim_mode
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "retryFetchErrors"],
        patch
            .retry_fetch_errors
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "maxAttempts"],
        patch
            .max_attempts
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_string_field(&mut root, &["ui", "theme"], patch.ui_theme)?;
    patch_optional_nullable_field(
        &mut root,
        &["ui", "hideBanner"],
        patch
            .ui_hide_banner
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["ui", "hideTips"],
        patch
            .ui_hide_tips
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["ui", "showLineNumbers"],
        patch
            .ui_show_line_numbers
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_string_field(
        &mut root,
        &["ui", "inlineThinkingMode"],
        patch.ui_inline_thinking_mode,
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["privacy", "usageStatisticsEnabled"],
        patch
            .usage_statistics_enabled
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "sessionRetention", "enabled"],
        patch
            .session_retention_enabled
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_string_field(
        &mut root,
        &["general", "sessionRetention", "maxAge"],
        patch.session_retention_max_age,
    )?;
    patch_optional_nullable_field(
        &mut root,
        &["general", "plan", "modelRouting"],
        patch
            .plan_model_routing
            .map(|value| value.map(serde_json::Value::from)),
    )?;
    patch_string_field(
        &mut root,
        &["security", "auth", "selectedType"],
        patch.security_auth_selected_type,
    )?;

    Ok(root)
}

pub fn gemini_config_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<GeminiConfigState> {
    let config_dir = gemini_config_dir(app)?;
    let config_path = gemini_config_path(app)?;
    let exists = config_path.exists();
    let root = parse_json_root(read_optional_file(&config_path)?)?;

    Ok(make_state(
        config_dir.to_string_lossy().to_string(),
        config_path.to_string_lossy().to_string(),
        exists,
        &root,
    ))
}

pub fn gemini_config_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: GeminiConfigPatch,
) -> crate::shared::error::AppResult<GeminiConfigState> {
    let path = gemini_config_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let current = parse_json_root(read_optional_file(&path)?)?;
    let next = patch_gemini_config(current, patch)?;
    let bytes = json_to_bytes(&next, "gemini/settings.json")?;
    let _ = write_file_atomic_if_changed(&path, &bytes)?;
    gemini_config_get(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_patch() -> GeminiConfigPatch {
        GeminiConfigPatch::default()
    }

    #[test]
    fn patch_preserves_unknown_keys_and_updates_nested_values() {
        let input = serde_json::json!({
            "model": {
                "name": "gemini-2.5-flash",
                "keep": "model-extra"
            },
            "general": {
                "retryFetchErrors": false,
                "other": 7
            },
            "topLevel": true
        });

        let patched = patch_gemini_config(
            input,
            GeminiConfigPatch {
                model_name: Some("gemini-2.5-pro".to_string()),
                retry_fetch_errors: Some(Some(true)),
                ..empty_patch()
            },
        )
        .expect("patch");

        assert_eq!(
            patched.pointer("/model/name").and_then(|v| v.as_str()),
            Some("gemini-2.5-pro")
        );
        assert_eq!(
            patched.pointer("/model/keep").and_then(|v| v.as_str()),
            Some("model-extra")
        );
        assert_eq!(
            patched
                .pointer("/general/retryFetchErrors")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            patched.pointer("/general/other").and_then(|v| v.as_i64()),
            Some(7)
        );
        assert_eq!(
            patched.get("topLevel").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn patch_deletes_string_and_nullable_values_and_cleans_empty_objects() {
        let input = serde_json::json!({
            "model": { "name": "gemini-2.5-pro" },
            "privacy": { "usageStatisticsEnabled": true },
            "other": "keep"
        });

        let patched = patch_gemini_config(
            input,
            GeminiConfigPatch {
                model_name: Some(String::new()),
                usage_statistics_enabled: Some(None),
                ..empty_patch()
            },
        )
        .expect("patch");

        assert!(patched.get("model").is_none(), "{patched}");
        assert!(patched.get("privacy").is_none(), "{patched}");
        assert_eq!(patched.get("other").and_then(|v| v.as_str()), Some("keep"));
    }

    #[test]
    fn make_state_extracts_nested_values() {
        let root = serde_json::json!({
            "model": {
                "name": "gemini-2.5-pro",
                "maxSessionTurns": -1,
                "compressionThreshold": 0.7
            },
            "general": {
                "defaultApprovalMode": "plan",
                "enableAutoUpdate": true,
                "enableNotifications": false,
                "vimMode": true,
                "retryFetchErrors": true,
                "maxAttempts": 5
            },
            "ui": {
                "theme": "ANSI",
                "hideBanner": true,
                "hideTips": false,
                "showLineNumbers": true,
                "inlineThinkingMode": "full"
            },
            "privacy": {
                "usageStatisticsEnabled": false
            }
        });

        let state = make_state(
            "C:/Users/x/.gemini".to_string(),
            "C:/Users/x/.gemini/settings.json".to_string(),
            true,
            &root,
        );

        assert_eq!(state.model_name.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(state.model_max_session_turns, Some(-1));
        assert_eq!(state.model_compression_threshold, Some(0.7));
        assert_eq!(state.default_approval_mode.as_deref(), Some("plan"));
        assert_eq!(state.enable_auto_update, Some(true));
        assert_eq!(state.enable_notifications, Some(false));
        assert_eq!(state.vim_mode, Some(true));
        assert_eq!(state.retry_fetch_errors, Some(true));
        assert_eq!(state.max_attempts, Some(5));
        assert_eq!(state.ui_theme.as_deref(), Some("ANSI"));
        assert_eq!(state.ui_hide_banner, Some(true));
        assert_eq!(state.ui_hide_tips, Some(false));
        assert_eq!(state.ui_show_line_numbers, Some(true));
        assert_eq!(state.ui_inline_thinking_mode.as_deref(), Some("full"));
        assert_eq!(state.usage_statistics_enabled, Some(false));
    }

    #[test]
    fn nullable_patch_deserializers_accept_empty_string_as_delete() {
        let patch: GeminiConfigPatch = serde_json::from_value(serde_json::json!({
            "modelMaxSessionTurns": "",
            "enableAutoUpdate": "",
            "maxAttempts": "",
            "modelCompressionThreshold": ""
        }))
        .expect("deserialize patch");

        assert_eq!(patch.model_max_session_turns, Some(None));
        assert_eq!(patch.enable_auto_update, Some(None));
        assert_eq!(patch.max_attempts, Some(None));
        assert_eq!(patch.model_compression_threshold, Some(None));
    }
}
