//! Usage: Codex CLI session scanning/parsing from `$CODEX_HOME/sessions/**.jsonl` (or `~/.codex/sessions`).

use super::{
    folder_name_from_path, truncate_string, validate_path_under_root,
    CliSessionsDisplayContentBlock, CliSessionsDisplayMessage, CliSessionsFolderLookupEntry,
    CliSessionsPaginatedMessages, CliSessionsProjectSummary, CliSessionsSessionSummary,
};
use crate::shared::error::{AppError, AppResult};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FIRST_PROMPT_MAX_LEN: usize = 200;
const MAX_TEXT_BLOCK_SIZE: usize = 20_000;
const MAX_ARGS_SIZE: usize = 10_000;
const MAX_OUTPUT_BLOCK_SIZE: usize = 30_000;

#[derive(Debug, Clone)]
struct CodexSessionMeta {
    id: String,
    cwd: Option<String>,
    project_path: Option<String>,
    cli_version: Option<String>,
    model_provider: Option<String>,
    git_branch: Option<String>,
}

fn unix_seconds_from_system_time(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

fn file_times(path: &Path) -> (Option<i64>, Option<i64>) {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return (None, None),
    };
    let created = meta.created().ok().and_then(unix_seconds_from_system_time);
    let modified = meta.modified().ok().and_then(unix_seconds_from_system_time);
    (created, modified)
}

fn scan_all_session_files(app: &tauri::AppHandle) -> AppResult<Vec<PathBuf>> {
    let sessions_dir = crate::codex_paths::codex_sessions_dir(app)?;
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = Vec::new();
    let year_dirs = match fs::read_dir(&sessions_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(files),
    };

    for year in year_dirs.flatten() {
        let year_path = year.path();
        if !year_path.is_dir() {
            continue;
        }
        let month_dirs = match fs::read_dir(&year_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for month in month_dirs.flatten() {
            let month_path = month.path();
            if !month_path.is_dir() {
                continue;
            }
            let day_dirs = match fs::read_dir(&month_path) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for day in day_dirs.flatten() {
                let day_path = day.path();
                if !day_path.is_dir() {
                    continue;
                }
                let jsonl_files = match fs::read_dir(&day_path) {
                    Ok(rd) => rd,
                    Err(_) => continue,
                };
                for file in jsonl_files.flatten() {
                    let path = file.path();
                    if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        files.push(path);
                    }
                }
            }
        }
    }

    Ok(files)
}

fn short_name_from_path(path: &str) -> String {
    let path = path.trim_end_matches(['/', '\\']);
    if let Some(pos) = path.rfind(['/', '\\']) {
        path[pos + 1..].to_string()
    } else {
        path.to_string()
    }
}

fn extract_session_meta(path: &Path) -> Option<CodexSessionMeta> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(20) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let row: Value = serde_json::from_str(trimmed).ok()?;
        let row_type = row.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if row_type != "session_meta" {
            continue;
        }
        let payload = row.get("payload")?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cwd = payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let project_path = payload
            .get("project_path")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let cli_version = payload
            .get("cli_version")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let model_provider = payload
            .get("model_provider")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let git_branch = payload
            .get("git")
            .and_then(|g| g.get("branch"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        return Some(CodexSessionMeta {
            id,
            cwd,
            project_path,
            cli_version,
            model_provider,
            git_branch,
        });
    }
    None
}

fn extract_message_content(payload: &Value) -> Vec<CliSessionsDisplayContentBlock> {
    let Some(content) = payload.get("content") else {
        return Vec::new();
    };

    let mut blocks = Vec::new();
    if let Some(arr) = content.as_array() {
        for item in arr {
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match item_type {
                "input_text" | "text" | "output_text" => {
                    let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let t = text.trim();
                    if !t.is_empty() {
                        blocks.push(CliSessionsDisplayContentBlock::Text {
                            text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                        });
                    }
                }
                "refusal" => {
                    let text = item.get("refusal").and_then(|v| v.as_str()).unwrap_or("");
                    let t = text.trim();
                    if !t.is_empty() {
                        blocks.push(CliSessionsDisplayContentBlock::Text {
                            text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                        });
                    }
                }
                _ => {}
            }
        }
        return blocks;
    }

    if let Some(s) = content.as_str() {
        let t = s.trim();
        if !t.is_empty() {
            blocks.push(CliSessionsDisplayContentBlock::Text {
                text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
            });
        }
    }

    blocks
}

