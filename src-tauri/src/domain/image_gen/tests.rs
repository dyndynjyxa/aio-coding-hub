use super::config::{config_connection, config_get, config_set};
use super::transport::{
    build_request_url, decode_multipart_files, is_disallowed_ip, is_image_content_type,
    resolve_timeout, validate_fetch_image_url, validate_request_path, ImageGenMultipartFile,
};
use crate::db;
use std::net::IpAddr;
use std::time::Duration;

fn test_db(name: &str) -> (tempfile::TempDir, db::Db) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = db::init_for_tests(&dir.path().join(name)).expect("init db");
    (dir, db)
}

// -- config --

#[test]
fn config_get_returns_unconfigured_defaults_when_missing() {
    let (_dir, db) = test_db("image-gen-missing.db");

    let view = config_get(&db, "gpt-image").expect("config_get");

    assert_eq!(view.adapter_id, "gpt-image");
    assert_eq!(view.base_url, "");
    assert_eq!(view.model, "");
    assert!(!view.api_key_configured);
}

#[test]
fn config_set_replace_clear_preserve_semantics() {
    let (_dir, db) = test_db("image-gen-semantics.db");

    // replace: Some(value)
    let view = config_set(
        &db,
        "gpt-image",
        "https://api.example.com",
        "gpt-image-2",
        Some("sk-secret"),
    )
    .expect("set with key");
    assert!(view.api_key_configured);
    assert_eq!(view.base_url, "https://api.example.com");
    assert_eq!(view.model, "gpt-image-2");

    // preserve: None keeps the stored key while updating other fields
    let view = config_set(
        &db,
        "gpt-image",
        "https://api2.example.com",
        "gpt-image-2-2026-04-21",
        None,
    )
    .expect("set preserve");
    assert!(view.api_key_configured);
    assert_eq!(view.base_url, "https://api2.example.com");
    assert_eq!(view.model, "gpt-image-2-2026-04-21");
    let (base_url, api_key) = config_connection(&db, "gpt-image").expect("connection");
    assert_eq!(base_url, "https://api2.example.com");
    assert_eq!(api_key, "sk-secret");

    // clear: Some("")
    let view = config_set(
        &db,
        "gpt-image",
        "https://api3.example.com",
        "gpt-image-2",
        Some(""),
    )
    .expect("set clear");
    assert!(!view.api_key_configured);
    let (_base_url, api_key) = config_connection(&db, "gpt-image").expect("connection");
    assert_eq!(api_key, "");
    // clear 只清 key：base_url/model 同请求值一并落库。
    let persisted = config_get(&db, "gpt-image").expect("config_get after clear");
    assert_eq!(persisted.base_url, "https://api3.example.com");
    assert_eq!(persisted.model, "gpt-image-2");
    assert!(!persisted.api_key_configured);
}

#[test]
fn config_view_never_contains_api_key_plaintext() {
    let (_dir, db) = test_db("image-gen-no-leak.db");

    config_set(
        &db,
        "gpt-image",
        "https://api.example.com",
        "gpt-image-2",
        Some("sk-super-secret"),
    )
    .expect("set with key");

    let view = config_get(&db, "gpt-image").expect("config_get");
    let serialized = serde_json::to_string(&view).expect("serialize view");
    assert!(!serialized.contains("sk-super-secret"));
    assert!(serialized.contains("\"apiKeyConfigured\":true"));
}

