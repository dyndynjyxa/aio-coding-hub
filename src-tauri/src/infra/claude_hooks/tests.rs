use super::*;

fn group(command: &str) -> ClaudeHookGroup {
    ClaudeHookGroup {
        event: "PreToolUse".to_string(),
        matcher: "Edit|Write".to_string(),
        hooks: vec![ClaudeHookEntry {
            hook_type: "command".to_string(),
            command: command.to_string(),
            timeout: Some(30),
        }],
    }
}

#[test]
fn missing_settings_file_reads_as_empty_hooks() {
    let root = json_root_from_bytes(None, "读取 Hooks 空配置").expect("parse missing");

    assert!(parse_hooks_from_root(&root).is_empty());
}

#[test]
fn invalid_settings_json_fails_closed_instead_of_empty_hooks() {
    let err =
        json_root_from_bytes(Some(b"{invalid json".to_vec()), "读取 Hooks 空配置").unwrap_err();
    let message = err.to_string();

    assert!(
        message.contains("settings.json 解析失败，拒绝读取 Hooks 空配置以保护现有配置"),
        "unexpected error: {message}"
    );
}

#[test]
fn groups_to_json_round_trips_supported_hook_fields() {
    let groups = vec![group("echo ok")];
    let root = serde_json::json!({ "hooks": groups_to_json(&groups) });
    let parsed = parse_hooks_from_root(&root);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].event, groups[0].event);
    assert_eq!(parsed[0].matcher, groups[0].matcher);
    assert_eq!(parsed[0].hooks[0].hook_type, groups[0].hooks[0].hook_type);
    assert_eq!(parsed[0].hooks[0].command, groups[0].hooks[0].command);
    assert_eq!(parsed[0].hooks[0].timeout, groups[0].hooks[0].timeout);
}

#[test]
fn groups_to_json_preserves_non_hook_root_fields_when_patched_by_caller() {
    let mut root = serde_json::json!({
        "env": { "KEEP": "1" },
        "permissions": { "allow": ["Bash(ls)"] }
    });
    root.as_object_mut()
        .expect("root object")
        .insert("hooks".to_string(), groups_to_json(&[group("echo keep")]));

    assert_eq!(root["env"]["KEEP"], "1");
    assert_eq!(root["permissions"]["allow"][0], "Bash(ls)");
    assert_eq!(parse_hooks_from_root(&root).len(), 1);
}

#[test]
fn groups_to_json_with_existing_preserves_unknown_hook_fields_and_entries() {
    let existing = serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Edit|Write",
                "metadata": { "owner": "user" },
                "hooks": [
                    {
                        "type": "custom",
                        "payload": { "position": "before" }
                    },
                    {
                        "type": "command",
                        "command": "echo old 1",
                        "timeout": 30,
                        "description": "keep first"
                    },
                    "legacy-middle",
                    {
                        "type": "command",
                        "command": "echo old 2",
                        "note": "keep second"
                    },
                    {
                        "type": "custom",
                        "payload": { "position": "after" }
                    }
                ]
            }]
        }
    });

    let patched = groups_to_json_with_existing(
        &[ClaudeHookGroup {
            event: "PreToolUse".to_string(),
            matcher: "Edit|Write".to_string(),
            hooks: vec![
                ClaudeHookEntry {
                    hook_type: "command".to_string(),
                    command: "echo new 1".to_string(),
                    timeout: None,
                },
                ClaudeHookEntry {
                    hook_type: "command".to_string(),
                    command: "echo new 2".to_string(),
                    timeout: Some(5),
                },
            ],
        }],
        Some(&existing),
    );

    let group = &patched["PreToolUse"][0];
    assert_eq!(group["metadata"]["owner"], "user");
    assert_eq!(group["hooks"][0]["type"], "custom");
    assert_eq!(group["hooks"][0]["payload"]["position"], "before");
    assert_eq!(group["hooks"][1]["command"], "echo new 1");
    assert_eq!(group["hooks"][1]["description"], "keep first");
    assert!(group["hooks"][1].get("timeout").is_none());
    assert_eq!(group["hooks"][2], "legacy-middle");
    assert_eq!(group["hooks"][3]["command"], "echo new 2");
    assert_eq!(group["hooks"][3]["note"], "keep second");
    assert_eq!(group["hooks"][3]["timeout"], 5);
    assert_eq!(group["hooks"][4]["type"], "custom");
    assert_eq!(group["hooks"][4]["payload"]["position"], "after");
}

#[test]
fn read_optional_claude_hooks_settings_file_rejects_oversized_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("settings.json");
    std::fs::write(&path, vec![b'x'; CLAUDE_HOOKS_SETTINGS_MAX_BYTES + 1]).expect("write settings");

    let err = read_optional_claude_hooks_settings_file(&path)
        .unwrap_err()
        .to_string();

    assert!(err.contains("too large"));
}

#[test]
fn ensure_claude_hooks_settings_len_rejects_oversized_output() {
    let bytes = vec![b'x'; CLAUDE_HOOKS_SETTINGS_MAX_BYTES + 1];

    let err = ensure_claude_hooks_settings_len(&bytes, "claude/hooks settings.json")
        .unwrap_err()
        .to_string();

    assert!(err.contains("claude/hooks settings.json too large"));
}