fn parse_all_messages(path: &Path) -> Result<Vec<CliSessionsDisplayMessage>, String> {
    let file = fs::File::open(path).map_err(|e| format!("failed to open file: {e}"))?;
    let reader = BufReader::new(file);
    let mut messages: Vec<CliSessionsDisplayMessage> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let row: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let row_type = row.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let timestamp = row
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let payload = match row.get("payload") {
            Some(p) => p,
            None => continue,
        };

        if row_type != "response_item" {
            continue;
        }

        let payload_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match payload_type {
            "message" => {
                let role = payload.get("role").and_then(|v| v.as_str()).unwrap_or("");
                if role == "developer" || role == "system" {
                    continue;
                }
                if role != "user" && role != "assistant" {
                    continue;
                }
                let blocks = extract_message_content(payload);
                if blocks.is_empty() {
                    continue;
                }
                messages.push(CliSessionsDisplayMessage {
                    uuid: None,
                    role: role.to_string(),
                    timestamp: timestamp.clone(),
                    model: None,
                    content: blocks,
                });
            }
            "function_call" => {
                let name = payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let arguments = payload
                    .get("arguments")
                    .map(|v| {
                        if let Some(s) = v.as_str() {
                            if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                                serde_json::to_string_pretty(&parsed)
                                    .unwrap_or_else(|_| s.to_string())
                            } else {
                                s.to_string()
                            }
                        } else {
                            serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
                        }
                    })
                    .unwrap_or_default();
                let call_id = payload
                    .get("call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                messages.push(CliSessionsDisplayMessage {
                    uuid: None,
                    role: "assistant".to_string(),
                    timestamp: timestamp.clone(),
                    model: None,
                    content: vec![CliSessionsDisplayContentBlock::FunctionCall {
                        name,
                        arguments: truncate_string(&arguments, MAX_ARGS_SIZE),
                        call_id,
                    }],
                });
            }
            "function_call_output" => {
                let call_id = payload
                    .get("call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let output = payload
                    .get("output")
                    .map(|v| {
                        if let Some(s) = v.as_str() {
                            s.to_string()
                        } else {
                            serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
                        }
                    })
                    .unwrap_or_default();

                messages.push(CliSessionsDisplayMessage {
                    uuid: None,
                    role: "tool".to_string(),
                    timestamp: timestamp.clone(),
                    model: None,
                    content: vec![CliSessionsDisplayContentBlock::FunctionCallOutput {
                        call_id,
                        output: truncate_string(&output, MAX_OUTPUT_BLOCK_SIZE),
                    }],
                });
            }
            "reasoning" => {
                let text = payload
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        payload
                            .get("summary")
                            .and_then(|s| s.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                                    .filter(|t| !t.trim().is_empty())
                                    .collect::<Vec<&str>>()
                                    .join("\n")
                            })
                    })
                    .unwrap_or_default();
                let t = text.trim();
                if t.is_empty() {
                    continue;
                }
                messages.push(CliSessionsDisplayMessage {
                    uuid: None,
                    role: "assistant".to_string(),
                    timestamp: timestamp.clone(),
                    model: None,
                    content: vec![CliSessionsDisplayContentBlock::Reasoning {
                        text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                    }],
                });
            }
            _ => {}
        }
    }

    Ok(messages)
}

fn extract_first_prompt(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.contains("\"type\"") || !trimmed.contains("response_item") {
            continue;
        }

        let row: Value = serde_json::from_str(trimmed).ok()?;
        let row_type = row.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if row_type != "response_item" {
            continue;
        }
        let payload = row.get("payload")?;
        let payload_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if payload_type != "message" {
            continue;
        }
        let role = payload.get("role").and_then(|v| v.as_str()).unwrap_or("");
        if role != "user" {
            continue;
        }

        let blocks = extract_message_content(payload);
        for block in blocks {
            if let CliSessionsDisplayContentBlock::Text { text } = block {
                let t = text.trim();
                if !t.is_empty() {
                    return Some(truncate_string(t, FIRST_PROMPT_MAX_LEN));
                }
            }
        }
    }
    None
}

