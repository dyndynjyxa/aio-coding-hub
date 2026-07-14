//! Usage: Shared, format-preserving access to the user-level Grok CLI config.

use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic_if_changed};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use toml_edit::DocumentMut;

const GROK_HOME_ENV: &str = "GROK_HOME";
pub(crate) const GROK_CONFIG_MAX_BYTES: usize = 1024 * 1024;
pub(crate) const DEFAULT_GROK_MODEL: &str = "grok-build";
const GROK_MODEL_ID_MAX_CHARS: usize = 256;

static PATH_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GrokApiBackend {
    #[default]
    Responses,
    ChatCompletions,
}

impl GrokApiBackend {
    pub(crate) fn as_config_value(self) -> &'static str {
        match self {
            Self::Responses => "responses",
            Self::ChatCompletions => "chat_completions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct GrokProxyPreferences {
    pub model_id: String,
    pub api_backend: GrokApiBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GrokPreferenceSource {
    ExistingConfig,
    Fallback,
    AioSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GrokPolicyKind {
    ManagedSystem,
    ManagedUser,
    RequirementsUser,
    RequirementsSystem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct GrokPolicyFileState {
    pub kind: GrokPolicyKind,
    pub path: String,
    pub exists: bool,
}

impl Default for GrokProxyPreferences {
    fn default() -> Self {
        Self {
            model_id: DEFAULT_GROK_MODEL.to_string(),
            api_backend: GrokApiBackend::Responses,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct GrokConfigState {
    pub config_path: String,
    pub file_exists: bool,
    pub preferences: GrokProxyPreferences,
    pub aio_preferences: Option<GrokProxyPreferences>,
    pub effective_preferences: GrokProxyPreferences,
    pub preference_source: GrokPreferenceSource,
    pub default_profile: Option<String>,
    pub session_summary_profile: Option<String>,
    pub web_search_profile: Option<String>,
    pub image_description_profile: Option<String>,
    pub policy_files: Vec<GrokPolicyFileState>,
}

pub(crate) fn validate_preferences(
    mut preferences: GrokProxyPreferences,
) -> crate::shared::error::AppResult<GrokProxyPreferences> {
    preferences.model_id = preferences.model_id.trim().to_string();
    if preferences.model_id.is_empty()
        || preferences.model_id.chars().any(char::is_control)
        || preferences.model_id.chars().count() > GROK_MODEL_ID_MAX_CHARS
    {
        return Err(format!(
            "SEC_INVALID_INPUT: Grok model_id must be 1-{GROK_MODEL_ID_MAX_CHARS} characters without control characters"
        )
        .into());
    }
    Ok(preferences)
}

pub(crate) fn merge_aio_preferences(
    candidate: &GrokProxyPreferences,
    aio_preferences: Option<GrokProxyPreferences>,
    has_existing_profile: bool,
) -> (GrokProxyPreferences, GrokPreferenceSource) {
    match aio_preferences {
        Some(preferences) => (preferences, GrokPreferenceSource::AioSettings),
        None if has_existing_profile => (candidate.clone(), GrokPreferenceSource::ExistingConfig),
        None => (candidate.clone(), GrokPreferenceSource::Fallback),
    }
}

fn policy_file_states(grok_home: &Path) -> Vec<GrokPolicyFileState> {
    let definitions = [
        (
            GrokPolicyKind::ManagedSystem,
            PathBuf::from("/etc/grok/managed_config.toml"),
        ),
        (
            GrokPolicyKind::ManagedUser,
            grok_home.join("managed_config.toml"),
        ),
        (
            GrokPolicyKind::RequirementsUser,
            grok_home.join("requirements.toml"),
        ),
        (
            GrokPolicyKind::RequirementsSystem,
            PathBuf::from("/etc/grok/requirements.toml"),
        ),
    ];
    definitions
        .into_iter()
        .map(|(kind, path)| GrokPolicyFileState {
            kind,
            exists: path.exists(),
            path: path.to_string_lossy().into_owned(),
        })
        .collect()
}

fn lexical_normalize_absolute(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn lock_path_key(path: &Path) -> crate::shared::error::AppResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("GROK_CONFIG_PATH_RESOLVE_FAILED: {error}"))?
            .join(path)
    };
    let mut ancestor = absolute.clone();
    let mut missing = Vec::new();

    loop {
        if ancestor.exists() {
            let mut normalized = ancestor.canonicalize().map_err(|error| {
                format!(
                    "GROK_CONFIG_PATH_RESOLVE_FAILED: {}: {error}",
                    ancestor.display()
                )
            })?;
            for component in missing.iter().rev() {
                normalized.push(component);
            }
            return Ok(normalized);
        }

        let Some(file_name) = ancestor.file_name() else {
            return Ok(lexical_normalize_absolute(&absolute));
        };
        missing.push(file_name.to_os_string());
        if !ancestor.pop() {
            return Ok(lexical_normalize_absolute(&absolute));
        }
    }
}

pub(crate) fn paths_equivalent(left: &Path, right: &Path) -> crate::shared::error::AppResult<bool> {
    Ok(lock_path_key(left)? == lock_path_key(right)?)
}

fn path_lock(path: &Path) -> crate::shared::error::AppResult<Arc<Mutex<()>>> {
    let path = lock_path_key(path)?;
    let locks = PATH_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: path registry")?;
    Ok(Arc::clone(
        locks
            .entry(path)
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    ))
}

fn expand_tilde(home: &Path, raw: &str) -> Option<PathBuf> {
    if raw == "~" {
        return Some(home.to_path_buf());
    }
    raw.strip_prefix("~/")
        .or_else(|| raw.strip_prefix("~\\"))
        .map(|suffix| home.join(suffix))
}

pub(crate) fn resolve_grok_home(home: &Path, override_value: Option<&str>) -> PathBuf {
    let Some(raw) = override_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return home.join(".grok");
    };
    if let Some(expanded) = expand_tilde(home, raw) {
        return expanded;
    }
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        home.join(path)
    }
}

pub(crate) fn grok_home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    let home = crate::app_paths::home_dir(app)?;
    let override_value = std::env::var(GROK_HOME_ENV).ok();
    Ok(resolve_grok_home(&home, override_value.as_deref()))
}

pub(crate) fn config_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(grok_home_dir(app)?.join("config.toml"))
}

pub(crate) fn skills_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(grok_home_dir(app)?.join("skills"))
}

