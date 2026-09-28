//! Normalizes nullable and object-valued fields in Responses API output JSON.

use serde_json::Value;

pub(crate) fn normalize_response_output_payload(root: &mut Value) -> Vec<String> {
    let Some(object) = root.as_object_mut() else {
        return Vec::new();
    };
    if object.get("object").and_then(Value::as_str) != Some("response") {
        return Vec::new();
    }

    let mut fixes = Vec::new();
    if let Some(output) = object.get_mut("output") {
        if output.is_null() {
            *output = Value::Array(Vec::new());
            fixes.push("output".to_string());
        } else if let Some(items) = output.as_array_mut() {
            for (index, item) in items.iter_mut().enumerate() {
                normalize_output_item(item, &mut fixes, &format!("output[{index}]"));
            }
        }
    }
    if let Some(tools) = object.get_mut("tools") {
        if tools.is_null() {
            *tools = Value::Array(Vec::new());
            fixes.push("tools".to_string());
        }
    }
    fixes
}

fn normalize_output_item(item: &mut Value, fixes: &mut Vec<String>, path: &str) {
    let Some(object) = item.as_object_mut() else {
        return;
    };

    if let Some(content) = object.get_mut("content") {
        if content.is_null() {
            *content = Value::Array(Vec::new());
            fixes.push(format!("{path}.content"));
        } else if let Some(parts) = content.as_array_mut() {
            for (index, part) in parts.iter_mut().enumerate() {
                normalize_content_part(part, fixes, &format!("{path}.content[{index}]"));
            }
        }
    }
    if let Some(summary) = object.get_mut("summary") {
        if summary.is_null() {
            *summary = Value::Array(Vec::new());
            fixes.push(format!("{path}.summary"));
        }
    }

    normalize_arguments(object, "arguments", fixes, path);
    if let Some(function) = object.get_mut("function").and_then(Value::as_object_mut) {
        normalize_arguments(function, "arguments", fixes, &format!("{path}.function"));
    }
    if let Some(tool_calls) = object.get_mut("tool_calls").and_then(Value::as_array_mut) {
        for (index, tool_call) in tool_calls.iter_mut().enumerate() {
            let Some(tool_call) = tool_call.as_object_mut() else {
                continue;
            };
            if let Some(function) = tool_call.get_mut("function").and_then(Value::as_object_mut) {
                normalize_arguments(
                    function,
                    "arguments",
                    fixes,
                    &format!("{path}.tool_calls[{index}].function"),
                );
            }
        }
    }
}

fn normalize_content_part(part: &mut Value, fixes: &mut Vec<String>, path: &str) {
    let Some(object) = part.as_object_mut() else {
        return;
    };
    if let Some(text) = object.get_mut("text") {
        if text.is_null() {
            *text = Value::String(String::new());
            fixes.push(format!("{path}.text"));
        }
    }
    if let Some(annotations) = object.get_mut("annotations") {
        if annotations.is_null() {
            *annotations = Value::Array(Vec::new());
            fixes.push(format!("{path}.annotations"));
        }
    }
    if let Some(logprobs) = object.get_mut("logprobs") {
        if logprobs.is_null() {
            *logprobs = Value::Array(Vec::new());
            fixes.push(format!("{path}.logprobs"));
        }
    }
}

fn normalize_arguments(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    fixes: &mut Vec<String>,
    path: &str,
) {
    let Some(value) = object.get_mut(key) else {
        return;
    };
    let normalized = if value.is_string() {
        return;
    } else if value.is_null() {
        "{}".to_string()
    } else {
        serde_json::to_string(&*value).unwrap_or_else(|_| "{}".to_string())
    };
    *value = Value::String(normalized);
    fixes.push(format!("{path}.{key}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_cch_nullable_output_matrix_and_arguments() {
        let mut body = json!({
            "object": "response",
            "output": [{
                "content": [{"text": null, "annotations": null, "logprobs": null}],
                "summary": null,
                "arguments": {"top": true},
                "function": {"arguments": null},
                "tool_calls": [{"function": {"arguments": [1, 2]}}]
            }],
            "tools": null
        });
        let fixes = normalize_response_output_payload(&mut body);
        assert_eq!(body["output"][0]["content"][0]["text"], "");
        assert_eq!(body["output"][0]["content"][0]["annotations"], json!([]));
        assert_eq!(body["output"][0]["content"][0]["logprobs"], json!([]));
        assert_eq!(body["output"][0]["summary"], json!([]));
        assert_eq!(body["output"][0]["arguments"], "{\"top\":true}");
        assert_eq!(body["output"][0]["function"]["arguments"], "{}");
        assert_eq!(
            body["output"][0]["tool_calls"][0]["function"]["arguments"],
            "[1,2]"
        );
        assert_eq!(body["tools"], json!([]));
        assert_eq!(fixes.len(), 8);
    }

    #[test]
    fn preserves_existing_strings_and_unrelated_fields() {
        let mut body = json!({
            "object": "response",
            "output": [{"content": "not-an-array", "arguments": "already", "id": "keep"}],
            "custom": {"value": 1}
        });
        let original_id = body["output"][0]["id"].clone();
        let fixes = normalize_response_output_payload(&mut body);
        assert!(fixes.is_empty());
        assert_eq!(body["output"][0]["id"], original_id);
        assert_eq!(body["custom"]["value"], 1);
    }

    #[test]
    fn ignores_non_response_roots_and_missing_fields() {
        for mut body in [
            json!({"object":"chat.completion","output":null}),
            json!({"object":"response","output":[]}),
            json!([]),
        ] {
            assert!(normalize_response_output_payload(&mut body).is_empty());
        }
    }
}
