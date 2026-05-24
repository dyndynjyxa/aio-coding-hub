mod support;

use support::TestApp;

fn settings_path(app: &TestApp) -> std::path::PathBuf {
    app.home_dir().join(".claude").join("settings.json")
}

fn write_settings(app: &TestApp, content: &str) {
    let path = settings_path(app);
    std::fs::create_dir_all(path.parent().expect("settings parent")).expect("create .claude");
    std::fs::write(path, content).expect("write settings");
}

#[test]
fn claude_hooks_get_fails_closed_on_malformed_settings_json() {
    let app = TestApp::new();
    let handle = app.handle();
    write_settings(&app, "{invalid json");

    let err = aio_coding_hub_lib::test_support::cli_manager_claude_hooks_get_json(&handle)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("settings.json 解析失败，拒绝读取 Hooks 空配置以保护现有配置"),
        "unexpected error: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(settings_path(&app)).expect("read settings"),
        "{invalid json"
    );
}

#[test]
fn claude_hooks_get_fails_closed_on_non_object_settings_json() {
    let app = TestApp::new();
    let handle = app.handle();
    write_settings(&app, "[]");

    let err = aio_coding_hub_lib::test_support::cli_manager_claude_hooks_get_json(&handle)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("settings.json 根节点不是 JSON 对象，拒绝读取 Hooks 空配置以保护现有配置"),
        "unexpected error: {err}"
    );
}

#[test]
fn claude_hooks_set_fails_closed_on_malformed_settings_json_without_overwrite() {
    let app = TestApp::new();
    let handle = app.handle();
    write_settings(&app, "{invalid json");

    let err = aio_coding_hub_lib::test_support::cli_manager_claude_hooks_set_json(
        &handle,
        serde_json::json!({
            "groups": [{
                "event": "PreToolUse",
                "matcher": "",
                "hooks": [{
                    "hook_type": "command",
                    "command": "echo ok",
                    "timeout": null
                }]
            }]
        }),
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("settings.json 解析失败，拒绝覆写以保护现有配置"),
        "unexpected error: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(settings_path(&app)).expect("read settings"),
        "{invalid json"
    );
}

#[test]
fn claude_hooks_set_preserves_unmanaged_top_level_settings() {
    let app = TestApp::new();
    let handle = app.handle();
    write_settings(
        &app,
        r#"{
  "model": "claude-sonnet-4-20250514",
  "env": { "KEEP": "1" },
  "hooks": {}
}
"#,
    );

    let state = aio_coding_hub_lib::test_support::cli_manager_claude_hooks_set_json(
        &handle,
        serde_json::json!({
            "groups": [{
                "event": "PreToolUse",
                "matcher": "Edit|Write",
                "hooks": [{
                    "hook_type": "command",
                    "command": "echo ok",
                    "timeout": 30
                }]
            }]
        }),
    )
    .expect("set hooks");

    assert_eq!(state["groups"].as_array().map(Vec::len), Some(1));

    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(settings_path(&app)).expect("read settings"))
            .expect("parse settings");
    assert_eq!(saved["model"], "claude-sonnet-4-20250514");
    assert_eq!(saved["env"]["KEEP"], "1");
    assert_eq!(saved["hooks"]["PreToolUse"][0]["matcher"], "Edit|Write");
    assert_eq!(
        saved["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "echo ok"
    );
}
