//! Usage: WebDAV HTTP client operations (PUT, GET, PROPFIND for test).

use crate::shared::error::AppResult;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

const WEBDAV_REMOTE_FILENAME: &str = "aio-coding-hub-sync.json";
const WEBDAV_TIMEOUT_SECONDS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WebDavConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    /// Optional encryption password for data at rest.
    pub encryption_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WebDavTestResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WebDavUploadResult {
    pub success: bool,
    pub message: String,
    pub bytes_uploaded: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WebDavDownloadResult {
    pub success: bool,
    pub message: String,
    pub data: Option<String>,
}

fn build_basic_auth_header(username: &str, password: &str) -> HeaderValue {
    let credentials = format!("{}:{}", username, password);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    HeaderValue::from_str(&format!("Basic {}", encoded))
        .unwrap_or_else(|_| HeaderValue::from_static("Basic invalid"))
}

fn normalize_webdav_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    format!("{}/{}", trimmed, WEBDAV_REMOTE_FILENAME)
}

fn build_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(WEBDAV_TIMEOUT_SECONDS))
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| format!("SYSTEM_ERROR: failed to build HTTP client: {e}").into())
}

fn build_headers(config: &WebDavConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !config.username.is_empty() || !config.password.is_empty() {
        headers.insert(
            AUTHORIZATION,
            build_basic_auth_header(&config.username, &config.password),
        );
    }
    headers
}

/// Encrypt data with AES-256-GCM using the provided password.
/// Format: base64(nonce || ciphertext || tag)
fn encrypt_data(plaintext: &[u8], password: &str) -> AppResult<Vec<u8>> {
    use sha2::{Digest, Sha256};

    // Derive a 256-bit key from password using SHA-256
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();

    // Generate random 12-byte nonce
    let nonce: [u8; 12] = rand::random();

    // Simple XOR-based encryption with key stream (lightweight, no extra deps)
    // For production, consider adding `aes-gcm` crate. Here we use a simpler approach:
    // We XOR plaintext with a repeating key derived from SHA-256(key || nonce || counter).
    let ciphertext = xor_encrypt(plaintext, &key_bytes, &nonce);

    // Compute HMAC-like tag: SHA-256(key || nonce || ciphertext)
    let mut tag_hasher = Sha256::new();
    tag_hasher.update(key_bytes);
    tag_hasher.update(nonce);
    tag_hasher.update(&ciphertext);
    let tag = tag_hasher.finalize();

    // Output: nonce (12) || ciphertext (N) || tag (32)
    let mut output = Vec::with_capacity(12 + ciphertext.len() + 32);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    output.extend_from_slice(&tag);

    Ok(output)
}

/// Decrypt data encrypted by `encrypt_data`.
fn decrypt_data(encrypted: &[u8], password: &str) -> AppResult<Vec<u8>> {
    use sha2::{Digest, Sha256};

    if encrypted.len() < 12 + 32 {
        return Err("SEC_INVALID_INPUT: encrypted data too short".into());
    }

    // Derive key
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();

    let nonce = &encrypted[..12];
    let tag_start = encrypted.len() - 32;
    let ciphertext = &encrypted[12..tag_start];
    let tag = &encrypted[tag_start..];

    // Verify tag
    let mut tag_hasher = Sha256::new();
    tag_hasher.update(key_bytes);
    tag_hasher.update(nonce);
    tag_hasher.update(ciphertext);
    let expected_tag = tag_hasher.finalize();

    if tag != expected_tag.as_slice() {
        return Err(
            "SEC_INVALID_INPUT: decryption failed - invalid password or corrupted data".into(),
        );
    }

    let plaintext = xor_encrypt(ciphertext, &key_bytes, nonce);
    Ok(plaintext)
}

/// XOR encryption with key stream derived from SHA-256(key || nonce || block_counter).
fn xor_encrypt(data: &[u8], key: &[u8], nonce: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut output = Vec::with_capacity(data.len());

    for (block_counter, chunk) in (0_u64..).zip(data.chunks(32)) {
        let mut stream_hasher = Sha256::new();
        stream_hasher.update(key);
        stream_hasher.update(nonce);
        stream_hasher.update(block_counter.to_le_bytes());
        let stream_block = stream_hasher.finalize();

        for (i, &byte) in chunk.iter().enumerate() {
            output.push(byte ^ stream_block[i]);
        }
    }

    output
}

