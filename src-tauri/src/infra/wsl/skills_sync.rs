//! WSL skills sync: sync skill directories to WSL distros.

use crate::shared::error::AppResult;
use crate::shared::fs::{
    read_file_with_max_len, read_optional_file_with_max_len, write_file_atomic,
};

use super::shell::{
    bash_single_quote, remove_wsl_dir, run_wsl_bash_script_capture, write_wsl_file,
    wsl_has_managed_skill_dir, wsl_path_exists, wsl_resolve_codex_home_script,
};
use super::types::{WslSkillFileSyncEntry, WslSkillSyncEntry};

pub(super) const WSL_SKILL_MANAGED_MARKER_FILE: &str = ".aio-coding-hub.managed";
pub(super) const WSL_SKILL_SOURCE_MARKER_FILE: &str = ".aio-coding-hub.source.json";
const WSL_SKILLS_MANIFEST_MAX_BYTES: usize = 256 * 1024;
const WSL_SKILL_FILE_MAX_BYTES: usize = 1024 * 1024;
const WSL_SKILL_TOTAL_MAX_BYTES: usize = 8 * 1024 * 1024;
const WSL_SKILL_MAX_FILES: usize = 256;
const WSL_SKILL_COMPONENT_MAX_CHARS: usize = 128;
const WSL_SKILL_RELATIVE_PATH_MAX_CHARS: usize = 512;

fn ensure_wsl_skill_bytes_within_limit(
    bytes_len: usize,
    max_len: usize,
    label: &str,
) -> AppResult<()> {
    if bytes_len > max_len {
        return Err(format!("SEC_INVALID_INPUT: {label} too large (max {max_len} bytes)").into());
    }
    Ok(())
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslSkillsManifest {
    distro: String,
    cli_key: String,
    managed_keys: Vec<String>,
    updated_at: i64,
}

fn wsl_skills_manifest_path(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    let dir = crate::app_paths::app_data_dir(app)?
        .join("wsl-skills-sync")
        .join(distro)
        .join(cli_key);
    Ok(dir.join("manifest.json"))
}

pub(super) fn read_wsl_skills_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> Vec<String> {
    let path = match wsl_skills_manifest_path(app, distro, cli_key) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    let bytes = match read_optional_file_with_max_len(&path, WSL_SKILLS_MANIFEST_MAX_BYTES) {
        Ok(Some(bytes)) => bytes,
        Ok(None) | Err(_) => return Vec::new(),
    };
    match serde_json::from_slice::<WslSkillsManifest>(&bytes) {
        Ok(manifest) => manifest.managed_keys,
        Err(_) => Vec::new(),
    }
}

pub(super) fn write_wsl_skills_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    managed_keys: &[String],
) -> AppResult<()> {
    let path = wsl_skills_manifest_path(app, distro, cli_key)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create wsl-skills-sync dir: {e}"))?;
    }
    let manifest = WslSkillsManifest {
        distro: distro.to_string(),
        cli_key: cli_key.to_string(),
        managed_keys: managed_keys.to_vec(),
        updated_at: crate::shared::time::now_unix_seconds(),
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize wsl skills manifest: {e}"))?;
    ensure_wsl_skill_bytes_within_limit(
        json.len(),
        WSL_SKILLS_MANIFEST_MAX_BYTES,
        "WSL skills manifest",
    )?;
    write_file_atomic(&path, json.as_bytes())?;
    Ok(())
}

fn resolve_wsl_skills_root(distro: &str, cli_key: &str) -> AppResult<String> {
    let resolve_script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
{resolver}
case {cli_key} in
  claude) echo "$HOME/.claude/skills" ;;
  codex) echo "$p/skills" ;;
  gemini) echo "$HOME/.gemini/skills" ;;
esac
"#,
        resolver = wsl_resolve_codex_home_script("p"),
        cli_key = bash_single_quote(cli_key)
    );
    let resolved = run_wsl_bash_script_capture(distro, &resolve_script)?;
    let resolved = resolved.trim().to_string();
    if resolved.is_empty() || !resolved.starts_with('/') {
        return Err(format!("failed to resolve skills dir for {cli_key}: {resolved}").into());
    }
    Ok(resolved)
}

fn validate_wsl_skill_component(value: &str, label: &str) -> AppResult<()> {
    let path = std::path::Path::new(value);
    if value.trim().is_empty() {
        return Err(format!("SEC_INVALID_INPUT: empty skill {label}").into());
    }
    if char_len(value) > WSL_SKILL_COMPONENT_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill {label} is too long (max {WSL_SKILL_COMPONENT_MAX_CHARS} chars)"
        )
        .into());
    }
    if path.components().count() != 1 {
        return Err(format!("SEC_INVALID_INPUT: invalid skill {label}: {value}").into());
    }
    match path.components().next() {
        Some(std::path::Component::Normal(_)) => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: invalid skill {label}: {value}").into()),
    }
}

