use regex::Regex;
use std::sync::LazyLock;

static BILLING_HEADER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*x-anthropic-billing-header\s*:").unwrap());

#[derive(Debug, Clone, Copy)]
pub(super) struct BillingHeaderRectifierResult {
    pub(super) applied: bool,
    pub(super) removed_count: usize,
}

/// Preserve Claude Code identity on Anthropic/OAuth requests. Return a separate
/// body so a third-party provider's cleanup cannot leak into failover targets.
pub(super) fn rectify_for_provider(
    auth_mode: &str,
    base_url: &str,
    body: &[u8],
) -> Option<(Vec<u8>, usize)> {
    let is_anthropic = reqwest::Url::parse(base_url).ok().is_some_and(|url| {
        url.host_str()
            .is_some_and(|host| host.trim_end_matches('.') == "api.anthropic.com")
    });
    if auth_mode == "oauth" || is_anthropic {
        return None;
    }

    let mut root = serde_json::from_slice(body).ok()?;
    let result = rectify(&mut root);
    if !result.applied {
        return None;
    }
    Some((serde_json::to_vec(&root).ok()?, result.removed_count))
}

/// Remove `x-anthropic-billing-header` text blocks from the request body's `system` field.
///
/// Claude Code CLI v2.1.36+ injects these blocks into the system prompt. Non-Anthropic
/// upstreams (e.g. Amazon Bedrock) reject them with 400.
fn rectify(body: &mut serde_json::Value) -> BillingHeaderRectifierResult {
    let Some(obj) = body.as_object_mut() else {
        return BillingHeaderRectifierResult {
            applied: false,
            removed_count: 0,
        };
    };

    let Some(system) = obj.get_mut("system") else {
        return BillingHeaderRectifierResult {
            applied: false,
            removed_count: 0,
        };
    };

    // Case 1: system is a plain string
    if let Some(text) = system.as_str() {
        if BILLING_HEADER_PATTERN.is_match(text) {
            obj.remove("system");
            return BillingHeaderRectifierResult {
                applied: true,
                removed_count: 1,
            };
        }
        return BillingHeaderRectifierResult {
            applied: false,
            removed_count: 0,
        };
    }

    // Case 2: system is an array of content blocks
    if let Some(arr) = system.as_array_mut() {
        let original_len = arr.len();
        arr.retain(|block| {
            let Some(block_obj) = block.as_object() else {
                return true;
            };
            let is_text_block = block_obj.get("type").and_then(|v| v.as_str()) == Some("text");
            if !is_text_block {
                return true;
            }
            let Some(text) = block_obj.get("text").and_then(|v| v.as_str()) else {
                return true;
            };
            !BILLING_HEADER_PATTERN.is_match(text)
        });

        let removed_count = original_len - arr.len();
        return BillingHeaderRectifierResult {
            applied: removed_count > 0,
            removed_count,
        };
    }

    BillingHeaderRectifierResult {
        applied: false,
        removed_count: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn provider_cleanup_preserves_official_and_oauth_identity_after_failover() {
        for system in [
            json!("x-anthropic-billing-header: cc_version=test"),
            json!([
                {"type":"text", "text":"x-anthropic-billing-header: cc_version=test"},
                {"type":"text", "text":"Classify whether this command is safe.", "cache_control":{"type":"ephemeral"}}
            ]),
        ] {
            let original = serde_json::to_vec(&json!({
                "model":"claude-test", "system":system, "messages":[]
            }))
            .unwrap();
            let (third_party, removed) =
                rectify_for_provider("api_key", "https://proxy.example/v1", &original)
                    .expect("third-party cleanup");
            assert_eq!(removed, 1);
            assert!(!String::from_utf8_lossy(&third_party).contains("x-anthropic-billing-header"));
            for (auth_mode, base_url) in [
                ("oauth", "https://api.anthropic.com/v1"),
                ("oauth", "https://proxy.example/v1"),
                ("api_key", "https://api.anthropic.com"),
                ("api_key", "https://API.ANTHROPIC.COM/v1/"),
                ("api_key", "https://api.anthropic.com.:443/v1"),
            ] {
                assert!(rectify_for_provider(auth_mode, base_url, &original).is_none());
            }
            let source: serde_json::Value = serde_json::from_slice(&original).unwrap();
            assert_eq!(source["system"], system);
            if let Some(blocks) = system.as_array() {
                let cleaned: serde_json::Value = serde_json::from_slice(&third_party).unwrap();
                assert_eq!(cleaned["system"], json!([blocks[1]]));
            }
        }
    }

    #[test]
    fn official_host_check_does_not_match_paths_or_lookalike_hosts() {
        let body = br#"{"system":"x-anthropic-billing-header: test"}"#;
        for base_url in [
            "https://api.anthropic.com.proxy.example/v1",
            "https://proxy.example/api.anthropic.com",
            "https://api.anthropic.com@proxy.example/v1",
        ] {
            assert!(rectify_for_provider("api_key", base_url, body).is_some());
        }
        assert!(rectify_for_provider("api_key", "https://proxy.example", b"not json").is_none());
        assert!(rectify_for_provider(
            "api_key",
            "https://proxy.example",
            br#"{"system":"Keep me"}"#
        )
        .is_none());
    }

    #[test]
    fn system_string_matching_is_removed() {
        let mut body = json!({
            "model": "claude-3-5-sonnet",
            "system": "x-anthropic-billing-header: abc123",
            "messages": []
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_count, 1);
        assert!(body.get("system").is_none());
    }

    #[test]
    fn system_string_not_matching_is_kept() {
        let mut body = json!({
            "model": "claude-3-5-sonnet",
            "system": "You are a helpful assistant.",
            "messages": []
        });

        let result = rectify(&mut body);

        assert!(!result.applied);
        assert_eq!(result.removed_count, 0);
        assert!(body.get("system").is_some());
    }

    #[test]
    fn system_array_filters_matching_blocks() {
        let mut body = json!({
            "model": "claude-3-5-sonnet",
            "system": [
                {"type": "text", "text": "You are a helpful assistant."},
                {"type": "text", "text": "x-anthropic-billing-header: abc123"},
                {"type": "text", "text": "Be concise."}
            ],
            "messages": []
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_count, 1);
        let system = body.get("system").unwrap().as_array().unwrap();
        assert_eq!(system.len(), 2);
        assert_eq!(
            system[0].get("text").unwrap().as_str().unwrap(),
            "You are a helpful assistant."
        );
        assert_eq!(
            system[1].get("text").unwrap().as_str().unwrap(),
            "Be concise."
        );
    }

    #[test]
    fn system_array_no_match_is_unchanged() {
        let mut body = json!({
            "model": "claude-3-5-sonnet",
            "system": [
                {"type": "text", "text": "You are a helpful assistant."},
                {"type": "text", "text": "Be concise."}
            ],
            "messages": []
        });

        let result = rectify(&mut body);

        assert!(!result.applied);
        assert_eq!(result.removed_count, 0);
        assert_eq!(body.get("system").unwrap().as_array().unwrap().len(), 2);
    }

    #[test]
    fn system_absent_is_noop() {
        let mut body = json!({
            "model": "claude-3-5-sonnet",
            "messages": []
        });

        let result = rectify(&mut body);

        assert!(!result.applied);
        assert_eq!(result.removed_count, 0);
    }

    #[test]
    fn non_text_blocks_are_preserved() {
        let mut body = json!({
            "system": [
                {"type": "text", "text": "  X-Anthropic-Billing-Header: val"},
                {"type": "image", "source": {"type": "base64"}}
            ]
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_count, 1);
        let system = body.get("system").unwrap().as_array().unwrap();
        assert_eq!(system.len(), 1);
        assert_eq!(system[0].get("type").unwrap().as_str().unwrap(), "image");
    }

    #[test]
    fn case_insensitive_matching() {
        let mut body = json!({
            "system": "  X-ANTHROPIC-BILLING-HEADER: something"
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_count, 1);
    }

    #[test]
    fn body_not_object_is_noop() {
        let mut body = json!("just a string");

        let result = rectify(&mut body);

        assert!(!result.applied);
        assert_eq!(result.removed_count, 0);
    }

    #[test]
    fn multiple_billing_blocks_are_all_removed() {
        let mut body = json!({
            "system": [
                {"type": "text", "text": "x-anthropic-billing-header: val1"},
                {"type": "text", "text": "Keep this."},
                {"type": "text", "text": "x-anthropic-billing-header: val2"}
            ]
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.removed_count, 2);
        let system = body.get("system").unwrap().as_array().unwrap();
        assert_eq!(system.len(), 1);
        assert_eq!(
            system[0].get("text").unwrap().as_str().unwrap(),
            "Keep this."
        );
    }
}