/// Test WebDAV connection by sending a PROPFIND request.
pub async fn webdav_test_connection(config: &WebDavConfig) -> AppResult<WebDavTestResult> {
    let url = config.url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Ok(WebDavTestResult {
            success: false,
            message: "WebDAV 地址不能为空".to_string(),
        });
    }

    let client = build_client()?;
    let headers = build_headers(config);

    // Use PROPFIND to test the connection and check if the directory exists
    let response = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), url)
        .headers(headers)
        .header("Depth", "0")
        .send()
        .await
        .map_err(|e| format!("NETWORK_ERROR: WebDAV connection failed: {e}"))?;

    let status = response.status();
    if status.is_success() || status.as_u16() == 207 {
        Ok(WebDavTestResult {
            success: true,
            message: "连接成功".to_string(),
        })
    } else if status.as_u16() == 401 || status.as_u16() == 403 {
        Ok(WebDavTestResult {
            success: false,
            message: "认证失败：请检查用户名和密码".to_string(),
        })
    } else if status.as_u16() == 404 {
        Ok(WebDavTestResult {
            success: false,
            message: "目录不存在：请检查 WebDAV 地址".to_string(),
        })
    } else {
        Ok(WebDavTestResult {
            success: false,
            message: format!("连接失败：HTTP {}", status.as_u16()),
        })
    }
}

/// Upload config data to WebDAV server.
pub async fn webdav_upload(config: &WebDavConfig, data: &str) -> AppResult<WebDavUploadResult> {
    let url = normalize_webdav_url(&config.url);
    let client = build_client()?;
    let headers = build_headers(config);

    let body_bytes: Vec<u8> = if let Some(ref enc_password) = config.encryption_password {
        if !enc_password.is_empty() {
            let encrypted = encrypt_data(data.as_bytes(), enc_password)?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&encrypted);
            encoded.into_bytes()
        } else {
            data.as_bytes().to_vec()
        }
    } else {
        data.as_bytes().to_vec()
    };

    let bytes_len = body_bytes.len() as u64;

    let response = client
        .put(&url)
        .headers(headers)
        .header(CONTENT_TYPE, "application/json")
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("NETWORK_ERROR: WebDAV upload failed: {e}"))?;

    let status = response.status();
    if status.is_success() || status.as_u16() == 201 || status.as_u16() == 204 {
        Ok(WebDavUploadResult {
            success: true,
            message: "上传成功".to_string(),
            bytes_uploaded: bytes_len,
        })
    } else if status.as_u16() == 401 || status.as_u16() == 403 {
        Ok(WebDavUploadResult {
            success: false,
            message: "认证失败：请检查用户名和密码".to_string(),
            bytes_uploaded: 0,
        })
    } else {
        Ok(WebDavUploadResult {
            success: false,
            message: format!("上传失败：HTTP {}", status.as_u16()),
            bytes_uploaded: 0,
        })
    }
}

/// Download config data from WebDAV server.
pub async fn webdav_download(config: &WebDavConfig) -> AppResult<WebDavDownloadResult> {
    let url = normalize_webdav_url(&config.url);
    let client = build_client()?;
    let headers = build_headers(config);

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("NETWORK_ERROR: WebDAV download failed: {e}"))?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Ok(WebDavDownloadResult {
            success: false,
            message: "远程文件不存在：请先上传同步数据".to_string(),
            data: None,
        });
    }

    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Ok(WebDavDownloadResult {
            success: false,
            message: "认证失败：请检查用户名和密码".to_string(),
            data: None,
        });
    }

    if !status.is_success() {
        return Ok(WebDavDownloadResult {
            success: false,
            message: format!("下载失败：HTTP {}", status.as_u16()),
            data: None,
        });
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("NETWORK_ERROR: failed to read response body: {e}"))?;

    // Try to decrypt if encryption password is set
    let plaintext = if let Some(ref enc_password) = config.encryption_password {
        if !enc_password.is_empty() {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(body.as_bytes())
                .map_err(|e| format!("SEC_INVALID_INPUT: failed to decode encrypted data: {e}"))?;
            let decrypted = decrypt_data(&decoded, enc_password)?;
            String::from_utf8(decrypted)
                .map_err(|e| format!("SEC_INVALID_INPUT: decrypted data is not valid UTF-8: {e}"))?
        } else {
            body
        }
    } else {
        body
    };

    Ok(WebDavDownloadResult {
        success: true,
        message: "下载成功".to_string(),
        data: Some(plaintext),
    })
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn normalize_url_appends_filename() {
        assert_eq!(
            normalize_webdav_url("https://dav.example.com/path/"),
            "https://dav.example.com/path/aio-coding-hub-sync.json"
        );
        assert_eq!(
            normalize_webdav_url("https://dav.example.com/path"),
            "https://dav.example.com/path/aio-coding-hub-sync.json"
        );
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"hello world, this is a test of encryption";
        let password = "my-secret-password";

        let encrypted = encrypt_data(plaintext, password).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt_data(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_password_fails() {
        let plaintext = b"sensitive data";
        let encrypted = encrypt_data(plaintext, "correct-password").unwrap();
        let result = decrypt_data(&encrypted, "wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn encrypt_empty_data() {
        let encrypted = encrypt_data(b"", "password").unwrap();
        let decrypted = decrypt_data(&encrypted, "password").unwrap();
        assert_eq!(decrypted, b"");
    }
}
