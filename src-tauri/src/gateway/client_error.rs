//! Client protocol adapters for gateway failures. Routing and retry decisions stay in proxy.

use super::proxy::GatewayFailure;
use axum::http::StatusCode;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClientProtocol {
    OpenAiHttp,
    ResponsesWs,
    AnthropicHttp,
    GeminiHttp,
    Legacy,
}

impl ClientProtocol {
    pub(super) fn for_http_path(path: &str) -> Self {
        let path = path.trim_end_matches('/');
        if matches!(
            path,
            "/v1/responses" | "/responses" | "/v1/chat/completions" | "/chat/completions"
        ) {
            Self::OpenAiHttp
        } else if matches!(
            path,
            "/v1/messages" | "/messages" | "/v1/messages/count_tokens" | "/messages/count_tokens"
        ) {
            Self::AnthropicHttp
        } else if path.contains("/models/")
            && [":generateContent", ":streamGenerateContent", ":countTokens"]
                .iter()
                .any(|suffix| path.ends_with(suffix))
        {
            Self::GeminiHttp
        } else {
            Self::Legacy
        }
    }
}

fn openai_error_type(status: StatusCode) -> &'static str {
    if status.is_server_error() {
        "server_error"
    } else {
        "invalid_request_error"
    }
}

pub(super) fn encode_failure(protocol: ClientProtocol, failure: &GatewayFailure) -> Value {
    let mut value = serde_json::to_value(failure).expect("serializable gateway failure");
    match protocol {
        ClientProtocol::OpenAiHttp | ClientProtocol::ResponsesWs => {
            value["error"] = json!({"message":failure.message,"type":openai_error_type(failure.status),"code":failure.error_code});
            if protocol == ClientProtocol::ResponsesWs {
                value["type"] = json!("error");
                value["status"] = json!(failure.status.as_u16());
                if let Some(seconds) = failure.retry_after_seconds.filter(|value| *value > 0) {
                    value["headers"] = json!({"retry-after":seconds.to_string()});
                }
            }
        }
        ClientProtocol::AnthropicHttp => {
            value["type"] = json!("error");
            let error_type = match failure.status.as_u16() {
                401 => "authentication_error",
                403 => "permission_error",
                404 => "not_found_error",
                413 => "request_too_large",
                429 => "rate_limit_error",
                529 => "overloaded_error",
                _ if failure.status.is_server_error() => "api_error",
                _ => "invalid_request_error",
            };
            value["error"] = json!({"type":error_type,"message":failure.message});
        }
        ClientProtocol::GeminiHttp => {
            let status = match failure.status.as_u16() {
                400 => "INVALID_ARGUMENT",
                401 => "UNAUTHENTICATED",
                403 => "PERMISSION_DENIED",
                404 => "NOT_FOUND",
                429 => "RESOURCE_EXHAUSTED",
                503 => "UNAVAILABLE",
                504 => "DEADLINE_EXCEEDED",
                _ => "INTERNAL",
            };
            value["error"] =
                json!({"code":failure.status.as_u16(),"status":status,"message":failure.message});
        }
        ClientProtocol::Legacy => {}
    }
    value
}

pub(super) fn ws_error(status: StatusCode, code: &str, message: &str) -> Value {
    json!({"type":"error","status":status.as_u16(),"error":{"type":openai_error_type(status),"code":code,"message":message}})
}

/// Preserve stream error status; only a pre-stream recovery response may omit it.
pub(super) fn normalize_ws_error(mut event: Value, status: Option<StatusCode>) -> Value {
    if event.get("type").and_then(Value::as_str) == Some("response.failed") {
        return event;
    }
    let recovery_response = status == Some(StatusCode::BAD_REQUEST)
        && event.pointer("/error/code").and_then(Value::as_str)
            == Some("previous_response_not_found");
    let status = status.unwrap_or_else(|| super::responses_ws::gate::error_status(&event));
    event["type"] = json!("error");
    if recovery_response {
        // Older Codex clients treat a status-bearing 400 as terminal before checking the code.
        event.as_object_mut().unwrap().remove("status");
    } else {
        event["status"] = json!(status.as_u16());
    }
    if event.pointer("/error/type").is_none() {
        event["error"]["type"] = json!(openai_error_type(status));
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_context_recovery_omits_terminal_status_without_changing_other_errors() {
        let event = ws_error(
            StatusCode::BAD_REQUEST,
            "previous_response_not_found",
            "resend the full input",
        );
        let normalized = normalize_ws_error(event.clone(), Some(StatusCode::BAD_REQUEST));
        assert!(normalized.get("status").is_none());
        assert_eq!(normalized["error"], event["error"]);
        let streamed = normalize_ws_error(event.clone(), None);
        assert_eq!(streamed["status"], 400);
        assert_eq!(streamed["error"], event["error"]);
        assert_eq!(
            normalize_ws_error(event, Some(StatusCode::UNAUTHORIZED))["status"],
            401
        );
        assert_eq!(
            normalize_ws_error(
                ws_error(StatusCode::BAD_REQUEST, "invalid_request", "bad input"),
                None
            )["status"],
            400
        );
    }

    #[test]
    fn native_ws_errors_preserve_fields_and_failed_response_structure() {
        let native = json!({"error":{"type":"authentication_error","code":"invalid_api_key","message":"credential rejected","param":"authorization"}});
        let event = normalize_ws_error(native.clone(), Some(StatusCode::UNAUTHORIZED));
        assert_eq!(event["status"], 401);
        assert_eq!(event["error"], native["error"]);
        let limited = normalize_ws_error(
            json!({"type":"error","error":{"code":"rate_limit_exceeded","message":"busy"}}),
            None,
        );
        assert_eq!(limited["status"], 429);
        let failed = json!({"type":"response.failed","response":{"id":"resp_original","status":"failed","error":{"code":"server_error","message":"failed"}}});
        assert_eq!(normalize_ws_error(failed.clone(), None), failed);
    }
}