pub(crate) fn agents_md_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(grok_home_dir(app)?.join("AGENTS.md"))
}

fn read_document_unlocked(
    path: &Path,
) -> crate::shared::error::AppResult<(DocumentMut, bool, Option<Vec<u8>>)> {
    let original = read_optional_file_with_max_len(path, GROK_CONFIG_MAX_BYTES)?;
    let Some(bytes) = original.as_ref() else {
        return Ok((DocumentMut::new(), false, None));
    };
    let source = std::str::from_utf8(bytes)
        .map_err(|error| format!("GROK_CONFIG_INVALID_UTF8: {}: {error}", path.display()))?;
    let document = source.parse::<DocumentMut>().map_err(|error| {
        format!(
            "GROK_CONFIG_INVALID_TOML: failed to parse {}: {error}",
            path.display()
        )
    })?;
    Ok((document, true, original))
}

fn table_string(document: &DocumentMut, table: &str, key: &str) -> Option<String> {
    document
        .get(table)
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|table| table.get(key))
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string)
}

fn model_profile_string(document: &DocumentMut, profile: &str, key: &str) -> Option<String> {
    document
        .get("model")
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|models| models.get(profile))
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|profile| profile.get(key))
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string)
}

fn table_item(document: &DocumentMut, table: &str, key: &str) -> Option<toml_edit::Item> {
    document
        .get(table)
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|table| table.get(key))
        .cloned()
}

fn model_profile_item(document: &DocumentMut, profile: &str, key: &str) -> Option<toml_edit::Item> {
    document
        .get("model")
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|models| models.get(profile))
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|profile| profile.get(key))
        .cloned()
}

fn restore_table_item(
    document: &mut DocumentMut,
    table: &str,
    key: &str,
    baseline: Option<toml_edit::Item>,
) {
    if let Some(item) = baseline {
        document[table][key] = item;
    } else if let Some(table) = document
        .get_mut(table)
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        table.remove(key);
    }
}

fn restore_model_profile_item(
    document: &mut DocumentMut,
    profile: &str,
    key: &str,
    baseline: Option<toml_edit::Item>,
) {
    if let Some(item) = baseline {
        document["model"][profile][key] = item;
        return;
    }

    let Some(profiles) = document
        .get_mut("model")
        .and_then(toml_edit::Item::as_table_like_mut)
    else {
        return;
    };
    let Some(profile) = profiles
        .get_mut(profile)
        .and_then(toml_edit::Item::as_table_like_mut)
    else {
        return;
    };
    profile.remove(key);
}

