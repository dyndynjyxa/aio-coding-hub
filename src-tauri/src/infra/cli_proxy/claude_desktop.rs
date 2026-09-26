//! Claude Desktop 3P profile adapter. Requests still use AIO's gateway.

use super::{read_optional_cli_proxy_file, write_cli_proxy_file_atomic, PLACEHOLDER_KEY};
use crate::providers::{ProviderModelEligibility, ProviderModelPolicyV1};
use crate::shared::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(super) const PROFILE_ID: &str = "00000000-0000-4000-8000-000000a10d35";
const PROFILE_NAME: &str = "AIO Coding Hub";
const CONFIG_FILE: &str = "claude_desktop_config.json";
pub(crate) const MODEL_ROUTES: [&str; 4] = [
    "claude-sonnet-5",
    "claude-opus-5",
    "claude-fable-5",
    "claude-haiku-4-5",
];

/// Desktop takes 1M support for an explicit model list only from the
/// profile's `supports1m`; the picker then offers a `<route>[1m]` variant and
/// keeps 200k as the default. Only routes in `routes_with_1m` get it.
fn inference_models(routes_with_1m: &[&str]) -> Value {
    MODEL_ROUTES
        .iter()
        .map(|name| json!({ "name": name, "supports1m": routes_with_1m.contains(name) }))
        .collect()
}

/// A route offers 1M when some routable Desktop provider that can serve it
/// has the 1M checkbox on; the gateway sends 1M requests only to those.
pub(crate) fn routes_with_1m(policies: &[ProviderModelPolicyV1]) -> Vec<&'static str> {
    MODEL_ROUTES
        .into_iter()
        .filter(|route| {
            policies.iter().any(|policy| {
                policy.supports_1m && policy.eligibility(route) != ProviderModelEligibility::Blocked
            })
        })
        .collect()
}

fn load_routes_with_1m(db: &crate::db::Db) -> AppResult<Vec<&'static str>> {
    let policies =
        crate::providers::list_ready_model_policies_for_configured_routes(db, "claude_desktop")?;
    Ok(routes_with_1m(&policies))
}

/// Profile writes outside a provider change open the DB the same way the
/// Codex catalog projection does.
pub(super) fn app_routes_with_1m<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<Vec<&'static str>> {
    if !crate::db::db_path(app)?.exists() {
        return Ok(Vec::new());
    }
    load_routes_with_1m(&crate::db::init(app)?)
}

/// Checks the list shape only; the 1M flags follow provider changes through
/// `refresh_inference_models`, which needs the DB.
fn has_current_model_list(profile: &Value) -> bool {
    profile
        .get("inferenceModels")
        .and_then(Value::as_array)
        .is_some_and(|models| {
            models.len() == MODEL_ROUTES.len()
                && models.iter().zip(MODEL_ROUTES).all(|(model, route)| {
                    model.get("name").and_then(Value::as_str) == Some(route)
                        && model.get("supports1m").is_some_and(Value::is_boolean)
                })
        })
}

/// Provider changes only move the 1M flags, so rewrite just the profile's
/// model list. Desktop reads it on its next launch.
pub(super) fn refresh_inference_models<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
) -> AppResult<bool> {
    let paths = paths(app)?;
    let Some(current) = read_optional_cli_proxy_file(&paths.profile)? else {
        return Ok(false);
    };
    let mut value = object_from_bytes(Some(current), "desktop_profile")?;
    let models = inference_models(&load_routes_with_1m(db)?);
    if value.get("inferenceModels") == Some(&models) {
        return Ok(false);
    }
    value
        .as_object_mut()
        .expect("validated object")
        .insert("inferenceModels".into(), models);
    write_cli_proxy_file_atomic(&paths.profile, &json_bytes(&value)?)?;
    Ok(true)
}

