//! Usage: Conservative fake-200 response body detection for gateway logging.

fn non_null_error_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Object(items) => !items.is_empty(),
        serde_json::Value::Bool(value) => *value,
        serde_json::Value::Number(_) => true,
    }
}

pub(in crate::gateway) fn is_fake_200_non_stream_body(body_bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body_bytes) else {
        return false;
    };

    if value.get("error").is_some_and(non_null_error_value) {
        return true;
    }

    value.get("type").and_then(serde_json::Value::as_str) == Some("error")
        && value
            .as_object()
            .is_some_and(|object| object.contains_key("error") || object.contains_key("message"))
}

#[cfg(test)]
mod tests {
    use super::is_fake_200_non_stream_body;

    #[test]
    fn fake_200_non_stream_body_detects_explicit_error_payloads() {
        assert!(is_fake_200_non_stream_body(
            br#"{"error":{"message":"quota exhausted"}}"#
        ));
        assert!(is_fake_200_non_stream_body(
            br#"{"type":"error","message":"quota exhausted"}"#
        ));
    }

    #[test]
    fn fake_200_non_stream_body_ignores_successful_error_null_payloads() {
        assert!(!is_fake_200_non_stream_body(
            br#"{"id":"ok","error":null,"choices":[]}"#
        ));
        assert!(!is_fake_200_non_stream_body(
            br#"{"type":"message","content":[{"type":"text","text":"ok"}]}"#
        ));
    }
}
