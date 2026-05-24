//! Usage: Browse historical Claude/Codex CLI sessions (projects → sessions → messages).

use crate::shared::error::AppError;
use crate::{blocking, cli_sessions};
use serde::Deserialize;

const CLI_SESSIONS_MAX_TEXT_CHARS: usize = 4096;
const CLI_SESSIONS_MAX_DELETE_PATHS: usize = 512;
const CLI_SESSIONS_MAX_FOLDER_LOOKUP_ITEMS: usize = 512;

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub(crate) struct CliSessionsFolderLookupInput {
    source: String,
    session_id: String,
}

fn normalize_required_text(raw: &str, label: &str) -> Result<String, AppError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("{label} is required"),
        ));
    }
    if normalized.chars().count() > CLI_SESSIONS_MAX_TEXT_CHARS {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("{label} is too long (max {CLI_SESSIONS_MAX_TEXT_CHARS} chars)"),
        ));
    }
    Ok(normalized.to_string())
}

fn normalize_session_delete_paths(file_paths: Vec<String>) -> Result<Vec<String>, AppError> {
    if file_paths.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "filePaths is required"));
    }
    if file_paths.len() > CLI_SESSIONS_MAX_DELETE_PATHS {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("filePaths must contain at most {CLI_SESSIONS_MAX_DELETE_PATHS} entries"),
        ));
    }

    let mut normalized = Vec::with_capacity(file_paths.len());
    for file_path in file_paths {
        let trimmed = file_path.trim();
        if trimmed.is_empty() {
            continue;
        }
        normalized.push(normalize_required_text(trimmed, "filePath")?);
    }
    if normalized.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "filePaths is required"));
    }

    Ok(normalized)
}

