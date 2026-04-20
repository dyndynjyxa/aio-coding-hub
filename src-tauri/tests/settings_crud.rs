mod support;

use support::{json_bool, json_i64};

fn settings_command_update_json(preferred_port: u16) -> serde_json::Value {
    serde_json::json!({
        "preferredPort": preferred_port,
        "autoStart": false,
        "logRetentionDays": 7,
        "failoverMaxAttemptsPerProvider": 5,
        "failoverMaxProvidersToTry": 5
    })
}

fn command_settings(value: &serde_json::Value) -> &serde_json::Value {
    &value["settings"]
}

#[test]
fn settings_read_defaults() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let settings =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");

    // Verify key default values.
    assert_eq!(json_i64(&settings, "preferred_port"), 37123);
    assert!(!json_bool(&settings, "auto_start"));
    assert!(json_bool(&settings, "tray_enabled"));
    assert_eq!(settings["home_usage_period"], serde_json::json!("last15"));
    assert_eq!(json_i64(&settings, "log_retention_days"), 7);
    assert_eq!(json_i64(&settings, "failover_max_attempts_per_provider"), 5);
    assert_eq!(json_i64(&settings, "failover_max_providers_to_try"), 5);
    assert_eq!(json_i64(&settings, "circuit_breaker_failure_threshold"), 5);
    assert_eq!(
        json_i64(&settings, "circuit_breaker_open_duration_minutes"),
        30
    );
}

#[test]
fn settings_update_and_re_read() {
    let app = support::TestApp::new();
    let handle = app.handle();

    // Read defaults first to get the full structure.
    let defaults =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");

    // Modify a few fields.
    let mut update = defaults;
    update["preferred_port"] = serde_json::json!(38000);
    update["log_retention_days"] = serde_json::json!(7);
    update["failover_max_attempts_per_provider"] = serde_json::json!(3);

    let updated =
        aio_coding_hub_lib::test_support::settings_set_json(&handle, update).expect("update");

    assert_eq!(json_i64(&updated, "preferred_port"), 38000);
    assert_eq!(json_i64(&updated, "log_retention_days"), 7);
    assert_eq!(json_i64(&updated, "failover_max_attempts_per_provider"), 3);

    // Re-read to verify persistence.
    let re_read =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("re-read settings");

    assert_eq!(json_i64(&re_read, "preferred_port"), 38000);
    assert_eq!(json_i64(&re_read, "log_retention_days"), 7);
    assert_eq!(json_i64(&re_read, "failover_max_attempts_per_provider"), 3);
    // Fields not modified should retain their defaults.
    assert!(!json_bool(&re_read, "auto_start"));
    assert!(json_bool(&re_read, "tray_enabled"));
}

#[test]
fn settings_update_preserves_unmodified_fields() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let defaults =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");

    // Update only the port.
    let mut update = defaults.clone();
    update["preferred_port"] = serde_json::json!(39000);

    let updated =
        aio_coding_hub_lib::test_support::settings_set_json(&handle, update).expect("update port");

    assert_eq!(json_i64(&updated, "preferred_port"), 39000);
    // All other fields should match defaults.
    assert_eq!(
        json_i64(&updated, "circuit_breaker_failure_threshold"),
        json_i64(&defaults, "circuit_breaker_failure_threshold")
    );
    assert_eq!(
        json_i64(&updated, "circuit_breaker_open_duration_minutes"),
        json_i64(&defaults, "circuit_breaker_open_duration_minutes")
    );
    assert_eq!(
        json_bool(&updated, "auto_start"),
        json_bool(&defaults, "auto_start")
    );
}

#[test]
fn settings_cache_does_not_leak_across_distinct_app_paths() {
    {
        let app = support::TestApp::new();
        let handle = app.handle();

        let mut update =
            aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");
        update["preferred_port"] = serde_json::json!(39001);

        let persisted =
            aio_coding_hub_lib::test_support::settings_set_json(&handle, update).expect("update");
        assert_eq!(json_i64(&persisted, "preferred_port"), 39001);
    }

    {
        let app = support::TestApp::new();
        let handle = app.handle();

        let settings =
            aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("read defaults");
        assert_eq!(
            json_i64(&settings, "preferred_port"),
            37123,
            "settings cache should be scoped by settings.json path"
        );
    }
}

#[test]
fn settings_set_blocks_when_settings_json_is_corrupted() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let app_data_dir =
        aio_coding_hub_lib::test_support::app_data_dir(&handle).expect("app data dir");
    std::fs::create_dir_all(&app_data_dir).expect("create app data dir");
    let settings_path = app_data_dir.join("settings.json");
    std::fs::write(&settings_path, "{invalid json").expect("write corrupted settings");

    let err = aio_coding_hub_lib::test_support::settings_set_via_command_json(
        &handle,
        settings_command_update_json(38000),
    )
    .expect_err("settings_set should fail on corrupted settings.json");

    let err_text = err.to_string();
    assert!(
        err_text.contains("SETTINGS_RECOVERY_REQUIRED"),
        "unexpected error: {err_text}"
    );
    assert!(
        err_text.contains("invalid settings.json"),
        "unexpected error: {err_text}"
    );

    let content = std::fs::read_to_string(&settings_path).expect("read corrupted settings");
    assert_eq!(content, "{invalid json");
}

