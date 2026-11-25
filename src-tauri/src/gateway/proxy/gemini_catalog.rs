//! Project Gemini model discovery onto this provider's routable source names.

use crate::providers::{ProviderModelEligibility, ProviderModelPolicyV1};
use axum::{body::Bytes, http::Method};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap, HashSet};

pub(super) fn is_request(cli: &str, method: &Method, path: &str) -> bool {
    cli == "gemini" && *method == Method::GET && path.trim_end_matches('/') == "/v1beta/models"
}

// Generation routes a single path segment followed by an action suffix.
fn routable_id(id: &str) -> bool {
    !id.is_empty()
        && !id.chars().any(|ch| {
            ch.is_whitespace()
                || ch.is_control()
                || matches!(ch, '*' | '/' | '\\' | ':' | '?' | '#')
        })
}

fn configured_sources(policy: &ProviderModelPolicyV1) -> impl Iterator<Item = &str> {
    policy
        .model_patterns
        .iter()
        .chain(policy.mappings.iter().map(|m| &m.source))
        .map(String::as_str)
        .filter(|id| {
            routable_id(id)
                && routable_id(&policy.resolve_mapping(id))
                && policy.eligibility(id) == ProviderModelEligibility::Explicit
        })
}

// Code Assist has no public models.list endpoint. Exact names and aliases the
// user enrolled are enumerable; a wildcard does not establish any model ID.
pub(super) fn configured_catalog(policy: &ProviderModelPolicyV1) -> Bytes {
    let models: Vec<_> = configured_sources(policy)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            json!({"name": format!("models/{id}"), "displayName": id,
            "supportedGenerationMethods": ["generateContent"]})
        })
        .collect();
    Bytes::from(json!({"models": models}).to_string())
}

pub(super) fn project_upstream(body: &Bytes, policy: &ProviderModelPolicyV1) -> Option<Bytes> {
    let mut root: Value = serde_json::from_slice(body).ok()?;
    let models = root.get_mut("models")?.as_array_mut()?;
    let mut upstream = HashMap::new();
    let mut sources: Vec<String> = Vec::new();
    for item in models.drain(..) {
        if item
            .get("supportedGenerationMethods")
            .and_then(Value::as_array)
            .is_some_and(|methods| !methods.iter().any(|m| m == "generateContent"))
        {
            continue;
        }
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        let id = name.strip_prefix("models/").unwrap_or(name);
        if !routable_id(id) {
            continue;
        }
        sources.push(id.to_owned());
        for mapping in &policy.mappings {
            // An invertible wildcard alias can be expanded using a real upstream
            // ID. A wildcard mapped to one fixed target has no such inverse.
            if let Some((prefix, suffix)) = mapping.target.split_once('*') {
                if let Some(capture) = id.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix))
                {
                    sources.push(mapping.source.replace('*', capture));
                }
            }
        }
        upstream.insert(id.to_owned(), item);
    }
    sources.extend(configured_sources(policy).map(str::to_owned));
    let mut seen = HashSet::new();
    for id in sources {
        if !routable_id(&id)
            || !seen.insert(id.clone())
            || policy.eligibility(&id) == ProviderModelEligibility::Blocked
        {
            continue;
        }
        let target = policy.resolve_mapping(&id);
        let Some(mut item) = upstream.get(&target).cloned() else {
            continue;
        };
        item["name"] = json!(format!("models/{id}"));
        if id != target
            || item["displayName"]
                .as_str()
                .is_none_or(|name| name.trim().is_empty())
        {
            item["displayName"] = json!(id);
        }
        models.push(item);
    }
    // Preserve the upstream page token and model metadata.
    Some(Bytes::from(root.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ProviderModelMapping, ProviderModelMode};

    #[test]
    fn gemini_model_catalog_skips_malformed_entries_but_rejects_invalid_pages() {
        let policy = ProviderModelPolicyV1::all();
        let page = json!({
            "models": [null, {}, {"name": 42}, {"name": "models/ok", "displayName": "OK"}],
            "nextPageToken": "page-2"
        });
        let projected = project_upstream(&Bytes::from(page.to_string()), &policy).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&projected).unwrap(),
            json!({"models":[{"name":"models/ok", "displayName":"OK"}], "nextPageToken":"page-2"})
        );
        for invalid in ["{", "null", "{}", r#"{"models":{}}"#] {
            assert!(project_upstream(&Bytes::from_static(invalid.as_bytes()), &policy).is_none());
        }
    }

    #[test]
    fn gemini_model_catalog_preserves_metadata_and_excludes_ineligible_names() {
        let mut policy = ProviderModelPolicyV1::all();
        policy.mode = ProviderModelMode::Excluded;
        policy.model_patterns = vec!["blocked*".into()];
        policy.mappings.push(ProviderModelMapping {
            source: "blocked-alias".into(),
            target: "live".into(),
        });
        let upstream = Bytes::from_static(br#"{"models":[{"name":"models/blocked","displayName":"Blocked"},{"name":"models/live","displayName":"Upstream Friendly","outputTokenLimit":100},{"name":"models/not/routable"}]}"#);
        let actual: Value =
            serde_json::from_slice(&project_upstream(&upstream, &policy).unwrap()).unwrap();
        assert_eq!(
            actual,
            json!({"models":[{"name":"models/live","displayName":"Upstream Friendly","outputTokenLimit":100}]})
        );
        assert_eq!(
            configured_catalog(&policy),
            Bytes::from_static(br#"{"models":[]}"#)
        );
    }
}
