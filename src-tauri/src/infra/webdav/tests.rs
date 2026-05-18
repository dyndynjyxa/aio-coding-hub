//! Usage: Integration-level tests for WebDAV module (unit tests are in client.rs).

use super::*;

#[test]
fn config_serialization_roundtrip() {
    let config = WebDavConfig {
        url: "https://dav.example.com/sync/".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        encryption_password: Some("secret".to_string()),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: WebDavConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.url, config.url);
    assert_eq!(deserialized.username, config.username);
    assert_eq!(deserialized.password, config.password);
    assert_eq!(deserialized.encryption_password, config.encryption_password);
}

#[test]
fn config_without_encryption_password() {
    let json = r#"{"url":"https://dav.example.com","username":"u","password":"p"}"#;
    let config: WebDavConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.encryption_password, None);
}