fn remove_proxy_created_empty_tables(document: &mut DocumentMut, baseline: &DocumentMut) {
    let baseline_aio_exists = baseline
        .get("model")
        .and_then(toml_edit::Item::as_table_like)
        .is_some_and(|models| models.contains_key("aio"));
    let aio_is_empty = document
        .get("model")
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|models| models.get("aio"))
        .and_then(toml_edit::Item::as_table_like)
        .is_some_and(toml_edit::TableLike::is_empty);
    if !baseline_aio_exists && aio_is_empty {
        if let Some(models) = document
            .get_mut("model")
            .and_then(toml_edit::Item::as_table_like_mut)
        {
            models.remove("aio");
        }
    }

    for table_name in ["models", "model"] {
        let baseline_exists = baseline.contains_key(table_name);
        let current_is_empty = document
            .get(table_name)
            .and_then(toml_edit::Item::as_table_like)
            .is_some_and(toml_edit::TableLike::is_empty);
        if !baseline_exists && current_is_empty {
            document.remove(table_name);
        }
    }
}

fn inspect_document(path: &Path, file_exists: bool, document: &DocumentMut) -> GrokConfigState {
    let default_profile = table_string(document, "models", "default");
    let model_id = default_profile
        .as_deref()
        .and_then(|profile| model_profile_string(document, profile, "model"))
        .or_else(|| default_profile.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_GROK_MODEL.to_string());
    let api_backend = default_profile
        .as_deref()
        .and_then(|profile| model_profile_string(document, profile, "api_backend"))
        .and_then(|backend| match backend.as_str() {
            "responses" => Some(GrokApiBackend::Responses),
            "chat_completions" => Some(GrokApiBackend::ChatCompletions),
            _ => None,
        })
        .unwrap_or_default();

    let preferences = GrokProxyPreferences {
        model_id,
        api_backend,
    };
    let has_existing_profile = default_profile.is_some();
    let (effective_preferences, preference_source) =
        merge_aio_preferences(&preferences, None, has_existing_profile);
    let grok_home = path.parent().unwrap_or_else(|| Path::new(""));

    GrokConfigState {
        config_path: path.to_string_lossy().into_owned(),
        file_exists,
        preferences,
        aio_preferences: None,
        effective_preferences,
        preference_source,
        default_profile,
        session_summary_profile: table_string(document, "models", "session_summary"),
        web_search_profile: table_string(document, "models", "web_search"),
        image_description_profile: table_string(document, "models", "image_description"),
        policy_files: policy_file_states(grok_home),
    }
}

pub(crate) fn inspect_path(path: &Path) -> crate::shared::error::AppResult<GrokConfigState> {
    let lock = path_lock(path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    let (document, file_exists, _) = read_document_unlocked(path)?;
    Ok(inspect_document(path, file_exists, &document))
}

pub(crate) fn read_bytes_path(path: &Path) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let lock = path_lock(path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    read_optional_file_with_max_len(path, GROK_CONFIG_MAX_BYTES)
}

pub(crate) fn restore_bytes_path(
    path: &Path,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    let lock = path_lock(path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    match bytes {
        Some(bytes) => {
            if bytes.len() > GROK_CONFIG_MAX_BYTES {
                return Err(format!(
                    "SEC_INVALID_INPUT: Grok config too large (max {GROK_CONFIG_MAX_BYTES} bytes)"
                )
                .into());
            }
            write_file_atomic_if_changed(path, &bytes)?;
        }
        None if path.exists() => {
            std::fs::remove_file(path)
                .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
        }
        None => {}
    }
    Ok(())
}

pub(crate) fn inspect<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<GrokConfigState> {
    inspect_path(&config_path(app)?)
}

pub(crate) fn get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<GrokConfigState> {
    let mut state = inspect(app)?;
    let aio_preferences = crate::settings::read(app)?.grok_proxy_preferences;
    let (effective_preferences, preference_source) = merge_aio_preferences(
        &state.preferences,
        aio_preferences.clone(),
        state.default_profile.is_some(),
    );
    state.aio_preferences = aio_preferences;
    state.effective_preferences = effective_preferences;
    state.preference_source = preference_source;
    Ok(state)
}

pub(crate) fn set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    preferences: GrokProxyPreferences,
) -> crate::shared::error::AppResult<GrokConfigState> {
    let preferences = validate_preferences(preferences)?;
    let previous_settings = crate::settings::read(app)?;
    inspect(app)?;
    let mut next_settings = previous_settings.clone();
    next_settings.grok_proxy_preferences = Some(preferences);
    crate::settings::write(app, &next_settings)?;

    match get(app) {
        Ok(state) => Ok(state),
        Err(error) => {
            if let Err(rollback_error) = crate::settings::write(app, &previous_settings) {
                return Err(format!(
                    "GROK_PREFERENCES_TRANSACTION_ROLLBACK_FAILED: {error}; settings rollback failed: {rollback_error}"
                )
                .into());
            }
            Err(error)
        }
    }
}

pub(crate) fn apply_proxy_profile<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    preferences: &GrokProxyPreferences,
    placeholder_key: &str,
) -> crate::shared::error::AppResult<()> {
    let preferences = validate_preferences(preferences.clone())?;
    let base_url = format!("{base_origin}/grok/v1");
    mutate_path(&config_path(app)?, |document| {
        {
            let profile = ensure_model_profile(document, "aio")?;
            set_table_like_string(profile, "model", &preferences.model_id);
            set_table_like_string(profile, "base_url", &base_url);
            set_table_like_string(profile, "api_key", placeholder_key);
            set_table_like_string(
                profile,
                "api_backend",
                preferences.api_backend.as_config_value(),
            );
        }
        {
            let models = ensure_root_table(document, "models", false)?;
            set_table_like_string(models, "default", "aio");
            set_table_like_string(models, "session_summary", "aio");
        }
        Ok(())
    })
}

