//! Usage: Claude Code session scanning/parsing from `~/.claude/projects/*/*.jsonl`.

use super::{
    folder_name_from_path, truncate_string, validate_path_under_root,
    CliSessionsDisplayContentBlock, CliSessionsDisplayMessage, CliSessionsFolderLookupEntry,
    CliSessionsPaginatedMessages, CliSessionsProjectSummary, CliSessionsSessionSummary,
};
use crate::shared::error::{AppError, AppResult};
use serde::Deserialize;
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

/// Types of records to skip during parsing (large/irrelevant).
const SKIP_TYPES: &[&str] = &["file-history-snapshot", "progress"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRecord {
    #[serde(rename = "type")]
    record_type: String,
    uuid: Option<String>,
    timestamp: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMessage {
    role: String,
    content: ContentValue,
    model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ContentValue {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Option<serde_json::Value>,
        #[serde(default)]
        is_error: Option<bool>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionsIndex {
    #[allow(dead_code)]
    version: Option<u32>,
    #[serde(default)]
    entries: Vec<SessionsIndexFileEntry>,
    original_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionsIndexFileEntry {
    session_id: String,
    full_path: Option<String>,
    first_prompt: Option<String>,
    message_count: Option<u32>,
    git_branch: Option<String>,
    project_path: Option<String>,
    is_sidechain: Option<bool>,
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

fn home_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

fn claude_projects_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(home_dir(app)?.join(".claude").join("projects"))
}

/// Decode a Claude-CLI encoded project directory name back to the original filesystem path.
///
/// Claude CLI encodes project paths by replacing `/` (or `\` on Windows) with `-`.
/// This encoding is **lossy**: a literal hyphen in a directory name becomes
/// indistinguishable from a path separator. We therefore try a filesystem-validated
/// greedy decode first and fall back to the naive replacement only when validation
/// is impossible (e.g. the path no longer exists on disk).
fn decode_project_path(encoded: &str) -> String {
    if cfg!(windows) {
        if encoded.len() >= 2 && encoded.chars().nth(1) == Some('-') {
            let drive = &encoded[0..1];
            let rest = &encoded[2..];
            let path_part = rest.replace('-', "\\");
            format!("{}:{}", drive, path_part)
        } else {
            encoded.replace('-', "\\")
        }
    } else {
        decode_unix_path_validated(encoded).unwrap_or_else(|| encoded.replace('-', "/"))
    }
}

/// Greedily reconstruct a Unix path from a `-`-encoded directory name by checking
/// the filesystem at each step to decide whether a `-` was originally a `/` or a
/// literal hyphen.
fn decode_unix_path_validated(encoded: &str) -> Option<String> {
    let stripped = encoded.strip_prefix('-').unwrap_or(encoded);
    let segments: Vec<&str> = stripped.split('-').collect();
    if segments.is_empty() {
        return None;
    }

    let mut resolved = String::new();
    let mut buf = String::new();

    for (i, seg) in segments.iter().enumerate() {
        if buf.is_empty() {
            buf.push_str(seg);
        } else {
            buf.push('-');
            buf.push_str(seg);
        }

        let is_last = i == segments.len() - 1;
        if is_last {
            resolved.push('/');
            resolved.push_str(&buf);
        } else {
            let candidate = format!("{}/{}", resolved, buf);
            if Path::new(&candidate).is_dir() {
                resolved = candidate;
                buf.clear();
            }
        }
    }

    if Path::new(&resolved).exists() {
        Some(resolved)
    } else {
        None
    }
}

/// Extract the project working directory from the first `user`-type message in any
/// JSONL session file within the given project directory.
fn extract_cwd_from_sessions(project_dir: &Path) -> Option<String> {
    let rd = fs::read_dir(project_dir).ok()?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Some(cwd) = extract_cwd_from_jsonl(&path) {
                return Some(cwd);
            }
        }
    }
    None
}

fn extract_cwd_from_jsonl(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(30) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("user") {
            continue;
        }
        if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str()) {
            let cwd = cwd.trim();
            if !cwd.is_empty() {
                return Some(cwd.to_string());
            }
        }
    }
    None
}

fn short_name_from_path(path: &str) -> String {
    let path = path.trim_end_matches(['/', '\\']);
    if let Some(pos) = path.rfind(['/', '\\']) {
        path[pos + 1..].to_string()
    } else {
        path.to_string()
    }
}

fn validate_project_id(project_id: &str) -> AppResult<()> {
    let trimmed = project_id.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "projectId is required"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("invalid projectId: {trimmed}"),
        ));
    }
    Ok(())
}

