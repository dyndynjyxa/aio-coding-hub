//! Usage: Browse historical sessions from local Claude/Codex CLI logs (projects → sessions → messages).

mod claude;
mod codex;
mod types;

pub use types::{
    CliSessionsDisplayContentBlock, CliSessionsDisplayMessage, CliSessionsFolderLookupEntry,
    CliSessionsPaginatedMessages, CliSessionsProjectSummary, CliSessionsSessionSummary,
};

use crate::shared::error::{AppError, AppResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CliSessionsFolderLookupKey {
    pub source: CliSessionsSource,
    pub session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliSessionsSource {
    Claude,
    Codex,
}

impl std::str::FromStr for CliSessionsSource {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "claude" => Ok(CliSessionsSource::Claude),
            "codex" => Ok(CliSessionsSource::Codex),
            other => Err(AppError::new(
                "SEC_INVALID_INPUT",
                format!("unknown source: {other}"),
            )),
        }
    }
}

impl CliSessionsSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CliSessionsSource::Claude => "claude",
            CliSessionsSource::Codex => "codex",
        }
    }
}

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 200;

fn normalize_page_size(raw: usize) -> usize {
    let v = if raw == 0 { DEFAULT_PAGE_SIZE } else { raw };
    v.clamp(1, MAX_PAGE_SIZE)
}

pub fn projects_list(
    app: &tauri::AppHandle,
    source: CliSessionsSource,
    wsl_distro: Option<&str>,
) -> AppResult<Vec<CliSessionsProjectSummary>> {
    if let Some(distro) = wsl_distro {
        crate::wsl::validate_distro(distro)?;
        return match source {
            CliSessionsSource::Claude => claude::wsl_projects_list(distro),
            CliSessionsSource::Codex => codex::wsl_projects_list(distro),
        };
    }
    match source {
        CliSessionsSource::Claude => claude::projects_list(app),
        CliSessionsSource::Codex => codex::projects_list(app),
    }
}

pub fn sessions_list(
    app: &tauri::AppHandle,
    source: CliSessionsSource,
    project_id: &str,
    wsl_distro: Option<&str>,
) -> AppResult<Vec<CliSessionsSessionSummary>> {
    if let Some(distro) = wsl_distro {
        crate::wsl::validate_distro(distro)?;
        return match source {
            CliSessionsSource::Claude => claude::wsl_sessions_list(distro, project_id),
            CliSessionsSource::Codex => codex::wsl_sessions_list(distro, project_id),
        };
    }
    match source {
        CliSessionsSource::Claude => claude::sessions_list(app, project_id),
        CliSessionsSource::Codex => codex::sessions_list(app, project_id),
    }
}

pub fn folder_lookup_by_ids(
    app: &tauri::AppHandle,
    items: &[CliSessionsFolderLookupKey],
    wsl_distro: Option<&str>,
) -> AppResult<Vec<CliSessionsFolderLookupEntry>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(distro) = wsl_distro {
        crate::wsl::validate_distro(distro)?;
    }

    let mut claude_ids: Vec<String> = Vec::new();
    let mut codex_ids: Vec<String> = Vec::new();
    let mut seen = HashSet::new();

    for item in items {
        let session_id = item.session_id.trim();
        if session_id.is_empty() {
            continue;
        }
        let dedupe_key = format!("{}:{session_id}", item.source.as_str());
        if !seen.insert(dedupe_key) {
            continue;
        }
        match item.source {
            CliSessionsSource::Claude => claude_ids.push(session_id.to_string()),
            CliSessionsSource::Codex => codex_ids.push(session_id.to_string()),
        }
    }

    let mut out = Vec::new();

    if !claude_ids.is_empty() {
        out.extend(match wsl_distro {
            Some(distro) => claude::wsl_folder_lookup_by_session_ids(distro, &claude_ids)?,
            None => claude::folder_lookup_by_session_ids(app, &claude_ids)?,
        });
    }

    if !codex_ids.is_empty() {
        out.extend(match wsl_distro {
            Some(distro) => codex::wsl_folder_lookup_by_session_ids(distro, &codex_ids)?,
            None => codex::folder_lookup_by_session_ids(app, &codex_ids)?,
        });
    }

    Ok(out)
}

