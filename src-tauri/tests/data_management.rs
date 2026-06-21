mod support;

use support::json_u64;

fn db_related_paths(path: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let mut wal = path.to_path_buf().into_os_string();
    wal.push("-wal");

    let mut shm = path.to_path_buf().into_os_string();
    shm.push("-shm");

    (wal.into(), shm.into())
}

#[test]
fn db_disk_usage_after_init() {
    let app = support::TestApp::new();
    let handle = app.handle();

    // Init DB creates the SQLite file.
    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let usage = aio_coding_hub_lib::test_support::db_disk_usage_json(&handle).expect("disk usage");

    let db_bytes = json_u64(&usage, "db_bytes");
    let total_bytes = json_u64(&usage, "total_bytes");

    // After DB init, the main DB file should have non-zero size.
    assert!(
        db_bytes > 0,
        "db_bytes should be > 0 after init, got {db_bytes}"
    );
    assert!(
        total_bytes >= db_bytes,
        "total_bytes ({total_bytes}) should be >= db_bytes ({db_bytes})"
    );
}

#[test]
fn clear_request_logs_on_empty_db() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let result =
        aio_coding_hub_lib::test_support::request_logs_clear_all_json(&handle).expect("clear logs");

    let logs_deleted = json_u64(&result, "request_logs_deleted");

    // On a fresh DB, there should be nothing to delete.
    assert_eq!(logs_deleted, 0, "no request logs to delete in fresh DB");
}

#[test]
fn disk_usage_after_clear_logs() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    // Get initial usage.
    let before =
        aio_coding_hub_lib::test_support::db_disk_usage_json(&handle).expect("usage before");
    let before_total = json_u64(&before, "total_bytes");
    assert!(before_total > 0, "DB should have non-zero size");

    // Clear logs (no-op on empty DB, but should still succeed).
    aio_coding_hub_lib::test_support::request_logs_clear_all_json(&handle).expect("clear logs");

    // Get usage after clear — should still be valid.
    let after = aio_coding_hub_lib::test_support::db_disk_usage_json(&handle).expect("usage after");
    let after_total = json_u64(&after, "total_bytes");
    assert!(
        after_total > 0,
        "DB should still have non-zero size after clear"
    );
}

#[test]
fn app_data_reset_removes_db_files_and_allows_reinit() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let before =
        aio_coding_hub_lib::test_support::db_disk_usage_json(&handle).expect("usage before reset");
    assert!(
        json_u64(&before, "db_bytes") > 0,
        "db should have non-zero size before reset"
    );

    let db_path = aio_coding_hub_lib::test_support::db_path(&handle).expect("db path");
    let (wal_path, shm_path) = db_related_paths(&db_path);

    let reset_ok =
        aio_coding_hub_lib::test_support::app_data_reset(&handle).expect("reset app data");
    assert!(reset_ok, "app_data_reset should report success");

    assert!(
        !db_path.exists(),
        "db file should be removed after reset: {}",
        db_path.display()
    );
    assert!(
        !wal_path.exists(),
        "wal file should be removed after reset: {}",
        wal_path.display()
    );
    assert!(
        !shm_path.exists(),
        "shm file should be removed after reset: {}",
        shm_path.display()
    );

    aio_coding_hub_lib::test_support::init_db(&handle).expect("re-init db after reset");
    assert!(
        db_path.exists(),
        "db file should be recreated after re-init"
    );
}
