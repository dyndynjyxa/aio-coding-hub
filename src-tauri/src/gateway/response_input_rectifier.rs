//! Normalizes the OpenAI Responses API `input` shorthand before provider routing.

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseInputRectifierAction {
    String,
    Object,
    EmptyString,
}

impl ResponseInputRectifierAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::String => "string_to_array",
            Self::Object => "object_to_array",
            Self::EmptyString => "empty_string_to_empty_array",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseInputRectifierOriginalType {
    String,
    Object,
}

impl ResponseInputRectifierOriginalType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResponseInputRectifierResult {
    pub(crate) action: ResponseInputRectifierAction,
    pub(crate) original_type: ResponseInputRectifierOriginalType,
}

pub(crate) fn rectify_response_input(root: &mut Value) -> Option<ResponseInputRectifierResult> {
    let object = root.as_object_mut()?;
    let input = object.get_mut("input")?;

    match input {
        Value::Array(_) => None,
        Value::String(value) => {
            let text = std::mem::take(value);
            if text.is_empty() {
                *input = Value::Array(Vec::new());
                Some(ResponseInputRectifierResult {
                    action: ResponseInputRectifierAction::EmptyString,
                    original_type: ResponseInputRectifierOriginalType::String,
                })
            } else {
                *input = Value::Array(vec![Value::Object(response_message(text))]);
                Some(ResponseInputRectifierResult {
                    action: ResponseInputRectifierAction::String,
                    original_type: ResponseInputRectifierOriginalType::String,
                })
            }
        }
        Value::Object(value) if value.contains_key("role") || value.contains_key("type") => {
            let item = std::mem::replace(input, Value::Null);
            *input = Value::Array(vec![item]);
            Some(ResponseInputRectifierResult {
                action: ResponseInputRectifierAction::Object,
                original_type: ResponseInputRectifierOriginalType::Object,
            })
        }
        _ => None,
    }
}

fn response_message(text: String) -> Map<String, Value> {
    let mut content = Map::new();
    content.insert("type".to_string(), Value::String("input_text".to_string()));
    content.insert("text".to_string(), Value::String(text));

    let mut message = Map::new();
    message.insert("role".to_string(), Value::String("user".to_string()));
    message.insert(
        "content".to_string(),
        Value::Array(vec![Value::Object(content)]),
    );
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_non_empty_string_input() {
        let mut root = json!({"model":"gpt-5","input":"hello"});
        let result = rectify_response_input(&mut root).expect("changed");
        assert_eq!(result.action.as_str(), "string_to_array");
        assert_eq!(
            root["input"],
            json!([{"role":"user","content":[{"type":"input_text","text":"hello"}]}])
        );
    }

    #[test]
    fn normalizes_empty_string_input() {
        let mut root = json!({"input":""});
        let result = rectify_response_input(&mut root).expect("changed");
        assert_eq!(result.action, ResponseInputRectifierAction::EmptyString);
        assert_eq!(root["input"], json!([]));
    }

    #[test]
    fn wraps_message_and_tool_objects_but_preserves_all_fields() {
        for input in [
            json!({"role":"user","content":"hello","custom":true}),
            json!({"type":"function_call_output","call_id":"call_1","output":"ok"}),
        ] {
            let mut root = json!({"input": input.clone(), "metadata":{"keep":true}});
            let result = rectify_response_input(&mut root).expect("changed");
            assert_eq!(result.action, ResponseInputRectifierAction::Object);
            assert_eq!(root["input"], json!([input]));
            assert_eq!(root["metadata"]["keep"], true);
        }
    }

    #[test]
    fn passes_through_arrays_null_scalars_and_unrecognized_objects() {
        for input in [
            json!([]),
            Value::Null,
            json!(42),
            json!({"content":"without role"}),
        ] {
            let original = json!({"input": input});
            let mut root = original.clone();
            assert!(rectify_response_input(&mut root).is_none());
            assert_eq!(root, original);
        }
    }

    #[test]
    fn passes_through_non_object_roots() {
        for mut root in [Value::Null, json!([]), json!("input")] {
            assert!(rectify_response_input(&mut root).is_none());
        }
    }
}
