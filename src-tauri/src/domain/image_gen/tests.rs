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