#[derive(Debug, Clone)]
pub(super) struct DesktopPaths {
    pub(super) threep: PathBuf,
    pub(super) profile: PathBuf,
    pub(super) meta: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ClaudeDesktopConfigStatus {
    pub threep_config_path: String,
    pub profile_path: String,
    pub deployment_mode: Option<String>,
    pub applied_profile_id: Option<String>,
    pub applied_profile_name: Option<String>,
}

fn paths_from_dir(threep: PathBuf) -> DesktopPaths {
    let library = threep.join("configLibrary");
    DesktopPaths {
        threep: threep.join(CONFIG_FILE),
        profile: library.join(format!("{PROFILE_ID}.json")),
        meta: library.join("_meta.json"),
    }
}

/// Desktop reads `deploymentMode` and the config library only from its 3P
/// user data directory; the 1P config never decides the deployment mode.
pub(super) fn paths<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<DesktopPaths> {
    // Desktop uses `CLAUDE_USER_DATA_DIR` as its 3P directory on every platform.
    if let Some(threep) = std::env::var_os("CLAUDE_USER_DATA_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(paths_from_dir(threep));
    }
    let home = super::home_dir(app)?;
    #[cfg(windows)]
    {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Local"));
        // The installed Windows app bundle uses LOCALAPPDATA/Claude-3p for
        // 3P data, even when the MSIX 1P data lives in LocalCache/Roaming.
        Ok(paths_from_dir(local.join("Claude-3p")))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(paths_from_dir(
            home.join("Library")
                .join("Application Support")
                .join("Claude-3p"),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        Ok(paths_from_dir(config.join("Claude-3p")))
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = home;
        Err("CLAUDE_DESKTOP_UNSUPPORTED_PLATFORM".into())
    }
}

pub(crate) fn mcp_config_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(paths(app)?.threep)
}

pub(crate) const NOT_INITIALIZED: &str = "CLAUDE_DESKTOP_NOT_INITIALIZED";
// Desktop uses this organization when the active 3P profile names none.
const DEFAULT_ORG_ID: &str = "00000000-0000-4000-8000-000000000001";

pub(crate) fn is_not_initialized(error: &AppError) -> bool {
    error.to_string().starts_with(NOT_INITIALIZED)
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.char_indices().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        })
}

struct DesktopIdentity {
    account: String,
    org: String,
}

/// 3P Desktop keys local sessions and Skills by the device ID in `ant-did`
/// (base64 UUID) and the active profile's `deploymentOrganizationUuid`.
fn identity(dir: &Path) -> AppResult<DesktopIdentity> {
    use base64::Engine;

    let Some(raw) = read_optional_cli_proxy_file(&dir.join("ant-did"))? else {
        return Err(format!(
            "{NOT_INITIALIZED}: 请先以当前配置启动一次 Claude Desktop，再管理它的 Prompts/Skills"
        )
        .into());
    };
    let account = base64::engine::general_purpose::STANDARD
        .decode(String::from_utf8_lossy(&raw).trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| is_uuid(value))
        // Desktop replaces an invalid ant-did with a new id on its next launch.
        .ok_or_else(|| {
            AppError::from(format!(
                "{NOT_INITIALIZED}: ant-did 无效，请先启动一次 Claude Desktop，再管理它的 Prompts/Skills"
            ))
        })?;

    let library = dir.join("configLibrary");
    let org = read_optional_cli_proxy_file(&library.join("_meta.json"))
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|meta| meta.get("appliedId")?.as_str().map(str::to_string))
        .filter(|id| is_uuid(id))
        .and_then(|id| read_optional_cli_proxy_file(&library.join(format!("{id}.json"))).ok())
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|profile| {
            profile
                .get("deploymentOrganizationUuid")?
                .as_str()
                .map(str::to_ascii_lowercase)
        })
        .filter(|id| is_uuid(id))
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    Ok(DesktopIdentity { account, org })
}

fn user_data_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    paths(app)?
        .threep
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "CLAUDE_DESKTOP_UNSUPPORTED_PLATFORM".into())
}

/// Desktop seeds this file into every new Cowork/Chat session as the user's
/// global instructions. It prefers the short `<account8>/<org8>` session root.
pub(crate) fn global_instructions_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<PathBuf> {
    let dir = user_data_dir(app)?;
    let id = identity(&dir)?;
    let sessions = dir.join("local-agent-mode-sessions");
    let short = sessions.join(&id.account[..8]).join(&id.org[..8]);
    let full = sessions.join(&id.account).join(&id.org);
    let root = match short.try_exists() {
        Ok(true) => short,
        Ok(false) => match full.try_exists() {
            Ok(false) => short,
            _ => full,
        },
        Err(_) => full,
    };
    Ok(root.join("memory").join("CLAUDE.md"))
}

