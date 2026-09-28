use sha2::{Digest, Sha256};

pub(crate) const CLAUDE_METADATA_USER_ID_JSON_SWITCH_VERSION: (u32, u32, u32) = (2, 1, 78);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaudeMetadataUserIdFormat {
    Legacy,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaudeMetadataUserIdParseResult {
    pub(crate) session_id: Option<String>,
    pub(crate) format: Option<ClaudeMetadataUserIdFormat>,
    pub(crate) device_id: Option<String>,
    pub(crate) account_uuid: Option<String>,
}

pub(super) struct ClaudeMetadataUserIdInjectionSkip {
    pub(super) reason: &'static str,
    pub(super) error: Option<String>,
}

pub(super) enum ClaudeMetadataUserIdInjectionOutcome {
    Injected { body_bytes: Vec<u8> },
    Skipped(ClaudeMetadataUserIdInjectionSkip),
}

pub(super) fn inject_from_json_bytes_with_ua(
    provider_id: i64,
    session_id: Option<&str>,
    body_bytes: &[u8],
    user_agent: Option<&str>,
) -> ClaudeMetadataUserIdInjectionOutcome {
    let Some(session_id) = session_id.map(str::trim).filter(|v| !v.is_empty()) else {
        return ClaudeMetadataUserIdInjectionOutcome::Skipped(ClaudeMetadataUserIdInjectionSkip {
            reason: "missing_session_id",
            error: None,
        });
    };

    let mut root = match serde_json::from_slice::<serde_json::Value>(body_bytes) {
        Ok(root) => root,
        Err(err) => {
            return ClaudeMetadataUserIdInjectionOutcome::Skipped(
                ClaudeMetadataUserIdInjectionSkip {
                    reason: "missing_body_json",
                    error: Some(err.to_string()),
                },
            );
        }
    };

    let Some(root_obj) = root.as_object_mut() else {
        return ClaudeMetadataUserIdInjectionOutcome::Skipped(ClaudeMetadataUserIdInjectionSkip {
            reason: "body_json_not_object",
            error: None,
        });
    };

    let user_id_exists = root_obj
        .get("metadata")
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("user_id"))
        .is_some_and(has_usable_user_id);
    if user_id_exists {
        return ClaudeMetadataUserIdInjectionOutcome::Skipped(ClaudeMetadataUserIdInjectionSkip {
            reason: "already_exists",
            error: None,
        });
    }

    let stable_hash = stable_hash_for_key(provider_id);
    let use_json_format = should_use_json_format(user_agent);
    let user_id = if use_json_format {
        format_user_id_json(&stable_hash, session_id)
    } else {
        format_user_id_legacy(&stable_hash, session_id)
    };

    let metadata = root_obj
        .entry("metadata")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !metadata.is_object() {
        *metadata = serde_json::Value::Object(serde_json::Map::new());
    }
    let meta_obj = metadata
        .as_object_mut()
        .expect("metadata must be an object");
    meta_obj.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.clone()),
    );

    match serde_json::to_vec(&root) {
        Ok(body_bytes) => ClaudeMetadataUserIdInjectionOutcome::Injected { body_bytes },
        Err(err) => {
            ClaudeMetadataUserIdInjectionOutcome::Skipped(ClaudeMetadataUserIdInjectionSkip {
                reason: "serialize_failed",
                error: Some(err.to_string()),
            })
        }
    }
}

fn format_user_id_legacy(stable_hash: &str, session_id: &str) -> String {
    format!("user_{stable_hash}_account__session_{session_id}")
}

fn format_user_id_json(stable_hash: &str, session_id: &str) -> String {
    // JSON format used by Claude Code CLI v2.1.78+: embed structured data as a JSON string.
    let obj = serde_json::json!({
        "device_id": stable_hash,
        "account_uuid": "",
        "session_id": session_id,
    });
    obj.to_string()
}

fn stable_hash_for_key(provider_id: i64) -> String {
    let seed = format!("claude_user_{provider_id}");
    let digest = Sha256::digest(seed.as_bytes());
    format!("{digest:x}")
}

/// Detect whether the CLI version supports JSON-format user_id.
/// Claude Code CLI v2.1.36+ uses JSON format; older versions use legacy string concatenation.
fn should_use_json_format(user_agent: Option<&str>) -> bool {
    parse_claude_client_version(user_agent)
        .map(|version| version >= CLAUDE_METADATA_USER_ID_JSON_SWITCH_VERSION)
        .unwrap_or(true)
}

fn parse_claude_client_version(user_agent: Option<&str>) -> Option<(u32, u32, u32)> {
    let ua = user_agent?.trim();
    let lower = ua.to_ascii_lowercase();
    let start = ["claude-cli/", "claude-vscode/", "claude-code/"]
        .iter()
        .filter_map(|prefix| lower.find(prefix).map(|offset| offset + prefix.len()))
        .min()?;
    let version_str = ua[start..].split_whitespace().next()?;
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts[2].parse().ok()?;
    Some((major, minor, patch))
}

