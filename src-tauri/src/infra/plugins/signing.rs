//! Usage: Plugin package checksum and signature verification helpers.

use crate::shared::error::{AppError, AppResult};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

pub(crate) fn verify_checksum(bytes: &[u8], expected: &str) -> AppResult<String> {
    let expected = expected.trim().to_ascii_lowercase();
    if !is_valid_sha256_checksum(&expected) {
        return Err(AppError::new(
            "PLUGIN_CHECKSUM_INVALID_FORMAT",
            "expected checksum must be sha256:<64 hex chars>",
        ));
    }
    let actual = format!("sha256:{:x}", Sha256::digest(bytes));
    if crate::shared::security::constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
        Ok(actual)
    } else {
        Err(AppError::new(
            "PLUGIN_CHECKSUM_MISMATCH",
            "plugin package checksum does not match market index",
        ))
    }
}

pub(crate) fn verify_ed25519_signature(
    bytes: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> AppResult<()> {
    let signature_bytes = decode_base64_exact(signature_b64, 64, "signature")?;
    let public_key_bytes = decode_base64_exact(public_key_b64, 32, "public key")?;
    let key_array: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
        AppError::new(
            "PLUGIN_SIGNATURE_INVALID_FORMAT",
            "Ed25519 public key must be 32 bytes",
        )
    })?;
    let verifying_key = VerifyingKey::from_bytes(&key_array).map_err(|_| {
        AppError::new(
            "PLUGIN_SIGNATURE_INVALID_FORMAT",
            "Ed25519 public key is invalid",
        )
    })?;
    let signature_array: [u8; 64] = signature_bytes.try_into().map_err(|_| {
        AppError::new(
            "PLUGIN_SIGNATURE_INVALID_FORMAT",
            "Ed25519 signature must be 64 bytes",
        )
    })?;
    let signature = Signature::from_bytes(&signature_array);
    verifying_key.verify(bytes, &signature).map_err(|_| {
        AppError::new(
            "PLUGIN_SIGNATURE_INVALID",
            "plugin package signature verification failed",
        )
    })
}

fn decode_base64_exact(value: &str, expected_len: usize, label: &str) -> AppResult<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|_| {
            AppError::new(
                "PLUGIN_SIGNATURE_INVALID_FORMAT",
                format!("Ed25519 {label} must be base64"),
            )
        })?;
    if bytes.len() == expected_len {
        Ok(bytes)
    } else {
        Err(AppError::new(
            "PLUGIN_SIGNATURE_INVALID_FORMAT",
            format!("Ed25519 {label} must be {expected_len} bytes"),
        ))
    }
}

fn is_valid_sha256_checksum(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_signature_verification_accepts_matching_checksum() {
        let actual = verify_checksum(
            b"plugin package bytes",
            "sha256:66138673fd1fe915e0fc26c23bad6afe1e99b6d44875b8e0efbade87c00ea36a",
        )
        .unwrap();

        assert_eq!(
            actual,
            "sha256:66138673fd1fe915e0fc26c23bad6afe1e99b6d44875b8e0efbade87c00ea36a"
        );
    }

    #[test]
    fn plugin_signature_verification_rejects_checksum_mismatch() {
        let err = verify_checksum(
            b"plugin package bytes",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_CHECKSUM_MISMATCH:"));
    }

    #[test]
    fn plugin_signature_verification_accepts_valid_ed25519_signature() {
        // RFC 8032 test vector 1.
        let public_key = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";
        let signature = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
        let signature_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            hex_to_bytes(signature).as_slice(),
        );

        verify_ed25519_signature(b"", &signature_b64, public_key).unwrap();
    }

    #[test]
    fn plugin_signature_verification_rejects_invalid_ed25519_signature() {
        let public_key = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";
        let signature = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
        let signature_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            hex_to_bytes(signature).as_slice(),
        );

        let err = verify_ed25519_signature(b"tampered", &signature_b64, public_key).unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_SIGNATURE_INVALID:"));
    }

    #[test]
    fn plugin_signature_verification_rejects_revoked_key() {
        let err = verify_ed25519_signature(b"payload", "not-base64", "not-base64").unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_SIGNATURE_INVALID_FORMAT:"));
    }

    fn hex_to_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
