//! Usage: Request model rewriting helpers (query/path/JSON body).

use crate::gateway::util::encode_url_component;

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