fn has_usable_user_id(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    }
}

pub(crate) fn parse_claude_metadata_user_id(user_id: &str) -> ClaudeMetadataUserIdParseResult {
    let empty = || ClaudeMetadataUserIdParseResult {
        session_id: None,
        format: None,
        device_id: None,
        account_uuid: None,
    };
    let trimmed = user_id.trim();
    if trimmed.is_empty() || trimmed.len() > 4096 {
        return empty();
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(object) = parsed.as_object() {
            let session_id = object
                .get("session_id")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            if session_id.is_some() {
                return ClaudeMetadataUserIdParseResult {
                    session_id,
                    format: Some(ClaudeMetadataUserIdFormat::Json),
                    device_id: object
                        .get("device_id")
                        .and_then(|value| value.as_str())
                        .map(ToOwned::to_owned),
                    account_uuid: object
                        .get("account_uuid")
                        .and_then(|value| value.as_str())
                        .map(ToOwned::to_owned),
                };
            }
        }
    }

    let Some(rest) = trimmed.strip_prefix("user_") else {
        return empty();
    };
    let Some((device_id, raw_session_id)) = rest.split_once("_account__session_") else {
        return empty();
    };
    let session_id = raw_session_id.trim();
    if device_id.is_empty() || session_id.is_empty() {
        return empty();
    }

    ClaudeMetadataUserIdParseResult {
        session_id: Some(session_id.to_string()),
        format: Some(ClaudeMetadataUserIdFormat::Legacy),
        device_id: Some(device_id.to_string()),
        account_uuid: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        inject_from_json_bytes_with_ua, parse_claude_client_version, parse_claude_metadata_user_id,
        should_use_json_format, ClaudeMetadataUserIdFormat, ClaudeMetadataUserIdInjectionOutcome,
    };
    use sha2::{Digest, Sha256};

    fn expected_hash(provider_id: i64) -> String {
        let seed = format!("claude_user_{provider_id}");
        let digest = Sha256::digest(seed.as_bytes());
        format!("{digest:x}")
    }

    #[test]
    fn injects_user_id_when_missing() {
        let body = serde_json::json!({
            "model": "claude-3-5-sonnet",
            "messages": [],
        });
        let provider_id = 123;
        let session_id = "sess-1";
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let outcome =
            inject_from_json_bytes_with_ua(provider_id, Some(session_id), encoded.as_slice(), None);

        let ClaudeMetadataUserIdInjectionOutcome::Injected { body_bytes } = outcome else {
            panic!("expected injected outcome");
        };

        let next: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("injected body should be json");
        let user_id = next
            .get("metadata")
            .and_then(|v| v.get("user_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let parsed: serde_json::Value = serde_json::from_str(user_id).expect("CCH metadata JSON");
        assert_eq!(parsed["device_id"], expected_hash(provider_id));
        assert_eq!(parsed["account_uuid"], "");
        assert_eq!(parsed["session_id"], session_id);
    }

    #[test]
    fn injects_json_format_for_new_cli_version() {
        let body = serde_json::json!({
            "model": "claude-3-5-sonnet",
            "messages": [],
        });
        let provider_id = 123;
        let session_id = "sess-1";
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let outcome = inject_from_json_bytes_with_ua(
            provider_id,
            Some(session_id),
            encoded.as_slice(),
            Some("claude-cli/2.1.78"),
        );

        let ClaudeMetadataUserIdInjectionOutcome::Injected { body_bytes } = outcome else {
            panic!("expected injected outcome");
        };

        let next: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("injected body should be json");
        let user_id_str = next
            .get("metadata")
            .and_then(|v| v.get("user_id"))
            .and_then(|v| v.as_str())
            .expect("user_id should be a string");

        // JSON format: the user_id value is itself a JSON string.
        let parsed: serde_json::Value =
            serde_json::from_str(user_id_str).expect("user_id should be valid JSON");
        assert!(parsed.get("device_id").is_some());
        assert!(parsed.get("account_uuid").is_some());
        assert!(parsed.get("session_id").is_some());
        assert_eq!(
            parsed.get("session_id").and_then(|v| v.as_str()),
            Some(session_id)
        );
    }

    #[test]
    fn injects_legacy_format_for_old_cli_version() {
        let body = serde_json::json!({
            "model": "claude-3-5-sonnet",
            "messages": [],
        });
        let provider_id = 123;
        let session_id = "sess-1";
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let outcome = inject_from_json_bytes_with_ua(
            provider_id,
            Some(session_id),
            encoded.as_slice(),
            Some("claude-cli/2.1.77"),
        );

        let ClaudeMetadataUserIdInjectionOutcome::Injected { body_bytes } = outcome else {
            panic!("expected injected outcome");
        };

        let next: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("injected body should be json");
        let user_id = next
            .get("metadata")
            .and_then(|v| v.get("user_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let stable_hash = expected_hash(provider_id);
        assert_eq!(
            user_id,
            format!("user_{stable_hash}_account__session_{session_id}")
        );
    }

    #[test]
    fn skips_when_user_id_already_exists() {
        let body = serde_json::json!({
            "model": "claude-3-5-sonnet",
            "messages": [],
            "metadata": {
                "user_id": "existing"
            }
        });
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let outcome = inject_from_json_bytes_with_ua(1, Some("sess-1"), encoded.as_slice(), None);

        let ClaudeMetadataUserIdInjectionOutcome::Skipped(skip) = outcome else {
            panic!("expected skipped outcome");
        };
        assert_eq!(skip.reason, "already_exists");
    }

    #[test]
    fn skips_when_session_id_missing() {
        let body = serde_json::json!({
            "messages": [],
        });
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let outcome = inject_from_json_bytes_with_ua(1, None, encoded.as_slice(), None);
        let ClaudeMetadataUserIdInjectionOutcome::Skipped(skip) = outcome else {
            panic!("expected skipped outcome");
        };
        assert_eq!(skip.reason, "missing_session_id");
    }

    #[test]
    fn skips_when_body_is_not_json() {
        let outcome = inject_from_json_bytes_with_ua(1, Some("sess-1"), b"not-json", None);
        let ClaudeMetadataUserIdInjectionOutcome::Skipped(skip) = outcome else {
            panic!("expected skipped outcome");
        };
        assert_eq!(skip.reason, "missing_body_json");
        assert!(skip.error.is_some());
    }

    #[test]
    fn parse_claude_client_version_extracts_version() {
        assert_eq!(
            parse_claude_client_version(Some("claude-cli/2.1.78 node/20.0.0")),
            Some((2, 1, 78))
        );
        assert_eq!(
            parse_claude_client_version(Some("claude-vscode/2.1.77")),
            Some((2, 1, 77))
        );
        assert_eq!(
            parse_claude_client_version(Some("Mozilla/5.0 claude-code/3.0.0")),
            Some((3, 0, 0))
        );
    }

    #[test]
    fn parse_claude_client_version_returns_none_for_non_claude_ua() {
        assert_eq!(parse_claude_client_version(Some("codex-cli/1.0.0")), None);
        assert_eq!(parse_claude_client_version(Some("")), None);
    }

    #[test]
    fn should_use_json_format_for_new_versions() {
        assert!(should_use_json_format(Some("claude-cli/2.1.78")));
        assert!(should_use_json_format(Some("claude-code/2.2.0")));
        assert!(should_use_json_format(Some("claude-code/3.0.0")));
    }

    #[test]
    fn should_use_legacy_format_for_old_versions() {
        assert!(!should_use_json_format(Some("claude-cli/2.1.77")));
        assert!(!should_use_json_format(Some("claude-code/2.0.0")));
        assert!(!should_use_json_format(Some("claude-code/1.0.0")));
        assert!(should_use_json_format(None));
    }

    #[test]
    fn blank_user_id_is_replaced_but_non_string_values_are_preserved() {
        let blank = serde_json::json!({"metadata": {"user_id": "  "}});
        let encoded = serde_json::to_vec(&blank).expect("serialize");
        assert!(matches!(
            inject_from_json_bytes_with_ua(1, Some("s"), &encoded, Some("claude-cli/2.1.78")),
            ClaudeMetadataUserIdInjectionOutcome::Injected { .. }
        ));

        let non_string = serde_json::json!({"metadata": {"user_id": {"keep": true}}});
        let encoded = serde_json::to_vec(&non_string).expect("serialize");
        let outcome = inject_from_json_bytes_with_ua(1, Some("s"), &encoded, None);
        let ClaudeMetadataUserIdInjectionOutcome::Skipped(skip) = outcome else {
            panic!("expected preserved non-string user id");
        };
        assert_eq!(skip.reason, "already_exists");
    }

    #[test]
    fn parses_json_and_legacy_session_ids() {
        let json = parse_claude_metadata_user_id(
            r#"{"device_id":"device","account_uuid":"acct","session_id":"json-session"}"#,
        );
        assert_eq!(json.session_id.as_deref(), Some("json-session"));
        assert_eq!(json.format, Some(ClaudeMetadataUserIdFormat::Json));
        assert_eq!(json.device_id.as_deref(), Some("device"));
        assert_eq!(json.account_uuid.as_deref(), Some("acct"));

        let legacy = parse_claude_metadata_user_id("user_device_account__session_legacy-session");
        assert_eq!(legacy.session_id.as_deref(), Some("legacy-session"));
        assert_eq!(legacy.format, Some(ClaudeMetadataUserIdFormat::Legacy));
    }

    #[test]
    fn unknown_user_agents_default_to_json_format() {
        assert!(should_use_json_format(Some("Mozilla/5.0")));
    }
}
