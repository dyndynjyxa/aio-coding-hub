use super::git_url::canonical_git_url_key;
use super::installed::installed_source_set;
use super::repo_cache::ensure_repo_cache;
use super::skill_md::{find_skill_md_files, parse_skill_md};
use super::types::AvailableSkillSummary;
use super::util::normalize_name;
use crate::db;
use std::collections::{BTreeMap, HashSet};

pub fn discover_available(
    app: &tauri::AppHandle,
    db: &db::Db,
    refresh: bool,
) -> Result<Vec<AvailableSkillSummary>, String> {
    fn subdir_score(source_subdir: &str) -> i32 {
        let subdir = source_subdir.trim_matches('/').to_ascii_lowercase();
        let mut score = 0;

        if subdir.starts_with(".claude/skills/") {
            score += 100;
        }
        if subdir.starts_with(".codex/skills/") {
            score += 100;
        }
        if subdir.starts_with(".gemini/skills/") {
            score += 100;
        }

        if subdir.starts_with("skills/") {
            score += 80;
        }

        if subdir.starts_with("cli/assets/") || subdir.contains("/cli/assets/") {
            score -= 120;
        }
        if subdir.starts_with("assets/") || subdir.contains("/assets/") {
            score -= 30;
        }
        if subdir.starts_with("examples/") || subdir.contains("/examples/") {
            score -= 20;
        }

        score
    }

    fn prefer_candidate(a: &AvailableSkillSummary, b: &AvailableSkillSummary) -> bool {
        if a.installed != b.installed {
            return b.installed;
        }

        let score_a = subdir_score(&a.source_subdir);
        let score_b = subdir_score(&b.source_subdir);
        if score_a != score_b {
            return score_b > score_a;
        }

        if a.source_subdir.len() != b.source_subdir.len() {
            return b.source_subdir.len() < a.source_subdir.len();
        }

        b.source_subdir < a.source_subdir
    }

    let conn = db.open_connection()?;

    let installed_sources = installed_source_set(&conn)?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT git_url, branch
FROM skill_repos
WHERE enabled = 1
ORDER BY updated_at DESC, id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare repo query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("DB_ERROR: failed to query enabled repos: {e}"))?;

    let mut repos = Vec::new();
    let mut seen_repos = HashSet::new();
    for row in rows {
        let (git_url, branch) =
            row.map_err(|e| format!("DB_ERROR: failed to read repo row: {e}"))?;
        let key = canonical_git_url_key(&git_url);
        let key = if key.is_empty() {
            git_url.trim().to_ascii_lowercase()
        } else {
            key
        };
        if seen_repos.insert(key) {
            repos.push((git_url, branch));
        }
    }

    let mut out = Vec::new();
    for (git_url, branch) in repos {
        let repo_dir = ensure_repo_cache(app, &git_url, &branch, refresh)?;
        let skill_mds = find_skill_md_files(&repo_dir)?;

        let mut best_by_name: BTreeMap<String, AvailableSkillSummary> = BTreeMap::new();

        for skill_md in skill_mds {
            let skill_dir = skill_md
                .parent()
                .ok_or_else(|| "SEC_INVALID_INPUT: invalid SKILL.md path".to_string())?;

            let (name, description) = match parse_skill_md(&skill_md) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let subdir_rel = skill_dir.strip_prefix(&repo_dir).map_err(|_| {
                "SEC_INVALID_INPUT: failed to compute skill relative path".to_string()
            })?;
            let source_subdir = subdir_rel
                .to_string_lossy()
                .replace('\\', "/")
                .trim_matches('/')
                .to_string();

            if source_subdir.is_empty() {
                continue;
            }

            let installed =
                installed_sources.contains(&format!("{}#{}#{}", git_url, branch, source_subdir));

            let candidate = AvailableSkillSummary {
                name,
                description,
                source_git_url: git_url.clone(),
                source_branch: branch.clone(),
                source_subdir,
                installed,
            };

            let key = normalize_name(&candidate.name);
            match best_by_name.get_mut(&key) {
                None => {
                    best_by_name.insert(key, candidate);
                }
                Some(existing) => {
                    if prefer_candidate(existing, &candidate) {
                        *existing = candidate;
                    }
                }
            }
        }

        out.extend(best_by_name.into_values());
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
