//! Usage: Read / patch Antigravity CLI `settings.json` (~/.gemini/antigravity-cli/settings.json).
//!
//! agy stores only non-default values and rejects unknown enum values with a blocking
//! "Settings Error" screen, so every enum written here is validated first. `modelProvider`
//! is deliberately never written: `"gemini"` there would make agy bypass the gateway
//! configured in ~/.gemini/config/config.json.

use crate::shared::fs::{
    is_symlink, read_optional_file_with_max_len, write_file_atomic_if_changed,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const GEMINI_CONFIG_MAX_BYTES: usize = 1024 * 1024;
const GEMINI_MODEL_LABEL_MAX_CHARS: usize = 200;
/// UI value for agy's explicit `default` agent mode, stored on disk as `""`.
/// An absent key means "CLI default" (`auto` where available since agy 1.5.7).
const AGENT_MODE_DEFAULT: &str = "default";

const AGENT_MODES: &[&str] = &[AGENT_MODE_DEFAULT, "accept-edits", "auto", "plan"];
const TOOL_PERMISSIONS: &[&str] = &[
    "request-review",
    "proceed-in-sandbox",
    "always-proceed",
    "strict",
];
const ARTIFACT_REVIEW_POLICIES: &[&str] = &["asks-for-review", "agent-decides", "always-proceed"];
const COLOR_SCHEMES: &[&str] = &[
    "terminal",
    "light",
    "solarized light",
    "colorblind-friendly light",
    "dark",
    "solarized dark",
    "colorblind-friendly dark",
    "tokyo night",
];
const ALT_SCREEN_MODES: &[&str] = &["default", "always", "never"];
const VERBOSITY_LEVELS: &[&str] = &["high", "medium", "low"];
const EDITOR_MODES: &[&str] = &["default", "vim"];

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiConfigState {
    pub config_dir: String,
    pub config_path: String,
    pub exists: bool,
    pub model: Option<String>,
    pub agent_mode: Option<String>,
    pub tool_permission: Option<String>,
    pub artifact_review_policy: Option<String>,
    pub enable_terminal_sandbox: Option<bool>,
    pub allow_non_workspace_access: Option<bool>,
    pub color_scheme: Option<String>,
    pub alt_screen_mode: Option<String>,
    pub verbosity: Option<String>,
    pub editor_mode: Option<String>,
    pub notifications: Option<bool>,
    pub show_tips: Option<bool>,
}

/// `None`/`null` leaves a key untouched; an empty string removes it (CLI default).
#[derive(Debug, Clone, Deserialize, specta::Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiConfigPatch {
    pub model: Option<String>,
    pub agent_mode: Option<String>,
    pub tool_permission: Option<String>,
    pub artifact_review_policy: Option<String>,
    pub enable_terminal_sandbox: Option<bool>,
    pub allow_non_workspace_access: Option<bool>,
    pub color_scheme: Option<String>,
    pub alt_screen_mode: Option<String>,
    pub verbosity: Option<String>,
    pub editor_mode: Option<String>,
    pub notifications: Option<bool>,
    pub show_tips: Option<bool>,
}

fn gemini_config_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::app_paths::home_dir(app)?
        .join(".gemini")
        .join("antigravity-cli"))
}

fn parse_json_root(bytes: Option<Vec<u8>>) -> crate::shared::error::AppResult<serde_json::Value> {
    let Some(bytes) = bytes else {
        return Ok(serde_json::json!({}));
    };
    // agy tolerates a UTF-8 BOM (fixed in 1.5.9); serde_json does not.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let root: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("failed to parse antigravity settings.json: {e}"))?;
    if !root.is_object() {
        return Err("antigravity settings.json root must be a JSON object".into());
    }
    Ok(root)
}

