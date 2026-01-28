//! Usage: Read / patch Codex user-level `config.toml` ($CODEX_HOME/config.toml).

use crate::codex_paths;
use crate::shared::fs::{read_optional_file, write_file_atomic_if_changed};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct CodexConfigState {
    pub config_dir: String,
    pub config_path: String,
    pub can_open_config_dir: bool,
    pub exists: bool,

    pub model: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub file_opener: Option<String>,
    pub hide_agent_reasoning: Option<bool>,
    pub show_raw_agent_reasoning: Option<bool>,

    pub history_persistence: Option<String>,
    pub history_max_bytes: Option<u64>,

    pub sandbox_workspace_write_network_access: Option<bool>,

    pub tui_animations: Option<bool>,
    pub tui_alternate_screen: Option<String>,
    pub tui_show_tooltips: Option<bool>,
    pub tui_scroll_invert: Option<bool>,

    pub features_unified_exec: Option<bool>,
    pub features_shell_snapshot: Option<bool>,
    pub features_apply_patch_freeform: Option<bool>,
    pub features_web_search_request: Option<bool>,
    pub features_shell_tool: Option<bool>,
    pub features_exec_policy: Option<bool>,
    pub features_experimental_windows_sandbox: Option<bool>,
    pub features_elevated_windows_sandbox: Option<bool>,
    pub features_remote_compaction: Option<bool>,
    pub features_remote_models: Option<bool>,
    pub features_powershell_utf8: Option<bool>,
    pub features_child_agents_md: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexConfigPatch {
    pub model: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox_mode: Option<String>,
    pub model_reasoning_effort: Option<String>,
    pub file_opener: Option<String>,
    pub hide_agent_reasoning: Option<bool>,
    pub show_raw_agent_reasoning: Option<bool>,

    pub history_persistence: Option<String>,
    pub history_max_bytes: Option<u64>,

    pub sandbox_workspace_write_network_access: Option<bool>,

    pub tui_animations: Option<bool>,
    pub tui_alternate_screen: Option<String>,
    pub tui_show_tooltips: Option<bool>,
    pub tui_scroll_invert: Option<bool>,

    pub features_unified_exec: Option<bool>,
    pub features_shell_snapshot: Option<bool>,
    pub features_apply_patch_freeform: Option<bool>,
    pub features_web_search_request: Option<bool>,
    pub features_shell_tool: Option<bool>,
    pub features_exec_policy: Option<bool>,
    pub features_experimental_windows_sandbox: Option<bool>,
    pub features_elevated_windows_sandbox: Option<bool>,
    pub features_remote_compaction: Option<bool>,
    pub features_remote_models: Option<bool>,
    pub features_powershell_utf8: Option<bool>,
    pub features_child_agents_md: Option<bool>,
}

fn is_symlink(path: &Path) -> Result<bool, String> {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if in_double {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '#' => return &line[..idx],
            _ => {}
        }
    }

    line
}

fn parse_table_header(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    if trimmed.starts_with("[[") {
        return None;
    }

    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();

    if inner.is_empty() {
        return None;
    }

    Some(inner.to_string())
}

fn parse_assignment(trimmed: &str) -> Option<(String, String)> {
    let (k, v) = trimmed.split_once('=')?;
    let key = k.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), v.trim().to_string()))
}

fn toml_unquote_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return None;
    }
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return Some(trimmed[1..trimmed.len() - 1].to_string());
    }
    None
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_u64(value: &str) -> Option<u64> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    raw.parse::<u64>().ok()
}

