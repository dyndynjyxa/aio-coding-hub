use super::git_url::canonical_git_url_key;
use super::installed::installed_source_set;
use super::repo_cache::ensure_repo_cache;
use super::skill_md::{find_skill_md_files, parse_skill_md};
use super::types::AvailableSkillSummary;
use crate::db;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
struct RepoDiscoverySource {
    id: i64,
    git_url: String,
    branch: String,
    enabled: bool,
}

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

fn row_to_repo_source(row: &rusqlite::Row<'_>) -> Result<RepoDiscoverySource, rusqlite::Error> {
    Ok(RepoDiscoverySource {
        id: row.get("id")?,
        git_url: row.get("git_url")?,
        branch: row.get("branch")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
    })
}

fn enabled_repo_sources(
    conn: &Connection,
) -> crate::shared::error::AppResult<Vec<RepoDiscoverySource>> {
    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT id, git_url, branch, enabled
    FROM skill_repos
    WHERE enabled = 1
    ORDER BY updated_at DESC, id DESC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare repo query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_repo_source)
        .map_err(|e| db_err!("failed to query enabled repos: {e}"))?;

    let mut repos = Vec::new();
    let mut seen_repos = HashSet::new();
    for row in rows {
        let repo = row.map_err(|e| db_err!("failed to read repo row: {e}"))?;
        let key = canonical_git_url_key(&repo.git_url);
        let key = if key.is_empty() {
            repo.git_url.trim().to_ascii_lowercase()
        } else {
            key
        };
        if seen_repos.insert(key) {
            repos.push(repo);
        }
    }

    Ok(repos)
}

fn repo_source_by_id(
    conn: &Connection,
    repo_id: i64,
) -> crate::shared::error::AppResult<RepoDiscoverySource> {
    if repo_id <= 0 {
        return Err(format!("SEC_INVALID_INPUT: invalid repo_id={repo_id}").into());
    }

    match conn.query_row(
        r#"
    SELECT id, git_url, branch, enabled
    FROM skill_repos
    WHERE id = ?1
    "#,
        params![repo_id],
        row_to_repo_source,
    ) {
        Ok(repo) => Ok(repo),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err("DB_NOT_FOUND: skill repo not found".to_string().into())
        }
        Err(err) => Err(db_err!("failed to query repo: {err}")),
    }
}

fn discover_repo_available_from_source(
    app: &tauri::AppHandle,
    installed_sources: &HashSet<String>,
    git_url: &str,
    branch: &str,
    refresh: bool,
) -> crate::shared::error::AppResult<Vec<AvailableSkillSummary>> {
    let repo_dir = ensure_repo_cache(app, git_url, branch, refresh)?;
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

        let subdir_rel = skill_dir
            .strip_prefix(&repo_dir)
            .map_err(|_| "SEC_INVALID_INPUT: failed to compute skill relative path".to_string())?;
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
            source_git_url: git_url.to_string(),
            source_branch: branch.to_string(),
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

    let mut out: Vec<_> = best_by_name.into_values().collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn discover_available(
    app: &tauri::AppHandle,
    db: &db::Db,
    refresh: bool,
) -> crate::shared::error::AppResult<Vec<AvailableSkillSummary>> {
    let conn = db.open_connection()?;
    let installed_sources = installed_source_set(&conn)?;
    let repos = enabled_repo_sources(&conn)?;

    let mut out = Vec::new();
    for repo in repos {
        match discover_repo_available_from_source(
            app,
            &installed_sources,
            &repo.git_url,
            &repo.branch,
            refresh,
        ) {
            Ok(rows) => out.extend(rows),
            Err(err) => {
                tracing::warn!(
                    repo_id = repo.id,
                    git_url = %repo.git_url,
                    branch = %repo.branch,
                    error = %err,
                    "skill repo discovery failed; continuing with remaining repos"
                );
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn discover_repo_available(
    app: &tauri::AppHandle,
    db: &db::Db,
    repo_id: i64,
    refresh: bool,
) -> crate::shared::error::AppResult<Vec<AvailableSkillSummary>> {
    let conn = db.open_connection()?;
    let installed_sources = installed_source_set(&conn)?;
    let repo = repo_source_by_id(&conn, repo_id)?;
    if !repo.enabled {
        return Ok(Vec::new());
    }

    discover_repo_available_from_source(
        app,
        &installed_sources,
        &repo.git_url,
        &repo.branch,
        refresh,
    )
}