fn validate_wsl_skill_relative_path(path: &str) -> AppResult<std::path::PathBuf> {
    let mut out = std::path::PathBuf::new();
    if path.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: empty skill relative path".into());
    }
    if char_len(path) > WSL_SKILL_RELATIVE_PATH_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill relative path too long (max {WSL_SKILL_RELATIVE_PATH_MAX_CHARS} chars)"
        )
        .into());
    }
    for component in std::path::Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            _ => {
                return Err(
                    format!("SEC_INVALID_INPUT: invalid skill relative path: {path}").into(),
                )
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err("SEC_INVALID_INPUT: empty skill relative path".into());
    }
    Ok(out)
}

fn relative_skill_path_string(path: &std::path::Path) -> AppResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => parts.push(
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
        return Err("SEC_INVALID_INPUT: empty skill relative path".into());
    }
    let out = parts.join("/");
    if char_len(&out) > WSL_SKILL_RELATIVE_PATH_MAX_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: skill relative path too long (max {WSL_SKILL_RELATIVE_PATH_MAX_CHARS} chars)"
        )
        .into());
    }
    Ok(out)
}

fn ensure_skill_path_within_root(
    root_dir: &std::path::Path,
    path: &std::path::Path,
) -> AppResult<()> {
    if path.starts_with(root_dir) {
        return Ok(());
    }
    Err(format!("WSL_SKILL_SYNC_BLOCKED_SYMLINK_ESCAPE: {}", path.display()).into())
}

fn collect_wsl_skill_dir_files(
    root_dir: &std::path::Path,
    dir: &std::path::Path,
    relative_root: &std::path::Path,
    files: &mut Vec<WslSkillFileSyncEntry>,
    visited_dirs: &mut std::collections::HashSet<std::path::PathBuf>,
    total_bytes: &mut usize,
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
        if name_str == WSL_SKILL_MANAGED_MARKER_FILE || name_str == WSL_SKILL_SOURCE_MARKER_FILE {
            continue;
        }

        let entry_path = entry.path();
        let relative_path = relative_root.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read file type {}: {e}", entry_path.display()))?;

        if file_type.is_symlink() {
            let resolved = std::fs::read_link(&entry_path)
                .map_err(|e| format!("failed to read symlink {}: {e}", entry_path.display()))?;
            let resolved = if resolved.is_absolute() {
                resolved
            } else {
                entry_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(resolved)
            };
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
                    collect_wsl_skill_dir_files(
                        root_dir,
                        &canonical,
                        &relative_path,
                        files,
                        visited_dirs,
                        total_bytes,
                    )?;
                }
            } else if resolved_meta.is_file() {
                push_wsl_skill_file(files, &relative_path, &canonical, total_bytes)?;
            } else {
                return Err(format!(
                    "WSL_SKILL_SYNC_BLOCKED_SPECIAL_FILE: {}",
                    canonical.display()
                )
                .into());
            }
            continue;
        }

        if file_type.is_dir() {
            let canonical = entry_path.canonicalize().map_err(|e| {
                format!("failed to resolve directory {}: {e}", entry_path.display())
            })?;
            ensure_skill_path_within_root(root_dir, &canonical)?;
            if visited_dirs.insert(canonical.clone()) {
                collect_wsl_skill_dir_files(
                    root_dir,
                    &canonical,
                    &relative_path,
                    files,
                    visited_dirs,
                    total_bytes,
                )?;
            }
            continue;
        }

        if !file_type.is_file() {
            return Err(format!(
                "WSL_SKILL_SYNC_BLOCKED_SPECIAL_FILE: {}",
                entry_path.display()
            )
            .into());
        }

        push_wsl_skill_file(files, &relative_path, &entry_path, total_bytes)?;
    }

    Ok(())
}

fn push_wsl_skill_file(
    files: &mut Vec<WslSkillFileSyncEntry>,
    relative_path: &std::path::Path,
    source_path: &std::path::Path,
    total_bytes: &mut usize,
) -> AppResult<()> {
    if files.len() >= WSL_SKILL_MAX_FILES {
        return Err(format!(
            "SEC_INVALID_INPUT: WSL skill has too many files (max {WSL_SKILL_MAX_FILES})"
        )
        .into());
    }

    let content = read_file_with_max_len(source_path, WSL_SKILL_FILE_MAX_BYTES)?;
    let next_total = total_bytes
        .checked_add(content.len())
        .ok_or_else(|| "SEC_INVALID_INPUT: WSL skill total size overflow".to_string())?;
    ensure_wsl_skill_bytes_within_limit(
        next_total,
        WSL_SKILL_TOTAL_MAX_BYTES,
        "WSL skill total content",
    )?;
    *total_bytes = next_total;

    files.push(WslSkillFileSyncEntry {
        relative_path: relative_skill_path_string(relative_path)?,
        content,
    });
    Ok(())
}

pub(super) fn export_wsl_skill_dir(dir: &std::path::Path) -> AppResult<Vec<WslSkillFileSyncEntry>> {
    let canonical_root = dir
        .canonicalize()
        .map_err(|e| format!("failed to resolve {}: {e}", dir.display()))?;
    let mut files = Vec::new();
    let mut visited_dirs = std::collections::HashSet::new();
    let mut total_bytes = 0;
    visited_dirs.insert(canonical_root.clone());
    collect_wsl_skill_dir_files(
        &canonical_root,
        &canonical_root,
        std::path::Path::new(""),
        &mut files,
        &mut visited_dirs,
        &mut total_bytes,
    )?;
    Ok(files)
}