pub(crate) fn is_proxy_profile_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    preferences: &GrokProxyPreferences,
    placeholder_key: &str,
) -> crate::shared::error::AppResult<bool> {
    let preferences = validate_preferences(preferences.clone())?;
    let path = config_path(app)?;
    let lock = path_lock(&path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    let (document, file_exists, _) = read_document_unlocked(&path)?;
    if !file_exists {
        return Ok(false);
    }

    let expected_base_url = format!("{base_origin}/grok/v1");
    Ok(
        table_string(&document, "models", "default").as_deref() == Some("aio")
            && table_string(&document, "models", "session_summary").as_deref() == Some("aio")
            && model_profile_string(&document, "aio", "model").as_deref()
                == Some(preferences.model_id.as_str())
            && model_profile_string(&document, "aio", "base_url").as_deref()
                == Some(expected_base_url.as_str())
            && model_profile_string(&document, "aio", "api_key").as_deref()
                == Some(placeholder_key)
            && model_profile_string(&document, "aio", "api_backend").as_deref()
                == Some(preferences.api_backend.as_config_value()),
    )
}

pub(crate) fn restore_proxy_fields_path(
    path: &Path,
    baseline_path: Option<&Path>,
) -> crate::shared::error::AppResult<()> {
    let baseline = match baseline_path {
        Some(path) => {
            let lock = path_lock(path)?;
            let _guard = lock
                .lock()
                .map_err(|_| "GROK_CONFIG_LOCK_POISONED: baseline path")?;
            let (document, file_exists, _) = read_document_unlocked(path)?;
            if !file_exists {
                return Err(format!(
                    "GROK_CONFIG_BACKUP_MISSING: expected baseline {}",
                    path.display()
                )
                .into());
            }
            document
        }
        None => DocumentMut::new(),
    };

    let lock = path_lock(path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    let (mut document, _, original) = read_document_unlocked(path)?;

    for key in ["default", "session_summary"] {
        restore_table_item(
            &mut document,
            "models",
            key,
            table_item(&baseline, "models", key),
        );
    }
    for key in ["model", "base_url", "api_key", "api_backend"] {
        restore_model_profile_item(
            &mut document,
            "aio",
            key,
            model_profile_item(&baseline, "aio", key),
        );
    }
    remove_proxy_created_empty_tables(&mut document, &baseline);

    let bytes = document.to_string().into_bytes();
    if baseline_path.is_none() && bytes.iter().all(u8::is_ascii_whitespace) {
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
        }
        return Ok(());
    }
    if original.as_deref() != Some(bytes.as_slice()) {
        write_file_atomic_if_changed(path, &bytes)?;
    }
    Ok(())
}

pub(crate) fn mutate_path<T>(
    path: &Path,
    mutation: impl FnOnce(&mut DocumentMut) -> crate::shared::error::AppResult<T>,
) -> crate::shared::error::AppResult<T> {
    let lock = path_lock(path)?;
    let _guard = lock
        .lock()
        .map_err(|_| "GROK_CONFIG_LOCK_POISONED: config path")?;
    let (mut document, _, original) = read_document_unlocked(path)?;
    let result = mutation(&mut document)?;
    let bytes = document.to_string().into_bytes();
    if bytes.len() > GROK_CONFIG_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: Grok config too large (max {GROK_CONFIG_MAX_BYTES} bytes)"
        )
        .into());
    }
    if original.as_deref() != Some(bytes.as_slice()) {
        write_file_atomic_if_changed(path, &bytes)?;
    }
    Ok(result)
}

