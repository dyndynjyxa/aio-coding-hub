//! Usage: Complete usage options for Grok streaming Chat Completions requests.

use axum::body::Bytes;

pub(super) fn ensure_stream_usage_option(
    cli_key: &str,
    forwarded_path: &str,
    body_bytes: &Bytes,
) -> Option<Bytes> {
    if cli_key != "grok"
        || !matches!(
            forwarded_path.trim_end_matches('/'),
            "/v1/chat/completions" | "/chat/completions"
        )
    {
        return None;
    }

    let mut root = serde_json::from_slice::<serde_json::Value>(body_bytes).ok()?;
    let object = root.as_object_mut()?;
    if object.get("stream").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }

    match object.get_mut("stream_options") {
        None | Some(serde_json::Value::Null) => {
            object.insert(
                "stream_options".to_string(),
                serde_json::json!({ "include_usage": true }),
            );
        }
        Some(serde_json::Value::Object(stream_options)) => {
            if stream_options
                .get("include_usage")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                return None;
            }
            stream_options.insert("include_usage".to_string(), serde_json::Value::Bool(true));
        }
        Some(_) => return None,
    }

    serde_json::to_vec(&root).ok().map(Bytes::from)
}

#[cfg(test)]
mod tests {
    use super::ensure_stream_usage_option;
    use axum::body::Bytes;
    use serde_json::{json, Value};

    fn apply(cli_key: &str, path: &str, body: Value) -> Option<Value> {
        let body = Bytes::from(serde_json::to_vec(&body).expect("serialize request"));
        ensure_stream_usage_option(cli_key, path, &body)
            .map(|body| serde_json::from_slice(&body).expect("parse rectified request"))
    }

    #[test]
    fn adds_missing_or_null_stream_options() {
        for body in [
            json!({"model":"grok","stream":true}),
            json!({"model":"grok","stream":true,"stream_options":null}),
        ] {
            let next = apply("grok", "/v1/chat/completions", body).expect("mutation");
            assert_eq!(next["stream_options"]["include_usage"], true);
        }
    }

    #[test]
    fn preserves_stream_option_siblings_and_forces_usage() {
        let next = apply(
            "grok",
            "/v1/chat/completions",
            json!({
                "stream": true,
                "stream_options": {"include_usage": false, "continuous_usage_stats": true}
            }),
        )
        .expect("mutation");

        assert_eq!(next["stream_options"]["include_usage"], true);
        assert_eq!(next["stream_options"]["continuous_usage_stats"], true);
    }

    #[test]
    fn is_noop_when_usage_is_already_enabled() {
        assert_eq!(
            apply(
                "grok",
                "/v1/chat/completions",
                json!({"stream":true,"stream_options":{"include_usage":true}}),
            ),
            None
        );
    }

    #[test]
    fn leaves_invalid_stream_options_unchanged() {
        for stream_options in [json!(false), json!("invalid"), json!([])] {
            assert_eq!(
                apply(
                    "grok",
                    "/v1/chat/completions",
                    json!({"stream":true,"stream_options":stream_options}),
                ),
                None
            );
        }
    }

    #[test]
    fn excludes_codex_responses_and_non_stream_requests() {
        for (cli_key, path, body) in [
            ("codex", "/v1/chat/completions", json!({"stream":true})),
            ("grok", "/v1/responses", json!({"stream":true})),
            ("grok", "/v1/chat/completions", json!({"stream":false})),
            ("grok", "/v1/chat/completions", json!({"stream":"true"})),
        ] {
            assert_eq!(apply(cli_key, path, body), None);
        }
    }
}