#[test]
fn config_rejects_empty_adapter_id() {
    let (_dir, db) = test_db("image-gen-bad-adapter.db");

    let err = config_get(&db, "   ").expect_err("empty adapter_id should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));
}

#[test]
fn config_connection_fails_when_config_missing() {
    let (_dir, db) = test_db("image-gen-conn-missing.db");

    let err = config_connection(&db, "gpt-image").expect_err("missing config should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));
}

// -- path allowlist --

#[test]
fn request_path_allowlist_accepts_only_image_endpoints() {
    assert!(validate_request_path("/v1/images/generations").is_ok());
    assert!(validate_request_path("/v1/images/edits").is_ok());

    for path in [
        "/v1/chat/completions",
        "/v1/images/generations/../chat",
        "v1/images/generations",
        "/v1/images/edits/",
        "",
    ] {
        let err = validate_request_path(path).expect_err("path should be rejected");
        assert!(err.contains("SEC_INVALID_INPUT"), "unexpected error: {err}");
    }
}

// -- base url validation & join --

#[test]
fn build_request_url_joins_and_validates_scheme() {
    let url =
        build_request_url("https://api.example.com", "/v1/images/generations").expect("https base");
    assert_eq!(
        url.as_str(),
        "https://api.example.com/v1/images/generations"
    );

    // trailing slash is trimmed
    let url =
        build_request_url("https://api.example.com/", "/v1/images/edits").expect("trailing slash");
    assert_eq!(url.as_str(), "https://api.example.com/v1/images/edits");

    // custom path relays keep their prefix
    let url = build_request_url("https://relay.example.com/openai", "/v1/images/generations")
        .expect("custom path");
    assert_eq!(
        url.as_str(),
        "https://relay.example.com/openai/v1/images/generations"
    );

    // http allowed only for loopback debugging hosts
    assert!(build_request_url("http://127.0.0.1:37123", "/v1/images/edits").is_ok());
    assert!(build_request_url("http://localhost:8080", "/v1/images/edits").is_ok());
    let err = build_request_url("http://evil.example.com", "/v1/images/edits")
        .expect_err("plain http should fail");
    assert!(err.contains("SEC_INVALID_INPUT"));

    let err = build_request_url("ftp://api.example.com", "/v1/images/edits")
        .expect_err("ftp should fail");
    assert!(err.contains("SEC_INVALID_INPUT"));

    let err = build_request_url("   ", "/v1/images/edits").expect_err("empty base_url should fail");
    assert!(err.contains("SEC_INVALID_INPUT"));
}

#[test]
fn build_request_url_deduplicates_v1_suffix() {
    let url =
        build_request_url("https://api.example.com/v1", "/v1/images/generations").expect("v1 base");
    assert_eq!(
        url.as_str(),
        "https://api.example.com/v1/images/generations"
    );

    let url =
        build_request_url("https://api.example.com/v1/", "/v1/images/edits").expect("v1 slash");
    assert_eq!(url.as_str(), "https://api.example.com/v1/images/edits");
}

// -- fetch_image validation --

#[test]
fn fetch_image_url_rejects_http_and_private_hosts() {
    assert!(validate_fetch_image_url("https://cdn.example.com/img.png").is_ok());
    assert!(validate_fetch_image_url("https://93.184.216.34/img.png").is_ok());

    for url in [
        "http://cdn.example.com/img.png",
        "https://127.0.0.1/img.png",
        "https://10.0.0.8/img.png",
        "https://192.168.1.2/img.png",
        "https://169.254.0.1/img.png",
        "https://[::1]/img.png",
        "not a url",
    ] {
        let err = validate_fetch_image_url(url).expect_err("url should be rejected");
        assert!(err.contains("SEC_INVALID_INPUT"), "unexpected error: {err}");
    }
}

#[test]
fn disallowed_ip_covers_loopback_private_and_v6_locals() {
    for ip in [
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.0.1",
        "169.254.10.10",
        "0.0.0.0",
        "255.255.255.255",
        "::1",
        "fc00::1",
        "fe80::1",
        "::ffff:192.168.0.1",
    ] {
        let ip: IpAddr = ip.parse().expect("parse ip");
        assert!(is_disallowed_ip(ip), "should be disallowed: {ip}");
    }

    for ip in [
        "93.184.216.34",
        "8.8.8.8",
        "2606:2800:220:1:248:1893:25c8:1946",
    ] {
        let ip: IpAddr = ip.parse().expect("parse ip");
        assert!(!is_disallowed_ip(ip), "should be allowed: {ip}");
    }
}

#[tokio::test]
async fn fetch_image_rejects_localhost_hostname() {
    let client = reqwest::Client::new();
    let err = super::fetch_image(&client, "https://localhost/img.png", Some(5))
        .await
        .expect_err("localhost should be rejected before any request");
    assert!(err.contains("private address"), "unexpected error: {err}");
}

// -- content type --

#[test]
fn image_content_type_check() {
    assert!(is_image_content_type("image/png"));
    assert!(is_image_content_type(" Image/JPEG; charset=binary"));
    assert!(!is_image_content_type("application/json"));
    assert!(!is_image_content_type("text/html"));
    assert!(!is_image_content_type(""));
}

// -- multipart --

#[test]
fn multipart_files_decode_preserves_field_filename_mime() {
    let files = vec![
        ImageGenMultipartFile {
            field: "image[]".to_string(),
            filename: "input-1.png".to_string(),
            mime: "image/png".to_string(),
            data_b64: "aGVsbG8=".to_string(), // "hello"
        },
        ImageGenMultipartFile {
            field: "image[]".to_string(),
            filename: "input-2.jpeg".to_string(),
            mime: "image/jpeg".to_string(),
            data_b64: "d29ybGQ=".to_string(), // "world"
        },
    ];

    let decoded = decode_multipart_files(&files).expect("decode files");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].field, "image[]");
    assert_eq!(decoded[0].filename, "input-1.png");
    assert_eq!(decoded[0].mime, "image/png");
    assert_eq!(decoded[0].bytes, b"hello");
    assert_eq!(decoded[1].filename, "input-2.jpeg");
    assert_eq!(decoded[1].bytes, b"world");
}