fn parse_string(value: &str) -> Option<String> {
    toml_unquote_string(value).or_else(|| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_key(raw: &str) -> String {
    let trimmed = raw.trim();
    toml_unquote_string(trimmed).unwrap_or_else(|| trimmed.to_string())
}

fn key_table_and_name(current_table: Option<&str>, key: &str) -> (Option<String>, String) {
    if let Some((t, k)) = key.split_once('.') {
        let t = normalize_key(t);
        let k = normalize_key(k);
        if !t.is_empty() && !k.is_empty() && !k.contains('.') {
            return (Some(t), k);
        }
    }

    let k = normalize_key(key);
    let table = current_table.map(|t| t.to_string());
    (table, k)
}

fn is_allowed_value(value: &str, allowed: &[&str]) -> bool {
    allowed.iter().any(|v| v.eq_ignore_ascii_case(value))
}

fn validate_enum_or_empty(key: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if is_allowed_value(trimmed, allowed) {
        return Ok(());
    }
    Err(format!(
        "SEC_INVALID_INPUT: invalid {key}={trimmed} (allowed: {})",
        allowed.join(", ")
    ))
}

fn toml_escape_basic_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                out.push_str(&format!("\\u{:04X}", code));
            }
            c => out.push(c),
        }
    }
    out
}

fn toml_string_literal(value: &str) -> String {
    format!("\"{}\"", toml_escape_basic_string(value))
}

fn first_table_header_line(lines: &[String]) -> usize {
    let mut in_multiline_double = false;
    let mut in_multiline_single = false;

    for (idx, line) in lines.iter().enumerate() {
        if !in_multiline_double && !in_multiline_single && is_any_table_header_line(line) {
            return idx;
        }

        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    lines.len()
}

fn upsert_root_key(lines: &mut Vec<String>, key: &str, value: Option<String>) {
    let first_table = first_table_header_line(lines);

    let mut target_idx: Option<usize> = None;
    let mut in_multiline_double = false;
    let mut in_multiline_single = false;
    for (idx, line) in lines.iter().take(first_table).enumerate() {
        if in_multiline_double || in_multiline_single {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }

        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }
        let Some((k, _)) = parse_assignment(cleaned) else {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        };
        if normalize_key(&k) == key {
            target_idx = Some(idx);
            break;
        }

        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    match (target_idx, value) {
        (Some(idx), Some(v)) => {
            lines[idx] = format!("{key} = {v}");
        }
        (Some(idx), None) => {
            lines.remove(idx);
        }
        (None, Some(v)) => {
            let mut insert_at = 0;
            while insert_at < first_table {
                let trimmed = lines[insert_at].trim_start();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    insert_at += 1;
                    continue;
                }
                break;
            }
            lines.insert(insert_at, format!("{key} = {v}"));
            if insert_at + 1 < lines.len() && !lines[insert_at + 1].trim().is_empty() {
                lines.insert(insert_at + 1, String::new());
            }
        }
        (None, None) => {}
    }
}

fn root_key_exists(lines: &[String], key: &str) -> bool {
    let first_table = first_table_header_line(lines);

    let mut in_multiline_double = false;
    let mut in_multiline_single = false;
    for line in lines.iter().take(first_table) {
        if in_multiline_double || in_multiline_single {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }

        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }
        let Some((k, _)) = parse_assignment(cleaned) else {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        };
        if normalize_key(&k) == key {
            return true;
        }

        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    false
}

fn find_table_block(lines: &[String], table_header: &str) -> Option<(usize, usize)> {
    let mut start: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == table_header {
            start = Some(idx);
            break;
        }
    }
    let start = start?;
    let end = lines[start.saturating_add(1)..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    Some((start, end))
}

