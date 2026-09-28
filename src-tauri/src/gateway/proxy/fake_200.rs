//! Usage: Conservative fake-200 response body detection for gateway logging.

const MAX_MESSAGE_CHECK_BYTES: usize = 1_000;
const HTML_SNIFF_BYTES: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gateway) enum Fake200Profile {
    Global,
    OpenAiConversation,
}

impl Fake200Profile {
    pub(in crate::gateway) fn for_request(cli_key: &str, path: &str) -> Self {
        let path = path.trim_end_matches('/');
        let expanded = matches!(cli_key, "codex" | "grok")
            && (matches!(path, "/v1/responses" | "/responses")
                || (cli_key == "grok"
                    && matches!(path, "/v1/chat/completions" | "/chat/completions")));

        if expanded {
            Self::OpenAiConversation
        } else {
            Self::Global
        }
    }

    pub(in crate::gateway) fn detects_empty_stream(self) -> bool {
        self == Self::OpenAiConversation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gateway) enum Fake200Reason {
    EmptyBody,
    HtmlBody,
    JsonErrorNonEmpty,
    JsonTypeError,
    JsonMessageKeywordMatch,
    OpenAiResponseFailed,
}

impl Fake200Reason {
    pub(in crate::gateway) const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyBody => "fake_200_empty_body",
            Self::HtmlBody => "fake_200_html_body",
            Self::JsonErrorNonEmpty => "fake_200_json_error_non_empty",
            Self::JsonTypeError => "fake_200_json_type_error",
            Self::JsonMessageKeywordMatch => "fake_200_json_message_keyword_match",
            Self::OpenAiResponseFailed => "fake_200_openai_response_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gateway) struct Fake200Detection {
    pub(in crate::gateway) reason: Fake200Reason,
}

fn non_null_error_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Object(items) => !items.is_empty(),
        serde_json::Value::Bool(value) => *value,
        // Preserve the gateway's existing behavior: any JSON number is explicit.
        serde_json::Value::Number(_) => true,
    }
}

fn trim_ascii_and_bom(mut bytes: &[u8]) -> &[u8] {
    bytes = trim_ascii(bytes);
    if let Some(without_bom) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        bytes = trim_ascii_start(without_bom);
    }
    bytes
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let bytes = trim_ascii_start(bytes);
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    &bytes[..end]
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

fn starts_with_ascii_case_insensitive(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn is_likely_html_document(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(HTML_SNIFF_BYTES)];
    for prefix in [b"<!doctype html".as_slice(), b"<html".as_slice()] {
        if !starts_with_ascii_case_insensitive(head, prefix) {
            continue;
        }
        return head
            .get(prefix.len())
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>');
    }
    false
}