#[test]
fn gateway_check_port_available_fails_when_settings_json_is_corrupted() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let app_data_dir =
        aio_coding_hub_lib::test_support::app_data_dir(&handle).expect("app data dir");
    std::fs::create_dir_all(&app_data_dir).expect("create app data dir");
    let settings_path = app_data_dir.join("settings.json");
    std::fs::write(&settings_path, "{invalid json").expect("write corrupted settings");

    let err = aio_coding_hub_lib::test_support::gateway_check_port_available_json(&handle, 37123)
        .expect_err("gateway_check_port_available should fail");

    let err_text = err.to_string();
    assert!(
        err_text.contains("invalid settings.json"),
        "unexpected error: {err_text}"
    );
}

#[test]
fn settings_set_via_command_syncs_runtime_upstream_proxy_state() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut enable_update = settings_command_update_json(38000);
    enable_update["upstreamProxyEnabled"] = serde_json::json!(true);
    enable_update["upstreamProxyUrl"] = serde_json::json!("http://127.0.0.1:7890");

    let enabled =
        aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, enable_update)
            .expect("enable upstream proxy");

    assert!(json_bool(
        command_settings(&enabled),
        "upstream_proxy_enabled"
    ));
    assert_eq!(
        command_settings(&enabled)["upstream_proxy_url"],
        serde_json::json!("http://127.0.0.1:7890")
    );
    assert_eq!(
        aio_coding_hub_lib::test_support::gateway_upstream_proxy_url_json(&handle)
            .expect("runtime proxy url"),
        Some("http://127.0.0.1:7890".to_string())
    );

    let mut disable_update = settings_command_update_json(38000);
    disable_update["upstreamProxyEnabled"] = serde_json::json!(false);
    disable_update["upstreamProxyUrl"] = serde_json::json!("http://127.0.0.1:7890");

    let disabled =
        aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, disable_update)
            .expect("disable upstream proxy");

    assert!(!json_bool(
        command_settings(&disabled),
        "upstream_proxy_enabled"
    ));
    assert_eq!(
        aio_coding_hub_lib::test_support::gateway_upstream_proxy_url_json(&handle)
            .expect("runtime proxy disabled"),
        None
    );
}

#[test]
fn settings_set_via_command_syncs_runtime_upstream_proxy_state_with_separate_credentials() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut update = settings_command_update_json(38000);
    update["upstreamProxyEnabled"] = serde_json::json!(true);
    update["upstreamProxyUrl"] = serde_json::json!("http://127.0.0.1:7890");
    update["upstreamProxyUsername"] = serde_json::json!("proxy-user");
    update["upstreamProxyPassword"] = serde_json::json!({
        "mode": "replace",
        "value": "secret"
    });

    let enabled = aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, update)
        .expect("enable upstream proxy with credentials");

    assert!(json_bool(
        command_settings(&enabled),
        "upstream_proxy_enabled"
    ));
    assert_eq!(
        command_settings(&enabled)["upstream_proxy_url"],
        serde_json::json!("http://127.0.0.1:7890")
    );
    assert_eq!(
        command_settings(&enabled)["upstream_proxy_username"],
        serde_json::json!("proxy-user")
    );
    assert_eq!(
        command_settings(&enabled)["upstream_proxy_password_configured"],
        serde_json::json!(true)
    );
    assert_eq!(
        aio_coding_hub_lib::test_support::gateway_upstream_proxy_url_json(&handle)
            .expect("runtime proxy url"),
        Some("http://127.0.0.1:7890".to_string())
    );
}

#[test]
fn settings_set_via_command_rejects_invalid_upstream_proxy() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut update = settings_command_update_json(38000);
    update["upstreamProxyEnabled"] = serde_json::json!(true);
    update["upstreamProxyUrl"] = serde_json::json!("ftp://127.0.0.1:7890");

    let err = aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, update)
        .expect_err("invalid upstream proxy should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("Invalid proxy scheme"),
        "unexpected error: {err_text}"
    );
}

#[test]
fn settings_set_via_command_rejects_proxy_password_without_username() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut update = settings_command_update_json(38000);
    update["upstreamProxyEnabled"] = serde_json::json!(true);
    update["upstreamProxyUrl"] = serde_json::json!("http://127.0.0.1:7890");
    update["upstreamProxyPassword"] = serde_json::json!({
        "mode": "replace",
        "value": "secret"
    });

    let err = aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, update)
        .expect_err("password without username should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("upstream_proxy_username cannot be empty"),
        "unexpected error: {err_text}"
    );
}

#[test]
fn settings_set_via_command_rejects_mixed_proxy_credentials() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut update = settings_command_update_json(38000);
    update["upstreamProxyEnabled"] = serde_json::json!(true);
    update["upstreamProxyUrl"] = serde_json::json!("http://inline:secret@127.0.0.1:7890");
    update["upstreamProxyUsername"] = serde_json::json!("proxy-user");
    update["upstreamProxyPassword"] = serde_json::json!({
        "mode": "replace",
        "value": "override"
    });

    let err = aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, update)
        .expect_err("mixed proxy credentials should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("either in proxy URL or username/password fields"),
        "unexpected error: {err_text}"
    );
}

#[test]
fn settings_set_via_command_requires_proxy_url_when_enabled() {
    let app = support::TestApp::new();
    let handle = app.handle();

    let mut update = settings_command_update_json(38000);
    update["upstreamProxyEnabled"] = serde_json::json!(true);
    update["upstreamProxyUrl"] = serde_json::json!("   ");

    let err = aio_coding_hub_lib::test_support::settings_set_via_command_json(&handle, update)
        .expect_err("enabled proxy without url should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("upstream_proxy_url cannot be empty"),
        "unexpected error: {err_text}"
    );
}