fn upsert_table_keys(lines: &mut Vec<String>, table: &str, items: Vec<(&str, Option<String>)>) {
    let header = format!("[{table}]");
    let has_any_value = items.iter().any(|(_, v)| v.is_some());

    if find_table_block(lines, &header).is_none() {
        if !has_any_value {
            return;
        }
        if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
            lines.push(String::new());
        }
        lines.push(header.clone());
    }

    for (key, value) in items {
        let Some((start, end)) = find_table_block(lines, &header) else {
            return;
        };

        let mut found_idx: Option<usize> = None;
        for (idx, line) in lines
            .iter()
            .enumerate()
            .take(end.min(lines.len()))
            .skip(start + 1)
        {
            let cleaned = strip_toml_comment(line).trim();
            if cleaned.is_empty() || cleaned.starts_with('#') {
                continue;
            }
            let Some((k, _)) = parse_assignment(cleaned) else {
                continue;
            };
            if normalize_key(&k) == key {
                found_idx = Some(idx);
                break;
            }
        }

        match (found_idx, value) {
            (Some(idx), Some(v)) => lines[idx] = format!("{key} = {v}"),
            (Some(idx), None) => {
                lines.remove(idx);
            }
            (None, Some(v)) => {
                let mut insert_at = end.min(lines.len());
                while insert_at > start + 1 && lines[insert_at - 1].trim().is_empty() {
                    insert_at -= 1;
                }
                lines.insert(insert_at, format!("{key} = {v}"));
            }
            (None, None) => {}
        }
    }

    // Normalize: remove blank lines inside the table, and keep a single blank line
    // separating it from the next table (if any).
    if let Some((start, end)) = find_table_block(lines, &header) {
        let has_next_table = end < lines.len();

        let mut body_end = end;
        while body_end > start + 1 && lines[body_end - 1].trim().is_empty() {
            body_end -= 1;
        }

        let mut replacement: Vec<String> = lines[start + 1..body_end]
            .iter()
            .filter(|line| !line.trim().is_empty())
            .cloned()
            .collect();

        if has_next_table {
            replacement.push(String::new());
        }

        lines.splice(start + 1..end, replacement);
    }

    // If the table becomes empty after applying the patch, drop the table header too.
    // This keeps config.toml clean when the only managed key is removed.
    if let Some((start, end)) = find_table_block(lines, &header) {
        let has_body_content = lines[start + 1..end]
            .iter()
            .any(|line| !line.trim().is_empty());
        if !has_body_content {
            lines.drain(start..end);
        }
    }
}

fn upsert_dotted_keys(lines: &mut Vec<String>, table: &str, items: Vec<(&str, Option<String>)>) {
    let first_table = first_table_header_line(lines);

    for (key, value) in items {
        let full_key = format!("{table}.{key}");
        let mut found_idx: Option<usize> = None;
        for (idx, line) in lines.iter().enumerate() {
            let cleaned = strip_toml_comment(line).trim();
            if cleaned.is_empty() || cleaned.starts_with('#') {
                continue;
            }
            let Some((k, _)) = parse_assignment(cleaned) else {
                continue;
            };
            if normalize_key(&k) == full_key {
                found_idx = Some(idx);
                break;
            }
        }

        match (found_idx, value) {
            (Some(idx), Some(v)) => lines[idx] = format!("{full_key} = {v}"),
            (Some(idx), None) => {
                lines.remove(idx);
            }
            (None, Some(v)) => {
                let mut insert_at = 0;
                while insert_at < first_table {
                    let trimmed = lines[insert_at].trim_start();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        insert_at += 1;
                        continue;
                    }
                    break;
                }
                lines.insert(insert_at, format!("{full_key} = {v}"));
                if insert_at + 1 < lines.len() && !lines[insert_at + 1].trim().is_empty() {
                    lines.insert(insert_at + 1, String::new());
                }
            }
            (None, None) => {}
        }
    }
}

fn remove_dotted_keys(lines: &mut Vec<String>, table: &str, keys: &[&str]) {
    let mut to_remove: Vec<usize> = Vec::new();
    let target_prefix = format!("{table}.");

    for (idx, line) in lines.iter().enumerate() {
        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            continue;
        }
        let Some((k, _)) = parse_assignment(cleaned) else {
            continue;
        };
        let key = normalize_key(&k);
        if !key.starts_with(&target_prefix) {
            continue;
        }
        let Some((_t, suffix)) = key.split_once('.') else {
            continue;
        };
        if keys.iter().any(|wanted| wanted == &suffix) {
            to_remove.push(idx);
        }
    }

    to_remove.sort_unstable();
    to_remove.dedup();
    for idx in to_remove.into_iter().rev() {
        lines.remove(idx);
    }
}

enum TableStyle {
    Table,
    Dotted,
}

const FEATURES_KEY_ORDER: [&str; 12] = [
    // Keep in sync with the UI order (CliManagerCodexTab / Features section).
    "shell_snapshot",
    "web_search_request",
    "unified_exec",
    "shell_tool",
    "exec_policy",
    "apply_patch_freeform",
    "remote_compaction",
    "remote_models",
    "powershell_utf8",
    "child_agents_md",
    "experimental_windows_sandbox",
    "elevated_windows_sandbox",
];