fn string_at(root: &serde_json::Value, key: &str) -> Option<String> {
    root.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

fn bool_at(root: &serde_json::Value, key: &str) -> Option<bool> {
    root.get(key).and_then(|v| v.as_bool())
}

fn make_state(
    config_dir: &Path,
    config_path: &Path,
    exists: bool,
    root: &serde_json::Value,
) -> GeminiConfigState {
    let agent_mode = match root.get("agentMode").and_then(|v| v.as_str()) {
        Some(raw) if raw.trim().is_empty() => Some(AGENT_MODE_DEFAULT.to_string()),
        _ => string_at(root, "agentMode"),
    };
    GeminiConfigState {
        config_dir: config_dir.to_string_lossy().to_string(),
        config_path: config_path.to_string_lossy().to_string(),
        exists,
        model: string_at(root, "model"),
        agent_mode,
        tool_permission: string_at(root, "toolPermission"),
        artifact_review_policy: string_at(root, "artifactReviewPolicy"),
        enable_terminal_sandbox: bool_at(root, "enableTerminalSandbox"),
        allow_non_workspace_access: bool_at(root, "allowNonWorkspaceAccess"),
        color_scheme: string_at(root, "colorScheme"),
        alt_screen_mode: string_at(root, "altScreenMode"),
        verbosity: string_at(root, "verbosity"),
        editor_mode: string_at(root, "editorMode"),
        notifications: bool_at(root, "notifications"),
        show_tips: bool_at(root, "showTips"),
    }
}

fn patch_enum(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    patch: Option<String>,
    allowed: &[&str],
) -> crate::shared::error::AppResult<()> {
    let Some(raw) = patch else {
        return Ok(());
    };
    let value = raw.trim();
    if value.is_empty() {
        root.remove(key);
        return Ok(());
    }
    if !allowed.contains(&value) {
        return Err(format!("SEC_INVALID_INPUT: unsupported {key}={value}").into());
    }
    let stored = if key == "agentMode" && value == AGENT_MODE_DEFAULT {
        ""
    } else {
        value
    };
    root.insert(
        key.to_string(),
        serde_json::Value::String(stored.to_string()),
    );
    Ok(())
}

fn patch_model(
    root: &mut serde_json::Map<String, serde_json::Value>,
    patch: Option<String>,
) -> crate::shared::error::AppResult<()> {
    let Some(raw) = patch else {
        return Ok(());
    };
    let value = raw.trim();
    if value.is_empty() {
        root.remove("model");
        return Ok(());
    }
    if value.chars().count() > GEMINI_MODEL_LABEL_MAX_CHARS || value.chars().any(char::is_control) {
        return Err("SEC_INVALID_INPUT: invalid model label".into());
    }
    root.insert(
        "model".to_string(),
        serde_json::Value::String(value.to_string()),
    );
    Ok(())
}

fn patch_bool(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    patch: Option<bool>,
) {
    if let Some(value) = patch {
        root.insert(key.to_string(), serde_json::Value::Bool(value));
    }
}

fn patch_gemini_config(
    mut root: serde_json::Value,
    patch: GeminiConfigPatch,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let map = root
        .as_object_mut()
        .ok_or("antigravity settings.json root must be a JSON object")?;

    patch_model(map, patch.model)?;
    patch_enum(map, "agentMode", patch.agent_mode, AGENT_MODES)?;
    patch_enum(
        map,
        "toolPermission",
        patch.tool_permission,
        TOOL_PERMISSIONS,
    )?;
    patch_enum(
        map,
        "artifactReviewPolicy",
        patch.artifact_review_policy,
        ARTIFACT_REVIEW_POLICIES,
    )?;
    patch_enum(map, "colorScheme", patch.color_scheme, COLOR_SCHEMES)?;
    patch_enum(
        map,
        "altScreenMode",
        patch.alt_screen_mode,
        ALT_SCREEN_MODES,
    )?;
    patch_enum(map, "verbosity", patch.verbosity, VERBOSITY_LEVELS)?;
    patch_enum(map, "editorMode", patch.editor_mode, EDITOR_MODES)?;
    patch_bool(map, "enableTerminalSandbox", patch.enable_terminal_sandbox);
    patch_bool(
        map,
        "allowNonWorkspaceAccess",
        patch.allow_non_workspace_access,
    );
    patch_bool(map, "notifications", patch.notifications);
    patch_bool(map, "showTips", patch.show_tips);

    Ok(root)
}

fn json_to_bytes(value: &serde_json::Value) -> crate::shared::error::AppResult<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(value)
        .map_err(|e| format!("failed to serialize antigravity settings.json: {e}"))?;
    out.push(b'\n');
    if out.len() > GEMINI_CONFIG_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: antigravity settings.json too large (max {GEMINI_CONFIG_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(out)
}

fn read_config(path: &Path) -> crate::shared::error::AppResult<serde_json::Value> {
    parse_json_root(read_optional_file_with_max_len(
        path,
        GEMINI_CONFIG_MAX_BYTES,
    )?)
}

fn write_config(path: &Path, root: &serde_json::Value) -> crate::shared::error::AppResult<()> {
    if path.exists() && is_symlink(path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }
    let changed = write_file_atomic_if_changed(path, &json_to_bytes(root)?)?;
    // agy keeps this file user-only (0600); it can hold proxy credentials.
    #[cfg(unix)]
    if changed {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("failed to restrict {}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = changed;
    Ok(())
}

fn config_get_at(dir: &Path) -> crate::shared::error::AppResult<GeminiConfigState> {
    let path = dir.join("settings.json");
    let root = read_config(&path)?;
    Ok(make_state(dir, &path, path.exists(), &root))
}

fn config_set_at(
    dir: &Path,
    patch: GeminiConfigPatch,
) -> crate::shared::error::AppResult<GeminiConfigState> {
    let path = dir.join("settings.json");
    let next = patch_gemini_config(read_config(&path)?, patch)?;
    write_config(&path, &next)?;
    config_get_at(dir)
}

pub fn gemini_config_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<GeminiConfigState> {
    config_get_at(&gemini_config_dir(app)?)
}

pub fn gemini_config_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: GeminiConfigPatch,
) -> crate::shared::error::AppResult<GeminiConfigState> {
    config_set_at(&gemini_config_dir(app)?, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(path).expect("read")).expect("json")
    }

    #[test]
    fn set_preserves_unknown_keys_and_writes_agy_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"$schema":"https://antigravity.google/schemas/v1/settings.json","trustedWorkspaces":["/w"],"permissions":{"allow":["x"]}}"#,
        )
        .expect("seed");

        let state = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                model: Some(" Gemini 3.8 Flash (Low) ".into()),
                tool_permission: Some("strict".into()),
                notifications: Some(true),
                ..Default::default()
            },
        )
        .expect("set");

        let root = read_json(&path);
        assert_eq!(root["model"], "Gemini 3.8 Flash (Low)");
        assert_eq!(root["toolPermission"], "strict");
        assert_eq!(root["notifications"], true);
        assert_eq!(root["trustedWorkspaces"][0], "/w");
        assert_eq!(root["permissions"]["allow"][0], "x");
        assert!(root.get("$schema").is_some());
        assert!(root.get("modelProvider").is_none());
        assert!(state.exists);
        assert_eq!(state.model.as_deref(), Some("Gemini 3.8 Flash (Low)"));
        assert_eq!(state.tool_permission.as_deref(), Some("strict"));
        assert_eq!(state.notifications, Some(true));
    }

    #[test]
    fn missing_file_reads_as_empty_and_is_created_user_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("antigravity-cli");

        let state = config_get_at(&nested).expect("get");
        assert!(!state.exists);
        assert_eq!(state.model, None);
        assert_eq!(state.agent_mode, None);

        config_set_at(
            &nested,
            GeminiConfigPatch {
                show_tips: Some(false),
                ..Default::default()
            },
        )
        .expect("set");
        let path = nested.join("settings.json");
        assert_eq!(read_json(&path), serde_json::json!({ "showTips": false }));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn empty_string_removes_key_and_null_leaves_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"model":"Gemini 3.1 Pro (High)","colorScheme":"dark","verbosity":"low"}"#,
        )
        .expect("seed");

        let state = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                model: Some(String::new()),
                color_scheme: Some("  ".into()),
                verbosity: None,
                ..Default::default()
            },
        )
        .expect("set");

        assert_eq!(state.model, None);
        assert_eq!(state.color_scheme, None);
        assert_eq!(state.verbosity.as_deref(), Some("low"));
    }

    #[test]
    fn agent_mode_default_round_trips_as_empty_string() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                agent_mode: Some("default".into()),
                ..Default::default()
            },
        )
        .expect("set");

        assert_eq!(
            read_json(&dir.path().join("settings.json"))["agentMode"],
            ""
        );
        assert_eq!(state.agent_mode.as_deref(), Some("default"));

        let state = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                agent_mode: Some(String::new()),
                ..Default::default()
            },
        )
        .expect("reset");
        assert_eq!(state.agent_mode, None);
    }

    #[test]
    fn rejects_unknown_enum_value_without_writing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"toolPermission":"strict"}"#).expect("seed");

        let err = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                tool_permission: Some("yolo".into()),
                notifications: Some(true),
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("SEC_INVALID_INPUT"), "{err}");
        assert_eq!(
            read_json(&path),
            serde_json::json!({ "toolPermission": "strict" })
        );
    }

    #[test]
    fn rejects_control_characters_in_model_label() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                model: Some("Gemini\n3".into()),
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("invalid model label"), "{err}");
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn reads_file_with_utf8_bom_and_reports_unknown_values_as_is() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(br#"{"verbosity":"future-level","enableTerminalSandbox":true}"#);
        std::fs::write(dir.path().join("settings.json"), bytes).expect("seed");

        let state = config_get_at(dir.path()).expect("get");
        assert_eq!(state.verbosity.as_deref(), Some("future-level"));
        assert_eq!(state.enable_terminal_sandbox, Some(true));
    }

    #[test]
    fn invalid_json_and_non_object_root_are_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");

        std::fs::write(&path, "{not json").expect("seed");
        assert!(config_get_at(dir.path()).is_err());

        std::fs::write(&path, "[]").expect("seed");
        let err = config_set_at(dir.path(), GeminiConfigPatch::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("must be a JSON object"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "[]");
    }

    #[test]
    fn rejects_oversized_settings_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("settings.json"),
            vec![b' '; GEMINI_CONFIG_MAX_BYTES + 1],
        )
        .expect("seed");
        let err = config_get_at(dir.path()).unwrap_err().to_string();
        assert!(err.contains("too large"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_write_through_symlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("real.json");
        std::fs::write(&target, "{}").expect("seed");
        std::os::unix::fs::symlink(&target, dir.path().join("settings.json")).expect("symlink");

        let err = config_set_at(
            dir.path(),
            GeminiConfigPatch {
                notifications: Some(true),
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("symlink"), "{err}");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "{}");
    }
}