pub fn messages_get(
    app: &tauri::AppHandle,
    source: CliSessionsSource,
    file_path: &str,
    page: usize,
    page_size: usize,
    from_end: bool,
    wsl_distro: Option<&str>,
) -> AppResult<CliSessionsPaginatedMessages> {
    let page_size = normalize_page_size(page_size);
    if let Some(distro) = wsl_distro {
        crate::wsl::validate_distro(distro)?;
        return match source {
            CliSessionsSource::Claude => {
                claude::wsl_messages_get(distro, file_path, page, page_size, from_end)
            }
            CliSessionsSource::Codex => {
                codex::wsl_messages_get(distro, file_path, page, page_size, from_end)
            }
        };
    }
    match source {
        CliSessionsSource::Claude => {
            claude::messages_get(app, file_path, page, page_size, from_end)
        }
        CliSessionsSource::Codex => codex::messages_get(app, file_path, page, page_size, from_end),
    }
}

/// Delete a session file. file_path must be a .jsonl file within a valid root directory.
pub fn session_delete(
    app: &tauri::AppHandle,
    source: CliSessionsSource,
    file_path: &str,
    wsl_distro: Option<&str>,
) -> AppResult<bool> {
    if let Some(distro) = wsl_distro {
        crate::wsl::validate_distro(distro)?;
        return match source {
            CliSessionsSource::Claude => claude::wsl_session_delete(distro, file_path),
            CliSessionsSource::Codex => codex::wsl_session_delete(distro, file_path),
        };
    }
    match source {
        CliSessionsSource::Claude => claude::session_delete(app, file_path),
        CliSessionsSource::Codex => codex::session_delete(app, file_path),
    }
}

pub(super) fn truncate_string(raw: &str, max_len: usize) -> String {
    if raw.len() <= max_len {
        return raw.to_string();
    }
    raw.chars().take(max_len).collect::<String>()
}

pub(super) fn folder_name_from_path(path: &str) -> Option<String> {
    let trimmed = path.trim().trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return None;
    }
    let name = trimmed
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(name.to_string())
}

/// Validates that a path is within the specified root directory.
/// Prevents path traversal attacks by canonicalizing both paths and checking boundaries.
///
/// # Arguments
/// * `path` - The path to validate
/// * `root` - The root directory that path must be within
///
/// # Returns
/// * `Ok(PathBuf)` - The canonicalized path if valid
/// * `Err(AppError)` - If path is outside root or cannot be resolved
///
/// # Security
/// This function defends against:
/// - Symlink attacks
/// - Relative path traversal (../, ../../etc/passwd)
/// - Absolute path escapes
pub fn validate_path_under_root(path: &Path, root: &Path) -> AppResult<PathBuf> {
    let canonical_root = std::fs::canonicalize(root).map_err(|e| {
        AppError::new(
            "SEC_INVALID_INPUT",
            format!("root directory not found: {e}"),
        )
    })?;

    let canonical_path = std::fs::canonicalize(path)
        .map_err(|e| AppError::new("SEC_INVALID_INPUT", format!("path not found: {e}")))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::new(
            "SEC_PATH_TRAVERSAL",
            "path is outside root directory",
        ));
    }

    Ok(canonical_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_validate_path_under_root_valid() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join("cli_sessions_test_root");
        fs::create_dir_all(&test_root).unwrap();

        let test_file = test_root.join("valid_file.txt");
        fs::write(&test_file, "test").unwrap();

        let result = validate_path_under_root(&test_file, &test_root);
        assert!(result.is_ok());

        fs::remove_dir_all(&test_root).ok();
    }

    #[test]
    fn test_validate_path_under_root_relative_traversal() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join("cli_sessions_test_root2");
        fs::create_dir_all(&test_root).unwrap();

        // Try to escape using relative path
        let malicious_path = test_root.join("../../etc/passwd");

        let result = validate_path_under_root(&malicious_path, &test_root);
        // Should fail because canonicalize will resolve to actual /etc/passwd
        // which is outside test_root, OR file doesn't exist
        assert!(result.is_err());

        fs::remove_dir_all(&test_root).ok();
    }

    #[test]
    fn test_validate_path_under_root_absolute_escape() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join("cli_sessions_test_root3");
        fs::create_dir_all(&test_root).unwrap();

        // Try to use absolute path outside root
        let outside_path = if cfg!(windows) {
            PathBuf::from("C:\\Windows\\System32")
        } else {
            PathBuf::from("/etc")
        };

        let result = validate_path_under_root(&outside_path, &test_root);
        // Should fail because path is outside root
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("SEC_PATH_TRAVERSAL"));
        }

        fs::remove_dir_all(&test_root).ok();
    }
}
