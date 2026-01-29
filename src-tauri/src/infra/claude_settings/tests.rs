use super::*;

fn empty_patch() -> ClaudeSettingsPatch {
    ClaudeSettingsPatch {
        model: None,
        output_style: None,
        language: None,
        always_thinking_enabled: None,
        show_turn_duration: None,
        spinner_tips_enabled: None,
        terminal_progress_bar_enabled: None,
        respect_gitignore: None,
        permissions_allow: None,
        permissions_ask: None,
        permissions_deny: None,
        env_mcp_timeout_ms: None,
        env_mcp_tool_timeout_ms: None,
        env_disable_error_reporting: None,
        env_disable_telemetry: None,
        env_disable_background_tasks: None,
        env_disable_terminal_title: None,
        env_claude_bash_no_login: None,
        env_claude_code_attribution_header_disabled: None,
        env_claude_code_blocking_limit_override: None,
        env_claude_code_max_output_tokens: None,
        env_enable_experimental_mcp_cli: None,
        env_enable_tool_search: None,
        env_max_mcp_output_tokens: None,
        env_claude_code_disable_nonessential_traffic: None,
        env_claude_code_proxy_resolves_hosts: None,
        env_claude_code_skip_prompt_history: None,
    }
}

#[test]
fn patch_env_preserves_unmanaged_keys() {
    let input = serde_json::json!({
      "env": {
        "ANTHROPIC_BASE_URL": "http://localhost:8080",
        "ANTHROPIC_AUTH_TOKEN": "aio-coding-hub",
        "MCP_TIMEOUT": "123"
      }
    });

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            env_mcp_timeout_ms: Some(0),
            env_disable_error_reporting: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch");

    let env = patched
        .as_object()
        .and_then(|o| o.get("env"))
        .and_then(|v| v.as_object())
        .expect("env object");

    assert_eq!(
        env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()),
        Some("http://localhost:8080")
    );
    assert_eq!(
        env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()),
        Some("aio-coding-hub")
    );
    assert!(env.get("MCP_TIMEOUT").is_none(), "{patched}");
    assert_eq!(
        env.get("DISABLE_ERROR_REPORTING").and_then(|v| v.as_str()),
        Some("1")
    );
}

#[test]
fn patch_env_attribution_header_disabled_can_write_zero_and_remove_key() {
    let input = serde_json::json!({
      "env": {
        "CLAUDE_CODE_ATTRIBUTION_HEADER": "1",
        "KEEP": "x"
      }
    });

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            env_claude_code_attribution_header_disabled: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch");

    let env = patched
        .as_object()
        .and_then(|o| o.get("env"))
        .and_then(|v| v.as_object())
        .expect("env object");

    assert_eq!(
        env.get("CLAUDE_CODE_ATTRIBUTION_HEADER")
            .and_then(|v| v.as_str()),
        Some("0")
    );
    assert_eq!(env.get("KEEP").and_then(|v| v.as_str()), Some("x"));

    let patched = patch_claude_settings(
        patched,
        ClaudeSettingsPatch {
            env_claude_code_attribution_header_disabled: Some(false),
            ..empty_patch()
        },
    )
    .expect("patch");

    let env = patched
        .as_object()
        .and_then(|o| o.get("env"))
        .and_then(|v| v.as_object())
        .expect("env object");

    assert!(
        env.get("CLAUDE_CODE_ATTRIBUTION_HEADER").is_none(),
        "{patched}"
    );
    assert_eq!(env.get("KEEP").and_then(|v| v.as_str()), Some("x"));
}

#[test]
fn patch_env_numeric_overrides_can_write_and_remove_keys() {
    let input = serde_json::json!({
      "env": {
        "CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE": "193000",
        "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "95",
        "CLAUDE_CODE_MAX_OUTPUT_TOKENS": "8192"
      },
      "keep": 1
    });

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            env_claude_code_blocking_limit_override: Some(0),
            env_claude_code_max_output_tokens: Some(0),
            ..empty_patch()
        },
    )
    .expect("patch");

    let env = patched
        .as_object()
        .and_then(|o| o.get("env"))
        .and_then(|v| v.as_object())
        .expect("env object");

    assert!(
        env.get("CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE").is_none(),
        "{patched}"
    );
    assert_eq!(
        env.get("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE")
            .and_then(|v| v.as_str()),
        Some("95")
    );
    assert!(
        env.get("CLAUDE_CODE_MAX_OUTPUT_TOKENS").is_none(),
        "{patched}"
    );
    assert_eq!(patched.get("keep").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn patch_permissions_can_remove_empty_object() {
    let input = serde_json::json!({
      "permissions": { "allow": ["Bash(ls:*)"] },
      "other": true
    });

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            permissions_allow: Some(vec![]),
            permissions_ask: Some(vec![]),
            permissions_deny: Some(vec![]),
            ..empty_patch()
        },
    )
    .expect("patch");

    assert!(patched.get("permissions").is_none(), "{patched}");
    assert_eq!(patched.get("other").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn patch_can_recover_non_object_permissions_and_env() {
    let input = serde_json::json!({
        "permissions": "oops",
        "env": ["bad"],
        "keep": { "a": 1 }
    });

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            permissions_allow: Some(vec!["Bash(ls:*)".to_string()]),
            env_disable_telemetry: Some(true),
            ..empty_patch()
        },
    )
    .expect("patch");

    let perms = patched
        .get("permissions")
        .and_then(|v| v.as_object())
        .expect("permissions object");
    let allow = perms
        .get("allow")
        .and_then(|v| v.as_array())
        .expect("allow array");
    assert_eq!(allow.len(), 1);
    assert_eq!(allow[0].as_str(), Some("Bash(ls:*)"));

    let env = patched
        .get("env")
        .and_then(|v| v.as_object())
        .expect("env object");
    assert_eq!(
        env.get("DISABLE_TELEMETRY").and_then(|v| v.as_str()),
        Some("1")
    );

    assert_eq!(
        patched
            .get("keep")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("a"))
            .and_then(|v| v.as_i64()),
        Some(1)
    );
}

#[test]
fn patch_non_object_root_is_replaced_with_object() {
    let input = serde_json::json!("not-an-object");

    let patched = patch_claude_settings(
        input,
        ClaudeSettingsPatch {
            model: Some("claude-3-5-sonnet".to_string()),
            ..empty_patch()
        },
    )
    .expect("patch");

    assert_eq!(
        patched.get("model").and_then(|v| v.as_str()),
        Some("claude-3-5-sonnet")
    );
}
