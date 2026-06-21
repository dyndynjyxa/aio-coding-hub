//! Usage: Plugin market index parsing and compatibility evaluation.

use crate::shared::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginMarketListing {
    pub(crate) plugin_id: String,
    pub(crate) name: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) download_url: Option<String>,
    pub(crate) checksum: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) risk_labels: Vec<String>,
    pub(crate) revoked: bool,
    pub(crate) compatible: bool,
    pub(crate) update_available: bool,
    pub(crate) install_block_reason: Option<String>,
}

pub(crate) fn parse_market_index(
    bytes: &[u8],
    host_version: &str,
    installed_versions: &HashMap<String, String>,
) -> AppResult<Vec<PluginMarketListing>> {
    let raw: RawMarketIndex = serde_json::from_slice(bytes).map_err(|error| {
        AppError::new(
            "PLUGIN_MARKET_INVALID_INDEX",
            format!("failed to parse plugin market index: {error}"),
        )
    })?;

    let mut listings = Vec::with_capacity(raw.plugins.len());
    for plugin in raw.plugins {
        validate_market_plugin_id(&plugin.id)?;
        let revoked = plugin.revoked.unwrap_or(false);
        let mut sorted_versions = plugin.versions;
        sorted_versions.sort_by(|a, b| compare_semver(&a.version, &b.version));

        let mut latest_any = None;
        for version in &sorted_versions {
            validate_market_version(version)?;
            latest_any = Some(version);
        }

        let compatible_version = sorted_versions
            .iter()
            .rev()
            .find(|version| version.compatibility.is_compatible(host_version));

        let selected = compatible_version.or(latest_any);
        let latest_version = selected.map(|version| version.version.clone());
        let installed = installed_versions.get(&plugin.id);
        let update_available = selected.zip(installed).is_some_and(|(version, installed)| {
            compare_semver(installed, &version.version).is_lt()
        });

        let install_block_reason = if revoked {
            Some("revoked".to_string())
        } else if compatible_version.is_none() {
            Some("incompatible".to_string())
        } else {
            None
        };
        let compatible = install_block_reason.is_none();

        listings.push(PluginMarketListing {
            plugin_id: plugin.id,
            name: plugin.name,
            latest_version,
            download_url: selected.map(|version| version.download_url.clone()),
            checksum: selected.map(|version| version.checksum.clone()),
            signature: selected.and_then(|version| version.signature.clone()),
            risk_labels: plugin.risk_labels,
            revoked,
            compatible,
            update_available,
            install_block_reason,
        });
    }

    Ok(listings)
}

