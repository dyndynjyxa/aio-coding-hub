//! Usage: Permission-trimmed gateway plugin hook context model.

use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum GatewayPluginHookName {
    RequestReceived,
    RequestAfterBodyRead,
    RequestBeforeProviderResolution,
    RequestBeforeSend,
    ResponseHeaders,
    ResponseChunk,
    ResponseAfter,
    Error,
    LogBeforePersist,
}

impl GatewayPluginHookName {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RequestReceived => "gateway.request.received",
            Self::RequestAfterBodyRead => "gateway.request.afterBodyRead",
            Self::RequestBeforeProviderResolution => "gateway.request.beforeProviderResolution",
            Self::RequestBeforeSend => "gateway.request.beforeSend",
            Self::ResponseHeaders => "gateway.response.headers",
            Self::ResponseChunk => "gateway.response.chunk",
            Self::ResponseAfter => "gateway.response.after",
            Self::Error => "gateway.error",
            Self::LogBeforePersist => "log.beforePersist",
        }
    }

    pub(crate) fn is_request_hook(self) -> bool {
        matches!(
            self,
            Self::RequestReceived
                | Self::RequestAfterBodyRead
                | Self::RequestBeforeProviderResolution
                | Self::RequestBeforeSend
        )
    }

    pub(crate) fn is_response_hook(self) -> bool {
        matches!(
            self,
            Self::ResponseHeaders | Self::ResponseChunk | Self::ResponseAfter | Self::Error
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayRequestHookInput {
    pub(crate) hook_name: GatewayPluginHookName,
    pub(crate) trace_id: String,
    pub(crate) cli_key: String,
    pub(crate) method: Method,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) requested_model: Option<String>,
}

impl GatewayRequestHookInput {
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(self.hook_name, self.trace_id.clone());

        if has_permission(permissions, "request.meta.read") {
            ctx.request.cli_key = Some(self.cli_key.clone());
            ctx.request.method = Some(self.method.as_str().to_string());
            ctx.request.path = Some(self.path.clone());
            ctx.request.query = self.query.clone();
            ctx.request.requested_model = self.requested_model.clone();
        }
        if has_permission(permissions, "request.header.read")
            || has_permission(permissions, "request.header.readSensitive")
        {
            ctx.request.headers = Some(headers_to_json_map(
                &self.headers,
                has_permission(permissions, "request.header.readSensitive"),
            ));
        }
        if has_permission(permissions, "request.body.read") {
            let body = bytes_to_visible_string(&self.body);
            ctx.request.normalized_messages = normalized_messages_from_body(&body);
            ctx.request.body = Some(body);
        }

        ctx
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayResponseHookInput {
    pub(crate) hook_name: GatewayPluginHookName,
    pub(crate) trace_id: String,
    pub(crate) status: u16,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl GatewayResponseHookInput {
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(self.hook_name, self.trace_id.clone());
        if has_permission(permissions, "response.header.read")
            || has_permission(permissions, "response.body.read")
        {
            ctx.response.status = Some(self.status);
        }
        if has_permission(permissions, "response.header.read") {
            ctx.response.headers = Some(headers_to_json_map(&self.headers, true));
        }
        if has_permission(permissions, "response.body.read") {
            ctx.response.body = Some(bytes_to_visible_string(&self.body));
        }
        ctx
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayStreamHookInput {
    pub(crate) trace_id: String,
    pub(crate) chunk: Bytes,
    pub(crate) sequence: u64,
}

impl GatewayStreamHookInput {
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(
            GatewayPluginHookName::ResponseChunk,
            self.trace_id.clone(),
        );
        if has_permission(permissions, "stream.inspect") {
            ctx.stream.sequence = Some(self.sequence);
            ctx.stream.chunk = Some(bytes_to_visible_string(&self.chunk));
        }
        ctx
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayLogHookInput {
    pub(crate) trace_id: String,
    pub(crate) message: String,
}

impl GatewayLogHookInput {
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(
            GatewayPluginHookName::LogBeforePersist,
            self.trace_id.clone(),
        );
        if has_permission(permissions, "log.redact") {
            ctx.log.message = Some(self.message.clone());
        }
        ctx
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleHookContext {
    pub(crate) hook_name: String,
    pub(crate) trace_id: String,
    pub(crate) request: GatewayVisibleRequestContext,
    pub(crate) response: GatewayVisibleResponseContext,
    pub(crate) stream: GatewayVisibleStreamContext,
    pub(crate) log: GatewayVisibleLogContext,
}

impl GatewayVisibleHookContext {
    fn new(hook_name: GatewayPluginHookName, trace_id: String) -> Self {
        Self {
            hook_name: hook_name.as_str().to_string(),
            trace_id,
            request: GatewayVisibleRequestContext::default(),
            response: GatewayVisibleResponseContext::default(),
            stream: GatewayVisibleStreamContext::default(),
            log: GatewayVisibleLogContext::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayNormalizedMessage {
    pub(crate) role: String,
    pub(crate) text: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleRequestContext {
    pub(crate) cli_key: Option<String>,
    pub(crate) method: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) query: Option<String>,
    pub(crate) headers: Option<serde_json::Map<String, serde_json::Value>>,
    pub(crate) body: Option<String>,
    pub(crate) normalized_messages: Vec<GatewayNormalizedMessage>,
    pub(crate) requested_model: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleResponseContext {
    pub(crate) status: Option<u16>,
    pub(crate) headers: Option<serde_json::Map<String, serde_json::Value>>,
    pub(crate) body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleStreamContext {
    pub(crate) sequence: Option<u64>,
    pub(crate) chunk: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleLogContext {
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GatewayHookAction {
    Continue,
    Block,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayHookResult {
    pub(crate) action: GatewayHookAction,
    pub(crate) request_body: Option<String>,
    pub(crate) response_body: Option<String>,
    pub(crate) stream_chunk: Option<String>,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) log_message: Option<String>,
    pub(crate) reason: Option<String>,
}

impl GatewayHookResult {
    pub(crate) fn continue_unchanged() -> Self {
        Self {
            action: GatewayHookAction::Continue,
            request_body: None,
            response_body: None,
            stream_chunk: None,
            headers: BTreeMap::new(),
            log_message: None,
            reason: None,
        }
    }
}

fn has_permission(permissions: &[String], permission: &str) -> bool {
    permissions.iter().any(|item| item == permission)
}

fn bytes_to_visible_string(bytes: &Bytes) -> String {
    String::from_utf8_lossy(bytes.as_ref()).into_owned()
}

fn normalized_messages_from_body(body: &str) -> Vec<GatewayNormalizedMessage> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let mut out = Vec::new();

    if let Some(messages) = root.get("messages").and_then(serde_json::Value::as_array) {
        for message in messages {
            collect_message_content(message, "messages.content", &mut out);
        }
    }

    if let Some(input) = root.get("input") {
        match input {
            serde_json::Value::String(text) => {
                push_normalized_message(&mut out, "user", text, "openai.responses.input")
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_responses_input_item(item, &mut out);
                }
            }
            _ => {}
        }
    }

    out
}

fn collect_message_content(
    message: &serde_json::Value,
    source_prefix: &'static str,
    out: &mut Vec<GatewayNormalizedMessage>,
) {
    let role = message_role(message, "user");
    match message.get("content") {
        Some(serde_json::Value::String(text)) => {
            push_normalized_message(out, &role, text, source_prefix);
        }
        Some(serde_json::Value::Array(parts)) => {
            for part in parts {
                if let Some(text) = part.get("text").and_then(serde_json::Value::as_str) {
                    push_normalized_message(out, &role, text, "messages.content.text");
                }
            }
        }
        _ => {}
    }
}

fn collect_responses_input_item(item: &serde_json::Value, out: &mut Vec<GatewayNormalizedMessage>) {
    let role = message_role(item, "user");
    match item.get("content") {
        Some(serde_json::Value::String(text)) => {
            push_normalized_message(out, &role, text, "openai.responses.content");
        }
        Some(serde_json::Value::Array(parts)) => {
            for part in parts {
                let Some(text) = part.get("text").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let source = if part
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|kind| kind == "input_text")
                {
                    "openai.responses.input_text"
                } else {
                    "openai.responses.content.text"
                };
                push_normalized_message(out, &role, text, source);
            }
        }
        _ => {}
    }
}

fn message_role(message: &serde_json::Value, default_role: &'static str) -> String {
    message
        .get("role")
        .and_then(serde_json::Value::as_str)
        .filter(|role| !role.trim().is_empty())
        .unwrap_or(default_role)
        .to_string()
}

fn push_normalized_message(
    out: &mut Vec<GatewayNormalizedMessage>,
    role: &str,
    text: &str,
    source: &'static str,
) {
    if text.is_empty() {
        return;
    }
    out.push(GatewayNormalizedMessage {
        role: role.to_string(),
        text: text.to_string(),
        source: source.to_string(),
    });
}

fn headers_to_json_map(
    headers: &HeaderMap,
    include_sensitive: bool,
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        if !include_sensitive && is_sensitive_header(&key) {
            continue;
        }
        if let Ok(value) = value.to_str() {
            out.insert(key, serde_json::Value::String(value.to_string()));
        }
    }
    out
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "set-cookie"
            | "x-api-key"
            | "api-key"
            | "anthropic-api-key"
            | "openai-api-key"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::http::{HeaderMap, HeaderValue, Method};

    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        headers.insert("x-public", HeaderValue::from_static("visible"));
        headers
    }

    #[test]
    fn gateway_plugin_context_trims_request_body_headers_and_sensitive_headers() {
        let input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-1".to_string(),
            cli_key: "codex".to_string(),
            method: Method::POST,
            path: "/v1/chat/completions".to_string(),
            query: Some("debug=1".to_string()),
            headers: headers(),
            body: Bytes::from_static(br#"{"prompt":"hello"}"#),
            requested_model: Some("gpt-test".to_string()),
        };

        let meta_only = input.visible_context(&["request.meta.read".to_string()]);
        assert!(meta_only.request.body.is_none());
        assert!(meta_only.request.headers.is_none());
        assert_eq!(meta_only.request.method.as_deref(), Some("POST"));

        let public_headers = input.visible_context(&["request.header.read".to_string()]);
        let visible = public_headers.request.headers.expect("headers visible");
        assert_eq!(
            visible.get("x-public").and_then(|v| v.as_str()),
            Some("visible")
        );
        assert!(!visible.contains_key("authorization"));

        let body_visible = input.visible_context(&["request.body.read".to_string()]);
        assert_eq!(
            body_visible.request.body.as_deref(),
            Some(r#"{"prompt":"hello"}"#)
        );

        let sensitive = input.visible_context(&[
            "request.header.read".to_string(),
            "request.header.readSensitive".to_string(),
        ]);
        assert_eq!(
            sensitive
                .request
                .headers
                .expect("headers visible")
                .get("authorization")
                .and_then(|v| v.as_str()),
            Some("Bearer secret")
        );
    }

    #[test]
    fn visible_request_context_extracts_codex_input_text_messages() {
        let input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-codex".to_string(),
            cli_key: "codex".to_string(),
            method: Method::POST,
            path: "/v1/responses".to_string(),
            query: None,
            headers: HeaderMap::new(),
            body: Bytes::from_static(
                br#"{"input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}]}"#,
            ),
            requested_model: Some("gpt-5.3-codex".to_string()),
        };

        let visible = input.visible_context(&["request.body.read".to_string()]);

        assert_eq!(
            visible.request.normalized_messages,
            vec![GatewayNormalizedMessage {
                role: "user".to_string(),
                text: "hello".to_string(),
                source: "openai.responses.input_text".to_string(),
            }]
        );
    }

    #[test]
    fn visible_request_context_extracts_claude_content_messages() {
        let input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-claude".to_string(),
            cli_key: "claude".to_string(),
            method: Method::POST,
            path: "/v1/messages".to_string(),
            query: None,
            headers: HeaderMap::new(),
            body: Bytes::from_static(
                br#"{"messages":[{"role":"user","content":[{"type":"text","text":"hello claude"}]}]}"#,
            ),
            requested_model: Some("claude-sonnet".to_string()),
        };

        let visible = input.visible_context(&["request.body.read".to_string()]);

        assert_eq!(
            visible.request.normalized_messages,
            vec![GatewayNormalizedMessage {
                role: "user".to_string(),
                text: "hello claude".to_string(),
                source: "messages.content.text".to_string(),
            }]
        );
    }

    #[test]
    fn gateway_plugin_context_trims_response_and_stream_by_permission() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let response = GatewayResponseHookInput {
            hook_name: GatewayPluginHookName::ResponseAfter,
            trace_id: "trace-2".to_string(),
            status: 200,
            headers,
            body: Bytes::from_static(br#"{"ok":true}"#),
        };

        let headers_only = response.visible_context(&["response.header.read".to_string()]);
        assert!(headers_only.response.body.is_none());
        assert_eq!(headers_only.response.status, Some(200));

        let body_visible = response.visible_context(&["response.body.read".to_string()]);
        assert_eq!(
            body_visible.response.body.as_deref(),
            Some(r#"{"ok":true}"#)
        );

        let chunk = GatewayStreamHookInput {
            trace_id: "trace-3".to_string(),
            chunk: Bytes::from_static(b"data: hello\n\n"),
            sequence: 7,
        };
        let hidden = chunk.visible_context(&[]);
        assert!(hidden.stream.chunk.is_none());
        let visible = chunk.visible_context(&["stream.inspect".to_string()]);
        assert_eq!(visible.stream.chunk.as_deref(), Some("data: hello\n\n"));
        assert_eq!(visible.stream.sequence, Some(7));
    }
}