/// Local Skills plugin that 3P Desktop loads into Chat, Cowork and Code.
pub(crate) fn skills_plugin_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<PathBuf> {
    let dir = user_data_dir(app)?;
    let id = identity(&dir)?;
    Ok(dir
        .join("local-agent-mode-sessions")
        .join("skills-plugin")
        .join(id.org)
        .join(id.account))
}

pub(super) fn inspect<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<ClaudeDesktopConfigStatus> {
    let paths = paths(app).ok()?;
    let config = read_optional_cli_proxy_file(&paths.threep)
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let meta = read_optional_cli_proxy_file(&paths.meta)
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let applied_profile_id = meta
        .as_ref()
        .and_then(|value| value.get("appliedId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let applied_profile_name = meta
        .as_ref()
        .and_then(|value| value.get("entries"))
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry.get("id").and_then(Value::as_str) == applied_profile_id.as_deref()
            })
        })
        .and_then(|entry| entry.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(ClaudeDesktopConfigStatus {
        threep_config_path: paths.threep.to_string_lossy().to_string(),
        profile_path: paths.profile.to_string_lossy().to_string(),
        deployment_mode: config
            .as_ref()
            .and_then(|value| value.get("deploymentMode"))
            .and_then(Value::as_str)
            .map(str::to_string),
        applied_profile_id,
        applied_profile_name,
    })
}

fn object_from_bytes(bytes: Option<Vec<u8>>, label: &str) -> AppResult<Value> {
    let value = match bytes {
        Some(bytes) if !bytes.is_empty() => {
            serde_json::from_slice::<Value>(&bytes).map_err(|error| {
                AppError::from(format!("CLAUDE_DESKTOP_INVALID_CONFIG: {label}: {error}"))
            })?
        }
        _ => json!({}),
    };
    if !value.is_object() {
        return Err(format!("CLAUDE_DESKTOP_INVALID_CONFIG: {label} must be an object").into());
    }
    Ok(value)
}

fn json_bytes(value: &Value) -> AppResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("CLAUDE_DESKTOP_SERIALIZE_FAILED: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn build_target(
    kind: &str,
    current: Option<Vec<u8>>,
    base_origin: &str,
    routes_with_1m: &[&str],
) -> AppResult<Vec<u8>> {
    let mut value = object_from_bytes(current, kind)?;
    let object = value.as_object_mut().expect("validated object");
    match kind {
        "desktop_threep_config" => {
            object.insert("deploymentMode".into(), json!("3p"));
        }
        "desktop_profile" => {
            // Desktop accepts only Claude role IDs. Actual upstream models are
            // mapped by AIO's provider model policy on /claude_desktop.
            value = json!({
                "coworkEgressAllowedHosts": ["*"],
                "disableDeploymentModeChooser": true,
                "inferenceProvider": "gateway",
                "inferenceGatewayBaseUrl": format!("{base_origin}/claude_desktop"),
                "inferenceGatewayAuthScheme": "bearer",
                "inferenceGatewayApiKey": PLACEHOLDER_KEY,
                "inferenceModels": inference_models(routes_with_1m)
            });
        }
        "desktop_meta" => {
            let mut entries = object
                .get("entries")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            entries.retain(|entry| entry.get("id").and_then(Value::as_str) != Some(PROFILE_ID));
            entries.push(json!({"id": PROFILE_ID, "name": PROFILE_NAME}));
            object.insert("entries".into(), Value::Array(entries));
            object.insert("appliedId".into(), json!(PROFILE_ID));
        }
        _ => return Err(format!("SEC_INVALID_INPUT: unknown desktop target {kind}").into()),
    }
    json_bytes(&value)
}