pub(crate) fn set_string(item: &mut toml_edit::Item, value: &str) {
    let decor = item.as_value().map(|existing| existing.decor().clone());
    *item = toml_edit::value(value);
    if let (Some(decor), Some(next)) = (decor, item.as_value_mut()) {
        *next.decor_mut() = decor;
    }
}

fn ensure_root_table<'a>(
    document: &'a mut DocumentMut,
    key: &str,
    implicit: bool,
) -> crate::shared::error::AppResult<&'a mut dyn toml_edit::TableLike> {
    if !document.contains_key(key) {
        let mut table = toml_edit::Table::new();
        table.set_implicit(implicit);
        document.insert(key, toml_edit::Item::Table(table));
    }
    document
        .get_mut(key)
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| format!("GROK_CONFIG_INVALID_SCHEMA: {key} must be a table").into())
}

fn ensure_model_profile<'a>(
    document: &'a mut DocumentMut,
    profile: &str,
) -> crate::shared::error::AppResult<&'a mut dyn toml_edit::TableLike> {
    let models = ensure_root_table(document, "model", true)?;
    if !models.contains_key(profile) {
        models.insert(profile, toml_edit::table());
    }
    models
        .get_mut(profile)
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| {
            format!("GROK_CONFIG_INVALID_SCHEMA: model.{profile} must be a table").into()
        })
}

