use super::limits::{
    SKILL_FILE_COUNT_MAX, SKILL_FILE_MAX_BYTES, SKILL_RELATIVE_PATH_MAX_CHARS,
    SKILL_SOURCE_METADATA_MAX_BYTES, SKILL_TOTAL_MAX_BYTES,
};
use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MANAGED_MARKER_FILE: &str = ".aio-coding-hub.managed";
const SOURCE_MARKER_FILE: &str = ".aio-coding-hub.source.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SkillSourceMetadata {
    pub source_git_url: String,
    pub source_branch: String,
    pub source_subdir: String,
}

pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) -> crate::shared::error::AppResult<()> {
    let mut visited = HashSet::new();
    let mut budget = SkillCopyBudget::default();
    copy_dir_recursive_impl(src, dst, Path::new(""), &mut visited, &mut budget)
}

#[derive(Default)]
struct SkillCopyBudget {
    files: usize,
    total_bytes: u64,
}

impl SkillCopyBudget {
    fn reserve_file(
        &mut self,
        source_path: &Path,
        relative_path: &Path,
        bytes: u64,
    ) -> crate::shared::error::AppResult<()> {
        validate_copy_relative_path(relative_path)?;

        if self.files >= SKILL_FILE_COUNT_MAX {
            return Err(format!(
                "SEC_INVALID_INPUT: too many skill files (max {SKILL_FILE_COUNT_MAX})"
            )
            .into());
        }

        if bytes > SKILL_FILE_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill file {} too large (max {SKILL_FILE_MAX_BYTES} bytes)",
                source_path.display()
            )
            .into());
        }

        let next_total = self
            .total_bytes
            .checked_add(bytes)
            .ok_or_else(|| "SEC_INVALID_INPUT: skill payload too large".to_string())?;
        if next_total > SKILL_TOTAL_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: skill payload too large (max {SKILL_TOTAL_MAX_BYTES} bytes)"
            )
            .into());
        }

        self.files += 1;
        self.total_bytes = next_total;
        Ok(())
    }
}

fn copy_dir_recursive_impl(
    src: &Path,
    dst: &Path,
    relative_root: &Path,
    visited: &mut HashSet<PathBuf>,
    budget: &mut SkillCopyBudget,
) -> crate::shared::error::AppResult<()> {
    let src_meta = std::fs::symlink_metadata(src)
        .map_err(|e| format!("failed to read metadata {}: {e}", src.display()))?;

    let actual_src = if src_meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)
            .map_err(|e| format!("failed to read symlink {}: {e}", src.display()))?;
        let resolved = if target.is_absolute() {
            target
        } else {
            src.parent().unwrap_or_else(|| Path::new(".")).join(&target)
        };
        let canonical = resolved.canonicalize().map_err(|e| {
            format!(
                "failed to resolve symlink target {}: {e}",
                resolved.display()
            )
        })?;

        if !visited.insert(canonical.clone()) {
            return Ok(());
        }
        canonical
    } else {
        src.to_path_buf()
    };

    let actual_meta = std::fs::metadata(&actual_src)
        .map_err(|e| format!("failed to read metadata {}: {e}", actual_src.display()))?;

    if !actual_meta.is_dir() {
        return Err(format!(
            "SEC_INVALID_INPUT: copy source is not a directory: {}",
            actual_src.display()
        )
        .into());
    }

    std::fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;
    let entries = std::fs::read_dir(&actual_src)
        .map_err(|e| format!("failed to read dir {}: {e}", actual_src.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", actual_src.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        let relative_path = relative_root.join(&file_name);

        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read file type {}: {e}", path.display()))?;

        if file_type.is_symlink() {
            let target = std::fs::read_link(&path)
                .map_err(|e| format!("failed to read symlink {}: {e}", path.display()))?;
            let resolved = if target.is_absolute() {
                target
            } else {
                path.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&target)
            };

            let target_meta = std::fs::metadata(&resolved).map_err(|e| {
                format!(
                    "failed to resolve symlink target {}: {e}",
                    resolved.display()
                )
            })?;

            if target_meta.is_dir() {
                validate_copy_relative_path(&relative_path)?;
                copy_dir_recursive_impl(&resolved, &dst_path, &relative_path, visited, budget)?;
            } else if target_meta.is_file() {
                copy_regular_skill_file(&resolved, &dst_path, &relative_path, budget)?;
            } else {
                return Err(
                    format!("SKILL_COPY_BLOCKED_SPECIAL_FILE: {}", resolved.display()).into(),
                );
            }
            continue;
        }

        if file_type.is_dir() {
            validate_copy_relative_path(&relative_path)?;
            copy_dir_recursive_impl(&path, &dst_path, &relative_path, visited, budget)?;
            continue;
        }

        if !file_type.is_file() {
            return Err(format!("SKILL_COPY_BLOCKED_SPECIAL_FILE: {}", path.display()).into());
        }

        copy_regular_skill_file(&path, &dst_path, &relative_path, budget)?;
    }
    Ok(())
}

