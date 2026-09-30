//! Standalone Codex search has its own protocol, independent of Responses.

use super::request_body::GatewayRequestBody;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde_json::Value;

pub(super) fn is_request(cli_key: &str, method: &Method, forwarded_path: &str) -> bool {
    cli_key == "codex"
        && method == Method::POST
        && matches!(
            forwarded_path.trim_end_matches('/'),
            "/alpha/search" | "/v1/alpha/search" | "/codex/alpha/search" | "/v1/codex/alpha/search"
        )
}

pub(super) fn session_id(root: Option<&Value>) -> Option<String> {
    crate::gateway::session_manager::sanitize_session_id(root?.get("id")?.as_str()?)
}

/// Preserve unknown search fields and clean only known Responses-only fields/headers.
/// The returned audit marker contains names, never request values.
pub(super) fn sanitize(headers: &mut HeaderMap, body: &mut GatewayRequestBody) -> Option<Value> {
    let removed_headers: Vec<_> = [
        "openai-beta",
        "session_id",
        "x-session-id",
        "conversation_id",
        "x-codex-beta-features",
        "x-codex-turn-state",
        "x-openai-internal-codex-responses-lite",
    ]
    .into_iter()
    .filter(|name| headers.remove(*name).is_some())
    .collect();

    let mut removed_fields = Vec::new();
    if let Ok(mut root) = serde_json::from_slice::<Value>(body.decoded()) {
        if let Some(object) = root.as_object_mut() {
            for field in ["prompt_cache_key", "prompt_cache_retention", "store"] {
                if object.remove(field).is_some() {
                    removed_fields.push(field);
                }
            }
        }
        if !removed_fields.is_empty() {
            body.replace_decoded(Bytes::from(root.to_string()));
        }
    }

    if removed_fields.is_empty() && removed_headers.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "type": "codex_alpha_search_compat",
        "scope": "request",
        "hit": true,
        "removedFields": removed_fields,
        "removedHeaders": removed_headers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_search_matches_only_codex_post_search_paths() {
        for path in [
            "/alpha/search",
            "/v1/alpha/search",
            "/codex/alpha/search",
            "/v1/codex/alpha/search",
        ] {
            assert!(is_request("codex", &Method::POST, path));
            assert!(is_request("codex", &Method::POST, &format!("{path}/")));
            assert!(!is_request("claude", &Method::POST, path));
            assert!(!is_request("codex", &Method::GET, path));
        }
        for path in [
            "/responses",
            "/v1/responses",
            "/alpha/search/extra",
            "/v1/chat/completions",
        ] {
            assert!(!is_request("codex", &Method::POST, path));
        }
    }

    #[test]
    fn alpha_search_leaves_clean_or_unparseable_bodies_byte_identical() {
        for raw in [
            r#"{ "id": "search", "future_field": 1 }"#,
            "[]",
            "null",
            "{invalid",
            "",
        ] {
            let raw = Bytes::from(raw);
            let mut headers = HeaderMap::new();
            let mut body = GatewayRequestBody::from_wire(raw.clone(), &headers, 1024);
            assert!(sanitize(&mut headers, &mut body).is_none());
            assert!(!body.is_mutated());
            assert_eq!(body.finalize_for_upstream(&mut headers, 1024), raw);
        }
    }

    #[test]
    fn alpha_search_cleanup_is_idempotent_and_preserves_nested_fields() {
        let mut headers = HeaderMap::new();
        headers.append("session_id", "private-session".parse().unwrap());
        headers.append("session_id", "second-session".parse().unwrap());
        let mut body = GatewayRequestBody::from_wire(
            Bytes::from_static(br#"{"store":null,"prompt_cache_key":"secret","prompt_cache_retention":"24h","settings":{"store":true}}"#),
            &headers,
            1024,
        );
        let marker = sanitize(&mut headers, &mut body).unwrap();
        assert_eq!(
            marker["removedFields"],
            serde_json::json!(["prompt_cache_key", "prompt_cache_retention", "store"])
        );
        assert_eq!(marker["removedHeaders"], serde_json::json!(["session_id"]));
        assert!(!marker.to_string().contains("secret"));
        assert!(!marker.to_string().contains("private-session"));
        assert_eq!(
            serde_json::from_slice::<Value>(body.decoded()).unwrap(),
            serde_json::json!({"settings": {"store": true}})
        );
        assert!(sanitize(&mut headers, &mut body).is_none());
        assert!(!headers.contains_key("session_id"));
    }
}