fn count_messages(path: &Path) -> u32 {
    let Ok(file) = fs::File::open(path) else {
        return 0;
    };
    let reader = BufReader::new(file);
    let mut count: u32 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Fast pre-filter.
        if !(trimmed.contains("\"type\":\"response_item\"")
            || trimmed.contains("\"type\": \"response_item\""))
        {
            continue;
        }
        if !(trimmed.contains("\"type\":\"message\"") || trimmed.contains("\"type\": \"message\""))
        {
            continue;
        }
        if trimmed.contains("\"developer\"") || trimmed.contains("\"system\"") {
            continue;
        }
        count = count.saturating_add(1);
    }
    count
}

fn resolve_and_validate_session_file_path(
    app: &tauri::AppHandle,
    file_path: &str,
) -> AppResult<PathBuf> {
    let root = crate::codex_paths::codex_sessions_dir(app)?;
    let root = fs::canonicalize(&root).map_err(|e| {
        AppError::new(
            "SEC_INVALID_INPUT",
            format!("codex sessions dir not found: {e}"),
        )
    })?;

    let raw = PathBuf::from(file_path);
    if raw.extension().map(|e| e != "jsonl").unwrap_or(true) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            "filePath must be a .jsonl file",
        ));
    }

    let resolved = super::validate_path_under_root(&raw, &root)?;
    Ok(resolved)
}

pub fn projects_list(app: &tauri::AppHandle) -> AppResult<Vec<CliSessionsProjectSummary>> {
    let files = scan_all_session_files(app)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut map: HashMap<String, CliSessionsProjectSummary> = HashMap::new();

    for file_path in files {
        let meta = extract_session_meta(&file_path);
        let cwd = meta
            .as_ref()
            .and_then(|m| m.cwd.clone())
            .unwrap_or_default();
        if cwd.trim().is_empty() {
            continue;
        }

        let (_, modified) = file_times(&file_path);
        let entry = map
            .entry(cwd.clone())
            .or_insert_with(|| CliSessionsProjectSummary {
                source: "codex".to_string(),
                id: cwd.clone(),
                display_path: cwd.clone(),
                short_name: short_name_from_path(&cwd),
                session_count: 0,
                last_modified: None,
                model_provider: meta.as_ref().and_then(|m| m.model_provider.clone()),
                wsl_distro: None,
            });

        entry.session_count += 1;
        if let Some(m) = modified {
            if entry.last_modified.map(|v| m > v).unwrap_or(true) {
                entry.last_modified = Some(m);
            }
        }
        if entry.model_provider.is_none() {
            entry.model_provider = meta.as_ref().and_then(|m| m.model_provider.clone());
        }
    }

    let mut out: Vec<CliSessionsProjectSummary> = map.into_values().collect();
    out.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(out)
}