fn validate_wsl_skill_files(skill: &WslSkillSyncEntry) -> AppResult<()> {
    if skill.files.is_empty() {
        return Err(format!("WSL_SKILL_SYNC_EMPTY: {}", skill.skill_key).into());
    }
    if skill.files.len() > WSL_SKILL_MAX_FILES {
        return Err(format!(
            "SEC_INVALID_INPUT: WSL skill {} has too many files (max {WSL_SKILL_MAX_FILES})",
            skill.skill_key
        )
        .into());
    }

    let mut total_bytes = 0usize;
    for file in &skill.files {
        validate_wsl_skill_relative_path(&file.relative_path)?;
        ensure_wsl_skill_bytes_within_limit(
            file.content.len(),
            WSL_SKILL_FILE_MAX_BYTES,
            "WSL skill file",
        )?;
        total_bytes = total_bytes
            .checked_add(file.content.len())
            .ok_or_else(|| "SEC_INVALID_INPUT: WSL skill total size overflow".to_string())?;
        ensure_wsl_skill_bytes_within_limit(
            total_bytes,
            WSL_SKILL_TOTAL_MAX_BYTES,
            "WSL skill total content",
        )?;
    }
    Ok(())
}

pub(super) fn sync_wsl_skills_for_cli(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    skills: &[WslSkillSyncEntry],
) -> AppResult<Vec<String>> {
    if !matches!(cli_key, "claude" | "codex" | "gemini") {
        return Err(format!("unknown cli_key: {cli_key}").into());
    }

    let skills_root = resolve_wsl_skills_root(distro, cli_key)?;
    let previous_keys = read_wsl_skills_manifest(app, distro, cli_key);
    let previous_set: std::collections::HashSet<String> = previous_keys.iter().cloned().collect();

    let mut next_keys = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for skill in skills {
        validate_wsl_skill_component(&skill.skill_key, "key")?;
        validate_wsl_skill_files(skill)?;
        if seen.insert(skill.skill_key.clone()) {
            next_keys.push(skill.skill_key.clone());
        }
    }
    next_keys.sort();

    for skill_key in previous_keys {
        if next_keys.binary_search(&skill_key).is_ok() {
            continue;
        }
        let target_dir = format!("{}/{}", skills_root.trim_end_matches('/'), skill_key);
        if !wsl_path_exists(distro, &target_dir)? {
            continue;
        }
        if !wsl_has_managed_skill_dir(distro, &target_dir)? {
            return Err(format!("WSL_SKILL_SYNC_BLOCKED_UNMANAGED: {target_dir}").into());
        }
        remove_wsl_dir(distro, &target_dir)?;
    }

    for skill in skills {
        let target_dir = format!("{}/{}", skills_root.trim_end_matches('/'), skill.skill_key);
        if wsl_path_exists(distro, &target_dir)? {
            if !wsl_has_managed_skill_dir(distro, &target_dir)? {
                let reason = if previous_set.contains(&skill.skill_key) {
                    "WSL_SKILL_SYNC_MANAGED_MARKER_MISSING"
                } else {
                    "WSL_SKILL_SYNC_BLOCKED_UNMANAGED"
                };
                return Err(format!("{reason}: {target_dir}").into());
            }
            remove_wsl_dir(distro, &target_dir)?;
        }

        for file in &skill.files {
            let relative_path = validate_wsl_skill_relative_path(&file.relative_path)?;
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");
            let target_path = format!("{target_dir}/{relative_path}");
            write_wsl_file(distro, &target_path, &file.content)?;
        }

        let marker_path = format!("{target_dir}/{WSL_SKILL_MANAGED_MARKER_FILE}");
        write_wsl_file(distro, &marker_path, b"aio-coding-hub\n")?;
    }

    Ok(next_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_wsl_skill_dir_rejects_oversized_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("large-skill");
        std::fs::create_dir_all(&root).expect("create skill dir");
        std::fs::write(
            root.join("SKILL.md"),
            vec![b'x'; WSL_SKILL_FILE_MAX_BYTES + 1],
        )
        .expect("write oversized skill file");

        let err = match export_wsl_skill_dir(&root) {
            Ok(_) => panic!("oversized skill file should fail"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn validate_wsl_skill_files_rejects_too_many_files() {
        let skill = WslSkillSyncEntry {
            skill_key: "large-skill".to_string(),
            files: (0..=WSL_SKILL_MAX_FILES)
                .map(|index| WslSkillFileSyncEntry {
                    relative_path: format!("file-{index}.txt"),
                    content: b"x".to_vec(),
                })
                .collect(),
        };

        let err = match validate_wsl_skill_files(&skill) {
            Ok(_) => panic!("too many files should fail"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("too many files"));
    }
}
