//! Usage: Read metadata written by the `npx skills` installer.

use super::fs_ops::SkillSourceMetadata;
use super::git_url::parse_github_owner_repo;
use super::util::validate_relative_subdir;
use crate::app_paths;
use crate::shared::fs::read_optional_file_with_max_len;
use serde_json::Value;
use std::path::Path;

const NPX_SKILL_LOCK_MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub(super) struct NpxSkillLock {
    entries: Vec<NpxSkillLockEntry>,
}

#[derive(Debug, Clone)]
struct NpxSkillLockEntry {
    keys: Vec<String>,
    source: SkillSourceMetadata,
}

impl NpxSkillLock {
    pub(super) fn read<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Self {
        let Ok(home) = app_paths::home_dir(app) else {
            return Self::default();
        };
        let mut paths = Vec::new();
        if let Some(state_home) = std::env::var_os("XDG_STATE_HOME")
            .map(std::path::PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
        {
            paths.push(state_home.join(".agents").join(".skill-lock.json"));
            paths.push(state_home.join("agents").join(".skill-lock.json"));
        }
        paths.push(home.join(".agents").join(".skill-lock.json"));

        let mut merged = Self::default();
        for path in paths {
            let Ok(Some(bytes)) = read_optional_file_with_max_len(&path, NPX_SKILL_LOCK_MAX_BYTES)
            else {
                continue;
            };
            let Ok(root) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            let lock = Self::from_json(&root);
            if !lock.entries.is_empty() {
                merged.entries.extend(lock.entries);
            }
        }

        merged
    }

    fn from_json(root: &Value) -> Self {
        let mut entries = Vec::new();
        let Some(skills) = root.get("skills").and_then(Value::as_object) else {
            return Self::default();
        };

        for (skill_key, value) in skills {
            let Some(source) = parse_source_metadata(value) else {
                continue;
            };

            let mut keys = vec![skill_key.as_str()];
            if let Some(file_name) = Path::new(&source.source_subdir)
                .file_name()
                .and_then(|value| value.to_str())
            {
                keys.push(file_name);
            }

            entries.push(NpxSkillLockEntry {
                keys: keys
                    .into_iter()
                    .map(normalize_match_key)
                    .filter(|key| !key.is_empty())
                    .collect(),
                source,
            });
        }

        Self { entries }
    }

    pub(super) fn source_for_local_skill(
        &self,
        dir_name: &str,
        _skill_name: &str,
    ) -> Option<SkillSourceMetadata> {
        if self.entries.is_empty() {
            return None;
        }

        let candidates = [
            normalize_match_key(dir_name),
            normalize_hyphen_key(dir_name),
        ];

        self.entries
            .iter()
            .find(|entry| {
                entry
                    .keys
                    .iter()
                    .any(|key| candidates.iter().any(|candidate| candidate == key))
            })
            .map(|entry| entry.source.clone())
    }
}

fn parse_source_metadata(value: &Value) -> Option<SkillSourceMetadata> {
    let source_type = text_field(value, "sourceType")
        .or_else(|| text_field(value, "source_type"))
        .or_else(|| text_field(value, "source").filter(|source| is_known_source_type(source)))
        .unwrap_or("");
    let source_git_url = source_url_field(
        value,
        &[
            "sourceUrl",
            "source_url",
            "gitUrl",
            "git_url",
            "repoUrl",
            "repo_url",
            "repository",
            "url",
            "source",
        ],
    )
    .and_then(|source| normalize_source_url(source, source_type))?;

    let source_branch = text_field_any(value, &["ref", "branch", "sourceBranch", "source_branch"])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("auto")
        .to_string();

    let raw_skill_path = text_field_any(
        value,
        &[
            "skillPath",
            "skill_path",
            "sourceSubdir",
            "source_subdir",
            "path",
        ],
    )?;
    let source_subdir = normalize_skill_path(raw_skill_path)?;

    Some(SkillSourceMetadata {
        source_git_url,
        source_branch,
        source_subdir,
    })
}

fn is_known_source_type(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "github" | "git" | "npm" | "local"
    )
}

fn text_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn text_field_any<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| text_field(value, key))
}

fn source_url_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        let Some(field) = text_field(value, key) else {
            continue;
        };
        if *key == "source" && is_known_source_type(field) {
            continue;
        }
        return Some(field);
    }
    None
}

fn normalize_source_url(raw: &str, source_type: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    let source_type = source_type.trim().to_ascii_lowercase();
    if matches!(source_type.as_str(), "npm" | "local") {
        return None;
    }
    if !matches!(source_type.as_str(), "" | "github" | "git") {
        return None;
    }

    if let Some(rest) = value.strip_prefix("github:") {
        let repo = rest.trim().trim_matches('/');
        if repo.split('/').filter(|part| !part.is_empty()).count() >= 2 {
            return Some(format!("https://github.com/{repo}"));
        }
    }
    if !value.contains("://") && !value.starts_with("git@") && source_type == "github" {
        let repo = value.trim().trim_matches('/');
        if repo.split('/').filter(|part| !part.is_empty()).count() >= 2 {
            return Some(format!("https://github.com/{repo}"));
        }
    }

    if !is_git_source_url(value, &source_type) {
        return None;
    }

    Some(value.to_string())
}