pub fn sessions_list(
    app: &tauri::AppHandle,
    project_id: &str,
) -> AppResult<Vec<CliSessionsSessionSummary>> {
    let cwd = project_id.trim();
    if cwd.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "projectId is required"));
    }

    let files = scan_all_session_files(app)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut out: Vec<CliSessionsSessionSummary> = Vec::new();

    for file_path in files {
        // Validate path is within sessions directory
        let sessions_dir = crate::codex_paths::codex_sessions_dir(app)?;
        if validate_path_under_root(&file_path, &sessions_dir).is_err() {
            continue;
        }
        let meta = extract_session_meta(&file_path);
        let matches = meta
            .as_ref()
            .and_then(|m| m.cwd.as_deref())
            .map(|v| v == cwd)
            .unwrap_or(false);
        if !matches {
            continue;
        }

        let session_id = meta
            .as_ref()
            .map(|m| m.id.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        let (created_at, modified_at) = file_times(&file_path);
        let first_prompt = extract_first_prompt(&file_path);
        let message_count = count_messages(&file_path);

        out.push(CliSessionsSessionSummary {
            source: "codex".to_string(),
            session_id,
            file_path: file_path.to_string_lossy().to_string(),
            first_prompt,
            message_count,
            created_at,
            modified_at,
            git_branch: meta.as_ref().and_then(|m| m.git_branch.clone()),
            project_path: Some(cwd.to_string()),
            is_sidechain: None,
            cwd: Some(cwd.to_string()),
            model_provider: meta.as_ref().and_then(|m| m.model_provider.clone()),
            cli_version: meta.as_ref().and_then(|m| m.cli_version.clone()),
            wsl_distro: None,
        });
    }

    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

fn folder_lookup_in_files(
    files: Vec<PathBuf>,
    sessions_dir: &Path,
    source: &str,
    target_session_ids: &[String],
) -> Vec<CliSessionsFolderLookupEntry> {
    if files.is_empty() || target_session_ids.is_empty() {
        return Vec::new();
    }

    let mut pending: HashSet<String> = target_session_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let mut out = Vec::new();

    for file_path in files {
        if pending.is_empty() {
            break;
        }
        if validate_path_under_root(&file_path, sessions_dir).is_err() {
            continue;
        }
        let meta = extract_session_meta(&file_path);
        let session_id = meta
            .as_ref()
            .map(|value| value.id.clone())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            });
        let Some(session_id) = session_id else {
            continue;
        };
        if !pending.contains(&session_id) {
            continue;
        }

        let folder_path = meta
            .as_ref()
            .and_then(|value| value.cwd.clone())
            .or_else(|| meta.as_ref().and_then(|value| value.project_path.clone()))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(folder_path) = folder_path else {
            continue;
        };
        let Some(folder_name) = folder_name_from_path(&folder_path) else {
            continue;
        };

        pending.remove(&session_id);
        out.push(CliSessionsFolderLookupEntry {
            source: source.to_string(),
            session_id,
            folder_name,
            folder_path,
        });
    }

    out
}

pub fn folder_lookup_by_session_ids(
    app: &tauri::AppHandle,
    target_session_ids: &[String],
) -> AppResult<Vec<CliSessionsFolderLookupEntry>> {
    let sessions_dir = crate::codex_paths::codex_sessions_dir(app)?;
    let files = scan_all_session_files(app)?;
    Ok(folder_lookup_in_files(
        files,
        &sessions_dir,
        "codex",
        target_session_ids,
    ))
}

