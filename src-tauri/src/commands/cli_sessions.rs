//! Usage: Browse historical Claude/Codex CLI sessions (projects → sessions → messages).

use crate::shared::error::AppError;
use crate::{blocking, cli_sessions};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub(crate) struct CliSessionsFolderLookupInput {
    source: String,
    session_id: String,
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
    let project_id = project_id.trim().to_string();
    if project_id.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "projectId is required").into());
    }

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
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "filePath is required").into());
    }

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
    if file_paths.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "filePaths is required").into());
    }

    blocking::run("cli_sessions_session_delete", move || {
        let mut failed: Vec<String> = Vec::new();
        for fp in &file_paths {
            let fp = fp.trim().to_string();
            if fp.is_empty() {
                continue;
            }
            if let Err(e) = cli_sessions::session_delete(&app, source, &fp, wsl_distro.as_deref()) {
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
    let mut normalized: Vec<cli_sessions::CliSessionsFolderLookupKey> = Vec::new();
    for item in items {
        let source = item.source.parse::<cli_sessions::CliSessionsSource>()?;
        let session_id = item.session_id.trim().to_string();
        if session_id.is_empty() {
            continue;
        }
        normalized.push(cli_sessions::CliSessionsFolderLookupKey { source, session_id });
    }

    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    blocking::run("cli_sessions_folder_lookup_by_ids", move || {
        cli_sessions::folder_lookup_by_ids(&app, &normalized, wsl_distro.as_deref())
    })
    .await
    .map_err(Into::into)
}