#[test]
fn multipart_files_reject_invalid_base64_and_empty_metadata() {
    let bad_b64 = vec![ImageGenMultipartFile {
        field: "image[]".to_string(),
        filename: "input-1.png".to_string(),
        mime: "image/png".to_string(),
        data_b64: "!!not-base64!!".to_string(),
    }];
    let err = decode_multipart_files(&bad_b64).expect_err("invalid base64 should fail");
    assert!(err.contains("SEC_INVALID_INPUT"));

    let empty_field = vec![ImageGenMultipartFile {
        field: "  ".to_string(),
        filename: "input-1.png".to_string(),
        mime: "image/png".to_string(),
        data_b64: "aGVsbG8=".to_string(),
    }];
    let err = decode_multipart_files(&empty_field).expect_err("empty field should fail");
    assert!(err.contains("field is required"));

    let empty_filename = vec![ImageGenMultipartFile {
        field: "image[]".to_string(),
        filename: "".to_string(),
        mime: "image/png".to_string(),
        data_b64: "aGVsbG8=".to_string(),
    }];
    let err = decode_multipart_files(&empty_filename).expect_err("empty filename should fail");
    assert!(err.contains("filename is required"));
}

// -- timeout --

#[test]
fn timeout_defaults_to_600_and_clamps_to_1_900() {
    assert_eq!(resolve_timeout(None), Duration::from_secs(600));
    assert_eq!(resolve_timeout(Some(0)), Duration::from_secs(1));
    assert_eq!(resolve_timeout(Some(30)), Duration::from_secs(30));
    assert_eq!(resolve_timeout(Some(10_000)), Duration::from_secs(900));
}

// -- history --

use super::history::{
    ensure_writable_dir, read_image, storage_cleanup, storage_stats, task_delete, task_persist,
    tasks_clear, tasks_list, ImageGenTaskFilePayload, ImageGenTaskPersistPayload,
};
use base64::Engine as _;
use std::path::Path;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn file_payload(bytes: &[u8], mime: &str) -> ImageGenTaskFilePayload {
    ImageGenTaskFilePayload {
        mime: mime.to_string(),
        data_b64: b64(bytes),
    }
}

