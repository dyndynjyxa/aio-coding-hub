//! Middleware: classifies Codex system-generated Responses requests from
//! trusted turn metadata and records a request-log marker.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::push_special_setting;
use axum::http::Method;
use serde_json::Value;

const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";
const MAX_CODEX_TURN_METADATA_BYTES: usize = 16 * 1024;
const CODEX_SYSTEM_REQUEST_SETTING_TYPE: &str = "codex_system_request";
const CODEX_SYSTEM_REQUEST_THREAD_SOURCE: &str = "system";

pub(in crate::gateway::proxy::handler) struct CodexRequestClassifierMiddleware;

impl CodexRequestClassifierMiddleware {
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        if let Some(setting) = codex_system_request_special_setting(
            &ctx.cli_key,
            &ctx.req_method,
            &ctx.forwarded_path,
            ctx.introspection_json.as_ref(),
        ) {
            push_special_setting(&ctx.special_settings, setting);
            ctx.provider_health_neutral = true;
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

fn codex_system_request_special_setting(
    cli_key: &str,
    method: &Method,
    forwarded_path: &str,
    introspection_json: Option<&Value>,
) -> Option<Value> {
    if cli_key != "codex"
        || *method != Method::POST
        || !matches!(
            forwarded_path,
            "/responses" | "/responses/" | "/v1/responses" | "/v1/responses/"
        )
    {
        return None;
    }

    let turn_metadata = introspection_json?
        .get("client_metadata")?
        .as_object()?
        .get(CODEX_TURN_METADATA_KEY)?
        .as_str()?;
    if turn_metadata.is_empty() || turn_metadata.len() > MAX_CODEX_TURN_METADATA_BYTES {
        return None;
    }

    let turn_metadata = serde_json::from_str::<Value>(turn_metadata).ok()?;
    let turn_metadata = turn_metadata.as_object()?;
    (turn_metadata.get("thread_source").and_then(Value::as_str)
        == Some(CODEX_SYSTEM_REQUEST_THREAD_SOURCE))
    .then(|| {
        serde_json::json!({
            "type": CODEX_SYSTEM_REQUEST_SETTING_TYPE,
            "threadSource": CODEX_SYSTEM_REQUEST_THREAD_SOURCE,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    fn request_body_with_turn_metadata(value: Value) -> Value {
        let mut client_metadata = Map::new();
        client_metadata.insert(CODEX_TURN_METADATA_KEY.to_string(), value);
        json!({
            "model": "gpt-5.4-mini",
            "client_metadata": client_metadata,
        })
    }

    fn classify(body: Option<&Value>) -> Option<Value> {
        codex_system_request_special_setting("codex", &Method::POST, "/v1/responses", body)
    }

    fn system_turn_metadata() -> String {
        json!({ "thread_source": "system" }).to_string()
    }

    #[test]
    fn classifies_system_turn_metadata() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        assert_eq!(
            classify(Some(&body)),
            Some(json!({
                "type": CODEX_SYSTEM_REQUEST_SETTING_TYPE,
                "threadSource": CODEX_SYSTEM_REQUEST_THREAD_SOURCE,
            }))
        );
    }

    #[test]
    fn accepts_only_exact_responses_paths() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        for path in [
            "/responses",
            "/responses/",
            "/v1/responses",
            "/v1/responses/",
        ] {
            assert!(codex_system_request_special_setting(
                "codex",
                &Method::POST,
                path,
                Some(&body),
            )
            .is_some());
        }

        for path in ["responses", "/v1/responses//", "/v1/responses/extra"] {
            assert!(codex_system_request_special_setting(
                "codex",
                &Method::POST,
                path,
                Some(&body),
            )
            .is_none());
        }
    }

    #[test]
    fn rejects_non_codex_or_non_post_requests() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        assert!(codex_system_request_special_setting(
            "claude",
            &Method::POST,
            "/v1/responses",
            Some(&body),
        )
        .is_none());
        assert!(codex_system_request_special_setting(
            "codex",
            &Method::GET,
            "/v1/responses",
            Some(&body),
        )
        .is_none());
    }

    #[test]
    fn rejects_missing_or_invalid_outer_metadata_shapes() {
        let bodies = [
            Value::Null,
            json!([]),
            json!({}),
            json!({ "client_metadata": null }),
            json!({ "client_metadata": [] }),
            json!({ "client_metadata": {} }),
        ];

        assert!(classify(None).is_none());
        for body in &bodies {
            assert!(classify(Some(body)).is_none());
        }
    }

    #[test]
    fn rejects_non_string_turn_metadata() {
        for value in [Value::Null, json!({}), json!([]), json!(1), json!(true)] {
            let body = request_body_with_turn_metadata(value);
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn rejects_empty_malformed_or_non_object_turn_metadata() {
        for raw in ["", "not-json", "null", "[]", "true"] {
            let body = request_body_with_turn_metadata(Value::String(raw.to_string()));
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn rejects_missing_non_string_or_non_system_thread_source() {
        for metadata in [
            json!({}),
            json!({ "thread_source": null }),
            json!({ "thread_source": 1 }),
            json!({ "thread_source": true }),
            json!({ "thread_source": {} }),
            json!({ "thread_source": "user" }),
            json!({ "thread_source": "SYSTEM" }),
        ] {
            let body = request_body_with_turn_metadata(Value::String(metadata.to_string()));
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn enforces_nested_metadata_byte_limit() {
        let prefix = r#"{"thread_source":"system","padding":""#;
        let suffix = r#""}"#;
        let padding = "x".repeat(MAX_CODEX_TURN_METADATA_BYTES - prefix.len() - suffix.len());
        let at_limit = format!("{prefix}{padding}{suffix}");
        assert_eq!(at_limit.len(), MAX_CODEX_TURN_METADATA_BYTES);

        let at_limit_body = request_body_with_turn_metadata(Value::String(at_limit.clone()));
        assert!(classify(Some(&at_limit_body)).is_some());

        let over_limit_body =
            request_body_with_turn_metadata(Value::String(format!("{at_limit} ")));
        assert!(classify(Some(&over_limit_body)).is_none());
    }
}
