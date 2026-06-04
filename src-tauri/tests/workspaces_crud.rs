mod support;

use support::{json_array, json_i64, json_str};

#[test]
fn workspaces_list_default_state() {
    let app = support::TestApp::new();
    let handle = app.handle();

    // Init DB so workspace tables are created.
    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let result = aio_coding_hub_lib::test_support::workspaces_list_json(&handle, "claude")
        .expect("list workspaces");

    let items = json_array(result.get("items").cloned().unwrap_or_default());
    // Fresh DB may have zero workspaces (no seed data) or a default depending on schema.
    // The important thing is the call succeeds and returns a valid structure.
    assert!(
        result.get("items").is_some(),
        "response should contain items"
    );
    // active_id may be null initially.
    let _ = items;
}

#[test]
fn workspace_create_and_list() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    // Create a workspace.
    let created = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "Test Workspace",
        false,
    )
    .expect("create workspace");

    let ws_id = json_i64(&created, "id");
    assert!(ws_id > 0, "workspace id should be positive");
    assert_eq!(json_str(&created, "cli_key"), "claude");
    assert_eq!(json_str(&created, "name"), "Test Workspace");
    assert!(json_i64(&created, "created_at") > 0);
    assert!(json_i64(&created, "updated_at") > 0);

    // List workspaces and verify the new one appears.
    let result = aio_coding_hub_lib::test_support::workspaces_list_json(&handle, "claude")
        .expect("list after create");
    let items = json_array(result.get("items").cloned().unwrap_or_default());
    assert!(
        items.iter().any(|w| json_i64(w, "id") == ws_id),
        "created workspace should appear in list"
    );
}

#[test]
fn workspace_rename() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let created = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "Original Name",
        false,
    )
    .expect("create workspace");

    let ws_id = json_i64(&created, "id");

    let renamed = aio_coding_hub_lib::test_support::workspace_rename_json(
        &handle,
        ws_id,
        "Renamed Workspace",
    )
    .expect("rename workspace");

    assert_eq!(json_i64(&renamed, "id"), ws_id);
    assert_eq!(json_str(&renamed, "name"), "Renamed Workspace");
}

#[test]
fn workspace_names_are_bounded_before_persistence() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let long_name = "x".repeat(129);
    let err = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle, "claude", &long_name, false,
    )
    .expect_err("oversized workspace name should fail");
    let err = err.to_string();
    assert!(
        err.contains("workspace name is too long"),
        "unexpected error: {err}"
    );

    let err = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "bad\nname",
        false,
    )
    .expect_err("control-character workspace name should fail");
    let err = err.to_string();
    assert!(
        err.contains("workspace name contains control characters"),
        "unexpected error: {err}"
    );

    let created = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle, "claude", "Bounded", false,
    )
    .expect("create workspace");
    let ws_id = json_i64(&created, "id");

    let err =
        aio_coding_hub_lib::test_support::workspace_rename_json(&handle, ws_id, &"y".repeat(129))
            .expect_err("oversized workspace rename should fail");
    let err = err.to_string();
    assert!(
        err.contains("workspace name is too long"),
        "unexpected error: {err}"
    );
}

#[test]
fn workspace_create_multiple_and_delete() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let w1 = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "Workspace A",
        false,
    )
    .expect("create workspace A");

    let w2 = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "Workspace B",
        false,
    )
    .expect("create workspace B");

    let id1 = json_i64(&w1, "id");
    let id2 = json_i64(&w2, "id");

    // Both should be in the list.
    let result = aio_coding_hub_lib::test_support::workspaces_list_json(&handle, "claude")
        .expect("list after creates");
    let items = json_array(result.get("items").cloned().unwrap_or_default());
    assert!(items.len() >= 2, "should have at least 2 workspaces");

    // Delete workspace B (non-active workspace should be deletable).
    // First, ensure workspace B is NOT the active one. The active_id
    // may be null or may be id1. Deleting the non-active one should work.
    let active_id = result.get("active_id").and_then(|v| v.as_i64());

    // Pick the one that is NOT active to delete.
    let to_delete = if active_id == Some(id2) { id1 } else { id2 };

    aio_coding_hub_lib::test_support::workspace_delete(&handle, to_delete)
        .expect("delete workspace");

    let result = aio_coding_hub_lib::test_support::workspaces_list_json(&handle, "claude")
        .expect("list after delete");
    let items = json_array(result.get("items").cloned().unwrap_or_default());
    assert!(
        !items.iter().any(|w| json_i64(w, "id") == to_delete),
        "deleted workspace should not appear in list"
    );
}

#[test]
fn workspace_delete_nonexistent_fails() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let err = aio_coding_hub_lib::test_support::workspace_delete(&handle, 99999)
        .expect_err("delete non-existent should fail");
    let err = err.to_string();
    assert!(err.contains("not found"), "unexpected error: {err}");
}

#[test]
fn workspace_create_duplicate_name_fails() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    aio_coding_hub_lib::test_support::workspace_create_json(&handle, "claude", "Duplicate", false)
        .expect("create first");

    let err = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "claude",
        "Duplicate",
        false,
    )
    .expect_err("duplicate name should fail");
    let err = err.to_string();
    assert!(
        err.contains("already exists") || err.contains("CONSTRAINT"),
        "unexpected error: {err}"
    );
}