fn copy_regular_skill_file(
    src: &Path,
    dst: &Path,
    relative_path: &Path,
    budget: &mut SkillCopyBudget,
) -> crate::shared::error::AppResult<()> {
    let metadata = std::fs::metadata(src)
        .map_err(|e| format!("failed to read metadata {}: {e}", src.display()))?;
    if !metadata.is_file() {
        return Err(format!("SKILL_COPY_BLOCKED_SPECIAL_FILE: {}", src.display()).into());
    }

    budget.reserve_file(src, relative_path, metadata.len())?;
    let copied = std::fs::copy(src, dst)
        .map_err(|e| format!("failed to copy {} -> {}: {e}", src.display(), dst.display()))?;
    if copied != metadata.len() {
        let _ = std::fs::remove_file(dst);
        return Err(format!(
            "SKILL_COPY_SOURCE_CHANGED: {} changed while copying",
            src.display()
        )
        .into());
    }
    Ok(())
}

fn validate_copy_relative_path(relative_path: &Path) -> crate::shared::error::AppResult<()> {
    let relative = relative_path.to_string_lossy();
    if relative.chars().count() > SKILL_RELATIVE_PATH_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill relative path too long (max {SKILL_RELATIVE_PATH_MAX_CHARS} chars)"
        )
        .into());
    }
    Ok(())
}

pub(super) fn skill_dir_content_hash(dir: &Path) -> crate::shared::error::AppResult<String> {
    const HASH_READ_CHUNK_SIZE: usize = 8 * 1024;

    if !dir.is_dir() {
        return Err(format!(
            "SEC_INVALID_INPUT: skill hash source is not a directory: {}",
            dir.display()
        )
        .into());
    }

    let mut files = Vec::new();
    let mut visited_dirs = HashSet::new();
    let mut budget = SkillCopyBudget::default();
    collect_skill_hash_files(
        dir,
        Path::new(""),
        &mut visited_dirs,
        &mut budget,
        &mut files,
    )?;

    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_READ_CHUNK_SIZE];
    for (relative_path, path) in files {
        hasher.update(b"file\0");
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\0");

        let mut file = std::fs::File::open(&path)
            .map_err(|e| format!("failed to open {} for hashing: {e}", path.display()))?;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|e| format!("failed to read {} for hashing: {e}", path.display()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        hasher.update(b"\0");
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn collect_skill_hash_files(
    dir: &Path,
    relative_root: &Path,
    visited_dirs: &mut HashSet<PathBuf>,
    budget: &mut SkillCopyBudget,
    files: &mut Vec<(String, PathBuf)>,
) -> crate::shared::error::AppResult<()> {
    let canonical = dir
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize {}: {e}", dir.display()))?;
    if !visited_dirs.insert(canonical) {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed to read dir entry {}: {e}", dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str == MANAGED_MARKER_FILE
            || file_name_str == SOURCE_MARKER_FILE
            || file_name_str == ".git"
        {
            continue;
        }

        let path = entry.path();
        let relative_path = relative_root.join(&file_name);
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))?;

        if metadata.is_dir() {
            validate_copy_relative_path(&relative_path)?;
            collect_skill_hash_files(&path, &relative_path, visited_dirs, budget, files)?;
            continue;
        }

        if !metadata.is_file() {
            return Err(format!("SKILL_HASH_BLOCKED_SPECIAL_FILE: {}", path.display()).into());
        }

        budget.reserve_file(&path, &relative_path, metadata.len())?;
        let relative = relative_path.to_string_lossy().replace('\\', "/");
        files.push((relative, path));
    }

    Ok(())
}

pub(super) fn write_marker(dir: &Path) -> crate::shared::error::AppResult<()> {
    let path = dir.join(MANAGED_MARKER_FILE);
    write_file_atomic(&path, b"aio-coding-hub\n")
}

pub(super) fn remove_marker(dir: &Path) {
    let path = dir.join(MANAGED_MARKER_FILE);
    let _ = std::fs::remove_file(path);
}

pub(super) fn write_source_metadata(
    dir: &Path,
    metadata: &SkillSourceMetadata,
) -> crate::shared::error::AppResult<()> {
    let path = dir.join(SOURCE_MARKER_FILE);
    let content = serde_json::to_vec_pretty(metadata).map_err(|e| {
        format!(
            "failed to serialize source metadata {}: {e}",
            path.display()
        )
    })?;
    write_file_atomic(&path, &content)
}

