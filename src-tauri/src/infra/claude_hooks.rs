//! Usage: Read / write the `hooks` section of Claude Code's `settings.json`.

use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic_if_changed};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CLAUDE_HOOKS_SETTINGS_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ClaudeHookEntry {
    pub hook_type: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ClaudeHookGroup {
    pub event: String,
    pub matcher: String,
    pub hooks: Vec<ClaudeHookEntry>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ClaudeHooksState {
    pub settings_path: String,
    pub groups: Vec<ClaudeHookGroup>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct ClaudeHooksSetInput {
    pub groups: Vec<ClaudeHookGroup>,
}

fn claude_settings_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::app_paths::home_dir(app)?
        .join(".claude")
        .join("settings.json"))
}

fn json_root_from_bytes(
    bytes: Option<Vec<u8>>,
    action: &str,
) -> crate::shared::error::AppResult<serde_json::Value> {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .map_err(|e| format!("settings.json 解析失败，拒绝{action}以保护现有配置: {e}").into()),
        None => Ok(serde_json::json!({})),
    }
}

fn read_optional_claude_hooks_settings_file(
    path: &Path,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    read_optional_file_with_max_len(path, CLAUDE_HOOKS_SETTINGS_MAX_BYTES)
}

fn ensure_claude_hooks_settings_len(
    bytes: &[u8],
    label: &str,
) -> crate::shared::error::AppResult<()> {
    if bytes.len() > CLAUDE_HOOKS_SETTINGS_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {label} too large (max {CLAUDE_HOOKS_SETTINGS_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

fn parse_hooks_from_root(root: &serde_json::Value) -> Vec<ClaudeHookGroup> {
    let Some(hooks_obj) = root.get("hooks").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut groups = Vec::new();
    for (event, matcher_groups) in hooks_obj {
        let Some(matcher_arr) = matcher_groups.as_array() else {
            continue;
        };
        for matcher_group in matcher_arr {
            let Some(mg) = matcher_group.as_object() else {
                continue;
            };
            let matcher = mg
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entries = mg
                .get("hooks")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|h| {
                            let obj = h.as_object()?;
                            let hook_type = obj
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("command")
                                .to_string();
                            let command = obj.get("command").and_then(|v| v.as_str())?.to_string();
                            let timeout = obj.get("timeout").and_then(|v| v.as_u64());
                            Some(ClaudeHookEntry {
                                hook_type,
                                command,
                                timeout,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            groups.push(ClaudeHookGroup {
                event: event.clone(),
                matcher,
                hooks: entries,
            });
        }
    }
    groups
}

#[derive(Debug, Clone)]
struct ExistingHookGroup {
    event: String,
    matcher: String,
    raw_group: serde_json::Map<String, serde_json::Value>,
    hook_slots: Vec<ExistingHookSlot>,
}

#[derive(Debug, Clone)]
enum ExistingHookSlot {
    Supported(serde_json::Map<String, serde_json::Value>),
    Unsupported(serde_json::Value),
}

fn existing_hook_groups_from_root(root: &serde_json::Value) -> Vec<ExistingHookGroup> {
    let Some(hooks_obj) = root.get("hooks").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut groups = Vec::new();
    for (event, matcher_groups) in hooks_obj {
        let Some(matcher_arr) = matcher_groups.as_array() else {
            continue;
        };
        for matcher_group in matcher_arr {
            let Some(mg) = matcher_group.as_object() else {
                continue;
            };
            let matcher = mg
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut hook_slots = Vec::new();

            if let Some(hooks) = mg.get("hooks").and_then(|v| v.as_array()) {
                for hook in hooks {
                    match hook.as_object() {
                        Some(obj) if obj.get("command").and_then(|v| v.as_str()).is_some() => {
                            hook_slots.push(ExistingHookSlot::Supported(obj.clone()));
                        }
                        _ => hook_slots.push(ExistingHookSlot::Unsupported(hook.clone())),
                    }
                }
            }

            groups.push(ExistingHookGroup {
                event: event.clone(),
                matcher,
                raw_group: mg.clone(),
                hook_slots,
            });
        }
    }

    groups
}

#[cfg(test)]
fn groups_to_json(groups: &[ClaudeHookGroup]) -> serde_json::Value {
    groups_to_json_with_existing(groups, None)
}

fn groups_to_json_with_existing(
    groups: &[ClaudeHookGroup],
    existing_root: Option<&serde_json::Value>,
) -> serde_json::Value {
    let existing_groups = existing_root
        .map(existing_hook_groups_from_root)
        .unwrap_or_default();
    let mut used_existing = vec![false; existing_groups.len()];

    let mut hooks_map = serde_json::Map::new();
    for (group_index, group) in groups.iter().enumerate() {
        let existing_index = existing_groups
            .iter()
            .enumerate()
            .find_map(|(index, existing)| {
                (!used_existing[index]
                    && existing.event == group.event
                    && existing.matcher == group.matcher)
                    .then_some(index)
            })
            .or_else(|| {
                (group_index < existing_groups.len() && !used_existing[group_index])
                    .then_some(group_index)
            });
        let existing = existing_index.map(|index| {
            used_existing[index] = true;
            &existing_groups[index]
        });

        let entry = hooks_map
            .entry(group.event.clone())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        let arr = entry.as_array_mut().expect("hooks event must be array");

        let hook_entries = merge_hook_entries(&group.hooks, existing);

        let mut mg = existing
            .map(|existing| existing.raw_group.clone())
            .unwrap_or_default();
        mg.insert(
            "matcher".to_string(),
            serde_json::Value::String(group.matcher.clone()),
        );
        mg.insert("hooks".to_string(), serde_json::Value::Array(hook_entries));
        arr.push(serde_json::Value::Object(mg));
    }
    serde_json::Value::Object(hooks_map)
}

fn hook_entry_to_json(
    hook: &ClaudeHookEntry,
    existing: Option<serde_json::Map<String, serde_json::Value>>,
) -> serde_json::Value {
    let mut obj = existing.unwrap_or_default();
    obj.insert(
        "type".to_string(),
        serde_json::Value::String(hook.hook_type.clone()),
    );
    obj.insert(
        "command".to_string(),
        serde_json::Value::String(hook.command.clone()),
    );
    if let Some(timeout) = hook.timeout {
        obj.insert(
            "timeout".to_string(),
            serde_json::Value::Number(timeout.into()),
        );
    } else {
        obj.remove("timeout");
    }
    serde_json::Value::Object(obj)
}

fn merge_hook_entries(
    hooks: &[ClaudeHookEntry],
    existing: Option<&ExistingHookGroup>,
) -> Vec<serde_json::Value> {
    let Some(existing) = existing else {
        return hooks
            .iter()
            .map(|hook| hook_entry_to_json(hook, None))
            .collect();
    };

    let mut next_hooks = hooks.iter();
    let mut entries = Vec::new();
    for slot in &existing.hook_slots {
        match slot {
            ExistingHookSlot::Supported(raw) => {
                if let Some(hook) = next_hooks.next() {
                    entries.push(hook_entry_to_json(hook, Some(raw.clone())));
                }
            }
            ExistingHookSlot::Unsupported(value) => entries.push(value.clone()),
        }
    }
    entries.extend(next_hooks.map(|hook| hook_entry_to_json(hook, None)));
    entries
}

pub fn claude_hooks_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<ClaudeHooksState> {
    let path = claude_settings_path(app)?;
    let root = json_root_from_bytes(
        read_optional_claude_hooks_settings_file(&path)?,
        "读取 Hooks 空配置",
    )?;
    if !root.is_object() {
        return Err(
            "settings.json 根节点不是 JSON 对象，拒绝读取 Hooks 空配置以保护现有配置"
                .to_string()
                .into(),
        );
    }
    let groups = parse_hooks_from_root(&root);
    Ok(ClaudeHooksState {
        settings_path: path.to_string_lossy().to_string(),
        groups,
    })
}

pub fn claude_hooks_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    input: ClaudeHooksSetInput,
) -> crate::shared::error::AppResult<ClaudeHooksState> {
    let path = claude_settings_path(app)?;
    if path.exists() && crate::shared::fs::is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let current = read_optional_claude_hooks_settings_file(&path)?;
    let mut root = json_root_from_bytes(current, "覆写")?;
    if !root.is_object() {
        return Err("settings.json 根节点不是 JSON 对象，拒绝覆写以保护现有配置"
            .to_string()
            .into());
    }
    let next_hooks = if input.groups.is_empty() {
        None
    } else {
        Some(groups_to_json_with_existing(&input.groups, Some(&root)))
    };
    let obj = root.as_object_mut().expect("root must be object");

    match next_hooks {
        Some(hooks) => {
            obj.insert("hooks".to_string(), hooks);
        }
        None => {
            obj.remove("hooks");
        }
    }

    let mut out = serde_json::to_vec_pretty(&root)
        .map_err(|e| format!("failed to serialize settings.json: {e}"))?;
    out.push(b'\n');
    ensure_claude_hooks_settings_len(&out, "claude/hooks settings.json")?;
    let _ = write_file_atomic_if_changed(&path, &out)?;
    claude_hooks_get(app)
}

#[cfg(test)]
mod tests;
