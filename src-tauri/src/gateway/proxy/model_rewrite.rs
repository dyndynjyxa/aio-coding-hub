//! Usage: Request model rewriting helpers (query/path/JSON body).

use crate::gateway::util::{encode_url_component, RequestedModelLocation};
use axum::body::Bytes;

/// Rewrites the requested model wherever the request carries it. Returns whether
/// anything actually changed. Shared by the generic model policy and the legacy
/// Claude mapping so both channels rewrite identically.
pub(super) fn rewrite_model_in_request(
    location: RequestedModelLocation,
    effective_model: &str,
    forwarded_path: &mut String,
    query: &mut Option<String>,
    body_bytes: &mut Bytes,
    strip_request_content_encoding: &mut bool,
) -> bool {
    match location {
        RequestedModelLocation::BodyJson => {
            let Ok(mut root) = serde_json::from_slice::<serde_json::Value>(body_bytes) else {
                return false;
            };
            if !replace_model_in_body_json(&mut root, effective_model) {
                return false;
            }
            let Ok(bytes) = serde_json::to_vec(&root) else {
                return false;
            };
            *body_bytes = Bytes::from(bytes);
            *strip_request_content_encoding = true;
            true
        }
        RequestedModelLocation::Query => {
            let Some(current) = query.as_deref() else {
                return false;
            };
            let next = replace_model_in_query(current, effective_model);
            let changed = next != current;
            if changed {
                *query = Some(next);
            }
            changed
        }
        RequestedModelLocation::Path => {
            let Some(next) = replace_model_in_path(forwarded_path, effective_model) else {
                return false;
            };
            let changed = next != *forwarded_path;
            if changed {
                *forwarded_path = next;
            }
            changed
        }
    }
}

pub(super) fn replace_model_in_query(query: &str, model: &str) -> String {
    let encoded = encode_url_component(model);
    let mut changed = false;
    let mut out: Vec<String> = Vec::new();

    for part in query.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            out.push(part.to_string());
            continue;
        };
        if key == "model" {
            out.push(format!("model={encoded}"));
            changed = changed || value != encoded;
        } else {
            out.push(part.to_string());
        }
    }

    if !changed {
        return query.to_string();
    }
    out.join("&")
}

pub(super) fn replace_model_in_path(path: &str, model: &str) -> Option<String> {
    let needle = "/models/";
    let idx = path.find(needle)?;
    let start = idx + needle.len();
    let rest = &path[start..];
    if rest.is_empty() {
        return None;
    }
    let end_rel = rest.find(['/', ':', '?']).unwrap_or(rest.len());
    let end = start + end_rel;

    let mut out = String::with_capacity(path.len().saturating_add(model.len()));
    out.push_str(&path[..start]);
    out.push_str(&encode_url_component(model));
    out.push_str(&path[end..]);
    Some(out)
}

pub(super) fn replace_model_in_body_json(root: &mut serde_json::Value, model: &str) -> bool {
    let Some(obj) = root.as_object_mut() else {
        return false;
    };

    let replacement = serde_json::Value::String(model.to_string());
    match obj.get_mut("model") {
        Some(current) => match current {
            serde_json::Value::String(_) => {
                *current = replacement;
                true
            }
            serde_json::Value::Object(m) => {
                if m.get("name").and_then(|v| v.as_str()).is_some() {
                    m.insert("name".to_string(), replacement);
                    return true;
                }
                if m.get("id").and_then(|v| v.as_str()).is_some() {
                    m.insert("id".to_string(), replacement);
                    return true;
                }

                *current = replacement;
                true
            }
            _ => {
                *current = replacement;
                true
            }
        },
        None => {
            obj.insert("model".to_string(), replacement);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rewrite_model_in_request;
    use crate::gateway::util::RequestedModelLocation;
    use axum::body::Bytes;

    #[test]
    fn rewrite_model_updates_body_query_and_path() {
        let mut path = "/v1/models/gpt-5.4".to_string();
        let mut query = Some("model=gpt-5.4&stream=true".to_string());
        let mut body = Bytes::from(r#"{"model":"gpt-5.4","input":[]}"#);
        let mut strip = false;

        assert!(rewrite_model_in_request(
            RequestedModelLocation::BodyJson,
            "upstream-5.4",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body)
                .expect("rewritten body")
                .get("model")
                .and_then(|value| value.as_str()),
            Some("upstream-5.4")
        );
        assert!(strip);

        assert!(rewrite_model_in_request(
            RequestedModelLocation::Query,
            "query-model",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(query.as_deref(), Some("model=query-model&stream=true"));

        assert!(rewrite_model_in_request(
            RequestedModelLocation::Path,
            "path-model",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(path, "/v1/models/path-model");

        // Non-JSON body cannot be rewritten.
        let mut binary_body = Bytes::from_static(b"\x1f\x8b not json");
        let mut strip2 = false;
        assert!(!rewrite_model_in_request(
            RequestedModelLocation::BodyJson,
            "x",
            &mut path,
            &mut query,
            &mut binary_body,
            &mut strip2,
        ));
        assert!(!strip2);
    }
}
