//! Skill file system utilities for config export/import.

use crate::shared::error::AppResult;
use crate::shared::fs::{read_file_with_max_len, read_optional_file_with_max_len};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use super::{
    LocalSkillExport, SkillFileExport, CONFIG_SKILL_FILE_COUNT_MAX, CONFIG_SKILL_FILE_MAX_BYTES,
    CONFIG_SKILL_MD_MAX_BYTES, CONFIG_SKILL_RELATIVE_PATH_MAX_CHARS,
    CONFIG_SKILL_SOURCE_METADATA_MAX_BYTES, CONFIG_SKILL_TOTAL_MAX_BYTES,
    SKILL_MANAGED_MARKER_FILE, SKILL_SOURCE_MARKER_FILE,
};

const CONFIG_SKILL_FILE_BASE64_MAX_BYTES: usize = CONFIG_SKILL_FILE_MAX_BYTES.div_ceil(3) * 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SkillSourceMetadataFile {
    pub(super) source_git_url: String,
    pub(super) source_branch: String,
    pub(super) source_subdir: String,
}

pub(super) fn ssot_skills_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    let cli_key = crate::shared::cli_key::SUPPORTED_CLI_KEYS[0];
    let paths = crate::skills::paths_get(app, cli_key)?;
    Ok(PathBuf::from(paths.ssot_dir))
}

pub(super) fn cli_skills_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> AppResult<PathBuf> {
    let paths = crate::skills::paths_get(app, cli_key)?;
    Ok(PathBuf::from(paths.cli_dir))
}

pub(super) fn local_skill_dirs(root: &Path) -> AppResult<Vec<PathBuf>> {
    let mut items = Vec::new();
    if !root.exists() {
        return Ok(items);
    }

    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("failed to read dir {}: {e}", root.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read file type {}: {e}", path.display()))?;
        if is_local_skill_dir(&path, &file_type) {
            items.push(path);
        }
    }
    items.sort();
    Ok(items)
}

fn is_local_skill_dir(path: &Path, file_type: &std::fs::FileType) -> bool {
    file_type.is_dir()
        && path.join("SKILL.md").exists()
        && !path.join(SKILL_MANAGED_MARKER_FILE).exists()
}

pub(super) fn export_skill_dir_files(
    dir: &Path,
    skip_source_marker: bool,
) -> AppResult<Vec<SkillFileExport>> {
    let mut collector = SkillFileCollector::default();
    let mut visited_dirs = HashSet::new();
    let canonical_root = dir
        .canonicalize()
        .map_err(|e| format!("failed to resolve {}: {e}", dir.display()))?;
    visited_dirs.insert(canonical_root.clone());
    collect_skill_dir_files(
        &canonical_root,
        &canonical_root,
        Path::new(""),
        &mut collector,
        &mut visited_dirs,
        skip_source_marker,
    )?;
    Ok(collector.files)
}

#[derive(Default)]
struct SkillFileCollector {
    files: Vec<SkillFileExport>,
    total_bytes: usize,
}

impl SkillFileCollector {
    fn push_file(&mut self, relative_path: &Path, source_path: &Path) -> AppResult<()> {
        if self.files.len() >= CONFIG_SKILL_FILE_COUNT_MAX {
            return Err(format!(
                "SEC_INVALID_INPUT: too many skill files (max {CONFIG_SKILL_FILE_COUNT_MAX})"
            )
            .into());
        }

        let relative_path = relative_path_string(relative_path)?;
        let content = read_file_with_max_len(source_path, CONFIG_SKILL_FILE_MAX_BYTES)?;
        let next_total = self
            .total_bytes
            .checked_add(content.len())
            .ok_or_else(|| "SEC_INVALID_INPUT: skill file payload too large".to_string())?;
        if next_total > CONFIG_SKILL_TOTAL_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill file payload too large (max {CONFIG_SKILL_TOTAL_MAX_BYTES} bytes)"
            )
            .into());
        }

        self.total_bytes = next_total;
        self.files.push(SkillFileExport {
            relative_path,
            content_base64: BASE64_STANDARD.encode(content),
        });
        Ok(())
    }
}