pub(crate) fn parse_signed_market_index(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
    host_version: &str,
    installed_versions: &HashMap<String, String>,
) -> AppResult<Vec<PluginMarketListing>> {
    crate::infra::plugins::signing::verify_ed25519_signature(bytes, signature, public_key)?;
    parse_market_index(bytes, host_version, installed_versions)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMarketIndex {
    #[allow(dead_code)]
    schema_version: String,
    plugins: Vec<RawMarketPlugin>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMarketPlugin {
    id: String,
    name: String,
    #[serde(default)]
    risk_labels: Vec<String>,
    #[serde(default)]
    revoked: Option<bool>,
    versions: Vec<RawMarketVersion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMarketVersion {
    version: String,
    download_url: String,
    checksum: String,
    #[serde(default)]
    signature: Option<String>,
    #[serde(rename = "hostCompatibility")]
    compatibility: RawHostCompatibility,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawHostCompatibility {
    app: String,
    #[serde(rename = "pluginApi")]
    plugin_api: String,
    #[allow(dead_code)]
    #[serde(default)]
    platforms: Vec<String>,
}

impl RawHostCompatibility {
    fn is_compatible(&self, host_version: &str) -> bool {
        matches_version_range(&self.app, host_version)
            && plugin_api_is_supported(self.plugin_api.as_str())
    }
}

fn validate_market_plugin_id(plugin_id: &str) -> AppResult<()> {
    if crate::app_paths::plugin_id_path_segment(plugin_id).is_err() || !plugin_id.contains('.') {
        return Err(AppError::new(
            "PLUGIN_MARKET_INVALID_PLUGIN_ID",
            format!("invalid plugin id in market index: {plugin_id}"),
        ));
    }
    Ok(())
}

fn validate_market_version(version: &RawMarketVersion) -> AppResult<()> {
    if parse_semver(&version.version).is_none() {
        return Err(AppError::new(
            "PLUGIN_MARKET_INVALID_VERSION",
            format!(
                "invalid plugin version in market index: {}",
                version.version
            ),
        ));
    }
    if !is_valid_checksum(&version.checksum) {
        return Err(AppError::new(
            "PLUGIN_MARKET_INVALID_CHECKSUM",
            format!("invalid checksum for plugin version {}", version.version),
        ));
    }
    if !(version.download_url.starts_with("https://")
        || version.download_url.starts_with("file://"))
    {
        return Err(AppError::new(
            "PLUGIN_MARKET_INVALID_DOWNLOAD_URL",
            format!("unsupported plugin download URL: {}", version.download_url),
        ));
    }
    Ok(())
}

fn is_valid_checksum(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn plugin_api_is_supported(range: &str) -> bool {
    let range = range.trim();
    if let Some(required) = range.strip_prefix('^') {
        return parse_semver(required).is_some_and(|(major, _, _)| major == 1);
    }
    parse_semver(range).is_some_and(|(major, _, _)| major == 1)
}

fn matches_version_range(range: &str, host_version: &str) -> bool {
    let Some(version) = parse_semver(host_version) else {
        return false;
    };
    let mut saw_constraint = false;
    for part in range.split_whitespace() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        saw_constraint = true;
        if let Some(raw) = part.strip_prefix(">=") {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version < bound {
                return false;
            }
        } else if let Some(raw) = part.strip_prefix('<') {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version >= bound {
                return false;
            }
        } else if let Some(raw) = part.strip_prefix('^') {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version < bound || version.0 != bound.0 {
                return false;
            }
        } else {
            let Some(bound) = parse_semver(part) else {
                return false;
            };
            if version != bound {
                return false;
            }
        }
    }
    saw_constraint
}

fn compare_semver(left: &str, right: &str) -> std::cmp::Ordering {
    parse_semver(left)
        .unwrap_or((0, 0, 0))
        .cmp(&parse_semver(right).unwrap_or((0, 0, 0)))
}

fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let core = version
        .trim()
        .split_once('-')
        .map_or(version.trim(), |(core, _)| core);
    let mut parts = core.split('.');
    let major = parse_semver_number(parts.next()?)?;
    let minor = parse_semver_number(parts.next()?)?;
    let patch = parse_semver_number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn parse_semver_number(value: &str) -> Option<u64> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return None;
    }
    value.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checksum(seed: u8) -> String {
        format!("sha256:{:064x}", seed)
    }

    fn market_index() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": "1.0.0",
            "plugins": [
                {
                    "id": "community.prompt-tools",
                    "name": "Prompt Tools",
                    "riskLabels": ["modifies_request_body"],
                    "versions": [
                        {
                            "version": "1.0.0",
                            "downloadUrl": "https://plugins.example.test/prompt-tools-1.0.0.aio-plugin",
                            "checksum": checksum(1),
                            "signature": "base64-signature-v1",
                            "hostCompatibility": {
                                "app": ">=0.56.0 <1.0.0",
                                "pluginApi": "^1.0.0",
                                "platforms": ["macos", "windows", "linux"]
                            }
                        },
                        {
                            "version": "1.1.0",
                            "downloadUrl": "https://plugins.example.test/prompt-tools-1.1.0.aio-plugin",
                            "checksum": checksum(2),
                            "signature": "base64-signature-v2",
                            "hostCompatibility": {
                                "app": ">=0.56.0 <1.0.0",
                                "pluginApi": "^1.0.0",
                                "platforms": ["macos", "windows", "linux"]
                            }
                        }
                    ]
                },
                {
                    "id": "community.future-only",
                    "name": "Future Only",
                    "riskLabels": [],
                    "versions": [
                        {
                            "version": "2.0.0",
                            "downloadUrl": "https://plugins.example.test/future-only.aio-plugin",
                            "checksum": checksum(3),
                            "signature": "base64-signature-v3",
                            "hostCompatibility": {
                                "app": ">=9.0.0",
                                "pluginApi": "^1.0.0",
                                "platforms": ["macos", "windows", "linux"]
                            }
                        }
                    ]
                },
                {
                    "id": "community.revoked",
                    "name": "Revoked Plugin",
                    "riskLabels": ["revoked"],
                    "revoked": true,
                    "versions": [
                        {
                            "version": "1.0.0",
                            "downloadUrl": "https://plugins.example.test/revoked.aio-plugin",
                            "checksum": checksum(4),
                            "signature": "base64-signature-v4",
                            "hostCompatibility": {
                                "app": ">=0.56.0 <1.0.0",
                                "pluginApi": "^1.0.0",
                                "platforms": ["macos", "windows", "linux"]
                            }
                        }
                    ]
                }
            ]
        })
    }

    #[test]
    fn plugin_market_index_lists_compatible_plugins_and_detects_updates() {
        let mut installed = HashMap::new();
        installed.insert("community.prompt-tools".to_string(), "1.0.0".to_string());

        let listings =
            parse_market_index(market_index().to_string().as_bytes(), "0.56.0", &installed)
                .unwrap();

        let prompt_tools = listings
            .iter()
            .find(|item| item.plugin_id == "community.prompt-tools")
            .unwrap();
        assert_eq!(prompt_tools.latest_version.as_deref(), Some("1.1.0"));
        assert_eq!(
            prompt_tools.download_url.as_deref(),
            Some("https://plugins.example.test/prompt-tools-1.1.0.aio-plugin")
        );
        assert!(prompt_tools.compatible);
        assert!(prompt_tools.update_available);
        assert_eq!(prompt_tools.install_block_reason, None);
    }

    #[test]
    fn plugin_market_index_marks_incompatible_plugins_as_blocked() {
        let listings = parse_market_index(
            market_index().to_string().as_bytes(),
            "0.56.0",
            &HashMap::new(),
        )
        .unwrap();

        let future = listings
            .iter()
            .find(|item| item.plugin_id == "community.future-only")
            .unwrap();
        assert!(!future.compatible);
        assert_eq!(future.install_block_reason.as_deref(), Some("incompatible"));
    }

    #[test]
    fn plugin_market_index_marks_revoked_plugins_as_blocked() {
        let listings = parse_market_index(
            market_index().to_string().as_bytes(),
            "0.56.0",
            &HashMap::new(),
        )
        .unwrap();

        let revoked = listings
            .iter()
            .find(|item| item.plugin_id == "community.revoked")
            .unwrap();
        assert!(revoked.revoked);
        assert!(!revoked.compatible);
        assert_eq!(revoked.install_block_reason.as_deref(), Some("revoked"));
    }

    #[test]
    fn plugin_market_index_rejects_invalid_checksum() {
        let mut raw = market_index();
        raw["plugins"][0]["versions"][1]["checksum"] = serde_json::json!("sha256:not-hex");

        let err =
            parse_market_index(raw.to_string().as_bytes(), "0.56.0", &HashMap::new()).unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_MARKET_INVALID_CHECKSUM:"));
    }

    #[test]
    fn plugin_market_index_verifies_ed25519_signature() {
        use base64::Engine;
        use ed25519_dalek::Signer;

        let bytes = market_index().to_string().into_bytes();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        let signature_b64 =
            base64::engine::general_purpose::STANDARD.encode(signing_key.sign(&bytes).to_bytes());
        let public_key_b64 = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());

        let listings = parse_signed_market_index(
            &bytes,
            &signature_b64,
            &public_key_b64,
            "0.56.0",
            &HashMap::new(),
        )
        .unwrap();

        assert!(listings
            .iter()
            .any(|item| item.plugin_id == "community.prompt-tools"));

        let err = parse_signed_market_index(
            b"{\"schemaVersion\":\"1.0.0\",\"plugins\":[]}",
            &signature_b64,
            &public_key_b64,
            "0.56.0",
            &HashMap::new(),
        )
        .unwrap_err();
        assert!(err.to_string().starts_with("PLUGIN_SIGNATURE_INVALID:"));
    }
}
