//! Claude Desktop 3P profile adapter. Requests still use AIO's gateway.

use super::{
    read_optional_cli_proxy_file, read_optional_cli_proxy_file_with_max_len,
    write_cli_proxy_file_atomic, PLACEHOLDER_KEY,
};
use crate::providers::{ProviderModelEligibility, ProviderModelPolicyV1};
use crate::shared::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(super) const PROFILE_ID: &str = "00000000-0000-4000-8000-000000a10d35";
const PROFILE_NAME: &str = "AIO Coding Hub";
const CONFIG_FILE: &str = "claude_desktop_config.json";
const MODEL_CATALOG_MAX_BYTES: usize = 8 * 1024 * 1024;
/// Used when Desktop has not cached its model catalog yet.
const FALLBACK_MODELS: [&str; 4] = [
    "claude-sonnet-5",
    "claude-opus-5",
    "claude-fable-5",
    "claude-haiku-4-5",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopModel {
    pub(crate) id: String,
    pub(crate) supports_1m: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogModel {
    id: String,
    main: bool,
    supports_1m: bool,
}

/// Desktop caches its signed model catalog (verified before it is written) as
/// `{"documentBytes": base64(json)}`. The `ccd` surface is the Desktop picker;
/// its `main` section offered on `gateway` is what Desktop lists for 3P.
fn parse_catalog(bytes: &[u8]) -> Option<Vec<CatalogModel>> {
    use base64::Engine;

    let cache = serde_json::from_slice::<Value>(bytes).ok()?;
    let document = base64::engine::general_purpose::STANDARD
        .decode(cache.get("documentBytes")?.as_str()?)
        .ok()?;
    let document = serde_json::from_slice::<Value>(&document).ok()?;
    let models = document
        .pointer("/surfaces/ccd/model_selector_config")?
        .as_array()?
        .iter()
        .find(|config| config.get("id").and_then(Value::as_str) == Some("ccd"))?
        .get("models")?
        .as_array()?;
    Some(
        models
            .iter()
            .filter_map(|model| {
                let id = model.get("id")?.as_str()?.trim();
                if id.is_empty() || id.len() > 255 || id.chars().any(char::is_control) {
                    return None;
                }
                let on_gateway = model
                    .get("offered_on")
                    .and_then(Value::as_array)
                    .is_some_and(|hosts| hosts.iter().any(|host| host == "gateway"));
                Some(CatalogModel {
                    id: id.to_string(),
                    main: on_gateway
                        && model.get("section").and_then(Value::as_str) == Some("main"),
                    // Desktop's own gateway discovery uses the same threshold.
                    supports_1m: model
                        .pointer("/runtime/max_input_tokens")
                        .and_then(Value::as_u64)
                        .is_some_and(|tokens| tokens >= 1_000_000),
                })
            })
            .collect(),
    )
}

/// Without a catalog entry, current Opus, Sonnet and Fable models take 1M
/// context and Haiku does not.
fn family_supports_1m(id: &str) -> bool {
    ["opus", "sonnet", "fable"]
        .iter()
        .any(|family| id.contains(family))
}

/// Like the Codex catalog projection: Desktop's own catalog is the baseline
/// and exact mapping sources of routable providers are added. Wildcard
/// sources only apply to listed models.
fn model_list(
    catalog: Option<&[CatalogModel]>,
    policies: &[ProviderModelPolicyV1],
) -> Vec<DesktopModel> {
    let catalog_1m = |id: &str| {
        catalog
            .and_then(|models| models.iter().find(|model| model.id == id))
            .map_or_else(|| family_supports_1m(id), |model| model.supports_1m)
    };
    let mut models = catalog
        .map(|models| {
            models
                .iter()
                .filter(|model| model.main)
                .map(|model| DesktopModel {
                    id: model.id.clone(),
                    supports_1m: model.supports_1m,
                })
                .collect::<Vec<_>>()
        })
        .filter(|models| !models.is_empty())
        .unwrap_or_else(|| {
            FALLBACK_MODELS
                .iter()
                .map(|id| DesktopModel {
                    id: id.to_string(),
                    supports_1m: family_supports_1m(id),
                })
                .collect()
        });

    let mut sources = policies
        .iter()
        .flat_map(|policy| {
            policy.mappings.iter().filter(|mapping| {
                !mapping.source.contains('*')
                    && policy.eligibility(&mapping.source) == ProviderModelEligibility::Explicit
            })
        })
        .map(|mapping| mapping.source.as_str())
        .collect::<Vec<_>>();
    sources.sort_unstable();
    sources.dedup();
    for source in sources {
        if !models.iter().any(|model| model.id == source) {
            models.push(DesktopModel {
                id: source.to_string(),
                supports_1m: catalog_1m(source),
            });
        }
    }
    models
}

fn load_catalog(paths: &DesktopPaths) -> Option<Vec<CatalogModel>> {
    let bytes =
        match read_optional_cli_proxy_file_with_max_len(&paths.catalog, MODEL_CATALOG_MAX_BYTES) {
            Ok(bytes) => bytes?,
            Err(error) => {
                tracing::warn!(error = %error, "failed to read Claude Desktop model catalog");
                return None;
            }
        };
    let models = parse_catalog(&bytes);
    if models.is_none() {
        tracing::warn!("Claude Desktop model catalog has an unexpected format");
    }
    models
}

/// The models Desktop's picker and `/claude_desktop/v1/models` offer.
pub(crate) fn load_models<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
) -> AppResult<Vec<DesktopModel>> {
    let catalog = load_catalog(&paths(app)?);
    let policies =
        crate::providers::list_ready_model_policies_for_configured_routes(db, "claude_desktop")?;
    Ok(model_list(catalog.as_deref(), &policies))
}

/// Profile writes outside a provider change open the DB the same way the
/// Codex catalog projection does.
pub(super) fn app_models<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<Vec<DesktopModel>> {
    if !crate::db::db_path(app)?.exists() {
        return Ok(model_list(load_catalog(&paths(app)?).as_deref(), &[]));
    }
    load_models(app, &crate::db::init(app)?)
}

/// Desktop takes 1M support for an explicit model list only from the
/// profile's `supports1m`; the picker then offers a `<model>[1m]` variant and
/// keeps 200k as the default.
fn inference_models(models: &[DesktopModel]) -> Value {
    models
        .iter()
        .map(|model| json!({ "name": model.id, "supports1m": model.supports_1m }))
        .collect()
}

/// Checks the list shape only: the catalog and provider changes move its
/// content through `refresh_inference_models`, which must not read as drift.
fn has_current_model_list(profile: &Value) -> bool {
    profile
        .get("inferenceModels")
        .and_then(Value::as_array)
        .is_some_and(|models| {
            !models.is_empty()
                && models.iter().all(|model| {
                    model.get("name").and_then(Value::as_str).is_some()
                        && model.get("supports1m").is_some_and(Value::is_boolean)
                })
        })
}

/// Catalog and provider changes only move the model list, so rewrite just
/// that part of the profile. Desktop reads it on its next launch.
pub(super) fn refresh_inference_models<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
) -> AppResult<bool> {
    let paths = paths(app)?;
    let Some(current) = read_optional_cli_proxy_file(&paths.profile)? else {
        return Ok(false);
    };
    let mut value = object_from_bytes(Some(current), "desktop_profile")?;
    let models = inference_models(&load_models(app, db)?);
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
    catalog: PathBuf,
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
        catalog: threep.join("model-catalog").join("published.json"),
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
    models: &[DesktopModel],
) -> AppResult<Vec<u8>> {
    let mut value = object_from_bytes(current, kind)?;
    let object = value.as_object_mut().expect("validated object");
    match kind {
        "desktop_threep_config" => {
            object.insert("deploymentMode".into(), json!("3p"));
        }
        "desktop_profile" => {
            // Desktop's picker offers these Claude model IDs. Actual upstream
            // models are mapped by AIO's provider model policy on /claude_desktop.
            value = json!({
                "coworkEgressAllowedHosts": ["*"],
                "disableDeploymentModeChooser": true,
                "inferenceProvider": "gateway",
                "inferenceGatewayBaseUrl": format!("{base_origin}/claude_desktop"),
                "inferenceGatewayAuthScheme": "bearer",
                "inferenceGatewayApiKey": PLACEHOLDER_KEY,
                "inferenceModels": inference_models(models)
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
    fn profile_uses_aio_route_and_listed_models() {
        let models = [
            DesktopModel {
                id: "claude-opus-5-5".into(),
                supports_1m: true,
            },
            DesktopModel {
                id: "claude-haiku-4-5-20251001".into(),
                supports_1m: false,
            },
        ];
        let value: Value = serde_json::from_slice(
            &build_target("desktop_profile", None, "http://127.0.0.1:1234", &models).unwrap(),
        )
        .unwrap();
        assert_eq!(
            value["inferenceGatewayBaseUrl"],
            "http://127.0.0.1:1234/claude_desktop"
        );
        assert_eq!(value["inferenceGatewayApiKey"], PLACEHOLDER_KEY);
        assert_eq!(
            value["inferenceModels"],
            json!([
                {"name": "claude-opus-5-5", "supports1m": true},
                {"name": "claude-haiku-4-5-20251001", "supports1m": false}
            ])
        );
        assert!(has_current_model_list(&value));
    }

    #[test]
    fn model_list_follows_desktop_catalog_and_exact_mapping_sources() {
        use crate::providers::ProviderModelMode::{All, Excluded};
        use base64::Engine;

        let model = |id: &str, section: &str, hosts: &[&str], tokens: u64| {
            json!({
                "id": id,
                "section": section,
                "offered_on": hosts,
                "runtime": {"max_input_tokens": tokens}
            })
        };
        let document = json!({"surfaces": {"ccd": {"model_selector_config": [{
            "id": "ccd",
            "models": [
                model("claude-opus-5-5", "main", &["first_party", "gateway"], 1_000_000),
                model("claude-haiku-4-5-20251001", "main", &["gateway"], 200_000),
                model("claude-first-party-only", "main", &["first_party"], 1_000_000),
                model("claude-opus-5", "overflow", &["gateway"], 1_000_000),
            ]
        }]}}});
        let cache = json!({"documentBytes": base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(&document).unwrap())});
        let catalog = parse_catalog(&serde_json::to_vec(&cache).unwrap()).unwrap();

        let policy = |mode, patterns: &[&str], mappings: &[(&str, &str)]| ProviderModelPolicyV1 {
            version: 1,
            mode,
            model_patterns: patterns.iter().map(|value| value.to_string()).collect(),
            mappings: mappings
                .iter()
                .map(|(source, target)| crate::providers::ProviderModelMapping {
                    source: source.to_string(),
                    target: target.to_string(),
                })
                .collect(),
        };
        let policies = [
            policy(
                All,
                &[],
                &[
                    ("gpt-5", "upstream"),
                    ("claude-opus-5", "upstream"),
                    ("claude-sonnet-*", "upstream"),
                ],
            ),
            policy(
                Excluded,
                &["claude-blocked"],
                &[("claude-blocked", "upstream")],
            ),
        ];
        let listed = |models: Vec<DesktopModel>| {
            models
                .into_iter()
                .map(|model| (model.id, model.supports_1m))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            listed(model_list(Some(&catalog), &policies)),
            [
                ("claude-opus-5-5".to_string(), true),
                ("claude-haiku-4-5-20251001".to_string(), false),
                ("claude-opus-5".to_string(), true),
                ("gpt-5".to_string(), false),
            ]
        );
        assert_eq!(
            listed(model_list(None, &[])),
            [
                ("claude-sonnet-5".to_string(), true),
                ("claude-opus-5".to_string(), true),
                ("claude-fable-5".to_string(), true),
                ("claude-haiku-4-5".to_string(), false),
            ]
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