fn collect_skill_dir_files(
    root_dir: &Path,
    dir: &Path,
    relative_root: &Path,
    files: &mut SkillFileCollector,
    visited_dirs: &mut HashSet<PathBuf>,
    skip_source_marker: bool,
) -> AppResult<()> {
    let mut entries = Vec::new();
    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?;
    for entry in read_dir {
        entries
            .push(entry.map_err(|e| format!("failed to read dir entry {}: {e}", dir.display()))?);
    }
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == SKILL_MANAGED_MARKER_FILE {
            continue;
        }
        if skip_source_marker && name_str == SKILL_SOURCE_MARKER_FILE {
            continue;
        }

        let entry_path = entry.path();
        let relative_path = relative_root.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read file type {}: {e}", entry_path.display()))?;

        if file_type.is_symlink() {
            let resolved = resolved_symlink_target(&entry_path)?;
            let canonical = resolved.canonicalize().map_err(|e| {
                format!(
                    "failed to resolve symlink target {}: {e}",
                    resolved.display()
                )
            })?;
            ensure_skill_path_within_root(root_dir, &canonical)?;
            let resolved_meta = std::fs::metadata(&canonical)
                .map_err(|e| format!("failed to read metadata {}: {e}", canonical.display()))?;
            if resolved_meta.is_dir() {
                if visited_dirs.insert(canonical.clone()) {
                    collect_skill_dir_files(
                        root_dir,
                        &canonical,
                        &relative_path,
                        files,
                        visited_dirs,
                        skip_source_marker,
                    )?;
                }
            } else if resolved_meta.is_file() {
                files.push_file(&relative_path, &canonical)?;
            } else {
                return Err(
                    format!("SKILL_EXPORT_BLOCKED_SPECIAL_FILE: {}", canonical.display()).into(),
                );
            }
            continue;
        }

        if file_type.is_dir() {
            let canonical = entry_path.canonicalize().map_err(|e| {
                format!("failed to resolve directory {}: {e}", entry_path.display())
            })?;
            if visited_dirs.insert(canonical.clone()) {
                collect_skill_dir_files(
                    root_dir,
                    &canonical,
                    &relative_path,
                    files,
                    visited_dirs,
                    skip_source_marker,
                )?;
            }
            continue;
        }

        if !file_type.is_file() {
            return Err(format!(
                "SKILL_EXPORT_BLOCKED_SPECIAL_FILE: {}",
                entry_path.display()
            )
            .into());
        }

        files.push_file(&relative_path, &entry_path)?;
    }

    Ok(())
}

fn ensure_skill_path_within_root(root_dir: &Path, path: &Path) -> AppResult<()> {
    if path.starts_with(root_dir) {
        return Ok(());
    }

    Err(format!("SKILL_EXPORT_BLOCKED_SYMLINK_ESCAPE: {}", path.display()).into())
}

fn resolved_symlink_target(path: &Path) -> AppResult<PathBuf> {
    let target = std::fs::read_link(path)
        .map_err(|e| format!("failed to read symlink {}: {e}", path.display()))?;
    Ok(if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(target)
    })
}

fn relative_path_string(path: &Path) -> AppResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| "SEC_INVALID_INPUT: invalid utf-8 skill path".to_string())?
                    .to_string(),
            ),
            _ => {
                return Err(format!(
                    "SEC_INVALID_INPUT: invalid skill relative path component in {}",
                    path.display()
                )
                .into())
            }
        }
    }

    if parts.is_empty() {
        return Err("SEC_INVALID_INPUT: empty skill relative path"
            .to_string()
            .into());
    }

    let relative_path = parts.join("/");
    if relative_path.chars().count() > CONFIG_SKILL_RELATIVE_PATH_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill relative path too long (max {CONFIG_SKILL_RELATIVE_PATH_MAX_CHARS} chars)"
        )
        .into());
    }
    Ok(relative_path)
}

pub(super) fn write_skill_files_to_dir(
    dir: &Path,
    files: &[SkillFileExport],
    source_metadata: Option<&SkillSourceMetadataFile>,
) -> AppResult<()> {
    let decoded_files = decode_skill_files_for_write(files)?;

    if dir.exists() {
        return Err(format!("SKILL_IMPORT_DIR_ALREADY_EXISTS: {}", dir.display()).into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {e}", dir.display()))?;

    for (relative_path, bytes) in decoded_files {
        let target = dir.join(&relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        crate::shared::fs::write_file_atomic(&target, &bytes)?;
    }

    let managed_marker = dir.join(SKILL_MANAGED_MARKER_FILE);
    if managed_marker.exists() {
        std::fs::remove_file(&managed_marker)
            .map_err(|e| format!("failed to remove {}: {e}", managed_marker.display()))?;
    }

    let source_marker = dir.join(SKILL_SOURCE_MARKER_FILE);
    if let Some(metadata) = source_metadata {
        let bytes = serde_json::to_vec_pretty(metadata)
            .map_err(|e| format!("SYSTEM_ERROR: failed to serialize source metadata: {e}"))?;
        crate::shared::fs::write_file_atomic(&source_marker, &bytes)?;
    } else if source_marker.exists() {
        std::fs::remove_file(&source_marker)
            .map_err(|e| format!("failed to remove {}: {e}", source_marker.display()))?;
    }

    Ok(())
}

fn decode_skill_files_for_write(files: &[SkillFileExport]) -> AppResult<Vec<(PathBuf, Vec<u8>)>> {
    if files.len() > CONFIG_SKILL_FILE_COUNT_MAX {
        return Err(format!(
            "SEC_INVALID_INPUT: too many skill files (max {CONFIG_SKILL_FILE_COUNT_MAX})"
        )
        .into());
    }

    let mut seen_paths = HashSet::new();
    let mut total_bytes = 0_usize;
    let mut decoded_files = Vec::with_capacity(files.len());
    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        if !seen_paths.insert(relative_path.clone()) {
            return Err(format!(
                "SEC_INVALID_INPUT: duplicate skill file path {}",
                file.relative_path
            )
            .into());
        }

        if file.content_base64.len() > CONFIG_SKILL_FILE_BASE64_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill file {} too large (max {CONFIG_SKILL_FILE_MAX_BYTES} bytes)",
                file.relative_path
            )
            .into());
        }

        let bytes = BASE64_STANDARD
            .decode(file.content_base64.as_bytes())
            .map_err(|e| {
                format!(
                    "SEC_INVALID_INPUT: invalid base64 for {}: {e}",
                    file.relative_path
                )
            })?;
        if bytes.len() > CONFIG_SKILL_FILE_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill file {} too large (max {CONFIG_SKILL_FILE_MAX_BYTES} bytes)",
                file.relative_path
            )
            .into());
        }
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| "SEC_INVALID_INPUT: skill file payload too large".to_string())?;
        if total_bytes > CONFIG_SKILL_TOTAL_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill file payload too large (max {CONFIG_SKILL_TOTAL_MAX_BYTES} bytes)"
            )
            .into());
        }
        decoded_files.push((relative_path, bytes));
    }

    Ok(decoded_files)
}

