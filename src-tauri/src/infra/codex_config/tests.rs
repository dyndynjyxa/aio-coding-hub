use super::*;

fn empty_patch() -> CodexConfigPatch {
    CodexConfigPatch {
        model: None,
        approval_policy: None,
        sandbox_mode: None,
        model_reasoning_effort: None,
        plan_mode_reasoning_effort: None,
        web_search: None,
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
        features_remote_models: None,
        features_multi_agent: None,
    }
}

#[test]
fn patch_creates_features_table_and_preserves_other_tables() {
    let input = r#"# header

[mcp_servers.exa]
type = "stdio"

"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_snapshot: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("[mcp_servers.exa]"), "{s}");
    assert!(s.contains("[features]"), "{s}");
    assert!(s.contains("shell_snapshot = true"), "{s}");
}

#[test]
fn patch_deletes_sandbox_workspace_write_network_access_and_table_when_false() {
    let input = r#"[sandbox_workspace_write]
network_access = false
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            sandbox_workspace_write_network_access: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("[sandbox_workspace_write]"), "{s}");
    assert!(!s.contains("network_access"), "{s}");
}

#[test]
fn patch_deletes_sandbox_workspace_write_network_access_but_preserves_other_keys() {
    let input = r#"[sandbox_workspace_write]
network_access = true
other = "keep"
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            sandbox_workspace_write_network_access: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("[sandbox_workspace_write]"), "{s}");
    assert!(s.contains("other = \"keep\""), "{s}");
    assert!(!s.contains("network_access ="), "{s}");
}

#[test]
fn patch_preserves_existing_features_when_setting_another() {
    let input = r#"[features]
shell_tool = true
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_snapshot: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("shell_tool = true"), "{s}");
    assert!(s.contains("shell_snapshot = true"), "{s}");
}

#[test]
fn patch_deletes_default_false_feature_when_disabled() {
    let input = r#"[features]
shell_snapshot = true
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_snapshot: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("shell_snapshot ="), "{s}");
}

#[test]
fn patch_writes_true_when_feature_enabled() {
    let input = r#"[features]
shell_tool = false
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_tool: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("shell_tool = false"), "{s}");
    assert!(s.contains("shell_tool = true"), "{s}");
}

#[test]
fn patch_writes_fast_mode_and_service_tier_when_enabled() {
    let input = r#"model = "gpt-5"

[features]
shell_tool = true
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            service_tier: Some("fast".to_string()),
            features_fast_mode: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("service_tier = \"fast\""), "{s}");
    assert!(s.contains("shell_tool = true"), "{s}");
    assert!(s.contains("fast_mode = true"), "{s}");
}

#[test]
fn patch_writes_model_linked_limits() {
    let out = patch_config_toml(
        None,
        CodexConfigPatch {
            model_context_window: Some(Some(1_000_000)),
            model_auto_compact_token_limit: Some(Some(900_000)),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("model_context_window = 1000000"), "{s}");
    assert!(s.contains("model_auto_compact_token_limit = 900000"), "{s}");
}

#[test]
fn patch_deletes_model_linked_limits_when_null() {
    let input = r#"model_context_window = 1000000
model_auto_compact_token_limit = 900000
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            model_context_window: Some(None),
            model_auto_compact_token_limit: Some(None),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("model_context_window ="), "{s}");
    assert!(!s.contains("model_auto_compact_token_limit ="), "{s}");
}

#[test]
fn patch_deletes_fast_mode_and_service_tier_when_disabled() {
    let input = r#"service_tier = "fast"

[features]
fast_mode = true
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            service_tier: Some(String::new()),
            features_fast_mode: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("service_tier ="), "{s}");
    assert!(!s.contains("fast_mode ="), "{s}");
}

#[test]
fn patch_deletes_feature_when_disabled() {
    let input = r#"[features]
shell_tool = true
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_tool: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(!s.contains("shell_tool ="), "{s}");
}