fn done_task_payload(id: &str, created_at: i64) -> ImageGenTaskPersistPayload {
    ImageGenTaskPersistPayload {
        id: id.to_string(),
        adapter_id: None,
        prompt: "a red square".to_string(),
        request_json: r#"{"size":"1024x1024"}"#.to_string(),
        status: "done".to_string(),
        error: None,
        usage_json: Some(r#"{"total_tokens":10}"#.to_string()),
        created_at,
        elapsed_ms: Some(1234),
        images: vec![file_payload(b"png-bytes", "image/png")],
        thumbs: vec![file_payload(b"thumb-bytes", "image/webp")],
        ref_images: vec![file_payload(b"ref-bytes", "image/png")],
    }
}

#[test]
fn history_persist_list_read_delete_full_chain() {
    let (_db_dir, db) = test_db("image-gen-history-chain.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    let row =
        task_persist(&db, storage.path(), done_task_payload("task-1", 100)).expect("persist task");

    let task_dir = storage.path().join("task-1");
    assert_eq!(row.id, "task-1");
    assert_eq!(row.adapter_id, "gpt-image");
    assert_eq!(row.status, "done");
    assert_eq!(row.created_at, 100);
    assert_eq!(row.elapsed_ms, Some(1234));
    assert_eq!(row.dir, task_dir.to_string_lossy().to_string());
    assert_eq!(
        std::fs::read(task_dir.join("image-1.png")).expect("image file"),
        b"png-bytes"
    );
    assert_eq!(
        std::fs::read(task_dir.join("thumb-1.webp")).expect("thumb file"),
        b"thumb-bytes"
    );
    assert_eq!(
        std::fs::read(task_dir.join("ref-1.png")).expect("ref file"),
        b"ref-bytes"
    );

    let listed = tasks_list(&db, None, 50).expect("list tasks");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].images.len(), 1);
    assert_eq!(
        listed[0].images[0].path,
        task_dir.join("image-1.png").to_string_lossy().to_string()
    );
    assert_eq!(
        listed[0].images[0].thumb_path.as_deref(),
        Some(task_dir.join("thumb-1.webp").to_string_lossy().as_ref())
    );
    assert_eq!(listed[0].images[0].mime, "image/png");
    assert_eq!(listed[0].ref_images.len(), 1);
    assert_eq!(listed[0].ref_images[0].thumb_path, None);

    let fetched =
        read_image(&db, storage.path(), &listed[0].images[0].path).expect("read image back");
    assert_eq!(fetched.mime, "image/png");
    assert_eq!(fetched.data_b64, b64(b"png-bytes"));

    let stats = storage_stats(&db, storage.path()).expect("stats");
    assert_eq!(stats.task_count, 1);
    assert_eq!(
        stats.total_bytes,
        (b"png-bytes".len() + b"thumb-bytes".len() + b"ref-bytes".len()) as i64
    );

    task_delete(&db, "task-1").expect("delete task");
    assert!(!task_dir.exists());
    assert!(tasks_list(&db, None, 50)
        .expect("list after delete")
        .is_empty());

    // Idempotent: deleting again succeeds.
    task_delete(&db, "task-1").expect("delete task twice");
}

#[test]
fn history_persists_failed_task_and_paginates_newest_first() {
    let (_db_dir, db) = test_db("image-gen-history-pagination.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    for (id, created_at) in [("t1", 1_i64), ("t2", 2), ("t3", 3)] {
        let mut payload = done_task_payload(id, created_at);
        if id == "t2" {
            payload.status = "error".to_string();
            payload.error = Some("HTTP_ERROR: upstream 500".to_string());
            payload.usage_json = None;
            payload.images = Vec::new();
            payload.thumbs = Vec::new();
        }
        task_persist(&db, storage.path(), payload).expect("persist");
    }

    let first_page = tasks_list(&db, None, 2).expect("first page");
    assert_eq!(
        first_page.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        vec!["t3", "t2"]
    );
    assert_eq!(first_page[1].status, "error");
    assert_eq!(
        first_page[1].error.as_deref(),
        Some("HTTP_ERROR: upstream 500")
    );
    assert!(first_page[1].images.is_empty());
    // Request snapshot survives for relay debugging.
    assert_eq!(first_page[1].request_json, r#"{"size":"1024x1024"}"#);

    let second_page = tasks_list(&db, Some(first_page[1].created_at), 2).expect("second page");
    assert_eq!(
        second_page
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec!["t1"]
    );

    // limit 0 is clamped to 1.
    assert_eq!(tasks_list(&db, None, 0).expect("clamped list").len(), 1);
}

#[test]
fn history_persist_upserts_on_id_conflict() {
    let (_db_dir, db) = test_db("image-gen-history-upsert.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    task_persist(&db, storage.path(), done_task_payload("t1", 1)).expect("persist");
    let mut updated = done_task_payload("t1", 2);
    updated.prompt = "a blue square".to_string();
    task_persist(&db, storage.path(), updated).expect("persist again");

    let listed = tasks_list(&db, None, 50).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].prompt, "a blue square");
    assert_eq!(listed[0].created_at, 2);
}

