//! TOML parsing helpers for Codex config.toml.

use super::types::{
    CodexConfigState, CodexConfigStateMeta, CodexConfigTomlValidationError,
    CodexConfigTomlValidationResult,
};

pub(super) fn strip_toml_comment(line: &str) -> &str {
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

pub(super) fn parse_table_header(trimmed: &str) -> Option<String> {
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

pub(super) fn parse_assignment(trimmed: &str) -> Option<(String, String)> {
    let (k, v) = trimmed.split_once('=')?;
    let key = k.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), v.trim().to_string()))
}

pub(super) fn toml_unquote_string(value: &str) -> Option<String> {
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

pub(super) fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_string(value: &str) -> Option<String> {
    toml_unquote_string(value).or_else(|| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(super) fn parse_u64(value: &str) -> Option<u64> {
    value.trim().replace('_', "").parse().ok()
}

pub(super) fn normalize_key(raw: &str) -> String {
    let trimmed = raw.trim();
    toml_unquote_string(trimmed).unwrap_or_else(|| trimmed.to_string())
}

pub(super) fn key_table_and_name(
    current_table: Option<&str>,
    key: &str,
) -> (Option<String>, String) {
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

pub(super) fn is_allowed_value(value: &str, allowed: &[&str]) -> bool {
    allowed.iter().any(|v| v.eq_ignore_ascii_case(value))
}

pub(super) fn validate_enum_or_empty(
    key: &str,
    value: &str,
    allowed: &[&str],
) -> crate::shared::error::AppResult<()> {
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
    )
    .into())
}

pub(super) fn is_any_table_header_line(line: &str) -> bool {
    let cleaned = strip_toml_comment(line).trim();
    cleaned.starts_with('[') && cleaned.ends_with(']') && !cleaned.is_empty()
}

pub(super) fn update_multiline_string_state(
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

pub(super) fn make_state_from_bytes(
    meta: CodexConfigStateMeta,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let CodexConfigStateMeta {
        config_dir,
        config_path,
        user_home_default_dir,
        user_home_default_path,
        follow_codex_home_dir,
        follow_codex_home_path,
        can_open_config_dir,
    } = meta;

    let exists = bytes.is_some();
    let mut state = CodexConfigState {
        config_dir,
        config_path,
        user_home_default_dir,
        user_home_default_path,
        follow_codex_home_dir,
        follow_codex_home_path,
        can_open_config_dir,
        exists,

        model: None,
        approval_policy: None,
        sandbox_mode: None,
        model_reasoning_effort: None,
        plan_mode_reasoning_effort: None,
        web_search: None,
        personality: None,
        model_context_window: None,
        model_auto_compact_token_limit: None,
        service_tier: None,

        sandbox_workspace_write_network_access: None,

        features_unified_exec: None,
        features_shell_snapshot: None,
        features_apply_patch_freeform: None,
        features_shell_tool: None,
        features_exec_policy: None,
        features_remote_compaction: None,
        features_fast_mode: None,
        features_responses_websockets_v2: None,
        features_multi_agent: None,
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
            ("", "plan_mode_reasoning_effort") => {
                state.plan_mode_reasoning_effort = parse_string(&raw_value)
            }
            ("", "web_search") => state.web_search = parse_string(&raw_value),
            ("", "personality") => {
                state.personality =
                    parse_string(&raw_value).filter(|value| !value.trim().is_empty())
            }
            ("", "model_context_window") => state.model_context_window = parse_u64(&raw_value),
            ("", "model_auto_compact_token_limit") => {
                state.model_auto_compact_token_limit = parse_u64(&raw_value)
            }
            ("", "service_tier") => state.service_tier = parse_string(&raw_value),

            ("sandbox_workspace_write", "network_access") => {
                state.sandbox_workspace_write_network_access = parse_bool(&raw_value)
            }

            ("features", "unified_exec") => state.features_unified_exec = parse_bool(&raw_value),
            ("features", "shell_snapshot") => {
                state.features_shell_snapshot = parse_bool(&raw_value)
            }
            ("features", "apply_patch_freeform") => {
                state.features_apply_patch_freeform = parse_bool(&raw_value)
            }
            ("features", "shell_tool") => state.features_shell_tool = parse_bool(&raw_value),
            ("features", "exec_policy") => state.features_exec_policy = parse_bool(&raw_value),
            ("features", "remote_compaction") => {
                state.features_remote_compaction = parse_bool(&raw_value)
            }
            ("features", "fast_mode") => state.features_fast_mode = parse_bool(&raw_value),
            ("features", "responses_websockets_v2") => {
                state.features_responses_websockets_v2 = parse_bool(&raw_value)
            }
            ("features", "multi_agent") => state.features_multi_agent = parse_bool(&raw_value),

            _ => {}
        }

        update_multiline_string_state(raw_line, &mut in_multiline_double, &mut in_multiline_single);
    }

    Ok(state)
}

pub(super) fn toml_span_start_to_line_column(input: &str, span_start: usize) -> Option<(u32, u32)> {
    let mut idx = span_start.min(input.len());
    while idx > 0 && !input.is_char_boundary(idx) {
        idx = idx.saturating_sub(1);
    }

    let prefix = &input[..idx];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .map(|line| line.chars().count() + 1)
        .unwrap_or(1);

    Some((u32::try_from(line).ok()?, u32::try_from(column).ok()?))
}

pub(super) fn validate_root_string_enum(
    table: &toml::value::Table,
    key: &str,
    allowed: &[&str],
) -> Option<CodexConfigTomlValidationError> {
    let value = table.get(key)?;
    let raw = match value.as_str() {
        Some(v) => v,
        None => {
            return Some(CodexConfigTomlValidationError {
                message: format!("invalid {key}: expected string"),
                line: None,
                column: None,
            });
        }
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if is_allowed_value(trimmed, allowed) {
        return None;
    }

    Some(CodexConfigTomlValidationError {
        message: format!("invalid {key}={trimmed} (allowed: {})", allowed.join(", ")),
        line: None,
        column: None,
    })
}

pub(super) fn validate_root_non_empty_string(
    table: &toml::value::Table,
    key: &str,
) -> Option<CodexConfigTomlValidationError> {
    let value = table.get(key)?;
    let raw = match value.as_str() {
        Some(v) => v,
        None => {
            return Some(CodexConfigTomlValidationError {
                message: format!("invalid {key}: expected string"),
                line: None,
                column: None,
            });
        }
    };

    if raw.trim().is_empty() {
        return Some(CodexConfigTomlValidationError {
            message: format!("invalid {key}: expected non-empty string"),
            line: None,
            column: None,
        });
    }

    None
}

pub(super) fn validate_codex_config_toml_raw(input: &str) -> CodexConfigTomlValidationResult {
    if input.trim().is_empty() {
        return CodexConfigTomlValidationResult {
            ok: true,
            error: None,
        };
    }

    match toml::from_str::<toml::Value>(input) {
        Ok(value) => {
            let table = match value.as_table() {
                Some(t) => t,
                None => {
                    return CodexConfigTomlValidationResult {
                        ok: false,
                        error: Some(CodexConfigTomlValidationError {
                            message: "invalid TOML: expected root table".to_string(),
                            line: None,
                            column: None,
                        }),
                    };
                }
            };

            if let Some(err) = validate_root_string_enum(
                table,
                "approval_policy",
                &["untrusted", "on-failure", "on-request", "never"],
            ) {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            if let Some(err) = validate_root_string_enum(
                table,
                "sandbox_mode",
                &["read-only", "workspace-write", "danger-full-access"],
            ) {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            if let Some(err) = validate_root_non_empty_string(table, "model_reasoning_effort") {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            if let Some(err) = validate_root_string_enum(
                table,
                "plan_mode_reasoning_effort",
                &["low", "medium", "high", "xhigh"],
            ) {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            if let Some(err) =
                validate_root_string_enum(table, "web_search", &["cached", "live", "disabled"])
            {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            if let Some(err) =
                validate_root_string_enum(table, "personality", &["pragmatic", "friendly"])
            {
                return CodexConfigTomlValidationResult {
                    ok: false,
                    error: Some(err),
                };
            }

            CodexConfigTomlValidationResult {
                ok: true,
                error: None,
            }
        }
        Err(err) => {
            let (line, column) = err
                .span()
                .and_then(|span| toml_span_start_to_line_column(input, span.start))
                .map(|(line, column)| (Some(line), Some(column)))
                .unwrap_or((None, None));

            CodexConfigTomlValidationResult {
                ok: false,
                error: Some(CodexConfigTomlValidationError {
                    message: {
                        let msg = err.message().trim();
                        if msg.is_empty() {
                            err.to_string()
                        } else {
                            msg.to_string()
                        }
                    },
                    line,
                    column,
                }),
            }
        }
    }
}