fn table_style(lines: &[String], table: &str) -> TableStyle {
    let header = format!("[{table}]");
    if lines.iter().any(|l| l.trim() == header) {
        return TableStyle::Table;
    }

    let prefix = format!("{table}.");
    if lines.iter().any(|l| {
        let cleaned = strip_toml_comment(l).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            return false;
        }
        let Some((k, _)) = parse_assignment(cleaned) else {
            return false;
        };
        normalize_key(&k).starts_with(&prefix)
    }) {
        return TableStyle::Dotted;
    }

    TableStyle::Table
}

fn has_table_or_dotted_keys(lines: &[String], table: &str) -> bool {
    let header = format!("[{table}]");

    let prefix = format!("{table}.");
    let mut in_multiline_double = false;
    let mut in_multiline_single = false;
    for line in lines {
        if in_multiline_double || in_multiline_single {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }

        if line.trim() == header {
            return true;
        }

        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
            continue;
        }

        if let Some((k, _)) = parse_assignment(cleaned) {
            if normalize_key(&k).starts_with(&prefix) {
                return true;
            }
        }

        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    false
}

/// Unified upsert that auto-detects and applies the appropriate table style.
fn upsert_keys_auto_style(
    lines: &mut Vec<String>,
    table: &str,
    dotted_keys: &[&str],
    items: Vec<(&str, Option<String>)>,
) {
    match table_style(lines, table) {
        TableStyle::Table => {
            remove_dotted_keys(lines, table, dotted_keys);
            upsert_table_keys(lines, table, items);
        }
        TableStyle::Dotted => {
            upsert_dotted_keys(lines, table, items);
        }
    }
}

fn is_any_table_header_line(line: &str) -> bool {
    let cleaned = strip_toml_comment(line).trim();
    cleaned.starts_with('[') && cleaned.ends_with(']') && !cleaned.is_empty()
}

fn update_multiline_string_state(
    line: &str,
    in_multiline_double: &mut bool,
    in_multiline_single: &mut bool,
) {
    let mut idx = 0usize;

    while idx < line.len() {
        if *in_multiline_double {
            if let Some(pos) = line[idx..].find("\"\"\"") {
                *in_multiline_double = false;
                idx += pos + 3;
                continue;
            }
            break;
        }

        if *in_multiline_single {
            if let Some(pos) = line[idx..].find("'''") {
                *in_multiline_single = false;
                idx += pos + 3;
                continue;
            }
            break;
        }

        let next_double = line[idx..].find("\"\"\"");
        let next_single = line[idx..].find("'''");
        match (next_double, next_single) {
            (None, None) => break,
            (Some(d), None) => {
                *in_multiline_double = true;
                idx += d + 3;
            }
            (None, Some(s)) => {
                *in_multiline_single = true;
                idx += s + 3;
            }
            (Some(d), Some(s)) => {
                if d <= s {
                    *in_multiline_double = true;
                    idx += d + 3;
                } else {
                    *in_multiline_single = true;
                    idx += s + 3;
                }
            }
        }
    }
}