pub fn messages_get(
    app: &tauri::AppHandle,
    file_path: &str,
    page: usize,
    page_size: usize,
    from_end: bool,
) -> AppResult<CliSessionsPaginatedMessages> {
    let resolved = resolve_and_validate_session_file_path(app, file_path)?;
    let messages = parse_all_messages(&resolved).map_err(AppError::from)?;

    let total = messages.len();
    let (start, end, has_more) = if from_end {
        let end = total.saturating_sub(page * page_size);
        let start = end.saturating_sub(page_size);
        let has_more = start > 0;
        (start, end, has_more)
    } else {
        let start = page * page_size;
        let end = (start + page_size).min(total);
        let has_more = end < total;
        (start, end, has_more)
    };

    let page_messages = if start < end && end <= total {
        messages[start..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(CliSessionsPaginatedMessages {
        messages: page_messages,
        total,
        page,
        page_size,
        has_more,
    })
}

pub fn session_delete(app: &tauri::AppHandle, file_path: &str) -> AppResult<bool> {
    let resolved = resolve_and_validate_session_file_path(app, file_path)?;
    fs::remove_file(&resolved).map_err(|e| {
        AppError::new(
            "INTERNAL_ERROR",
            format!("failed to delete session file: {e}"),
        )
    })?;
    Ok(true)
}

// ── WSL support ─────────────────────────────────────────────────────────────

fn wsl_codex_sessions_dir(distro: &str) -> AppResult<PathBuf> {
    let home = crate::wsl::resolve_wsl_home_unc(distro)?;
    Ok(home.join(".codex").join("sessions"))
}

fn wsl_scan_all_session_files(distro: &str) -> AppResult<Vec<PathBuf>> {
    let sessions_dir = wsl_codex_sessions_dir(distro)?;
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = Vec::new();
    let year_dirs = match fs::read_dir(&sessions_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(files),
    };

    for year in year_dirs.flatten() {
        let year_path = year.path();
        if !year_path.is_dir() {
            continue;
        }
        let month_dirs = match fs::read_dir(&year_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for month in month_dirs.flatten() {
            let month_path = month.path();
            if !month_path.is_dir() {
                continue;
            }
            let day_dirs = match fs::read_dir(&month_path) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for day in day_dirs.flatten() {
                let day_path = day.path();
                if !day_path.is_dir() {
                    continue;
                }
                let jsonl_files = match fs::read_dir(&day_path) {
                    Ok(rd) => rd,
                    Err(_) => continue,
                };
                for file in jsonl_files.flatten() {
                    let path = file.path();
                    if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                        files.push(path);
                    }
                }
            }
        }
    }

    Ok(files)
}

pub fn wsl_projects_list(distro: &str) -> AppResult<Vec<CliSessionsProjectSummary>> {
    let files = wsl_scan_all_session_files(distro)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut map: HashMap<String, CliSessionsProjectSummary> = HashMap::new();
    let distro_opt = Some(distro.to_string());

    for file_path in files {
        let meta = extract_session_meta(&file_path);
        let cwd = meta
            .as_ref()
            .and_then(|m| m.cwd.clone())
            .unwrap_or_default();
        if cwd.trim().is_empty() {
            continue;
        }

        let (_, modified) = file_times(&file_path);
        let entry = map
            .entry(cwd.clone())
            .or_insert_with(|| CliSessionsProjectSummary {
                source: "codex".to_string(),
                id: cwd.clone(),
                display_path: cwd.clone(),
                short_name: short_name_from_path(&cwd),
                session_count: 0,
                last_modified: None,
                model_provider: meta.as_ref().and_then(|m| m.model_provider.clone()),
                wsl_distro: distro_opt.clone(),
            });

        entry.session_count += 1;
        if let Some(m) = modified {
            if entry.last_modified.map(|v| m > v).unwrap_or(true) {
                entry.last_modified = Some(m);
            }
        }
        if entry.model_provider.is_none() {
            entry.model_provider = meta.as_ref().and_then(|m| m.model_provider.clone());
        }
    }

    let mut out: Vec<CliSessionsProjectSummary> = map.into_values().collect();
    out.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(out)
}

pub fn wsl_sessions_list(
    distro: &str,
    project_id: &str,
) -> AppResult<Vec<CliSessionsSessionSummary>> {
    let cwd = project_id.trim();
    if cwd.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "projectId is required"));
    }

    let files = wsl_scan_all_session_files(distro)?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let sessions_dir = wsl_codex_sessions_dir(distro)?;
    let distro_opt = Some(distro.to_string());
    let mut out: Vec<CliSessionsSessionSummary> = Vec::new();

    for file_path in files {
        if validate_path_under_root(&file_path, &sessions_dir).is_err() {
            continue;
        }
        let meta = extract_session_meta(&file_path);
        let matches = meta
            .as_ref()
            .and_then(|m| m.cwd.as_deref())
            .map(|v| v == cwd)
            .unwrap_or(false);
        if !matches {
            continue;
        }

        let session_id = meta
            .as_ref()
            .map(|m| m.id.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        let (created_at, modified_at) = file_times(&file_path);
        let first_prompt = extract_first_prompt(&file_path);
        let message_count = count_messages(&file_path);

        out.push(CliSessionsSessionSummary {
            source: "codex".to_string(),
            session_id,
            file_path: file_path.to_string_lossy().to_string(),
            first_prompt,
            message_count,
            created_at,
            modified_at,
            git_branch: meta.as_ref().and_then(|m| m.git_branch.clone()),
            project_path: Some(cwd.to_string()),
            is_sidechain: None,
            cwd: Some(cwd.to_string()),
            model_provider: meta.as_ref().and_then(|m| m.model_provider.clone()),
            cli_version: meta.as_ref().and_then(|m| m.cli_version.clone()),
            wsl_distro: distro_opt.clone(),
        });
    }

    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

pub fn wsl_folder_lookup_by_session_ids(
    distro: &str,
    target_session_ids: &[String],
) -> AppResult<Vec<CliSessionsFolderLookupEntry>> {
    let sessions_dir = wsl_codex_sessions_dir(distro)?;
    let files = wsl_scan_all_session_files(distro)?;
    Ok(folder_lookup_in_files(
        files,
        &sessions_dir,
        "codex",
        target_session_ids,
    ))
}

pub fn wsl_messages_get(
    distro: &str,
    file_path: &str,
    page: usize,
    page_size: usize,
    from_end: bool,
) -> AppResult<CliSessionsPaginatedMessages> {
    let root = wsl_codex_sessions_dir(distro)?;
    let raw = PathBuf::from(file_path);
    if raw.extension().map(|e| e != "jsonl").unwrap_or(true) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            "filePath must be a .jsonl file",
        ));
    }

    let resolved = super::validate_path_under_root(&raw, &root)?;
    let messages = parse_all_messages(&resolved).map_err(AppError::from)?;

    let total = messages.len();
    let (start, end, has_more) = if from_end {
        let end = total.saturating_sub(page * page_size);
        let start = end.saturating_sub(page_size);
        let has_more = start > 0;
        (start, end, has_more)
    } else {
        let start = page * page_size;
        let end = (start + page_size).min(total);
        let has_more = end < total;
        (start, end, has_more)
    };

    let page_messages = if start < end && end <= total {
        messages[start..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(CliSessionsPaginatedMessages {
        messages: page_messages,
        total,
        page,
        page_size,
        has_more,
    })
}

pub fn wsl_session_delete(distro: &str, file_path: &str) -> AppResult<bool> {
    let root = wsl_codex_sessions_dir(distro)?;
    let raw = PathBuf::from(file_path);
    if raw.extension().map(|e| e != "jsonl").unwrap_or(true) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            "filePath must be a .jsonl file",
        ));
    }
    let resolved = super::validate_path_under_root(&raw, &root)?;
    fs::remove_file(&resolved).map_err(|e| {
        AppError::new(
            "INTERNAL_ERROR",
            format!("failed to delete WSL session file: {e}"),
        )
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::folder_lookup_in_files;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn folder_lookup_prefers_cwd_and_falls_back_to_project_path() {
        let dir = tempdir().unwrap();
        let sessions_dir = dir.path();
        let day_dir = sessions_dir.join("2026").join("04").join("06");
        fs::create_dir_all(&day_dir).unwrap();

        let cwd_file = day_dir.join("session-cwd.jsonl");
        fs::write(
            &cwd_file,
            r#"{"type":"session_meta","payload":{"id":"codex-cwd","cwd":"/Users/demo/worktrees/current","project_path":"/Users/demo/worktrees/fallback"}}"#,
        )
        .unwrap();

        let fallback_file = day_dir.join("session-project.jsonl");
        fs::write(
            &fallback_file,
            r#"{"type":"session_meta","payload":{"id":"codex-project","project_path":"/Users/demo/projects/fallback-only"}}"#,
        )
        .unwrap();

        let out = folder_lookup_in_files(
            vec![cwd_file, fallback_file],
            sessions_dir,
            "codex",
            &[
                "codex-cwd".to_string(),
                "codex-project".to_string(),
                "missing".to_string(),
            ],
        );

        assert_eq!(out.len(), 2);

        let cwd_entry = out
            .iter()
            .find(|item| item.session_id == "codex-cwd")
            .unwrap();
        assert_eq!(cwd_entry.folder_name, "current");
        assert_eq!(cwd_entry.folder_path, "/Users/demo/worktrees/current");

        let fallback_entry = out
            .iter()
            .find(|item| item.session_id == "codex-project")
            .unwrap();
        assert_eq!(fallback_entry.folder_name, "fallback-only");
        assert_eq!(
            fallback_entry.folder_path,
            "/Users/demo/projects/fallback-only"
        );
    }
}
