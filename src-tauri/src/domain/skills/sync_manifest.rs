//! Usage: Persist the last Grok Skills sync root so managed targets can be rebound safely.

use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const MANIFEST_MAX_BYTES: usize = 256 * 1024;
static GROK_SKILLS_SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GrokSkillsSyncManifest {
    schema_version: u32,
    managed_by: String,
    cli_key: String,
    pub(super) root_path: String,
    pub(super) managed_keys: Vec<String>,
}

pub(super) fn lock() -> crate::shared::error::AppResult<MutexGuard<'static, ()>> {
    GROK_SKILLS_SYNC_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "GROK_SKILLS_SYNC_LOCK_POISONED".into())
}

fn manifest_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::app_paths::app_data_dir(app)?
        .join("skills-sync")
        .join("grok")
        .join("manifest.json"))
}

fn is_safe_skill_key(key: &str) -> bool {
    if key.contains('/') || key.contains('\\') {
        return false;
    }
    let mut components = Path::new(key).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

pub(super) fn read<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Option<GrokSkillsSyncManifest>> {
    let path = manifest_path(app)?;
    let Some(bytes) = read_optional_file_with_max_len(&path, MANIFEST_MAX_BYTES)? else {
        return Ok(None);
    };
    let manifest = serde_json::from_slice::<GrokSkillsSyncManifest>(&bytes)
        .map_err(|error| format!("failed to parse Grok Skills sync manifest: {error}"))?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION
        || manifest.managed_by != MANAGED_BY
        || manifest.cli_key != "grok"
        || !Path::new(&manifest.root_path).is_absolute()
        || manifest
            .managed_keys
            .iter()
            .any(|key| !is_safe_skill_key(key))
    {
        return Err("GROK_SKILLS_SYNC_MANIFEST_INVALID".into());
    }
    Ok(Some(manifest))
}

pub(super) fn write<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    root_path: &Path,
    mut managed_keys: Vec<String>,
) -> crate::shared::error::AppResult<()> {
    if !root_path.is_absolute() || managed_keys.iter().any(|key| !is_safe_skill_key(key)) {
        return Err("GROK_SKILLS_SYNC_MANIFEST_INVALID".into());
    }
    managed_keys.sort();
    managed_keys.dedup();
    let manifest = GrokSkillsSyncManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: "grok".to_string(),
        root_path: root_path.to_string_lossy().to_string(),
        managed_keys,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("failed to serialize Grok Skills sync manifest: {error}"))?;
    if bytes.len() > MANIFEST_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: Grok Skills sync manifest too large (max {MANIFEST_MAX_BYTES} bytes)"
        )
        .into());
    }
    write_file_atomic(&manifest_path(app)?, &bytes)
}

#[cfg(test)]
mod tests {
    use super::is_safe_skill_key;

    #[test]
    fn manifest_skill_keys_must_be_single_normal_path_components() {
        assert!(is_safe_skill_key("demo-skill"));
        assert!(!is_safe_skill_key(""));
        assert!(!is_safe_skill_key("."));
        assert!(!is_safe_skill_key(".."));
        assert!(!is_safe_skill_key("nested/demo"));
        assert!(!is_safe_skill_key("nested\\demo"));
    }
}