#[test]
fn patch_compacts_blank_lines_in_features_table() {
    let input = r#"[features]

shell_snapshot = true



[other]
foo = "bar"
"#;

    let out1 = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_tool: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let out2 = patch_config_toml(
        Some(out1),
        CodexConfigPatch {
            features_unified_exec: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out2).expect("utf8");
    assert!(
        s.contains(
            "[features]\n\
shell_snapshot = true\n\
unified_exec = true\n\
shell_tool = true\n\n\
[other]\n"
        ),
        "{s}"
    );
    assert!(!s.contains("[features]\n\n"), "{s}");
    assert!(!s.contains("true\n\nshell_tool"), "{s}");
    assert!(!s.contains("true\n\nunified_exec"), "{s}");
}

#[test]
fn patch_compacts_blank_lines_across_entire_file() {
    let input = r#"approval_policy = "never"


preferred_auth_method = "apikey"


[features]


shell_snapshot = true


[mcp_servers.exa]
type = "stdio"
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            features_shell_tool: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(
        s.contains(
            "approval_policy = \"never\"\n\
preferred_auth_method = \"apikey\"\n\n\
[features]\n\
shell_snapshot = true\n\
shell_tool = true\n\n\
[mcp_servers.exa]\n\
type = \"stdio\"\n"
        ),
        "{s}"
    );
    assert!(!s.contains("\n\n\n"), "{s}");
}

#[test]
fn validate_raw_allows_empty() {
    let out = validate_codex_config_toml_raw("");
    assert!(out.ok, "{out:?}");
    assert!(out.error.is_none(), "{out:?}");
}

#[test]
fn validate_raw_rejects_invalid_toml_with_location_when_available() {
    let out = validate_codex_config_toml_raw("approval_policy =");
    assert!(!out.ok, "{out:?}");
    let err = out.error.expect("error");
    assert!(!err.message.trim().is_empty(), "{err:?}");
    assert!(
        err.line.is_some() || err.column.is_some(),
        "expected line/column when available: {err:?}"
    );
}

#[test]
fn validate_raw_rejects_invalid_enum_values() {
    let out = validate_codex_config_toml_raw("approval_policy = \"nope\"");
    assert!(!out.ok, "{out:?}");
    let err = out.error.expect("error");
    assert!(err.message.contains("approval_policy"), "{err:?}");
    assert!(err.message.contains("allowed:"), "{err:?}");
}

#[test]
fn parse_reads_sandbox_mode_from_sandbox_table() {
    let input = r#"[sandbox]
mode = "read-only"
"#;

    let state = make_state_from_bytes(
        "dir".to_string(),
        "path".to_string(),
        true,
        Some(input.as_bytes().to_vec()),
    )
    .expect("make_state_from_bytes");

    assert_eq!(state.sandbox_mode.as_deref(), Some("read-only"));
}

#[test]
fn parse_prefers_root_sandbox_mode_over_sandbox_table() {
    let input = r#"sandbox_mode = "workspace-write"

[sandbox]
mode = "read-only"
"#;

    let state = make_state_from_bytes(
        "dir".to_string(),
        "path".to_string(),
        true,
        Some(input.as_bytes().to_vec()),
    )
    .expect("make_state_from_bytes");

    assert_eq!(state.sandbox_mode.as_deref(), Some("workspace-write"));
}

#[test]
fn parse_reads_service_tier_and_fast_mode() {
    let input = r#"service_tier = "fast"

[features]
fast_mode = true
"#;

    let state = make_state_from_bytes(
        "dir".to_string(),
        "path".to_string(),
        true,
        Some(input.as_bytes().to_vec()),
    )
    .expect("make_state_from_bytes");

    assert_eq!(state.service_tier.as_deref(), Some("fast"));
    assert_eq!(state.features_fast_mode, Some(true));
}

#[test]
fn parse_reads_model_linked_limits() {
    let input = r#"model_context_window = 1_000_000
model_auto_compact_token_limit = 900000
"#;

    let state = make_state_from_bytes(
        "dir".to_string(),
        "path".to_string(),
        true,
        Some(input.as_bytes().to_vec()),
    )
    .expect("make_state_from_bytes");

    assert_eq!(state.model_context_window, Some(1_000_000));
    assert_eq!(state.model_auto_compact_token_limit, Some(900_000));
}

#[test]
fn parse_ignores_table_headers_inside_multiline_strings() {
    let input = r#"prompt = """
[not_a_table]
foo = "bar"
"""

sandbox_mode = "read-only"
"#;

    let state = make_state_from_bytes(
        "dir".to_string(),
        "path".to_string(),
        true,
        Some(input.as_bytes().to_vec()),
    )
    .expect("make_state_from_bytes");

    assert_eq!(state.sandbox_mode.as_deref(), Some("read-only"));
}

#[test]
fn patch_updates_sandbox_table_mode_when_present() {
    let input = r#"[sandbox]
mode = "read-only"

[other]
foo = "bar"
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            sandbox_mode: Some("workspace-write".to_string()),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("[sandbox]\nmode = \"workspace-write\""), "{s}");
    assert!(!s.contains("sandbox_mode ="), "{s}");
}

#[test]
fn patch_updates_sandbox_dotted_mode_when_present() {
    let input = r#"sandbox.mode = "read-only"
"#;

    let out = patch_config_toml(
        Some(input.as_bytes().to_vec()),
        CodexConfigPatch {
            sandbox_mode: Some("danger-full-access".to_string()),
            ..empty_patch()
        },
    )
    .expect("patch_config_toml");

    let s = String::from_utf8(out).expect("utf8");
    assert!(s.contains("sandbox.mode = \"danger-full-access\""), "{s}");
    assert!(!s.contains("[sandbox]"), "{s}");
    assert!(!s.contains("sandbox_mode ="), "{s}");
}
