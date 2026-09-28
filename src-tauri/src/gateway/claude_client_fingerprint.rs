//! Sanitized Claude client identity diagnostics for final upstream attempts.

use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataUserIdState {
    StringNonblank,
    StringBlank,
    NonString,
    Missing,
    BodyUnavailable,
}

impl MetadataUserIdState {
    fn as_str(self) -> &'static str {
        match self {
            Self::StringNonblank => "string_nonblank",
            Self::StringBlank => "string_blank",
            Self::NonString => "non_string",
            Self::Missing => "missing",
            Self::BodyUnavailable => "body_unavailable",
        }
    }
}

pub(super) fn compute(forwarded_path: &str, headers: &HeaderMap, body: &Bytes) -> Option<String> {
    let normalized_path = forwarded_path.trim_end_matches('/');
    if !normalized_path.ends_with("/messages")
        && !normalized_path.ends_with("/messages/count_tokens")
    {
        return None;
    }

    let x_app = classify_x_app(headers);
    let ua_family = classify_user_agent(headers);
    let anthropic_beta_present = headers.contains_key("anthropic-beta");
    let metadata_user_id = metadata_user_id_state(headers, body);
    let three_header_match = x_app == "cli" && ua_family == "claude_cli" && anthropic_beta_present;
    let cch_confirmed = if normalized_path.ends_with("/count_tokens") {
        three_header_match
    } else {
        three_header_match
            && matches!(
                metadata_user_id,
                MetadataUserIdState::StringNonblank | MetadataUserIdState::StringBlank
            )
    };

    Some(format!(
        "x_app={x_app}|ua_family={ua_family}|anthropic_beta_present={anthropic_beta_present}|metadata_user_id={}|cch_confirmed={cch_confirmed}",
        metadata_user_id.as_str()
    ))
}

fn classify_x_app(headers: &HeaderMap) -> &'static str {
    let Some(value) = headers.get("x-app").and_then(|value| value.to_str().ok()) else {
        return "missing";
    };
    if value.trim().eq_ignore_ascii_case("cli") {
        "cli"
    } else {
        "other"
    }
}

fn classify_user_agent(headers: &HeaderMap) -> &'static str {
    let Some(value) = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return "missing";
    };
    let lower = value.to_ascii_lowercase();
    if lower
        .split_whitespace()
        .next()
        .is_some_and(|token| token.starts_with("claude-cli/"))
    {
        "claude_cli"
    } else {
        "other"
    }
}

fn metadata_user_id_state(headers: &HeaderMap, body: &Bytes) -> MetadataUserIdState {
    let body = crate::gateway::util::body_for_introspection(headers, body.as_ref());
    let Ok(root) = serde_json::from_slice::<Value>(body.as_ref()) else {
        return MetadataUserIdState::BodyUnavailable;
    };
    let Some(metadata) = root.get("metadata") else {
        return MetadataUserIdState::Missing;
    };
    let Some(metadata) = metadata.as_object() else {
        return MetadataUserIdState::NonString;
    };
    let Some(user_id) = metadata.get("user_id") else {
        return MetadataUserIdState::Missing;
    };
    match user_id {
        Value::String(value) if value.trim().is_empty() => MetadataUserIdState::StringBlank,
        Value::String(_) => MetadataUserIdState::StringNonblank,
        _ => MetadataUserIdState::NonString,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(user_agent: Option<&str>, x_app: Option<&str>, beta: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(value) = user_agent {
            headers.insert(header::USER_AGENT, HeaderValue::from_str(value).unwrap());
        }
        if let Some(value) = x_app {
            headers.insert("x-app", HeaderValue::from_str(value).unwrap());
        }
        if beta {
            headers.insert("anthropic-beta", HeaderValue::from_static("redacted-beta"));
        }
        headers
    }

    #[test]
    fn reports_only_cch_signal_categories() {
        let headers = headers(
            Some("claude-cli/2.1.99 (secret-version)"),
            Some("cli"),
            true,
        );
        let body = Bytes::from_static(
            br#"{"metadata":{"user_id":"{\"device_id\":\"secret-device\",\"session_id\":\"secret-session\"}"},"prompt":"secret-prompt"}"#,
        );
        let diagnostic = compute("/v1/messages", &headers, &body).expect("diagnostic");
        assert_eq!(
            diagnostic,
            "x_app=cli|ua_family=claude_cli|anthropic_beta_present=true|metadata_user_id=string_nonblank|cch_confirmed=true"
        );
        assert!(!diagnostic.contains("secret"));
        assert!(!diagnostic.contains("2.1.99"));
    }

    #[test]
    fn distinguishes_blank_non_string_missing_and_unavailable_metadata() {
        let cases = [
            (
                br#"{"metadata":{"user_id":" "}}"#.as_slice(),
                "string_blank",
            ),
            (br#"{"metadata":{"user_id":42}}"#.as_slice(), "non_string"),
            (br#"{}"#.as_slice(), "missing"),
            (b"not-json".as_slice(), "body_unavailable"),
        ];
        for (body, expected) in cases {
            let diagnostic = compute(
                "/v1/messages",
                &headers(Some("claude-cli/2.1.99"), Some("cli"), true),
                &Bytes::copy_from_slice(body),
            )
            .expect("diagnostic");
            assert!(diagnostic.contains(&format!("metadata_user_id={expected}")));
        }
    }

    #[test]
    fn count_tokens_uses_three_header_confirmation_without_body_metadata() {
        let diagnostic = compute(
            "/v1/messages/count_tokens",
            &headers(Some("claude-cli/2.1.99"), Some("cli"), true),
            &Bytes::from_static(b"{}"),
        )
        .expect("diagnostic");
        assert!(diagnostic.contains("cch_confirmed=true"));
    }

    #[test]
    fn excludes_other_paths() {
        let headers = headers(Some("claude-cli/2.1.99"), Some("cli"), true);
        assert!(compute("/v1/responses", &headers, &Bytes::from_static(b"{}")).is_none());
    }
}
