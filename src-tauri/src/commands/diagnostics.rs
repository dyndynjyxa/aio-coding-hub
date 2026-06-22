//! Usage: Read-only diagnostics for memory and data-size investigations.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::shared::time::now_unix_seconds;
use crate::{app_paths, blocking, codex_paths, db};
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TOP_FILE_LIMIT: usize = 10;
const TOP_PROMPT_LIMIT: usize = 10;
const SESSION_SCAN_MAX_FILES: u64 = 100_000;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsFileStat {
    path: String,
    bytes: u64,
    modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsDbStats {
    path: String,
    exists: bool,
    db_bytes: u64,
    wal_bytes: u64,
    shm_bytes: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsPromptTopItem {
    id: i64,
    workspace_id: i64,
    cli_key: String,
    name: String,
    enabled: bool,
    content_len: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsPromptStats {
    count: i64,
    total_content_len: i64,
    max_content_len: i64,
    top_items: Vec<AppMemoryDiagnosticsPromptTopItem>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsCliSessionStats {
    source: String,
    root: String,
    exists: bool,
    file_count: u64,
    total_bytes: u64,
    max_file_bytes: u64,
    truncated: bool,
    top_files: Vec<AppMemoryDiagnosticsFileStat>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub(crate) struct AppMemoryDiagnosticsSnapshot {
    generated_at_unix: i64,
    app_data_dir: String,
    db: AppMemoryDiagnosticsDbStats,
    prompt_stats: AppMemoryDiagnosticsPromptStats,
    cli_sessions: Vec<AppMemoryDiagnosticsCliSessionStats>,
}

fn unix_seconds_from_system_time(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn file_modified_at(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(unix_seconds_from_system_time)
}

fn top_files_insert(
    top_files: &mut Vec<AppMemoryDiagnosticsFileStat>,
    item: AppMemoryDiagnosticsFileStat,
) {
    top_files.push(item);
    top_files.sort_by_key(|item| std::cmp::Reverse(item.bytes));
    if top_files.len() > TOP_FILE_LIMIT {
        top_files.truncate(TOP_FILE_LIMIT);
    }
}

fn scan_jsonl_tree(source: &str, root: PathBuf) -> AppMemoryDiagnosticsCliSessionStats {
    let root_string = root.to_string_lossy().to_string();
    if !root.exists() {
        return AppMemoryDiagnosticsCliSessionStats {
            source: source.to_string(),
            root: root_string,
            exists: false,
            file_count: 0,
            total_bytes: 0,
            max_file_bytes: 0,
            truncated: false,
            top_files: Vec::new(),
        };
    }

    let mut stack = vec![root.clone()];
    let mut file_count = 0_u64;
    let mut total_bytes = 0_u64;
    let mut max_file_bytes = 0_u64;
    let mut top_files = Vec::new();
    let mut truncated = false;

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if !file_type.is_file() || path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
                continue;
            }

            file_count = file_count.saturating_add(1);
            if file_count > SESSION_SCAN_MAX_FILES {
                truncated = true;
                break;
            }

            let bytes = file_size(&path);
            total_bytes = total_bytes.saturating_add(bytes);
            max_file_bytes = max_file_bytes.max(bytes);
            top_files_insert(
                &mut top_files,
                AppMemoryDiagnosticsFileStat {
                    path: path.to_string_lossy().to_string(),
                    bytes,
                    modified_at: file_modified_at(&path),
                },
            );
        }

        if truncated {
            break;
        }
    }

    AppMemoryDiagnosticsCliSessionStats {
        source: source.to_string(),
        root: root_string,
        exists: true,
        file_count,
        total_bytes,
        max_file_bytes,
        truncated,
        top_files,
    }
}

fn db_stats(path: PathBuf) -> AppMemoryDiagnosticsDbStats {
    let wal_path = PathBuf::from(format!("{}-wal", path.to_string_lossy()));
    let shm_path = PathBuf::from(format!("{}-shm", path.to_string_lossy()));
    AppMemoryDiagnosticsDbStats {
        path: path.to_string_lossy().to_string(),
        exists: path.exists(),
        db_bytes: file_size(&path),
        wal_bytes: file_size(&wal_path),
        shm_bytes: file_size(&shm_path),
    }
}

fn prompt_stats(db: &db::Db) -> crate::shared::error::AppResult<AppMemoryDiagnosticsPromptStats> {
    let conn = db.open_connection()?;
    let (count, total_content_len, max_content_len) = conn
        .query_row(
            r#"
SELECT
  COUNT(1),
  COALESCE(SUM(length(content)), 0),
  COALESCE(MAX(length(content)), 0)
FROM prompts
"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("failed to read prompt diagnostics: {e}"))?
        .unwrap_or((0, 0, 0));

    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  p.id,
  p.workspace_id,
  w.cli_key,
  p.name,
  p.enabled,
  length(p.content) AS content_len
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
ORDER BY content_len DESC, p.id DESC
LIMIT ?1
"#,
        )
        .map_err(|e| format!("failed to prepare prompt diagnostics top query: {e}"))?;

    let rows = stmt
        .query_map([TOP_PROMPT_LIMIT as i64], |row| {
            Ok(AppMemoryDiagnosticsPromptTopItem {
                id: row.get("id")?,
                workspace_id: row.get("workspace_id")?,
                cli_key: row.get("cli_key")?,
                name: row.get("name")?,
                enabled: row.get::<_, i64>("enabled")? != 0,
                content_len: row.get("content_len")?,
            })
        })
        .map_err(|e| format!("failed to read prompt diagnostics top items: {e}"))?;

    let mut top_items = Vec::new();
    for row in rows {
        top_items.push(row.map_err(|e| format!("failed to read prompt diagnostics row: {e}"))?);
    }

    Ok(AppMemoryDiagnosticsPromptStats {
        count,
        total_content_len,
        max_content_len,
        top_items,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn app_memory_diagnostics_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<AppMemoryDiagnosticsSnapshot, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("app_memory_diagnostics_get", move || {
        let app_data_dir = app_paths::app_data_dir(&app)?;
        let db_path = db::db_path(&app)?;
        let prompt_stats = prompt_stats(&db)?;

        let codex_sessions = codex_paths::codex_sessions_dir(&app)
            .map(|path| scan_jsonl_tree("codex", path))
            .unwrap_or_else(|err| AppMemoryDiagnosticsCliSessionStats {
                source: "codex".to_string(),
                root: format!("ERROR: {err}"),
                exists: false,
                file_count: 0,
                total_bytes: 0,
                max_file_bytes: 0,
                truncated: false,
                top_files: Vec::new(),
            });
        let claude_sessions = scan_jsonl_tree(
            "claude",
            app_paths::home_dir(&app)?.join(".claude").join("projects"),
        );

        Ok::<AppMemoryDiagnosticsSnapshot, crate::shared::error::AppError>(
            AppMemoryDiagnosticsSnapshot {
                generated_at_unix: now_unix_seconds(),
                app_data_dir: app_data_dir.to_string_lossy().to_string(),
                db: db_stats(db_path),
                prompt_stats,
                cli_sessions: vec![codex_sessions, claude_sessions],
            },
        )
    })
    .await
    .map_err(Into::into)
}