pub(super) fn read_source_metadata(
    dir: &Path,
) -> crate::shared::error::AppResult<Option<SkillSourceMetadata>> {
    let path = dir.join(SOURCE_MARKER_FILE);
    let Some(bytes) = read_optional_file_with_max_len(&path, SKILL_SOURCE_METADATA_MAX_BYTES)?
    else {
        return Ok(None);
    };
    let metadata = serde_json::from_slice::<SkillSourceMetadata>(&bytes)
        .map_err(|e| format!("failed to parse source metadata {}: {e}", path.display()))?;
    Ok(Some(metadata))
}

pub(super) fn is_managed_dir(dir: &Path) -> bool {
    dir.join(MANAGED_MARKER_FILE).exists()
}

pub(super) fn exists_or_is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn normalize_for_prefix_check(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn canonicalize_allow_missing(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }

    let normalized = normalize_for_prefix_check(path);
    let mut cursor = normalized.as_path();
    let mut missing = Vec::new();

    loop {
        if let Ok(existing_prefix) = cursor.canonicalize() {
            let mut resolved = existing_prefix;
            for component in missing.iter().rev() {
                resolved.push(component);
            }
            return resolved;
        }

        let Some(file_name) = cursor.file_name() else {
            return normalized;
        };
        missing.push(file_name.to_os_string());

        let Some(parent) = cursor.parent() else {
            return normalized;
        };
        if parent == cursor {
            return normalized;
        }
        cursor = parent;
    }
}

pub(super) fn is_managed_link_to_ssot(dir: &Path, ssot_root: &Path) -> bool {
    if !is_symlink_or_junction(dir) {
        return false;
    }
    let Ok(target) = std::fs::read_link(dir) else {
        return false;
    };
    let resolved = if target.is_absolute() {
        target
    } else {
        dir.parent().unwrap_or_else(|| Path::new(".")).join(&target)
    };
    let canonical_target = canonicalize_allow_missing(&resolved);
    let canonical_ssot = canonicalize_allow_missing(ssot_root);
    canonical_target.starts_with(&canonical_ssot)
}

pub(super) use crate::shared::fs::is_symlink;

pub(super) fn has_skill_md(path: &Path) -> bool {
    skill_md_path(path).ok().flatten().is_some()
}

pub(super) fn skill_md_path(path: &Path) -> crate::shared::error::AppResult<Option<PathBuf>> {
    let exact = path.join("SKILL.md");
    if exact.exists() {
        return Ok(Some(exact));
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None);
        }
        Err(err) => return Err(format!("failed to read dir {}: {err}", path.display()).into()),
    };

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", path.display()))?;
        let file_name = entry.file_name();
        if file_name.to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
            return Ok(Some(entry.path()));
        }
    }

    Ok(None)
}

pub(super) fn remove_managed_dir(dir: &Path) -> crate::shared::error::AppResult<()> {
    if is_symlink_or_junction(dir) {
        return remove_symlink_or_junction(dir);
    }
    if !dir.exists() {
        return Ok(());
    }
    if !is_managed_dir(dir) {
        return Err(format!(
            "SKILL_REMOVE_BLOCKED_UNMANAGED: target exists but is not managed: {}",
            dir.display()
        )
        .into());
    }
    std::fs::remove_dir_all(dir).map_err(|e| format!("failed to remove {}: {e}", dir.display()))?;
    Ok(())
}

pub(super) fn is_symlink_or_junction(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return true;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    return true;
                }
            }
            false
        }
        Err(_) => false,
    }
}

fn remove_symlink_or_junction(path: &Path) -> crate::shared::error::AppResult<()> {
    #[cfg(windows)]
    {
        std::fs::remove_dir(path)
            .map_err(|e| format!("failed to remove junction/symlink {}: {e}", path.display()))?;
    }
    #[cfg(not(windows))]
    {
        std::fs::remove_file(path)
            .map_err(|e| format!("failed to remove symlink {}: {e}", path.display()))?;
    }
    Ok(())
}

pub(super) fn create_skill_link(
    ssot_dir: &Path,
    target: &Path,
) -> crate::shared::error::AppResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    #[cfg(windows)]
    {
        junction::create(ssot_dir, target).map_err(|e| {
            format!(
                "failed to create junction {} -> {}: {e}",
                target.display(),
                ssot_dir.display()
            )
        })?;
    }

    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(ssot_dir, target).map_err(|e| {
            format!(
                "failed to create symlink {} -> {}: {e}",
                target.display(),
                ssot_dir.display()
            )
        })?;
    }

    Ok(())
}