fn contains_ascii_case_insensitive(bytes: &[u8], needle: &[u8]) -> bool {
    bytes
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn openai_response_failed(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    let event_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let response = object
        .get("response")
        .and_then(serde_json::Value::as_object)
        .unwrap_or(object);
    let status = response
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let response_object = response
        .get("object")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let response_id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();

    let looks_like_response = event_type.starts_with("response.")
        || response_object == "response"
        || response_id.starts_with("resp_");
    let failed =
        event_type.eq_ignore_ascii_case("response.failed") || status.eq_ignore_ascii_case("failed");

    looks_like_response && failed
}

pub(in crate::gateway) fn detect_fake_200_non_stream_body(
    body_bytes: &[u8],
    profile: Fake200Profile,
) -> Option<Fake200Detection> {
    let trimmed = trim_ascii_and_bom(body_bytes);

    if profile == Fake200Profile::OpenAiConversation {
        if trimmed.is_empty() {
            return Some(Fake200Detection {
                reason: Fake200Reason::EmptyBody,
            });
        }
        if is_likely_html_document(trimmed) {
            return Some(Fake200Detection {
                reason: Fake200Reason::HtmlBody,
            });
        }
    }

    let serde_json::Value::Object(object) = serde_json::from_slice(trimmed).ok()? else {
        return None;
    };

    if profile == Fake200Profile::OpenAiConversation && openai_response_failed(&object) {
        return Some(Fake200Detection {
            reason: Fake200Reason::OpenAiResponseFailed,
        });
    }

    if object.get("error").is_some_and(non_null_error_value) {
        return Some(Fake200Detection {
            reason: Fake200Reason::JsonErrorNonEmpty,
        });
    }

    if object.get("type").and_then(serde_json::Value::as_str) == Some("error")
        && (object.contains_key("error") || object.contains_key("message"))
    {
        return Some(Fake200Detection {
            reason: Fake200Reason::JsonTypeError,
        });
    }

    if profile == Fake200Profile::OpenAiConversation
        && trimmed.len() < MAX_MESSAGE_CHECK_BYTES
        && object
            .get("message")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|message| contains_ascii_case_insensitive(message.as_bytes(), b"error"))
    {
        return Some(Fake200Detection {
            reason: Fake200Reason::JsonMessageKeywordMatch,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{detect_fake_200_non_stream_body, Fake200Profile, Fake200Reason};

    fn detect(body: &[u8], profile: Fake200Profile) -> Option<Fake200Reason> {
        detect_fake_200_non_stream_body(body, profile).map(|result| result.reason)
    }

    #[test]
    fn global_profile_preserves_explicit_error_detection() {
        assert_eq!(
            detect(
                br#"{"error":{"message":"quota exhausted"}}"#,
                Fake200Profile::Global
            ),
            Some(Fake200Reason::JsonErrorNonEmpty)
        );
        assert_eq!(
            detect(
                br#"{"type":"error","message":"quota exhausted"}"#,
                Fake200Profile::Global
            ),
            Some(Fake200Reason::JsonTypeError)
        );
        assert_eq!(
            detect(br#"{"error":0}"#, Fake200Profile::Global),
            Some(Fake200Reason::JsonErrorNonEmpty)
        );
    }

    #[test]
    fn global_profile_ignores_expanded_only_signals() {
        for body in [
            b"".as_slice(),
            b"<!doctype html><title>upstream error</title>".as_slice(),
            br#"{"message":"upstream error"}"#.as_slice(),
            br#"{"object":"response","status":"failed"}"#.as_slice(),
        ] {
            assert_eq!(detect(body, Fake200Profile::Global), None);
        }
    }

    #[test]
    fn openai_profile_detects_empty_and_bom_html_bodies() {
        assert_eq!(
            detect(b" \r\n\t", Fake200Profile::OpenAiConversation),
            Some(Fake200Reason::EmptyBody)
        );
        assert_eq!(
            detect(
                b"\xef\xbb\xbf  <!DOCTYPE html><title>bad gateway</title>",
                Fake200Profile::OpenAiConversation
            ),
            Some(Fake200Reason::HtmlBody)
        );
        assert_eq!(
            detect(
                b"\xef\xbb\xbf<html lang=en><body>bad gateway</body>",
                Fake200Profile::OpenAiConversation
            ),
            Some(Fake200Reason::HtmlBody)
        );
    }

    #[test]
    fn openai_profile_detects_response_failed_shapes() {
        for body in [
            br#"{"type":"response.failed","response":{"id":"resp_1","status":"failed"}}"#
                .as_slice(),
            br#"{"object":"response","status":"failed"}"#.as_slice(),
            br#"{"id":"resp_1","status":"failed"}"#.as_slice(),
            br#"{"response":{"object":"response","status":"failed"}}"#.as_slice(),
        ] {
            assert_eq!(
                detect(body, Fake200Profile::OpenAiConversation),
                Some(Fake200Reason::OpenAiResponseFailed)
            );
        }
    }

    #[test]
    fn openai_profile_limits_top_level_message_heuristic() {
        assert_eq!(
            detect(
                br#"{"message":"UPSTREAM ERROR"}"#,
                Fake200Profile::OpenAiConversation
            ),
            Some(Fake200Reason::JsonMessageKeywordMatch)
        );

        let large = format!(r#"{{"message":"error {}"}}"#, "x".repeat(1_000));
        assert_eq!(
            detect(large.as_bytes(), Fake200Profile::OpenAiConversation),
            None
        );
    }

    #[test]
    fn openai_profile_avoids_ambiguous_payloads() {
        for body in [
            br#"[]"#.as_slice(),
            br#"[{"error":"model generated content"}]"#.as_slice(),
            br#"{"content":"an error occurred in the story"}"#.as_slice(),
            br#"{"status":"failed"}"#.as_slice(),
            br#"{"object":"response","status":"completed","error":null}"#.as_slice(),
            br#"{"message":"ok""#.as_slice(),
            b"<div>error</div>".as_slice(),
        ] {
            assert_eq!(
                detect(body, Fake200Profile::OpenAiConversation),
                None,
                "body={}",
                String::from_utf8_lossy(body)
            );
        }
    }

    #[test]
    fn request_profile_is_limited_to_direct_openai_conversations() {
        assert_eq!(
            Fake200Profile::for_request("codex", "/v1/responses"),
            Fake200Profile::OpenAiConversation
        );
        assert_eq!(
            Fake200Profile::for_request("grok", "/v1/chat/completions"),
            Fake200Profile::OpenAiConversation
        );
        assert_eq!(
            Fake200Profile::for_request("codex", "/v1/chat/completions"),
            Fake200Profile::Global
        );
        assert_eq!(
            Fake200Profile::for_request("claude", "/v1/responses"),
            Fake200Profile::Global
        );
    }
}
