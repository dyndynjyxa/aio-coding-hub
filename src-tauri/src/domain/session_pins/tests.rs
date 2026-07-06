use super::*;

fn test_db() -> db::Db {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    // Keep tempdir alive for the test process lifetime.
    std::mem::forget(dir);
    db::init_for_tests(&db_path).expect("init db")
}

#[test]
fn upsert_get_roundtrip_default_and_custom() {
    let db = test_db();
    let mode = crate::sort_modes::create_mode(&db, "省钱档").expect("create mode");

    // Not pinned yet.
    assert_eq!(get_persistent_pin(&db, "claude", "s1").expect("get"), None);

    // Pin to Default (None).
    upsert_persistent_pin(&db, "claude", "s1", None).expect("pin default");
    assert_eq!(
        get_persistent_pin(&db, "claude", "s1").expect("get"),
        Some(None)
    );

    // Switch to custom mode.
    upsert_persistent_pin(&db, "claude", "s1", Some(mode.id)).expect("pin custom");
    assert_eq!(
        get_persistent_pin(&db, "claude", "s1").expect("get"),
        Some(Some(mode.id))
    );
}

#[test]
fn upsert_rejects_nonexistent_mode_and_bad_input() {
    let db = test_db();
    assert!(upsert_persistent_pin(&db, "claude", "s1", Some(99999)).is_err());
    assert!(upsert_persistent_pin(&db, "claude", "  ", Some(1)).is_err());
    assert!(upsert_persistent_pin(&db, "opencode", "s1", None).is_err()); // invalid cli
    assert_eq!(get_persistent_pin(&db, "claude", "s1").expect("get"), None);
}

#[test]
fn delete_removes_pin() {
    let db = test_db();
    upsert_persistent_pin(&db, "claude", "s1", None).expect("pin");
    assert!(delete_persistent_pin(&db, "claude", "s1").expect("delete"));
    // Idempotent: second delete returns false.
    assert!(!delete_persistent_pin(&db, "claude", "s1").expect("delete"));
    assert_eq!(get_persistent_pin(&db, "claude", "s1").expect("get"), None);
}

#[test]
fn deleting_sort_mode_cascades_to_pin() {
    let db = test_db();
    let mode = crate::sort_modes::create_mode(&db, "备用档").expect("create mode");
    upsert_persistent_pin(&db, "claude", "s1", Some(mode.id)).expect("pin custom");
    assert_eq!(
        get_persistent_pin(&db, "claude", "s1").expect("get"),
        Some(Some(mode.id))
    );

    // Deleting the mode cascades — the pin row is removed (user pinned something gone).
    crate::sort_modes::delete_mode_with_affected_cli_keys(&db, mode.id).expect("delete mode");
    assert_eq!(get_persistent_pin(&db, "claude", "s1").expect("get"), None);
}

#[test]
fn list_returns_all_pins() {
    let db = test_db();
    upsert_persistent_pin(&db, "claude", "s1", None).expect("pin");
    upsert_persistent_pin(&db, "codex", "s2", None).expect("pin");
    let rows = list_persistent_pins(&db).expect("list");
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|r| r.cli_key == "claude" && r.session_id == "s1"));
    assert!(rows
        .iter()
        .any(|r| r.cli_key == "codex" && r.session_id == "s2"));
}