fn read_sessions_index(project_dir: &Path) -> Option<SessionsIndex> {
    let path = project_dir.join("sessions-index.json");
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice::<SessionsIndex>(&bytes).ok()
}

fn count_jsonl_files(project_dir: &Path) -> usize {
    fs::read_dir(project_dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "jsonl")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn convert_content(content: &ContentValue) -> Vec<CliSessionsDisplayContentBlock> {
    match content {
        ContentValue::Text(s) => {
            let t = s.trim();
            if t.is_empty() {
                Vec::new()
            } else {
                vec![CliSessionsDisplayContentBlock::Text {
                    text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                }]
            }
        }
        ContentValue::Blocks(blocks) => {
            let mut result = Vec::new();
            for block in blocks {
                match block {
                    ContentBlock::Text { text } => {
                        let t = text.trim();
                        if !t.is_empty() {
                            result.push(CliSessionsDisplayContentBlock::Text {
                                text: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                            });
                        }
                    }
                    ContentBlock::Thinking { thinking } => {
                        let t = thinking.trim();
                        if !t.is_empty() {
                            result.push(CliSessionsDisplayContentBlock::Thinking {
                                thinking: truncate_string(t, MAX_TEXT_BLOCK_SIZE),
                            });
                        }
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        let input_str = serde_json::to_string_pretty(input)
                            .unwrap_or_else(|_| input.to_string());
                        result.push(CliSessionsDisplayContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: truncate_string(&input_str, MAX_ARGS_SIZE),
                        });
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => {
                        let content_str = match content {
                            Some(v) => match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Array(arr) => {
                                    let mut parts = Vec::new();
                                    for item in arr {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            if !text.trim().is_empty() {
                                                parts.push(text.to_string());
                                            }
                                        }
                                    }
                                    parts.join("\n")
                                }
                                _ => serde_json::to_string_pretty(v)
                                    .unwrap_or_else(|_| v.to_string()),
                            },
                            None => String::new(),
                        };
                        result.push(CliSessionsDisplayContentBlock::ToolResult {
                            tool_use_id: tool_use_id.clone(),
                            content: truncate_string(&content_str, MAX_OUTPUT_BLOCK_SIZE),
                            is_error: is_error.unwrap_or(false),
                        });
                    }
                    ContentBlock::Unknown => {}
                }
            }
            result
        }
    }
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
        if SKIP_TYPES
            .iter()
            .any(|t| trimmed.contains(&format!("\"type\":\"{t}\"")))
        {
            continue;
        }

        let record: RawRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if record.record_type != "user" && record.record_type != "assistant" {
            continue;
        }
        let Some(msg) = record.message else {
            continue;
        };

        let blocks = convert_content(&msg.content);
        if blocks.is_empty() {
            continue;
        }

        messages.push(CliSessionsDisplayMessage {
            uuid: record.uuid,
            role: msg.role,
            timestamp: record.timestamp,
            model: msg.model,
            content: blocks,
        });
    }

    Ok(messages)
}

