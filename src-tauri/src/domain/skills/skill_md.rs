use super::limits::{SKILL_DISCOVERY_DIR_MAX, SKILL_DISCOVERY_SKILL_MD_MAX, SKILL_MD_MAX_BYTES};
use crate::shared::fs::read_file_with_max_len;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn read_skill_md_text(path: &Path) -> Result<String, String> {
    let bytes = read_file_with_max_len(path, SKILL_MD_MAX_BYTES).map_err(|e| e.to_string())?;
    String::from_utf8(bytes)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid UTF-8 in SKILL.md: {e}"))
}

fn strip_quotes(input: &str) -> &str {
    let s = input.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[s.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn parse_front_matter(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim().to_string();
        let value = strip_quotes(v).trim().to_string();
        if key.is_empty() {
            continue;
        }
        out.insert(key, value);
    }
    out
}

pub(super) fn parse_skill_md(skill_md_path: &Path) -> Result<(String, String), String> {
    let text = read_skill_md_text(skill_md_path)?;
    let text = text.trim_start();
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        return Err("SEC_INVALID_INPUT: SKILL.md is empty".to_string());
    };
    if first.trim() != "---" {
        return Err("SEC_INVALID_INPUT: SKILL.md front matter is required".to_string());
    }

    let mut fm = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        fm.push_str(line);
        fm.push('\n');
    }

    let map = parse_front_matter(&fm);
    let name = map.get("name").cloned().unwrap_or_default();
    let desc = map.get("description").cloned().unwrap_or_default();

    if name.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: SKILL.md missing 'name'".to_string());
    }

    Ok((name.trim().to_string(), desc.trim().to_string()))
}

pub(super) fn find_skill_md_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited_dirs = 0_usize;

    while let Some(dir) = stack.pop() {
        if visited_dirs >= SKILL_DISCOVERY_DIR_MAX {
            return Err(format!(
                "SEC_INVALID_INPUT: too many skill directories (max {SKILL_DISCOVERY_DIR_MAX})"
            ));
        }
        visited_dirs += 1;

        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("failed to read dir entry {}: {e}", dir.display()))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let file_type = entry
                .file_type()
                .map_err(|e| format!("failed to read file type {}: {e}", path.display()))?;

            if file_type.is_dir() {
                if file_name == ".git" {
                    continue;
                }
                stack.push(path);
                continue;
            }

            if file_name.eq_ignore_ascii_case("SKILL.md") {
                if out.len() >= SKILL_DISCOVERY_SKILL_MD_MAX {
                    return Err(format!(
                        "SEC_INVALID_INPUT: too many SKILL.md files (max {SKILL_DISCOVERY_SKILL_MD_MAX})"
                    ));
                }
                out.push(path);
            }
        }
    }

    Ok(out)
}