#[test]
fn history_persist_rejects_invalid_input() {
    let (_db_dir, db) = test_db("image-gen-history-invalid.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    // Path traversal / separators / dots in the id.
    for bad_id in ["../evil", "a/b", "a\\b", "  ", "a.b"] {
        let payload = done_task_payload(bad_id, 1);
        let err = task_persist(&db, storage.path(), payload).expect_err("bad id should fail");
        assert!(err.to_string().contains("SEC_INVALID_INPUT"), "{bad_id}");
    }

    // Invalid status.
    let mut payload = done_task_payload("t1", 1);
    payload.status = "loading".to_string();
    let err = task_persist(&db, storage.path(), payload).expect_err("bad status should fail");
    assert!(err.to_string().contains("status must be"));

    // More thumbs than images.
    let mut payload = done_task_payload("t1", 1);
    payload.thumbs.push(file_payload(b"extra", "image/webp"));
    let err = task_persist(&db, storage.path(), payload).expect_err("extra thumbs should fail");
    assert!(err.to_string().contains("more thumbs than images"));

    // Invalid base64.
    let mut payload = done_task_payload("t1", 1);
    payload.images[0].data_b64 = "!!bad!!".to_string();
    let err = task_persist(&db, storage.path(), payload).expect_err("bad base64 should fail");
    assert!(err.to_string().contains("data_b64 is invalid"));

    // Oversized payload (rejected on the encoded-length pre-check).
    let mut payload = done_task_payload("t1", 1);
    payload.images[0].data_b64 = "A".repeat(97 * 1024 * 1024 / 3 * 4);
    let err = task_persist(&db, storage.path(), payload).expect_err("oversized should fail");
    assert!(err.to_string().contains("exceeds"));

    // Nothing persisted, no stray dirs.
    assert!(tasks_list(&db, None, 50).expect("list").is_empty());
    assert!(!storage.path().join("t1").exists());
}

#[test]
fn history_read_image_rejects_out_of_bounds_paths() {
    let (_db_dir, db) = test_db("image-gen-history-bounds.db");
    let outer = tempfile::tempdir().expect("outer tempdir");
    let storage = outer.path().join("store");
    std::fs::create_dir_all(&storage).expect("create storage");

    task_persist(&db, &storage, done_task_payload("t1", 1)).expect("persist");

    let secret = outer.path().join("secret.txt");
    std::fs::write(&secret, b"secret").expect("write secret");

    // Absolute path outside the storage dir.
    let err =
        read_image(&db, &storage, &secret.to_string_lossy()).expect_err("outside path should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));

    // `..` traversal that resolves outside the storage dir.
    let traversal = storage.join("t1").join("..").join("..").join("secret.txt");
    let err =
        read_image(&db, &storage, &traversal.to_string_lossy()).expect_err("traversal should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));

    // The storage root itself is not a servable file.
    let err =
        read_image(&db, &storage, &storage.to_string_lossy()).expect_err("root dir should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));

    // Nonexistent path.
    let err = read_image(&db, &storage, &storage.join("nope.png").to_string_lossy())
        .expect_err("missing path should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));

    // Symlink inside the storage dir escaping to the secret is rejected after
    // canonicalization.
    #[cfg(unix)]
    {
        let link = storage.join("t1").join("link.png");
        std::os::unix::fs::symlink(&secret, &link).expect("create symlink");
        let err = read_image(&db, &storage, &link.to_string_lossy())
            .expect_err("symlink escape should fail");
        assert!(err.to_string().contains("SEC_INVALID_INPUT"));
    }
}

#[test]
fn history_read_image_allows_db_recorded_dirs_after_storage_dir_change() {
    let (_db_dir, db) = test_db("image-gen-history-old-dir.db");
    let old_storage = tempfile::tempdir().expect("old storage");
    let new_storage = tempfile::tempdir().expect("new storage");

    task_persist(&db, old_storage.path(), done_task_payload("t1", 1)).expect("persist");
    let image_path = old_storage.path().join("t1").join("image-1.png");

    // Current storage dir moved elsewhere; the old task dir is still readable
    // because it is recorded in the DB.
    let fetched = read_image(&db, new_storage.path(), &image_path.to_string_lossy())
        .expect("old dir should stay readable");
    assert_eq!(fetched.mime, "image/png");
    assert_eq!(fetched.data_b64, b64(b"png-bytes"));
}

#[test]
fn history_cleanup_and_clear_boundaries() {
    let (_db_dir, db) = test_db("image-gen-history-cleanup.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    for (id, created_at) in [("t1", 1_i64), ("t2", 2), ("t3", 3)] {
        task_persist(&db, storage.path(), done_task_payload(id, created_at)).expect("persist");
    }

    // keep_count larger than total: nothing deleted.
    assert_eq!(storage_cleanup(&db, 10).expect("cleanup keep 10"), 0);
    assert_eq!(tasks_list(&db, None, 50).expect("list").len(), 3);

    // keep_count 1: the two oldest tasks (rows + dirs) are deleted.
    assert_eq!(storage_cleanup(&db, 1).expect("cleanup keep 1"), 2);
    let remaining = tasks_list(&db, None, 50).expect("list");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "t3");
    assert!(!storage.path().join("t1").exists());
    assert!(!storage.path().join("t2").exists());
    assert!(storage.path().join("t3").exists());

    // keep_count 0 behaves like clear.
    task_persist(&db, storage.path(), done_task_payload("t4", 4)).expect("persist");
    assert_eq!(storage_cleanup(&db, 0).expect("cleanup keep 0"), 2);
    assert!(tasks_list(&db, None, 50).expect("list").is_empty());
    assert!(!storage.path().join("t3").exists());
    assert!(!storage.path().join("t4").exists());

    // tasks_clear deletes rows and dirs and reports the count.
    task_persist(&db, storage.path(), done_task_payload("t5", 5)).expect("persist");
    task_persist(&db, storage.path(), done_task_payload("t6", 6)).expect("persist");
    assert_eq!(tasks_clear(&db).expect("clear"), 2);
    assert!(tasks_list(&db, None, 50).expect("list").is_empty());
    assert!(!storage.path().join("t5").exists());
    assert!(!storage.path().join("t6").exists());
}

#[test]
fn history_persist_removes_task_dir_when_db_write_fails() {
    let (_db_dir, db) = test_db("image-gen-history-rollback.db");
    let storage = tempfile::tempdir().expect("storage tempdir");

    {
        let conn = db.open_connection().expect("open db");
        conn.execute_batch("DROP TABLE image_gen_tasks")
            .expect("drop table to force db failure");
    }

    let err = task_persist(&db, storage.path(), done_task_payload("t1", 1))
        .expect_err("persist should fail when the table is gone");
    assert!(err.to_string().contains("DB_ERROR"));
    assert!(
        !storage.path().join("t1").exists(),
        "task dir must be rolled back after db failure"
    );
}

#[cfg(unix)]
#[test]
fn history_ensure_writable_dir_rejects_unwritable_path() {
    use std::os::unix::fs::PermissionsExt;

    let outer = tempfile::tempdir().expect("outer tempdir");
    let readonly = outer.path().join("readonly");
    std::fs::create_dir_all(&readonly).expect("create readonly dir");
    std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o555)).expect("chmod");

    let err = ensure_writable_dir(&readonly.join("sub")).expect_err("unwritable dir should fail");
    assert!(err.to_string().contains("SEC_INVALID_INPUT"));

    // Restore permissions so the tempdir can be cleaned up.
    std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o755))
        .expect("chmod restore");
}

#[test]
fn history_ensure_writable_dir_creates_missing_dir() {
    let outer = tempfile::tempdir().expect("outer tempdir");
    let target = outer.path().join("a").join("b");

    ensure_writable_dir(&target).expect("should create and validate dir");
    assert!(target.is_dir());
    assert!(!target.join(".aio-write-probe").exists());
}

#[test]
fn history_task_row_serialization_has_no_sensitive_fields() {
    let (_db_dir, db) = test_db("image-gen-history-serialize.db");
    let storage = tempfile::tempdir().expect("storage tempdir");
    task_persist(&db, storage.path(), done_task_payload("t1", 1)).expect("persist");

    let listed = tasks_list(&db, None, 50).expect("list");
    let json = serde_json::to_string(&listed[0]).expect("serialize row");
    // Rows carry paths + metadata only; never api keys or raw image payloads.
    assert!(!json.contains("api_key"));
    assert!(!json.contains("apiKey"));
    assert!(!json.contains("dataB64"));
    let _ = Path::new(&listed[0].dir);
}