fn normalize_table_body_remove_blank_lines(body: &mut Vec<String>) {
    let mut in_multiline_double = false;
    let mut in_multiline_single = false;

    let mut out: Vec<String> = Vec::new();
    for line in body.iter() {
        if line.trim().is_empty() && !in_multiline_double && !in_multiline_single {
            continue;
        }
        out.push(line.clone());
        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    *body = out;
}

fn normalize_features_table_body_order(body: &mut Vec<String>, key_order: &[&str]) {
    #[derive(Debug)]
    struct Chunk {
        key: Option<String>,
        lines: Vec<String>,
    }

    let mut pending_comments: Vec<String> = Vec::new();
    let mut chunks: Vec<Chunk> = Vec::new();

    for line in body.iter() {
        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() {
            continue;
        }
        if cleaned.starts_with('#') {
            pending_comments.push(line.clone());
            continue;
        }

        let key = parse_assignment(cleaned).map(|(k, _)| normalize_key(&k));

        let mut lines: Vec<String> = Vec::new();
        lines.append(&mut pending_comments);
        lines.push(line.clone());
        chunks.push(Chunk { key, lines });
    }

    if !pending_comments.is_empty() {
        chunks.push(Chunk {
            key: None,
            lines: pending_comments,
        });
    }

    let mut consumed: Vec<bool> = vec![false; chunks.len()];
    let mut out: Vec<String> = Vec::new();

    for wanted in key_order {
        for (idx, chunk) in chunks.iter().enumerate() {
            if consumed[idx] {
                continue;
            }
            if chunk.key.as_deref() == Some(*wanted) {
                out.extend(chunk.lines.iter().cloned());
                consumed[idx] = true;
            }
        }
    }

    for (idx, chunk) in chunks.into_iter().enumerate() {
        if !consumed[idx] {
            out.extend(chunk.lines);
        }
    }

    *body = out;
}

fn normalize_toml_layout(lines: &mut Vec<String>) {
    struct Segment {
        header: Option<String>,
        body: Vec<String>,
    }

    let mut segments: Vec<Segment> = vec![Segment {
        header: None,
        body: Vec::new(),
    }];

    let mut in_multiline_double = false;
    let mut in_multiline_single = false;

    for line in lines.iter() {
        let is_header =
            !in_multiline_double && !in_multiline_single && is_any_table_header_line(line);

        if is_header {
            segments.push(Segment {
                header: Some(line.clone()),
                body: Vec::new(),
            });
        } else {
            segments
                .last_mut()
                .expect("at least one segment")
                .body
                .push(line.clone());
        }

        update_multiline_string_state(line, &mut in_multiline_double, &mut in_multiline_single);
    }

    for seg in segments.iter_mut() {
        normalize_table_body_remove_blank_lines(&mut seg.body);
        if let Some(header_line) = seg.header.as_deref() {
            if strip_toml_comment(header_line).trim() == "[features]" {
                normalize_features_table_body_order(&mut seg.body, &FEATURES_KEY_ORDER);
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    for seg in segments {
        let mut seg_lines: Vec<String> = Vec::new();
        if let Some(header) = seg.header {
            seg_lines.push(header);
        }
        seg_lines.extend(seg.body);

        if seg_lines.is_empty() {
            continue;
        }

        if !out.is_empty() && !out.last().unwrap_or(&String::new()).trim().is_empty() {
            out.push(String::new());
        }
        while out.len() >= 2
            && out.last().unwrap_or(&String::new()).trim().is_empty()
            && out[out.len() - 2].trim().is_empty()
        {
            out.pop();
        }

        out.extend(seg_lines);
    }

    let first_non_empty = out
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(out.len());
    out.drain(0..first_non_empty);

    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }

    *lines = out;
}

/// Helper to build optional string value from Option<String>, trimming and filtering empty.
fn opt_string_value(raw: Option<&str>) -> Option<String> {
    raw.map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(toml_string_literal)
}

/// Helper to build optional u64 value, treating 0 as None (remove from config).
fn opt_u64_value(value: Option<u64>) -> Option<String> {
    value
        .and_then(|n| if n == 0 { None } else { Some(n) })
        .map(|n| n.to_string())
}

fn make_state_from_bytes(
    config_dir: String,
    config_path: String,
    can_open_config_dir: bool,
    bytes: Option<Vec<u8>>,
) -> Result<CodexConfigState, String> {
    let exists = bytes.is_some();
    let mut state = CodexConfigState {
        config_dir,
        config_path,
        can_open_config_dir,
        exists,

        model: None,
        approval_policy: None,
        sandbox_mode: None,
        model_reasoning_effort: None,
        file_opener: None,
        hide_agent_reasoning: None,
        show_raw_agent_reasoning: None,

        history_persistence: None,
        history_max_bytes: None,

        sandbox_workspace_write_network_access: None,

        tui_animations: None,
        tui_alternate_screen: None,
        tui_show_tooltips: None,
        tui_scroll_invert: None,

        features_unified_exec: None,
        features_shell_snapshot: None,
        features_apply_patch_freeform: None,
        features_web_search_request: None,
        features_shell_tool: None,
        features_exec_policy: None,
        features_experimental_windows_sandbox: None,
        features_elevated_windows_sandbox: None,
        features_remote_compaction: None,
        features_remote_models: None,
        features_powershell_utf8: None,
        features_child_agents_md: None,
    };

    let Some(bytes) = bytes else {
        return Ok(state);
    };

    let s = String::from_utf8(bytes)
        .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?;

    let mut current_table: Option<String> = None;
    let mut in_multiline_double = false;
    let mut in_multiline_single = false;
    for raw_line in s.lines() {
        if in_multiline_double || in_multiline_single {
            update_multiline_string_state(
                raw_line,
                &mut in_multiline_double,
                &mut in_multiline_single,
            );
            continue;
        }

        let line = strip_toml_comment(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            update_multiline_string_state(
                raw_line,
                &mut in_multiline_double,
                &mut in_multiline_single,
            );
            continue;
        }

        if let Some(table) = parse_table_header(trimmed) {
            current_table = Some(table);
            update_multiline_string_state(
                raw_line,
                &mut in_multiline_double,
                &mut in_multiline_single,
            );
            continue;
        }

        let Some((raw_key, raw_value)) = parse_assignment(trimmed) else {
            update_multiline_string_state(
                raw_line,
                &mut in_multiline_double,
                &mut in_multiline_single,
            );
            continue;
        };

        let (table, key) = key_table_and_name(current_table.as_deref(), &raw_key);
        let table = table.as_deref().unwrap_or("");

        match (table, key.as_str()) {
            ("", "model") => state.model = parse_string(&raw_value),
            ("", "approval_policy") => state.approval_policy = parse_string(&raw_value),
            ("", "sandbox_mode") => state.sandbox_mode = parse_string(&raw_value),
            ("sandbox", "mode") => {
                if state.sandbox_mode.is_none() {
                    state.sandbox_mode = parse_string(&raw_value);
                }
            }
            ("", "model_reasoning_effort") => {
                state.model_reasoning_effort = parse_string(&raw_value)
            }
            ("", "file_opener") => state.file_opener = parse_string(&raw_value),
            ("", "hide_agent_reasoning") => state.hide_agent_reasoning = parse_bool(&raw_value),
            ("", "show_raw_agent_reasoning") => {
                state.show_raw_agent_reasoning = parse_bool(&raw_value)
            }

            ("history", "persistence") => state.history_persistence = parse_string(&raw_value),
            ("history", "max_bytes") => state.history_max_bytes = parse_u64(&raw_value),

            ("sandbox_workspace_write", "network_access") => {
                state.sandbox_workspace_write_network_access = parse_bool(&raw_value)
            }

            ("tui", "animations") => state.tui_animations = parse_bool(&raw_value),
            ("tui", "alternate_screen") => state.tui_alternate_screen = parse_string(&raw_value),
            ("tui", "show_tooltips") => state.tui_show_tooltips = parse_bool(&raw_value),
            ("tui", "scroll_invert") => state.tui_scroll_invert = parse_bool(&raw_value),

            ("features", "unified_exec") => state.features_unified_exec = parse_bool(&raw_value),
            ("features", "shell_snapshot") => {
                state.features_shell_snapshot = parse_bool(&raw_value)
            }
            ("features", "apply_patch_freeform") => {
                state.features_apply_patch_freeform = parse_bool(&raw_value)
            }
            ("features", "web_search_request") => {
                state.features_web_search_request = parse_bool(&raw_value)
            }
            ("features", "shell_tool") => state.features_shell_tool = parse_bool(&raw_value),
            ("features", "exec_policy") => state.features_exec_policy = parse_bool(&raw_value),
            ("features", "experimental_windows_sandbox") => {
                state.features_experimental_windows_sandbox = parse_bool(&raw_value)
            }
            ("features", "elevated_windows_sandbox") => {
                state.features_elevated_windows_sandbox = parse_bool(&raw_value)
            }
            ("features", "remote_compaction") => {
                state.features_remote_compaction = parse_bool(&raw_value)
            }
            ("features", "remote_models") => state.features_remote_models = parse_bool(&raw_value),
            ("features", "powershell_utf8") => {
                state.features_powershell_utf8 = parse_bool(&raw_value)
            }
            ("features", "child_agents_md") => {
                state.features_child_agents_md = parse_bool(&raw_value)
            }

            _ => {}
        }

        update_multiline_string_state(raw_line, &mut in_multiline_double, &mut in_multiline_single);
    }

    Ok(state)
}

pub fn codex_config_get(app: &tauri::AppHandle) -> Result<CodexConfigState, String> {
    let path = codex_paths::codex_config_toml_path(app)?;
    let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
    let bytes = read_optional_file(&path)?;

    let can_open_config_dir = app
        .path()
        .home_dir()
        .ok()
        .map(|home| {
            let allowed_root = home.join(".codex");
            path_is_under_allowed_root(&dir, &allowed_root)
        })
        .unwrap_or(false);

    make_state_from_bytes(
        dir.to_string_lossy().to_string(),
        path.to_string_lossy().to_string(),
        can_open_config_dir,
        bytes,
    )
}

#[cfg(windows)]
fn normalize_path_for_prefix_match(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(windows)]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    let dir_s = normalize_path_for_prefix_match(dir);
    let root_s = normalize_path_for_prefix_match(allowed_root);
    dir_s == root_s || dir_s.starts_with(&(root_s + "/"))
}

#[cfg(not(windows))]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    dir.starts_with(allowed_root)
}

fn patch_config_toml(current: Option<Vec<u8>>, patch: CodexConfigPatch) -> Result<Vec<u8>, String> {
    validate_enum_or_empty(
        "approval_policy",
        patch.approval_policy.as_deref().unwrap_or(""),
        &["untrusted", "on-failure", "on-request", "never"],
    )?;
    validate_enum_or_empty(
        "sandbox_mode",
        patch.sandbox_mode.as_deref().unwrap_or(""),
        &["read-only", "workspace-write", "danger-full-access"],
    )?;
    validate_enum_or_empty(
        "model_reasoning_effort",
        patch.model_reasoning_effort.as_deref().unwrap_or(""),
        &["minimal", "low", "medium", "high", "xhigh"],
    )?;
    validate_enum_or_empty(
        "file_opener",
        patch.file_opener.as_deref().unwrap_or(""),
        &["vscode", "vscode-insiders", "windsurf", "cursor", "none"],
    )?;
    validate_enum_or_empty(
        "history.persistence",
        patch.history_persistence.as_deref().unwrap_or(""),
        &["save-all", "none"],
    )?;
    validate_enum_or_empty(
        "tui.alternate_screen",
        patch.tui_alternate_screen.as_deref().unwrap_or(""),
        &["auto", "always", "never"],
    )?;

    let input = match current {
        Some(bytes) => String::from_utf8(bytes)
            .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?,
        None => String::new(),
    };

    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    if let Some(raw) = patch.model.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "model",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(raw) = patch.approval_policy.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "approval_policy",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(raw) = patch.sandbox_mode.as_deref() {
        let trimmed = raw.trim();
        let value = (!trimmed.is_empty()).then(|| toml_string_literal(trimmed));

        if root_key_exists(&lines, "sandbox_mode") {
            upsert_root_key(&mut lines, "sandbox_mode", value);
        } else if has_table_or_dotted_keys(&lines, "sandbox") {
            upsert_keys_auto_style(&mut lines, "sandbox", &["mode"], vec![("mode", value)]);
        } else {
            upsert_root_key(&mut lines, "sandbox_mode", value);
        }
    }
    if let Some(raw) = patch.model_reasoning_effort.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "model_reasoning_effort",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(raw) = patch.file_opener.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "file_opener",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(v) = patch.hide_agent_reasoning {
        upsert_root_key(
            &mut lines,
            "hide_agent_reasoning",
            v.then(|| "true".to_string()),
        );
    }
    if let Some(v) = patch.show_raw_agent_reasoning {
        upsert_root_key(
            &mut lines,
            "show_raw_agent_reasoning",
            v.then(|| "true".to_string()),
        );
    }

    // history.*
    if patch.history_persistence.is_some() || patch.history_max_bytes.is_some() {
        let mut items: Vec<(&str, Option<String>)> = Vec::new();
        if patch.history_persistence.is_some() {
            items.push((
                "persistence",
                opt_string_value(patch.history_persistence.as_deref()),
            ));
        }
        if patch.history_max_bytes.is_some() {
            items.push(("max_bytes", opt_u64_value(patch.history_max_bytes)));
        }

        upsert_keys_auto_style(&mut lines, "history", &["persistence", "max_bytes"], items);
    }

    // sandbox_workspace_write.*
    if let Some(v) = patch.sandbox_workspace_write_network_access {
        upsert_keys_auto_style(
            &mut lines,
            "sandbox_workspace_write",
            &["network_access"],
            vec![("network_access", v.then(|| "true".to_string()))],
        );
    }

    // tui.*
    let has_any_tui_patch = patch.tui_animations.is_some()
        || patch.tui_alternate_screen.is_some()
        || patch.tui_show_tooltips.is_some()
        || patch.tui_scroll_invert.is_some();
    if has_any_tui_patch {
        let tui_keys = [
            "animations",
            "alternate_screen",
            "show_tooltips",
            "scroll_invert",
        ];

        let mut items: Vec<(&str, Option<String>)> = Vec::new();
        if let Some(v) = patch.tui_animations {
            items.push(("animations", v.then(|| "true".to_string())));
        }
        if patch.tui_alternate_screen.is_some() {
            items.push((
                "alternate_screen",
                opt_string_value(patch.tui_alternate_screen.as_deref()),
            ));
        }
        if let Some(v) = patch.tui_show_tooltips {
            items.push(("show_tooltips", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.tui_scroll_invert {
            items.push(("scroll_invert", v.then(|| "true".to_string())));
        }

        upsert_keys_auto_style(&mut lines, "tui", &tui_keys, items);
    }

    // features.*
    let has_any_feature_patch = patch.features_unified_exec.is_some()
        || patch.features_shell_snapshot.is_some()
        || patch.features_apply_patch_freeform.is_some()
        || patch.features_web_search_request.is_some()
        || patch.features_shell_tool.is_some()
        || patch.features_exec_policy.is_some()
        || patch.features_experimental_windows_sandbox.is_some()
        || patch.features_elevated_windows_sandbox.is_some()
        || patch.features_remote_compaction.is_some()
        || patch.features_remote_models.is_some()
        || patch.features_powershell_utf8.is_some()
        || patch.features_child_agents_md.is_some();

    if has_any_feature_patch {
        let mut items: Vec<(&str, Option<String>)> = Vec::new();

        // UI semantics: `true` => write `key = true`, `false` => delete the key (do not write `false`).
        if let Some(v) = patch.features_unified_exec {
            items.push(("unified_exec", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_shell_snapshot {
            items.push(("shell_snapshot", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_apply_patch_freeform {
            items.push(("apply_patch_freeform", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_web_search_request {
            items.push(("web_search_request", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_shell_tool {
            items.push(("shell_tool", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_exec_policy {
            items.push(("exec_policy", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_experimental_windows_sandbox {
            items.push((
                "experimental_windows_sandbox",
                v.then(|| "true".to_string()),
            ));
        }
        if let Some(v) = patch.features_elevated_windows_sandbox {
            items.push(("elevated_windows_sandbox", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_remote_compaction {
            items.push(("remote_compaction", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_remote_models {
            items.push(("remote_models", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_powershell_utf8 {
            items.push(("powershell_utf8", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_child_agents_md {
            items.push(("child_agents_md", v.then(|| "true".to_string())));
        }

        upsert_keys_auto_style(&mut lines, "features", &FEATURES_KEY_ORDER, items);
    }

    normalize_toml_layout(&mut lines);

    if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
        lines.push(String::new());
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

pub fn codex_config_set(
    app: &tauri::AppHandle,
    patch: CodexConfigPatch,
) -> Result<CodexConfigState, String> {
    let path = codex_paths::codex_config_toml_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        ));
    }

    let current = read_optional_file(&path)?;
    let next = patch_config_toml(current, patch)?;
    let _ = write_file_atomic_if_changed(&path, &next)?;
    codex_config_get(app)
}

#[cfg(test)]
mod tests;