fn normalize_folder_lookup_items(
    items: Vec<CliSessionsFolderLookupInput>,
) -> Result<Vec<cli_sessions::CliSessionsFolderLookupKey>, AppError> {
    if items.len() > CLI_SESSIONS_MAX_FOLDER_LOOKUP_ITEMS {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!(
                "folder lookup items must contain at most {CLI_SESSIONS_MAX_FOLDER_LOOKUP_ITEMS} entries"
            ),
        ));
    }

    let mut normalized: Vec<cli_sessions::CliSessionsFolderLookupKey> =
        Vec::with_capacity(items.len());
    for item in items {
        let source = item.source.parse::<cli_sessions::CliSessionsSource>()?;
        let session_id = item.session_id.trim();
        if session_id.is_empty() {
            continue;
        }
        normalized.push(cli_sessions::CliSessionsFolderLookupKey {
            source,
            session_id: normalize_required_text(session_id, "sessionId")?,
        });
    }

    Ok(normalized)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_sessions_projects_list(
    app: tauri::AppHandle,
    source: String,
    wsl_distro: Option<String>,
) -> Result<Vec<cli_sessions::CliSessionsProjectSummary>, String> {
    let source = source.parse::<cli_sessions::CliSessionsSource>()?;
    blocking::run("cli_sessions_projects_list", move || {
        cli_sessions::projects_list(&app, source, wsl_distro.as_deref())
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_sessions_sessions_list(
    app: tauri::AppHandle,
    source: String,
    project_id: String,
    wsl_distro: Option<String>,
) -> Result<Vec<cli_sessions::CliSessionsSessionSummary>, String> {
    let source = source.parse::<cli_sessions::CliSessionsSource>()?;
    let project_id = normalize_required_text(&project_id, "projectId")?;

    blocking::run("cli_sessions_sessions_list", move || {
        cli_sessions::sessions_list(&app, source, &project_id, wsl_distro.as_deref())
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_sessions_messages_get(
    app: tauri::AppHandle,
    source: String,
    file_path: String,
    page: u32,
    page_size: u32,
    from_end: Option<bool>,
    wsl_distro: Option<String>,
) -> Result<cli_sessions::CliSessionsPaginatedMessages, String> {
    let source = source.parse::<cli_sessions::CliSessionsSource>()?;
    let file_path = normalize_required_text(&file_path, "filePath")?;

    let from_end = from_end.unwrap_or(true);
    let page = page as usize;
    let page_size = page_size as usize;

    blocking::run("cli_sessions_messages_get", move || {
        cli_sessions::messages_get(
            &app,
            source,
            &file_path,
            page,
            page_size,
            from_end,
            wsl_distro.as_deref(),
        )
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_sessions_session_delete(
    app: tauri::AppHandle,
    source: String,
    file_paths: Vec<String>,
    wsl_distro: Option<String>,
) -> Result<Vec<String>, String> {
    let source = source.parse::<cli_sessions::CliSessionsSource>()?;
    let file_paths = normalize_session_delete_paths(file_paths)?;

    blocking::run("cli_sessions_session_delete", move || {
        let mut failed: Vec<String> = Vec::new();
        for fp in &file_paths {
            if let Err(e) = cli_sessions::session_delete(&app, source, fp, wsl_distro.as_deref()) {
                failed.push(format!("{fp}: {e}"));
            }
        }
        Ok::<Vec<String>, AppError>(failed)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn cli_sessions_folder_lookup_by_ids(
    app: tauri::AppHandle,
    items: Vec<CliSessionsFolderLookupInput>,
    wsl_distro: Option<String>,
) -> Result<Vec<cli_sessions::CliSessionsFolderLookupEntry>, String> {
    let normalized = normalize_folder_lookup_items(items)?;

    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    blocking::run("cli_sessions_folder_lookup_by_ids", move || {
        cli_sessions::folder_lookup_by_ids(&app, &normalized, wsl_distro.as_deref())
    })
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_required_text_trims_and_bounds_values() {
        assert_eq!(
            normalize_required_text("  project  ", "projectId").expect("projectId"),
            "project"
        );
        assert!(normalize_required_text("   ", "projectId")
            .expect_err("blank projectId should fail")
            .to_string()
            .contains("SEC_INVALID_INPUT"));
        assert!(
            normalize_required_text(&"x".repeat(CLI_SESSIONS_MAX_TEXT_CHARS + 1), "projectId")
                .expect_err("oversized projectId should fail")
                .to_string()
                .contains("SEC_INVALID_INPUT")
        );
    }

    #[test]
    fn normalize_session_delete_paths_filters_empty_and_caps_batches() {
        let normalized = normalize_session_delete_paths(vec![
            " /tmp/a.jsonl ".to_string(),
            "   ".to_string(),
            "/tmp/b.jsonl".to_string(),
        ])
        .expect("normalize delete paths");
        assert_eq!(normalized, vec!["/tmp/a.jsonl", "/tmp/b.jsonl"]);

        assert!(normalize_session_delete_paths(Vec::new())
            .expect_err("empty batch should fail")
            .to_string()
            .contains("SEC_INVALID_INPUT"));
        assert!(normalize_session_delete_paths(vec!["   ".to_string()])
            .expect_err("blank-only batch should fail")
            .to_string()
            .contains("SEC_INVALID_INPUT"));
        assert!(normalize_session_delete_paths(vec![
            "/tmp/session.jsonl".to_string();
            CLI_SESSIONS_MAX_DELETE_PATHS + 1
        ])
        .expect_err("oversized delete batch should fail")
        .to_string()
        .contains("SEC_INVALID_INPUT"));
    }

    #[test]
    fn normalize_folder_lookup_items_filters_empty_and_caps_batches() {
        let normalized = normalize_folder_lookup_items(vec![
            CliSessionsFolderLookupInput {
                source: "claude".to_string(),
                session_id: " s1 ".to_string(),
            },
            CliSessionsFolderLookupInput {
                source: "codex".to_string(),
                session_id: "   ".to_string(),
            },
        ])
        .expect("normalize lookup items");
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0].source,
            cli_sessions::CliSessionsSource::Claude
        );
        assert_eq!(normalized[0].session_id, "s1");

        assert!(normalize_folder_lookup_items(
            (0..=CLI_SESSIONS_MAX_FOLDER_LOOKUP_ITEMS)
                .map(|index| CliSessionsFolderLookupInput {
                    source: "claude".to_string(),
                    session_id: format!("s{index}"),
                })
                .collect()
        )
        .expect_err("oversized lookup batch should fail")
        .to_string()
        .contains("SEC_INVALID_INPUT"));
    }
}
