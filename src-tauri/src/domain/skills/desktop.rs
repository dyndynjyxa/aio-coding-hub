//! Usage: Claude Desktop skills-plugin manifest.
//!
//! Desktop loads only the skill dirs listed in `manifest.json` next to
//! `skills/`, and deletes unlisted dirs on its next sync. Entries marked
//! `syncManaged: false` survive that sync, so each user skill dir gets one.

use super::fs_ops::skill_md_path;
use super::skill_md::parse_skill_md;
use crate::shared::error::AppResult;
use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};
use crate::shared::time::now_unix_millis;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) const CLI_KEY: &str = "claude_desktop";
const MANIFEST_MAX_BYTES: usize = 4 * 1024 * 1024;
const PLUGIN_JSON: &str = "{\n  \"name\": \"anthropic-skills\",\n  \"version\": \"1.0.0\",\n  \"description\": \"Anthropic-managed skills for Claude Desktop\"\n}\n";

/// Desktop matches manifest names to dir names case-insensitively, after
/// replacing characters that are invalid in file names.
fn dir_key(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            '<' | '>' | '"' | '|' | '?' | '*' | '\\' | '/' => '_',
            _ => ch,
        })
        .collect::<String>()
        .to_lowercase()
}

fn manifest_path(skills_root: &Path) -> PathBuf {
    skills_root
        .parent()
        .unwrap_or(skills_root)
        .join("manifest.json")
}

fn read_manifest(path: &Path) -> AppResult<Map<String, Value>> {
    let Some(bytes) = read_optional_file_with_max_len(path, MANIFEST_MAX_BYTES)? else {
        return Ok(Map::new());
    };
    match serde_json::from_slice(&bytes) {
        Ok(Value::Object(manifest)) => Ok(manifest),
        _ => Err(format!("CLAUDE_DESKTOP_INVALID_CONFIG: {}", path.display()).into()),
    }
}

fn entry_key(entry: &Value) -> Option<String> {
    entry.get("name").and_then(Value::as_str).map(dir_key)
}

fn is_user_entry(entry: &Value) -> bool {
    entry.get("creatorType").and_then(Value::as_str) == Some("user")
}

fn is_local_user_entry(entry: &Value) -> bool {
    is_user_entry(entry) && entry.get("syncManaged").and_then(Value::as_bool) == Some(false)
}

/// Skills Desktop ships itself. AIO does not list, move or overwrite them.
pub(crate) struct BuiltinSkills(HashSet<String>);

impl BuiltinSkills {
    /// Empty for every CLI other than Claude Desktop.
    pub(crate) fn load(cli_key: &str, skills_root: &Path) -> AppResult<Self> {
        let mut keys = HashSet::new();
        if cli_key == CLI_KEY {
            let manifest = read_manifest(&manifest_path(skills_root))?;
            let entries = manifest.get("skills").and_then(Value::as_array);
            for entry in entries.into_iter().flatten() {
                if !is_user_entry(entry) {
                    keys.extend(entry_key(entry));
                }
            }
        }
        Ok(Self(keys))
    }

    pub(crate) fn contains(&self, dir_name: &str) -> bool {
        self.0.contains(&dir_key(dir_name))
    }
}

/// Keys of every dir under `skills_root`, and the skill dirs among them with
/// their dir name and SKILL.md description (`None` when it cannot be parsed).
type SkillDirsOnDisk = (HashSet<String>, BTreeMap<String, (String, Option<String>)>);

fn skill_dirs_on_disk(skills_root: &Path) -> AppResult<SkillDirsOnDisk> {
    let mut all = HashSet::new();
    let mut skills = BTreeMap::new();
    let entries = match std::fs::read_dir(skills_root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((all, skills)),
        Err(e) => return Err(format!("failed to read dir {}: {e}", skills_root.display()).into()),
    };
    for entry in entries {
        let entry = entry
            .map_err(|e| format!("failed to read dir entry {}: {e}", skills_root.display()))?;
        let path = entry.path();
        let Some(dir_name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if dir_name.starts_with('.') || !path.is_dir() {
            continue;
        }
        all.insert(dir_key(dir_name));
        let Some(skill_md) = skill_md_path(&path)? else {
            continue;
        };
        let description = parse_skill_md(&skill_md)
            .ok()
            .map(|(_, description)| description);
        skills.insert(dir_key(dir_name), (dir_name.to_string(), description));
    }
    Ok((all, skills))
}

fn now_iso() -> String {
    chrono::DateTime::from_timestamp_millis(now_unix_millis())
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Lists every user skill dir under `skills_root` in Desktop's manifest and
/// drops local entries whose dir is gone. Desktop's own entries stay as they are.
pub(super) fn reconcile(cli_key: &str, skills_root: &Path) -> AppResult<()> {
    if cli_key != CLI_KEY {
        return Ok(());
    }
    let path = manifest_path(skills_root);
    let mut manifest = read_manifest(&path)?;
    let mut entries = match manifest.remove("skills") {
        None => Vec::new(),
        Some(Value::Array(entries)) => entries,
        Some(_) => return Err(format!("CLAUDE_DESKTOP_INVALID_CONFIG: {}", path.display()).into()),
    };
    let before = entries.clone();
    let (dirs_on_disk, skills_on_disk) = skill_dirs_on_disk(skills_root)?;

    // Only a dir that is gone drops its entry; one without a readable SKILL.md
    // right now (being copied or edited) keeps it.
    entries.retain(|entry| {
        !is_local_user_entry(entry)
            || entry_key(entry).is_some_and(|key| dirs_on_disk.contains(&key))
    });
    for (key, (dir_name, description)) in skills_on_disk {
        match entries
            .iter_mut()
            .find(|entry| entry_key(entry).as_deref() == Some(key.as_str()))
        {
            Some(entry) => {
                let Some(description) = description else {
                    continue;
                };
                let stale =
                    entry.get("description").and_then(Value::as_str) != Some(description.as_str());
                if is_local_user_entry(entry) && stale {
                    entry["description"] = Value::String(description);
                    entry["updatedAt"] = Value::String(now_iso());
                }
            }
            None => entries.push(json!({
                "skillId": dir_name,
                "name": dir_name,
                "description": description.unwrap_or_default(),
                "creatorType": "user",
                "syncManaged": false,
                "updatedAt": now_iso(),
                "enabled": true,
            })),
        }
    }
    if entries == before {
        return Ok(());
    }

    let plugin_json = path.with_file_name(".claude-plugin").join("plugin.json");
    if !plugin_json.exists() {
        write_file_atomic(&plugin_json, PLUGIN_JSON.as_bytes())?;
    }
    manifest.insert("lastUpdated".to_string(), json!(now_unix_millis()));
    manifest.insert("skills".to_string(), Value::Array(entries));
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("failed to serialize {}: {e}", path.display()))?;
    write_file_atomic(&path, &bytes)
}