fn extract_first_prompt(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.ok()?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if SKIP_TYPES
            .iter()
            .any(|t| trimmed.contains(&format!("\"type\":\"{t}\"")))
        {
            continue;
        }

        let record: RawRecord = serde_json::from_str(trimmed).ok()?;
        if record.record_type != "user" {
            continue;
        }
        let msg = record.message?;
        if msg.role != "user" {
            continue;
        }

        match msg.content {
            ContentValue::Text(s) => {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(truncate_string(t, FIRST_PROMPT_MAX_LEN));
                }
            }
            ContentValue::Blocks(blocks) => {
                for block in blocks {
                    if let ContentBlock::Text { text } = block {
                        let t = text.trim();
                        if !t.is_empty() {
                            return Some(truncate_string(t, FIRST_PROMPT_MAX_LEN));
                        }
                    }
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
        if SKIP_TYPES
            .iter()
            .any(|t| trimmed.contains(&format!("\"type\":\"{t}\"")))
        {
            continue;
        }
        let record: RawRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if record.record_type != "user" && record.record_type != "assistant" {
            continue;
        }
        let Some(msg) = record.message else {
            continue;
        };
        if convert_content(&msg.content).is_empty() {
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
    let root = claude_projects_dir(app)?;
    let root = fs::canonicalize(&root).map_err(|e| {
        AppError::new(
            "SEC_INVALID_INPUT",
            format!("claude projects dir not found: {e}"),
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
    let projects_dir = claude_projects_dir(app)?;
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out: Vec<CliSessionsProjectSummary> = Vec::new();

    let entries = fs::read_dir(&projects_dir).map_err(|e| {
        AppError::new(
            "INTERNAL_ERROR",
            format!("failed to read claude projects dir: {e}"),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let encoded_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => continue,
        };

        let session_count = count_jsonl_files(&path);
        if session_count == 0 {
            continue;
        }

        let original_path = read_sessions_index(&path)
            .and_then(|idx| idx.original_path)
            .or_else(|| extract_cwd_from_sessions(&path))
            .unwrap_or_else(|| decode_project_path(&encoded_name));

        let short_name = short_name_from_path(&original_path);
        let (_, modified) = file_times(&path);

        out.push(CliSessionsProjectSummary {
            source: "claude".to_string(),
            id: encoded_name,
            display_path: original_path,
            short_name,
            session_count,
            last_modified: modified,
            model_provider: None,
            wsl_distro: None,
        });
    }

    out.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(out)
}

pub fn sessions_list(
    app: &tauri::AppHandle,
    project_id: &str,
) -> AppResult<Vec<CliSessionsSessionSummary>> {
    validate_project_id(project_id)?;

    let projects_dir = claude_projects_dir(app)?;
    let project_dir = projects_dir.join(project_id);
    if !project_dir.is_dir() {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("project not found: {project_id}"),
        ));
    }

    let index = read_sessions_index(&project_dir);
    let original_path = index.as_ref().and_then(|idx| idx.original_path.clone());
    let default_project_path = original_path
        .clone()
        .unwrap_or_else(|| decode_project_path(project_id));

    let mut disk_sessions: HashMap<String, PathBuf> = HashMap::new();
    if let Ok(rd) = fs::read_dir(&project_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.trim().is_empty() {
                        disk_sessions.insert(stem.to_string(), path);
                    }
                }
            }
        }
    }

    let mut out: Vec<CliSessionsSessionSummary> = Vec::new();
    let mut indexed: HashSet<String> = HashSet::new();

    if let Some(index) = index {
        for entry in index.entries {
            indexed.insert(entry.session_id.clone());
            let file_path = entry
                .full_path
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(|| project_dir.join(format!("{}.jsonl", entry.session_id)));
            // Validate path is within projects directory
            if validate_path_under_root(&file_path, &projects_dir).is_err() {
                continue;
            }

            if !file_path.exists() {
                continue;
            }

            let (created_at, modified_at) = file_times(&file_path);
            let first_prompt = entry
                .first_prompt
                .clone()
                .or_else(|| extract_first_prompt(&file_path));
            let message_count = entry
                .message_count
                .unwrap_or_else(|| count_messages(&file_path));

            out.push(CliSessionsSessionSummary {
                source: "claude".to_string(),
                session_id: entry.session_id,
                file_path: file_path.to_string_lossy().to_string(),
                first_prompt,
                message_count,
                created_at,
                modified_at,
                git_branch: entry.git_branch,
                project_path: entry
                    .project_path
                    .or_else(|| original_path.clone())
                    .or_else(|| Some(default_project_path.clone())),
                is_sidechain: entry.is_sidechain,
                cwd: None,
                model_provider: None,
                cli_version: None,
                wsl_distro: None,
            });
        }
    }

    for (session_id, file_path) in disk_sessions {
        if indexed.contains(&session_id) {
            continue;
        }
        // Validate path is within projects directory
        if validate_path_under_root(&file_path, &projects_dir).is_err() {
            continue;
        }
        let (created_at, modified_at) = file_times(&file_path);
        let first_prompt = extract_first_prompt(&file_path);
        let message_count = count_messages(&file_path);
        if message_count == 0 {
            continue;
        }

        out.push(CliSessionsSessionSummary {
            source: "claude".to_string(),
            session_id,
            file_path: file_path.to_string_lossy().to_string(),
            first_prompt,
            message_count,
            created_at,
            modified_at,
            git_branch: None,
            project_path: original_path
                .clone()
                .or_else(|| Some(default_project_path.clone())),
            is_sidechain: None,
            cwd: None,
            model_provider: None,
            cli_version: None,
            wsl_distro: None,
        });
    }

    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(out)
}

fn folder_lookup_in_projects_dir(
    projects_dir: &Path,
    decode_project_path: fn(&str) -> String,
    source: &str,
    target_session_ids: &[String],
) -> AppResult<Vec<CliSessionsFolderLookupEntry>> {
    if target_session_ids.is_empty() || !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut pending: HashSet<String> = target_session_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    if pending.is_empty() {
        return Ok(Vec::new());
    }

    let mut out: Vec<CliSessionsFolderLookupEntry> = Vec::new();
    let entries = fs::read_dir(projects_dir).map_err(|e| {
        AppError::new(
            "INTERNAL_ERROR",
            format!("failed to read claude projects dir: {e}"),
        )
    })?;

    for entry in entries.flatten() {
        if pending.is_empty() {
            break;
        }

        let project_dir = entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        let encoded_name = match project_dir.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => continue,
        };

        let index = read_sessions_index(&project_dir);
        let original_path = index
            .as_ref()
            .and_then(|idx| idx.original_path.clone())
            .or_else(|| extract_cwd_from_sessions(&project_dir))
            .unwrap_or_else(|| decode_project_path(&encoded_name));

        let default_project_path = if original_path.trim().is_empty() {
            decode_project_path(&encoded_name)
        } else {
            original_path.clone()
        };

        if let Some(index) = index {
            for item in index.entries {
                if !pending.contains(&item.session_id) {
                    continue;
                }
                let folder_path = item
                    .project_path
                    .or_else(|| Some(original_path.clone()))
                    .unwrap_or_else(|| default_project_path.clone());
                let Some(folder_name) = folder_name_from_path(&folder_path) else {
                    continue;
                };
                pending.remove(&item.session_id);
                out.push(CliSessionsFolderLookupEntry {
                    source: source.to_string(),
                    session_id: item.session_id,
                    folder_name,
                    folder_path,
                });
            }
        }

        if pending.is_empty() {
            break;
        }

        if let Ok(rd) = fs::read_dir(&project_dir) {
            for file in rd.flatten() {
                if pending.is_empty() {
                    break;
                }
                let file_path = file.path();
                if file_path
                    .extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
                {
                    let Some(session_id) = file_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                    else {
                        continue;
                    };
                    if !pending.contains(&session_id) {
                        continue;
                    }
                    let folder_path = default_project_path.clone();
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
            }
        }
    }

    Ok(out)
}

pub fn folder_lookup_by_session_ids(
    app: &tauri::AppHandle,
    target_session_ids: &[String],
) -> AppResult<Vec<CliSessionsFolderLookupEntry>> {
    let projects_dir = claude_projects_dir(app)?;
    folder_lookup_in_projects_dir(
        &projects_dir,
        decode_project_path,
        "claude",
        target_session_ids,
    )
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

/// Decode a WSL project directory name into a Unix path.
/// e.g. `home-diao-project` → `/home/diao/project`
fn decode_project_path_unix(encoded: &str) -> String {
    format!("/{}", encoded.replace('-', "/"))
}

fn wsl_claude_projects_dir(distro: &str) -> AppResult<PathBuf> {
    let home = crate::wsl::resolve_wsl_home_unc(distro)?;
    Ok(home.join(".claude").join("projects"))
}

pub fn wsl_projects_list(distro: &str) -> AppResult<Vec<CliSessionsProjectSummary>> {
    let projects_dir = wsl_claude_projects_dir(distro)?;
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out: Vec<CliSessionsProjectSummary> = Vec::new();

    let entries = fs::read_dir(&projects_dir).map_err(|e| {
        AppError::new(
            "INTERNAL_ERROR",
            format!("failed to read WSL claude projects dir: {e}"),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let encoded_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => continue,
        };

        let session_count = count_jsonl_files(&path);
        if session_count == 0 {
            continue;
        }

        let original_path = read_sessions_index(&path)
            .and_then(|idx| idx.original_path)
            .unwrap_or_else(|| decode_project_path_unix(&encoded_name));

        let short_name = short_name_from_path(&original_path);
        let (_, modified) = file_times(&path);

        out.push(CliSessionsProjectSummary {
            source: "claude".to_string(),
            id: encoded_name,
            display_path: original_path,
            short_name,
            session_count,
            last_modified: modified,
            model_provider: None,
            wsl_distro: Some(distro.to_string()),
        });
    }

    out.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(out)
}

pub fn wsl_sessions_list(
    distro: &str,
    project_id: &str,
) -> AppResult<Vec<CliSessionsSessionSummary>> {
    validate_project_id(project_id)?;

    let projects_dir = wsl_claude_projects_dir(distro)?;
    let project_dir = projects_dir.join(project_id);
    if !project_dir.is_dir() {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("project not found in WSL: {project_id}"),
        ));
    }

    let index = read_sessions_index(&project_dir);
    let original_path = index.as_ref().and_then(|idx| idx.original_path.clone());
    let default_project_path = original_path
        .clone()
        .unwrap_or_else(|| decode_project_path_unix(project_id));

    let mut disk_sessions: HashMap<String, PathBuf> = HashMap::new();
    if let Ok(rd) = fs::read_dir(&project_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.trim().is_empty() {
                        disk_sessions.insert(stem.to_string(), path);
                    }
                }
            }
        }
    }

    let mut out: Vec<CliSessionsSessionSummary> = Vec::new();
    let mut indexed: HashSet<String> = HashSet::new();
    let distro_opt = Some(distro.to_string());

    if let Some(index) = index {
        for entry in index.entries {
            indexed.insert(entry.session_id.clone());
            let file_path = entry
                .full_path
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(|| project_dir.join(format!("{}.jsonl", entry.session_id)));
            if validate_path_under_root(&file_path, &projects_dir).is_err() {
                continue;
            }
            if !file_path.exists() {
                continue;
            }

            let (created_at, modified_at) = file_times(&file_path);
            let first_prompt = entry
                .first_prompt
                .clone()
                .or_else(|| extract_first_prompt(&file_path));
            let message_count = entry
                .message_count
                .unwrap_or_else(|| count_messages(&file_path));

            out.push(CliSessionsSessionSummary {
                source: "claude".to_string(),
                session_id: entry.session_id,
                file_path: file_path.to_string_lossy().to_string(),
                first_prompt,
                message_count,
                created_at,
                modified_at,
                git_branch: entry.git_branch,
                project_path: entry
                    .project_path
                    .or_else(|| original_path.clone())
                    .or_else(|| Some(default_project_path.clone())),
                is_sidechain: entry.is_sidechain,
                cwd: None,
                model_provider: None,
                cli_version: None,
                wsl_distro: distro_opt.clone(),
            });
        }
    }

    for (session_id, file_path) in disk_sessions {
        if indexed.contains(&session_id) {
            continue;
        }
        if validate_path_under_root(&file_path, &projects_dir).is_err() {
            continue;
        }
        let (created_at, modified_at) = file_times(&file_path);
        let first_prompt = extract_first_prompt(&file_path);
        let message_count = count_messages(&file_path);
        if message_count == 0 {
            continue;
        }

        out.push(CliSessionsSessionSummary {
            source: "claude".to_string(),
            session_id,
            file_path: file_path.to_string_lossy().to_string(),
            first_prompt,
            message_count,
            created_at,
            modified_at,
            git_branch: None,
            project_path: original_path
                .clone()
                .or_else(|| Some(default_project_path.clone())),
            is_sidechain: None,
            cwd: None,
            model_provider: None,
            cli_version: None,
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
    let projects_dir = wsl_claude_projects_dir(distro)?;
    folder_lookup_in_projects_dir(
        &projects_dir,
        decode_project_path_unix,
        "claude",
        target_session_ids,
    )
}

pub fn wsl_messages_get(
    distro: &str,
    file_path: &str,
    page: usize,
    page_size: usize,
    from_end: bool,
) -> AppResult<CliSessionsPaginatedMessages> {
    let root = wsl_claude_projects_dir(distro)?;
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
    let root = wsl_claude_projects_dir(distro)?;
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
    use super::folder_lookup_in_projects_dir;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn folder_lookup_uses_project_path_and_disk_fallback() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("users-demo-default-root");
        fs::create_dir_all(&project_dir).unwrap();

        fs::write(
            project_dir.join("sessions-index.json"),
            r#"{
  "originalPath": "/Users/demo/default-root",
  "entries": [
    {
      "sessionId": "claude-indexed",
      "projectPath": "/Users/demo/feature-a"
    }
  ]
}"#,
        )
        .unwrap();
        fs::write(project_dir.join("claude-disk.jsonl"), "").unwrap();

        let out = folder_lookup_in_projects_dir(
            dir.path(),
            |encoded| encoded.replace('-', "/"),
            "claude",
            &[
                "claude-indexed".to_string(),
                "claude-disk".to_string(),
                "missing".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(out.len(), 2);

        let indexed_entry = out
            .iter()
            .find(|item| item.session_id == "claude-indexed")
            .unwrap();
        assert_eq!(indexed_entry.folder_name, "feature-a");
        assert_eq!(indexed_entry.folder_path, "/Users/demo/feature-a");

        let disk_entry = out
            .iter()
            .find(|item| item.session_id == "claude-disk")
            .unwrap();
        assert_eq!(disk_entry.folder_name, "default-root");
        assert_eq!(disk_entry.folder_path, "/Users/demo/default-root");
    }
}
