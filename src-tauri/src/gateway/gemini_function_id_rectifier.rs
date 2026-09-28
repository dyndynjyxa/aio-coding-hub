use regex::Regex;
use std::sync::LazyLock;

pub(super) type GeminiFunctionIdRectifierTrigger = &'static str;

pub(super) const TRIGGER_UNKNOWN_FUNCTION_ID_FIELD: GeminiFunctionIdRectifierTrigger =
    "unknown_function_id_field";

static ID_VIOLATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)unknown name \\*"id\\*" at\s+(?:'([^':\n]+)'|([^\s':\n]+))"#)
        .expect("valid Gemini function id violation regex")
});

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct GeminiFunctionIdRectifierResult {
    pub(super) applied: bool,
    pub(super) stripped_function_call_ids: usize,
    pub(super) stripped_function_response_ids: usize,
}

fn is_function_path(path: &str) -> bool {
    path.split('.').any(|segment| {
        let base = segment.split('[').next().unwrap_or(segment);
        matches!(
            base,
            "function_call" | "function_response" | "functioncall" | "functionresponse"
        )
    })
}

pub(super) fn detect_trigger(error_message: &str) -> Option<GeminiFunctionIdRectifierTrigger> {
    if error_message.trim().is_empty() {
        return None;
    }

    let lower = error_message.to_ascii_lowercase();
    ID_VIOLATION_PATTERN
        .captures_iter(&lower)
        .filter_map(|captures| captures.get(1).or_else(|| captures.get(2)))
        .any(|path| is_function_path(path.as_str()))
        .then_some(TRIGGER_UNKNOWN_FUNCTION_ID_FIELD)
}

fn strip_ids_from_contents(contents: Option<&mut serde_json::Value>) -> (usize, usize) {
    let Some(contents) = contents.and_then(serde_json::Value::as_array_mut) else {
        return (0, 0);
    };
    let mut calls = 0usize;
    let mut responses = 0usize;

    for content in contents {
        let Some(parts) = content
            .get_mut("parts")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for part in parts {
            let Some(part) = part.as_object_mut() else {
                continue;
            };
            for key in ["functionCall", "function_call"] {
                if part
                    .get_mut(key)
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|function| function.remove("id").is_some())
                {
                    calls = calls.saturating_add(1);
                }
            }
            for key in ["functionResponse", "function_response"] {
                if part
                    .get_mut(key)
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|function| function.remove("id").is_some())
                {
                    responses = responses.saturating_add(1);
                }
            }
        }
    }

    (calls, responses)
}

pub(super) fn rectify(message: &mut serde_json::Value) -> GeminiFunctionIdRectifierResult {
    let Some(message) = message.as_object_mut() else {
        return GeminiFunctionIdRectifierResult::default();
    };
    let top_level = strip_ids_from_contents(message.get_mut("contents"));
    let wrapped = message
        .get_mut("request")
        .and_then(serde_json::Value::as_object_mut)
        .map(|request| strip_ids_from_contents(request.get_mut("contents")))
        .unwrap_or_default();
    let stripped_function_call_ids = top_level.0.saturating_add(wrapped.0);
    let stripped_function_response_ids = top_level.1.saturating_add(wrapped.1);

    GeminiFunctionIdRectifierResult {
        applied: stripped_function_call_ids > 0 || stripped_function_response_ids > 0,
        stripped_function_call_ids,
        stripped_function_response_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_camel_snake_and_escaped_function_paths() {
        for message in [
            r#"Invalid JSON payload received. Unknown name "id" at 'contents[1].parts[0].function_call': Cannot find field."#,
            r#"Unknown name "id" at contents[1].parts[0].functionResponse: Cannot find field."#,
            r#"Upstream: {\"error\":{\"message\":\"Unknown name \\\"id\\\" at 'request.contents[0].parts[0].functionCall'\"}}"#,
        ] {
            assert_eq!(
                detect_trigger(message),
                Some(TRIGGER_UNKNOWN_FUNCTION_ID_FIELD),
                "message={message}"
            );
        }
    }

    #[test]
    fn binds_each_id_violation_to_its_own_path() {
        let unrelated_id_then_function_name = concat!(
            r#"Unknown name "id" at 'contents[0].parts[0].text': Cannot find field. "#,
            r#"Unknown name "name" at 'contents[0].parts[1].function_call': Cannot find field."#,
        );
        assert_eq!(detect_trigger(unrelated_id_then_function_name), None);
        assert_eq!(
            detect_trigger(
                r#"Unknown name "id" at 'tool_config.function_calling_config': Cannot find field."#
            ),
            None
        );
    }

    #[test]
    fn strips_only_function_ids_from_top_level_and_wrapped_shapes() {
        let mut body = json!({
            "id": "root-id",
            "contents": [{"parts": [
                {"id":"part-id","functionCall":{"id":"call-1","name":"search","args":{}},"thoughtSignature":"sig"},
                {"function_response":{"id":"response-1","name":"search","response":{"ok":true}}}
            ]}],
            "request": {"contents": [{"parts": [
                {"function_call":{"id":"call-2","name":"lookup","args":{"q":"x"}}},
                {"functionResponse":{"id":"response-2","name":"lookup","response":{"value":1}}}
            ]}]}
        });

        let result = rectify(&mut body);

        assert!(result.applied);
        assert_eq!(result.stripped_function_call_ids, 2);
        assert_eq!(result.stripped_function_response_ids, 2);
        assert_eq!(body["id"], "root-id");
        assert_eq!(body["contents"][0]["parts"][0]["id"], "part-id");
        assert_eq!(body["contents"][0]["parts"][0]["thoughtSignature"], "sig");
        assert!(body["contents"][0]["parts"][0]["functionCall"]
            .get("id")
            .is_none());
        assert_eq!(
            body["request"]["contents"][0]["parts"][0]["function_call"]["args"]["q"],
            "x"
        );
    }

    #[test]
    fn is_noop_for_missing_or_non_object_function_values() {
        for mut body in [
            json!({}),
            json!({"contents":null}),
            json!({"contents":[{"parts":[{"functionCall":"invalid"}]}]}),
            json!({"contents":[{"parts":[{"functionCall":{"name":"search"}}]}]}),
        ] {
            let before = body.clone();
            let result = rectify(&mut body);
            assert!(!result.applied);
            assert_eq!(body, before);
        }
    }
}