fn validate_skill_file_relative_path(relative_path: &str) -> AppResult<PathBuf> {
    if relative_path.chars().count() > CONFIG_SKILL_RELATIVE_PATH_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill relative path too long (max {CONFIG_SKILL_RELATIVE_PATH_MAX_CHARS} chars)"
        )
        .into());
    }

    let path = Path::new(relative_path);
    if path.as_os_str().is_empty() {
        return Err("SEC_INVALID_INPUT: empty skill relative path"
            .to_string()
            .into());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            _ => {
                return Err(format!(
                    "SEC_INVALID_INPUT: invalid skill relative path {}",
                    relative_path
                )
                .into())
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("SEC_INVALID_INPUT: empty skill relative path"
            .to_string()
            .into());
    }

    Ok(normalized)
}

pub(super) fn validate_local_dir_name(dir_name: &str) -> AppResult<String> {
    let trimmed = dir_name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return Err(format!("SEC_INVALID_INPUT: invalid local skill dir_name={dir_name}").into());
    }
    Ok(trimmed.to_string())
}

pub(super) fn build_local_skill_source_metadata(
    local_skill: &LocalSkillExport,
) -> AppResult<Option<SkillSourceMetadataFile>> {
    match (
        local_skill.source_git_url.as_deref().map(str::trim),
        local_skill.source_branch.as_deref().map(str::trim),
        local_skill.source_subdir.as_deref().map(str::trim),
    ) {
        (None, None, None) => Ok(None),
        (Some(git_url), Some(branch), Some(source_subdir))
            if !git_url.is_empty() && !branch.is_empty() && !source_subdir.is_empty() =>
        {
            Ok(Some(SkillSourceMetadataFile {
                source_git_url: git_url.to_string(),
                source_branch: branch.to_string(),
                source_subdir: source_subdir.to_string(),
            }))
        }
        _ => Err(format!(
            "SEC_INVALID_INPUT: incomplete local skill source metadata for cli_key={}, dir_name={}",
            local_skill.cli_key, local_skill.dir_name
        )
        .into()),
    }
}

pub(super) fn read_local_skill_source_metadata(
    path: &Path,
) -> AppResult<Option<SkillSourceMetadataFile>> {
    let source_path = path.join(SKILL_SOURCE_MARKER_FILE);
    let Some(bytes) =
        read_optional_file_with_max_len(&source_path, CONFIG_SKILL_SOURCE_METADATA_MAX_BYTES)?
    else {
        return Ok(None);
    };
    let metadata = serde_json::from_slice::<SkillSourceMetadataFile>(&bytes)
        .map_err(|e| format!("failed to parse {}: {e}", source_path.display()))?;
    Ok(Some(metadata))
}

pub(super) fn parse_skill_md_metadata(skill_md_path: &Path) -> AppResult<(String, String)> {
    let bytes = read_file_with_max_len(skill_md_path, CONFIG_SKILL_MD_MAX_BYTES)?;
    let text = String::from_utf8(bytes)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid UTF-8 in SKILL.md: {e}"))?;
    let text = text.trim_start();
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        return Err("SEC_INVALID_INPUT: SKILL.md is empty".to_string().into());
    };
    if first.trim() != "---" {
        return Err("SEC_INVALID_INPUT: SKILL.md front matter is required"
            .to_string()
            .into());
    }

    let mut front_matter = HashMap::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string();
        front_matter.insert(key.trim().to_string(), value);
    }

    let name = front_matter.get("name").cloned().unwrap_or_default();
    let description = front_matter.get("description").cloned().unwrap_or_default();
    if name.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: SKILL.md missing 'name'"
            .to_string()
            .into());
    }
    Ok((name.trim().to_string(), description.trim().to_string()))
}

pub(super) fn remove_dir_if_exists(path: &Path) -> AppResult<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
    }
    Ok(())
}