fn set_table_like_string(table: &mut dyn toml_edit::TableLike, key: &str, value: &str) {
    if let Some(item) = table.get_mut(key) {
        set_string(item, value);
    } else {
        table.insert(key, toml_edit::value(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn resolves_default_and_overridden_grok_home() {
        let home = std::path::PathBuf::from("/tmp/aio-grok-home");

        assert_eq!(resolve_grok_home(&home, None), home.join(".grok"));
        assert_eq!(
            resolve_grok_home(&home, Some("~/custom-grok")),
            home.join("custom-grok")
        );
        assert_eq!(
            resolve_grok_home(&home, Some("relative-grok")),
            home.join("relative-grok")
        );
        assert_eq!(
            resolve_grok_home(&home, Some("/opt/grok-home")),
            std::path::PathBuf::from("/opt/grok-home")
        );
    }

    #[test]
    fn missing_config_uses_grok_build_responses_candidate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");

        let state = inspect_path(&path).expect("inspect missing config");

        assert!(!state.file_exists);
        assert_eq!(state.preferences.model_id, "grok-build");
        assert_eq!(state.preferences.api_backend, GrokApiBackend::Responses);
        assert_eq!(state.default_profile, None);
    }

    #[test]
    fn existing_default_profile_supplies_model_and_explicit_backend() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"[models]
default = "custom"
session_summary = "summary"
web_search = "search"
image_description = "vision"

[model.custom]
model = "grok-4-fast"
api_backend = "chat_completions"
"#,
        )
        .expect("write fixture");

        let state = inspect_path(&path).expect("inspect config");

        assert!(state.file_exists);
        assert_eq!(state.default_profile.as_deref(), Some("custom"));
        assert_eq!(state.session_summary_profile.as_deref(), Some("summary"));
        assert_eq!(state.web_search_profile.as_deref(), Some("search"));
        assert_eq!(state.image_description_profile.as_deref(), Some("vision"));
        assert_eq!(state.preferences.model_id, "grok-4-fast");
        assert_eq!(
            state.preferences.api_backend,
            GrokApiBackend::ChatCompletions
        );
    }

    #[test]
    fn profile_id_is_model_fallback_and_unsupported_backend_becomes_responses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"[models]
default = "grok-3"

[model.grok-3]
api_backend = "messages"
"#,
        )
        .expect("write fixture");

        let state = inspect_path(&path).expect("inspect config");

        assert_eq!(state.preferences.model_id, "grok-3");
        assert_eq!(state.preferences.api_backend, GrokApiBackend::Responses);
    }

    #[test]
    fn mutation_preserves_comments_order_and_unknown_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# keep this comment\nunknown = \"keep\"\n\n[models]\ndefault = \"old\" # inline\n",
        )
        .expect("write fixture");

        mutate_path(&path, |document| {
            set_string(&mut document["models"]["default"], "aio");
            Ok(())
        })
        .expect("mutate config");

        let updated = std::fs::read_to_string(&path).expect("read updated");
        assert!(updated.starts_with("# keep this comment\nunknown = \"keep\""));
        assert!(updated.contains("default = \"aio\" # inline"));
    }

    #[test]
    fn invalid_toml_is_rejected_without_overwriting_original() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let original = b"[models\ndefault = broken\n";
        std::fs::write(&path, original).expect("write invalid fixture");

        let error = mutate_path(&path, |_| Ok(())).expect_err("invalid TOML must fail");

        assert!(error.to_string().contains("GROK_CONFIG_INVALID_TOML"));
        assert_eq!(std::fs::read(&path).expect("read original"), original);
    }

    #[test]
    fn oversized_config_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(&path, vec![b'x'; GROK_CONFIG_MAX_BYTES + 1]).expect("write fixture");

        let error = inspect_path(&path).expect_err("oversized config must fail");

        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn concurrent_mutations_on_same_path_do_not_lose_updates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Arc::new(dir.path().join("config.toml"));
        let mut handles = Vec::new();

        for index in 0..8 {
            let path = Arc::clone(&path);
            handles.push(std::thread::spawn(move || {
                let key = format!("server-{index}");
                mutate_path(&path, |document| {
                    document["mcp_servers"][&key]["command"] = toml_edit::value("npx");
                    Ok(())
                })
            }));
        }

        for handle in handles {
            handle.join().expect("join").expect("mutation");
        }

        let contents = std::fs::read_to_string(&*path).expect("read config");
        let document = contents
            .parse::<toml_edit::DocumentMut>()
            .expect("valid TOML");
        for index in 0..8 {
            assert_eq!(
                document["mcp_servers"][&format!("server-{index}")]["command"].as_str(),
                Some("npx")
            );
        }
    }

    #[test]
    fn equivalent_config_paths_share_the_same_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).expect("create nested dir");
        let direct = dir.path().join("config.toml");
        let equivalent = nested.join("..").join("config.toml");

        let direct_lock = path_lock(&direct).expect("direct lock");
        let equivalent_lock = path_lock(&equivalent).expect("equivalent lock");

        assert!(Arc::ptr_eq(&direct_lock, &equivalent_lock));
    }

    #[test]
    fn aio_preferences_override_file_candidate_without_mutating_it() {
        let candidate = GrokProxyPreferences::default();
        let saved = GrokProxyPreferences {
            model_id: "grok-4-fast".to_string(),
            api_backend: GrokApiBackend::ChatCompletions,
        };

        let (effective, source) = merge_aio_preferences(&candidate, Some(saved.clone()), true);

        assert_eq!(effective, saved);
        assert_eq!(source, GrokPreferenceSource::AioSettings);
        assert_eq!(candidate, GrokProxyPreferences::default());
    }

    #[test]
    fn preference_source_distinguishes_existing_config_from_fallback() {
        let candidate = GrokProxyPreferences::default();

        assert_eq!(
            merge_aio_preferences(&candidate, None, true).1,
            GrokPreferenceSource::ExistingConfig
        );
        assert_eq!(
            merge_aio_preferences(&candidate, None, false).1,
            GrokPreferenceSource::Fallback
        );
    }

    #[test]
    fn preference_validation_trims_model_and_rejects_invalid_values() {
        assert_eq!(
            validate_preferences(GrokProxyPreferences {
                model_id: "  grok-4-fast  ".to_string(),
                api_backend: GrokApiBackend::Responses,
            })
            .expect("valid preferences")
            .model_id,
            "grok-4-fast"
        );

        for invalid in ["", "   ", "grok\nmodel"] {
            assert!(validate_preferences(GrokProxyPreferences {
                model_id: invalid.to_string(),
                api_backend: GrokApiBackend::Responses,
            })
            .is_err());
        }
    }
}