pub(super) fn is_applied<R: tauri::Runtime>(app: &tauri::AppHandle<R>, base_origin: &str) -> bool {
    let Ok(paths) = paths(app) else { return false };
    let Ok(Some(meta)) = read_optional_cli_proxy_file(&paths.meta) else {
        return false;
    };
    let Ok(Some(profile)) = read_optional_cli_proxy_file(&paths.profile) else {
        return false;
    };
    let Ok(Some(config)) = read_optional_cli_proxy_file(&paths.threep) else {
        return false;
    };
    let Ok(meta) = serde_json::from_slice::<Value>(&meta) else {
        return false;
    };
    let Ok(profile) = serde_json::from_slice::<Value>(&profile) else {
        return false;
    };
    let Ok(config) = serde_json::from_slice::<Value>(&config) else {
        return false;
    };
    meta.get("appliedId").and_then(Value::as_str) == Some(PROFILE_ID)
        && config.get("deploymentMode").and_then(Value::as_str) == Some("3p")
        && profile
            .get("inferenceGatewayBaseUrl")
            .and_then(Value::as_str)
            == Some(format!("{base_origin}/claude_desktop").as_str())
        // A profile written by an older build is re-synced on gateway start.
        && has_current_model_list(&profile)
}

pub(super) fn is_managed<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let Ok(paths) = paths(app) else { return false };
    let Ok(Some(meta)) = read_optional_cli_proxy_file(&paths.meta) else {
        return false;
    };
    serde_json::from_slice::<Value>(&meta)
        .ok()
        .and_then(|value| {
            value
                .get("appliedId")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .as_deref()
        == Some(PROFILE_ID)
}

/// Remove only fields managed by AIO. Other profiles, MCP servers and user
/// preferences can change while the proxy is active and must survive disable.
pub(super) fn merge_restore(kind: &str, target: &Path, backup: Option<&Path>) -> AppResult<()> {
    let current = read_optional_cli_proxy_file(target)?;
    let original = match backup {
        Some(path) => Some(super::read_cli_proxy_file(path)?),
        None => None,
    };
    let mut value = object_from_bytes(current.clone(), kind)?;
    let original_value = object_from_bytes(original.clone(), kind)?;
    let object = value.as_object_mut().expect("validated object");
    let original_object = original_value.as_object().expect("validated object");
    match kind {
        // Earlier manifests also wrote the 1P config; keep restoring it.
        "desktop_normal_config" | "desktop_threep_config" => {
            if object.get("deploymentMode").and_then(Value::as_str) == Some("3p") {
                if let Some(previous) = original_object.get("deploymentMode") {
                    object.insert("deploymentMode".into(), previous.clone());
                } else {
                    object.remove("deploymentMode");
                }
            }
        }
        "desktop_meta" => {
            if let Some(entries) = object.get_mut("entries").and_then(Value::as_array_mut) {
                entries.retain(|entry| entry.get("id").and_then(Value::as_str) != Some(PROFILE_ID));
            }
            if object.get("appliedId").and_then(Value::as_str) == Some(PROFILE_ID) {
                let previous = original_object
                    .get("appliedId")
                    .and_then(Value::as_str)
                    .filter(|id| {
                        object
                            .get("entries")
                            .and_then(Value::as_array)
                            .is_some_and(|entries| {
                                entries.iter().any(|entry| {
                                    entry.get("id").and_then(Value::as_str) == Some(*id)
                                })
                            })
                    });
                if let Some(previous) = previous {
                    object.insert("appliedId".into(), json!(previous));
                } else {
                    let next = object
                        .get("entries")
                        .and_then(Value::as_array)
                        .and_then(|entries| {
                            entries
                                .iter()
                                .find_map(|entry| entry.get("id").and_then(Value::as_str))
                        })
                        .map(str::to_string);
                    if let Some(next) = next {
                        object.insert("appliedId".into(), json!(next));
                    } else {
                        object.remove("appliedId");
                    }
                }
            }
        }
        "desktop_profile" => {
            // This file has an AIO-specific ID. Restore any pre-existing file;
            // otherwise remove only a profile that still points at AIO.
            if let Some(bytes) = original {
                return write_cli_proxy_file_atomic(target, &bytes);
            }
            let owned = object
                .get("inferenceGatewayBaseUrl")
                .and_then(Value::as_str)
                .is_some_and(|url| url.ends_with("/claude_desktop"));
            if owned && target.exists() {
                std::fs::remove_file(target)
                    .map_err(|error| format!("failed to remove {}: {error}", target.display()))?;
            }
            return Ok(());
        }
        _ => return Err(format!("SEC_INVALID_INPUT: unknown desktop target {kind}").into()),
    }
    if current.is_none() && value == json!({}) {
        return Ok(());
    }
    if original.is_none() && value == json!({}) {
        if target.exists() {
            std::fs::remove_file(target)
                .map_err(|error| format!("failed to remove {}: {error}", target.display()))?;
        }
        return Ok(());
    }
    write_cli_proxy_file_atomic(target, &json_bytes(&value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn new() -> Self {
            Self(Vec::new())
        }

        fn set(&mut self, key: &'static str, value: &std::path::Path) {
            self.0.push((key, std::env::var_os(key)));
            std::env::set_var(key, value);
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, previous) in self.0.drain(..).rev() {
                match previous {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            crate::test_support::clear_settings_cache();
        }
    }

    #[test]
    fn profile_uses_aio_route_and_claude_safe_models() {
        let value: Value = serde_json::from_slice(
            &build_target(
                "desktop_profile",
                None,
                "http://127.0.0.1:1234",
                &["claude-opus-5"],
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            value["inferenceGatewayBaseUrl"],
            "http://127.0.0.1:1234/claude_desktop"
        );
        assert_eq!(value["inferenceGatewayApiKey"], PLACEHOLDER_KEY);
        assert_eq!(value["inferenceModels"].as_array().unwrap().len(), 4);
        assert!(value["inferenceModels"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["name"].as_str().unwrap().starts_with("claude-")
                && v["supports1m"] == (v["name"] == "claude-opus-5")));
        assert!(has_current_model_list(&value));
    }

    #[test]
    fn only_checked_providers_that_serve_a_route_offer_1m() {
        let policy = |mode, patterns: &[&str], supports_1m| ProviderModelPolicyV1 {
            version: 1,
            mode,
            model_patterns: patterns.iter().map(|value| value.to_string()).collect(),
            mappings: Vec::new(),
            supports_1m,
        };
        use crate::providers::ProviderModelMode::{All, Excluded, Selected};

        assert!(routes_with_1m(&[policy(All, &[], false)]).is_empty());
        assert_eq!(routes_with_1m(&[policy(All, &[], true)]), MODEL_ROUTES);
        assert_eq!(
            routes_with_1m(&[
                policy(Selected, &["claude-sonnet-5"], true),
                policy(Excluded, &["claude-haiku-*"], true),
                policy(All, &[], false),
            ]),
            ["claude-sonnet-5", "claude-opus-5", "claude-fable-5"]
        );
    }

    #[test]
    fn invalid_ant_did_counts_as_not_initialized() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_not_initialized(&identity(dir.path()).err().unwrap()));
        std::fs::write(dir.path().join("ant-did"), "not-a-uuid").unwrap();
        assert!(is_not_initialized(&identity(dir.path()).err().unwrap()));
    }

    #[test]
    fn meta_preserves_other_profiles() {
        let before = br#"{"appliedId":"other","entries":[{"id":"other","name":"Other"}]}"#.to_vec();
        let value: Value = serde_json::from_slice(
            &build_target("desktop_meta", Some(before), "http://127.0.0.1:1234", &[]).unwrap(),
        )
        .unwrap();
        assert_eq!(value["entries"].as_array().unwrap().len(), 2);
        assert_eq!(value["appliedId"], PROFILE_ID);
    }

    #[test]
    fn restore_returns_to_previous_manager_without_losing_new_entries() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("_meta.json");
        let backup = temp.path().join("previous.json");
        let previous = json!({
            "appliedId": "00000000-0000-4000-8000-000000157210",
            "entries": [
                {"id": "00000000-0000-4000-8000-000000157210", "name": "CC Switch"}
            ],
            "otherSetting": true
        });
        std::fs::write(&backup, json_bytes(&previous).unwrap()).unwrap();
        let mut managed: Value = serde_json::from_slice(
            &build_target(
                "desktop_meta",
                Some(json_bytes(&previous).unwrap()),
                "http://127.0.0.1:1234",
                &[],
            )
            .unwrap(),
        )
        .unwrap();
        managed["entries"].as_array_mut().unwrap().push(json!({
            "id": "11111111-1111-4111-8111-111111111111", "name": "User profile"
        }));
        managed["newSetting"] = json!("keep");
        std::fs::write(&target, json_bytes(&managed).unwrap()).unwrap();

        merge_restore("desktop_meta", &target, Some(&backup)).unwrap();
        let restored: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(restored["appliedId"], previous["appliedId"]);
        assert_eq!(restored["entries"].as_array().unwrap().len(), 2);
        assert_eq!(restored["newSetting"], "keep");
    }

    #[test]
    fn restore_preserves_mcp_and_preference_changes() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join(CONFIG_FILE);
        let backup = temp.path().join("previous.json");
        let previous = json!({"deploymentMode": "1p", "preferences": {"theme": "light"}});
        std::fs::write(&backup, json_bytes(&previous).unwrap()).unwrap();
        let mut managed: Value = serde_json::from_slice(
            &build_target(
                "desktop_threep_config",
                Some(json_bytes(&previous).unwrap()),
                "http://127.0.0.1:1234",
                &[],
            )
            .unwrap(),
        )
        .unwrap();
        managed["preferences"]["theme"] = json!("dark");
        managed["mcpServers"] = json!({"user-server": {"command": "node"}});
        std::fs::write(&target, json_bytes(&managed).unwrap()).unwrap();

        merge_restore("desktop_threep_config", &target, Some(&backup)).unwrap();
        let restored: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(restored["deploymentMode"], "1p");
        assert_eq!(restored["preferences"]["theme"], "dark");
        assert_eq!(restored["mcpServers"]["user-server"]["command"], "node");
    }

    #[test]
    fn malformed_existing_config_is_never_replaced() {
        assert!(build_target(
            "desktop_threep_config",
            Some(b"{bad".to_vec()),
            "http://127.0.0.1:1234",
            &[]
        )
        .is_err());
    }

    #[test]
    fn proxy_toggle_restores_existing_cc_switch_profile() {
        let _lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().unwrap();
        let mut env = EnvRestore::new();
        env.set("LOCALAPPDATA", temp.path());
        env.set("XDG_CONFIG_HOME", temp.path());
        env.set("AIO_CODING_HUB_HOME_DIR", temp.path());
        env.set(
            "AIO_CODING_HUB_DOTDIR_NAME",
            std::path::Path::new(".aio-test"),
        );
        crate::test_support::clear_settings_cache();

        let app = tauri::test::mock_app();
        let handle = app.handle();
        let resolved = paths(handle).unwrap();
        let library = resolved.meta.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(
            &resolved.threep,
            br#"{"deploymentMode":"3p","preferences":{"theme":"dark"}}"#,
        )
        .unwrap();
        let cc_id = "00000000-0000-4000-8000-000000157210";
        let cc_file = library.join(format!("{cc_id}.json"));
        let cc_profile = br#"{"inferenceGatewayBaseUrl":"http://127.0.0.1:15721/claude-desktop"}"#;
        std::fs::write(&cc_file, cc_profile).unwrap();
        let meta_path = library.join("_meta.json");
        std::fs::write(
            &meta_path,
            json_bytes(
                &json!({"appliedId": cc_id, "entries": [{"id": cc_id, "name": "CC Switch"}]}),
            )
            .unwrap(),
        )
        .unwrap();

        let enabled =
            super::super::set_enabled(handle, "claude_desktop", true, "http://127.0.0.1:37123")
                .unwrap();
        assert!(enabled.ok, "{}", enabled.message);
        assert!(is_applied(handle, "http://127.0.0.1:37123"));
        assert_eq!(std::fs::read(&cc_file).unwrap(), cc_profile);
        let selected: Value = serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
        assert_eq!(selected["appliedId"], PROFILE_ID);

        let disabled =
            super::super::set_enabled(handle, "claude_desktop", false, "http://127.0.0.1:37123")
                .unwrap();
        assert!(disabled.ok, "{}", disabled.message);
        let restored: Value = serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
        assert_eq!(restored["appliedId"], cc_id);
        assert!(!library.join(format!("{PROFILE_ID}.json")).exists());
        assert_eq!(std::fs::read(&cc_file).unwrap(), cc_profile);
        let config: Value =
            serde_json::from_slice(&std::fs::read(&resolved.threep).unwrap()).unwrap();
        assert_eq!(config["preferences"]["theme"], "dark");
    }
}
