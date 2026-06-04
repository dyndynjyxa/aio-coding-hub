mod support;

#[test]
fn codex_config_toml_raw_set_refuses_invalid_input_without_writing() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let path =
        aio_coding_hub_lib::test_support::codex_config_toml_path(&handle).expect("codex path");

    assert!(!path.exists(), "precondition: config.toml should not exist");

    let _ = aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
        &handle,
        "approval_policy =".to_string(),
    )
    .expect_err("invalid TOML should fail");

    assert!(
        !path.exists(),
        "invalid TOML should not create/modify config.toml"
    );

    std::fs::create_dir_all(path.parent().expect("parent")).expect("create codex dir");
    std::fs::write(&path, "approval_policy = \"on-request\"\n").expect("write initial");

    let _ = aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
        &handle,
        "approval_policy = \"nope\"\n".to_string(),
    )
    .expect_err("invalid enum should fail");

    let got = std::fs::read_to_string(&path).expect("read after failed write");
    assert_eq!(got, "approval_policy = \"on-request\"\n");
}

#[test]
fn codex_config_get_rejects_oversized_config_toml() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let path =
        aio_coding_hub_lib::test_support::codex_config_toml_path(&handle).expect("codex path");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create codex dir");
    std::fs::write(&path, vec![b'x'; 1024 * 1024 + 1]).expect("write oversized config");

    let err = aio_coding_hub_lib::test_support::codex_config_get_json(&handle)
        .expect_err("oversized config should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("too large"),
        "unexpected error: {err_text}"
    );
}

#[test]
fn codex_config_toml_raw_set_rejects_oversized_input_without_writing() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let path =
        aio_coding_hub_lib::test_support::codex_config_toml_path(&handle).expect("codex path");
    assert!(!path.exists(), "precondition: config.toml should not exist");

    let err = aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
        &handle,
        "x".repeat(1024 * 1024 + 1),
    )
    .expect_err("oversized TOML should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("too large"),
        "unexpected error: {err_text}"
    );
    assert!(!path.exists(), "oversized TOML should not be written");
}

#[test]
fn codex_config_toml_raw_set_uses_settings_override_directory() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut settings =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");
    settings["codex_home_mode"] = serde_json::json!("custom");
    settings["codex_home_override"] =
        serde_json::json!(app.home_dir().join("override-codex").join("config.toml"));
    let _ = aio_coding_hub_lib::test_support::settings_set_json(&handle, settings).expect("write");

    aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
        &handle,
        "approval_policy = \"on-request\"\n".to_string(),
    )
    .expect("write raw toml");

    let path =
        aio_coding_hub_lib::test_support::codex_config_toml_path(&handle).expect("codex path");
    assert_eq!(
        path,
        app.home_dir().join("override-codex").join("config.toml")
    );
    let got = std::fs::read_to_string(&path).expect("read override config");
    assert_eq!(got, "approval_policy = \"on-request\"\n");
}
