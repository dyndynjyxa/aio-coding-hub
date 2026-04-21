//! Config.toml mutation (line-level patching) for Codex configuration.

use super::parsing::{
    is_any_table_header_line, normalize_key, parse_assignment, strip_toml_comment,
    update_multiline_string_state, validate_enum_or_empty,
};
use super::types::CodexConfigPatch;

pub(super) fn toml_escape_basic_string(value: &str) -> String {
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

pub(super) fn toml_string_literal(value: &str) -> String {
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

pub(super) fn root_key_exists(lines: &[String], key: &str) -> bool {
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

pub(super) const FEATURES_KEY_ORDER: [&str; 9] = [
    // Keep a stable persisted order for feature flags in config.toml.
    "shell_snapshot",
    "unified_exec",
    "shell_tool",
    "exec_policy",
    "apply_patch_freeform",
    "remote_compaction",
    "fast_mode",
    "responses_websockets_v2",
    "multi_agent",
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

pub(super) fn has_table_or_dotted_keys(lines: &[String], table: &str) -> bool {
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

/// Rename the `[model_providers.<from_key>]` table to `[model_providers.<to_key>]`.
/// Also updates the `name` field inside the table and any dotted keys.
fn rename_model_provider_table(lines: &mut Vec<String>, from_key: &str, to_key: &str) {
    let from_header = format!("[model_providers.{from_key}]");
    let to_header = format!("[model_providers.{to_key}]");

    // Rename table header if exists
    for line in lines.iter_mut() {
        if line.trim() == from_header {
            *line = to_header.clone();
            break;
        }
    }

    // Find the renamed table and update the `name` field inside
    if let Some(start) = lines.iter().position(|l| l.trim() == to_header) {
        let end = lines[start + 1..]
            .iter()
            .position(|line| line.trim().starts_with('['))
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());

        // Find and update the `name` key within the table
        let mut found = false;
        for line in lines[start + 1..end].iter_mut() {
            let cleaned = strip_toml_comment(line).trim();
            if cleaned.is_empty() || cleaned.starts_with('#') {
                continue;
            }
            if let Some((k, _)) = parse_assignment(cleaned) {
                if normalize_key(&k) == "name" {
                    *line = format!("name = {}", toml_string_literal(to_key));
                    found = true;
                    break;
                }
            }
        }

        // If not found, insert after the header
        if !found {
            lines.insert(start + 1, format!("name = {}", toml_string_literal(to_key)));
        }
    }

    // Also rename any dotted keys like `model_providers.aio.name` to `model_providers.OpenAI.name`
    let from_prefix = format!("model_providers.{from_key}.");
    let to_prefix = format!("model_providers.{to_key}.");
    for line in lines.iter_mut() {
        let cleaned = strip_toml_comment(line).trim();
        if cleaned.is_empty() || cleaned.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = parse_assignment(cleaned) {
            let normalized = normalize_key(&k);
            if normalized.starts_with(&from_prefix) {
                let suffix = &normalized[from_prefix.len()..];
                *line = format!("{to_prefix}{suffix} = {v}");
            }
        }
    }
}

/// Unified upsert that auto-detects and applies the appropriate table style.
pub(super) fn upsert_keys_auto_style(
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

pub(super) fn normalize_toml_layout(lines: &mut Vec<String>) {
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

pub(super) fn patch_config_toml(
    current: Option<Vec<u8>>,
    patch: CodexConfigPatch,
) -> crate::shared::error::AppResult<Vec<u8>> {
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
        "plan_mode_reasoning_effort",
        patch.plan_mode_reasoning_effort.as_deref().unwrap_or(""),
        &["low", "medium", "high", "xhigh"],
    )?;
    validate_enum_or_empty(
        "web_search",
        patch.web_search.as_deref().unwrap_or(""),
        &["cached", "live", "disabled"],
    )?;
    validate_enum_or_empty(
        "personality",
        patch.personality.as_deref().unwrap_or(""),
        &["pragmatic", "friendly"],
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

    // Cleanup retired feature keys on any save so config.toml converges to the
    // current contract instead of preserving dead toggles indefinitely.
    upsert_keys_auto_style(
        &mut lines,
        "features",
        &["remote_models"],
        vec![("remote_models", None)],
    );

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
    if let Some(raw) = patch.plan_mode_reasoning_effort.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "plan_mode_reasoning_effort",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(raw) = patch.web_search.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "web_search",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(raw) = patch.personality.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "personality",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
    }
    if let Some(value) = patch.model_context_window {
        upsert_root_key(
            &mut lines,
            "model_context_window",
            value.map(|next| next.to_string()),
        );
    }
    if let Some(value) = patch.model_auto_compact_token_limit {
        upsert_root_key(
            &mut lines,
            "model_auto_compact_token_limit",
            value.map(|next| next.to_string()),
        );
    }
    if let Some(raw) = patch.service_tier.as_deref() {
        let trimmed = raw.trim();
        upsert_root_key(
            &mut lines,
            "service_tier",
            (!trimmed.is_empty()).then(|| toml_string_literal(trimmed)),
        );
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

    // features.*
    let has_any_feature_patch = patch.features_unified_exec.is_some()
        || patch.features_shell_snapshot.is_some()
        || patch.features_apply_patch_freeform.is_some()
        || patch.features_shell_tool.is_some()
        || patch.features_exec_policy.is_some()
        || patch.features_remote_compaction.is_some()
        || patch.features_fast_mode.is_some()
        || patch.features_responses_websockets_v2.is_some()
        || patch.features_multi_agent.is_some();

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
        if let Some(v) = patch.features_shell_tool {
            items.push(("shell_tool", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_exec_policy {
            items.push(("exec_policy", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_remote_compaction {
            items.push(("remote_compaction", v.then(|| "true".to_string())));

            // When remote_compaction is enabled, Codex requires the provider to be named
            // "OpenAI" for the Remote Compact feature to work. Rename the entire
            // [model_providers.aio] table to [model_providers.OpenAI] and update model_provider.
            // See: https://github.com/dyndynjyxa/aio-coding-hub/issues/197
            if v {
                upsert_root_key(
                    &mut lines,
                    "model_provider",
                    Some(toml_string_literal("OpenAI")),
                );
                rename_model_provider_table(&mut lines, "aio", "OpenAI");
            } else {
                upsert_root_key(
                    &mut lines,
                    "model_provider",
                    Some(toml_string_literal("aio")),
                );
                rename_model_provider_table(&mut lines, "OpenAI", "aio");
            }
        }
        if let Some(v) = patch.features_fast_mode {
            items.push(("fast_mode", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_responses_websockets_v2 {
            items.push(("responses_websockets_v2", v.then(|| "true".to_string())));
        }
        if let Some(v) = patch.features_multi_agent {
            items.push(("multi_agent", v.then(|| "true".to_string())));
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
