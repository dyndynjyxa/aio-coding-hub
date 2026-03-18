mod support;

use support::{json_bool, json_i64};

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