fn is_git_source_url(value: &str, source_type: &str) -> bool {
    if parse_github_owner_repo(value).is_some() {
        return true;
    }

    let lower = value.to_ascii_lowercase();
    if value.starts_with("git@") || lower.starts_with("ssh://") || lower.starts_with("git://") {
        return true;
    }
    if lower.ends_with(".git") {
        return true;
    }

    source_type == "git" && (lower.starts_with("http://") || lower.starts_with("https://"))
}

fn normalize_skill_path(raw: &str) -> Option<String> {
    let normalized = raw.trim().replace('\\', "/");
    let normalized = normalized.trim_matches('/');
    if normalized.is_empty() {
        return None;
    }

    let source_subdir = normalized
        .strip_suffix("/SKILL.md")
        .or_else(|| normalized.strip_suffix("/skill.md"))
        .unwrap_or(normalized)
        .trim_matches('/')
        .to_string();

    if source_subdir.is_empty() || validate_relative_subdir(&source_subdir).is_err() {
        return None;
    }
    Some(source_subdir)
}

fn normalize_match_key(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn normalize_hyphen_key(input: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in input.trim().chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            out.push(ch);
            previous_dash = false;
            continue;
        }
        if !out.is_empty() && !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_vercel_skills_lock_shape() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "schemaVersion": 1,
            "skills": {
                "code-review": {
                    "source": "github",
                    "sourceUrl": "https://github.com/acme/skills",
                    "ref": "main",
                    "skillPath": "packs/code-review/SKILL.md",
                    "skillFolderHash": "abc"
                }
            }
        }));

        let source = lock
            .source_for_local_skill("code-review", "Code Review")
            .expect("source");

        assert_eq!(source.source_git_url, "https://github.com/acme/skills");
        assert_eq!(source.source_branch, "main");
        assert_eq!(source.source_subdir, "packs/code-review");
    }

    #[test]
    fn matches_sanitized_skill_name() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "code-review": {
                    "sourceUrl": "github:acme/skills",
                    "ref": "main",
                    "skillPath": "code-review"
                }
            }
        }));

        assert!(lock
            .source_for_local_skill("Code Review", "Code Review")
            .is_some());
    }

    #[test]
    fn reads_source_type_github_with_repo_shorthand() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "review-skill": {
                    "sourceType": "github",
                    "source": "acme/skills",
                    "ref": "main",
                    "skillPath": "review-skill/SKILL.md"
                }
            }
        }));

        let source = lock
            .source_for_local_skill("review-skill", "Review Skill")
            .expect("source");

        assert_eq!(source.source_git_url, "https://github.com/acme/skills");
    }

    #[test]
    fn ignores_source_when_it_is_only_a_source_type() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "review-skill": {
                    "source": "github",
                    "ref": "main",
                    "skillPath": "review-skill/SKILL.md"
                }
            }
        }));

        assert!(lock
            .source_for_local_skill("review-skill", "Review Skill")
            .is_none());
    }

    #[test]
    fn ignores_non_git_source_types() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "npm-skill": {
                    "sourceType": "npm",
                    "sourceUrl": "https://registry.npmjs.org/@acme/skills",
                    "skillPath": "npm-skill/SKILL.md"
                },
                "local-skill": {
                    "sourceType": "local",
                    "sourceUrl": "https://github.com/acme/skills",
                    "skillPath": "local-skill/SKILL.md"
                }
            }
        }));

        assert!(lock
            .source_for_local_skill("npm-skill", "Npm Skill")
            .is_none());
        assert!(lock
            .source_for_local_skill("local-skill", "Local Skill")
            .is_none());
    }

    #[test]
    fn does_not_match_by_display_name_only() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "remote-review": {
                    "name": "Review Skill",
                    "sourceUrl": "https://github.com/acme/skills",
                    "skillPath": "packs/remote-review/SKILL.md"
                }
            }
        }));

        assert!(lock
            .source_for_local_skill("manual-review", "Review Skill")
            .is_none());
    }

    #[test]
    fn does_not_match_slug_name_when_directory_differs() {
        let lock = NpxSkillLock::from_json(&serde_json::json!({
            "skills": {
                "remote-review": {
                    "name": "context7",
                    "sourceUrl": "https://github.com/acme/skills",
                    "skillPath": "packs/remote-review/SKILL.md"
                }
            }
        }));

        assert!(lock
            .source_for_local_skill("context7", "context7")
            .is_none());
    }
}
