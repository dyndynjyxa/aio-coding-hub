//! Usage: Permission-trimmed gateway plugin hook context model.

use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const DEFAULT_PLUGIN_CONTEXT_STREAM_BYTES: usize = 64 * 1024;
pub(crate) const DEFAULT_PLUGIN_CONTEXT_LOG_BYTES: usize = 64 * 1024;
pub(crate) const DEFAULT_PLUGIN_NORMALIZED_MESSAGE_LIMIT: usize = 64;
pub(crate) const DEFAULT_PLUGIN_NORMALIZED_MESSAGE_TEXT_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GatewayPluginContextBudget {
    pub(crate) body_bytes: usize,
    pub(crate) stream_bytes: usize,
    pub(crate) log_bytes: usize,
    pub(crate) normalized_messages: usize,
    pub(crate) normalized_message_text_bytes: usize,
}

impl Default for GatewayPluginContextBudget {
    fn default() -> Self {
        Self {
            body_bytes: crate::gateway::util::max_request_body_bytes(),
            stream_bytes: DEFAULT_PLUGIN_CONTEXT_STREAM_BYTES,
            log_bytes: DEFAULT_PLUGIN_CONTEXT_LOG_BYTES,
            normalized_messages: DEFAULT_PLUGIN_NORMALIZED_MESSAGE_LIMIT,
            normalized_message_text_bytes: DEFAULT_PLUGIN_NORMALIZED_MESSAGE_TEXT_BYTES,
        }
    }
}

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

    #[allow(dead_code)]
    pub(crate) fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "gateway.request.received" => Some(Self::RequestReceived),
            "gateway.request.afterBodyRead" => Some(Self::RequestAfterBodyRead),
            "gateway.request.beforeProviderResolution" => {
                Some(Self::RequestBeforeProviderResolution)
            }
            "gateway.request.beforeSend" => Some(Self::RequestBeforeSend),
            "gateway.response.headers" => Some(Self::ResponseHeaders),
            "gateway.response.chunk" => Some(Self::ResponseChunk),
            "gateway.response.after" => Some(Self::ResponseAfter),
            "gateway.error" => Some(Self::Error),
            "log.beforePersist" => Some(Self::LogBeforePersist),
            _ => None,
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
    #[allow(dead_code)]
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        self.visible_context_with_budget(permissions, GatewayPluginContextBudget::default())
    }

    pub(crate) fn visible_context_with_budget(
        &self,
        permissions: &[String],
        budget: GatewayPluginContextBudget,
    ) -> GatewayVisibleHookContext {
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
            let (body, body_truncated) = visible_string_with_limit(&self.body, budget.body_bytes);
            let (messages, messages_truncated) = normalized_messages_from_body_with_budget(
                &body,
                budget.normalized_messages,
                budget.normalized_message_text_bytes,
            );
            ctx.request.normalized_messages = messages;
            ctx.request.normalized_messages_truncated = messages_truncated || body_truncated;
            ctx.request.body = Some(body);
            ctx.request.body_truncated = body_truncated;
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
    #[allow(dead_code)]
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        self.visible_context_with_budget(permissions, GatewayPluginContextBudget::default())
    }

    pub(crate) fn visible_context_with_budget(
        &self,
        permissions: &[String],
        budget: GatewayPluginContextBudget,
    ) -> GatewayVisibleHookContext {
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
            let (body, body_truncated) = visible_string_with_limit(&self.body, budget.body_bytes);
            ctx.response.body = Some(body);
            ctx.response.body_truncated = body_truncated;
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
    #[allow(dead_code)]
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        self.visible_context_with_budget(permissions, GatewayPluginContextBudget::default())
    }

    pub(crate) fn visible_context_with_budget(
        &self,
        permissions: &[String],
        budget: GatewayPluginContextBudget,
    ) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(
            GatewayPluginHookName::ResponseChunk,
            self.trace_id.clone(),
        );
        if has_permission(permissions, "stream.inspect") {
            ctx.stream.sequence = Some(self.sequence);
            let (chunk, chunk_truncated) =
                visible_string_with_limit(&self.chunk, budget.stream_bytes);
            ctx.stream.chunk = Some(chunk);
            ctx.stream.chunk_truncated = chunk_truncated;
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
    #[allow(dead_code)]
    pub(crate) fn visible_context(&self, permissions: &[String]) -> GatewayVisibleHookContext {
        self.visible_context_with_budget(permissions, GatewayPluginContextBudget::default())
    }

    pub(crate) fn visible_context_with_budget(
        &self,
        permissions: &[String],
        budget: GatewayPluginContextBudget,
    ) -> GatewayVisibleHookContext {
        let mut ctx = GatewayVisibleHookContext::new(
            GatewayPluginHookName::LogBeforePersist,
            self.trace_id.clone(),
        );
        if has_permission(permissions, "log.redact") {
            let (message, message_truncated) = text_with_limit(&self.message, budget.log_bytes);
            ctx.log.message = Some(message);
            ctx.log.message_truncated = message_truncated;
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
    pub(crate) body_truncated: bool,
    pub(crate) normalized_messages: Vec<GatewayNormalizedMessage>,
    pub(crate) normalized_messages_truncated: bool,
    pub(crate) requested_model: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleResponseContext {
    pub(crate) status: Option<u16>,
    pub(crate) headers: Option<serde_json::Map<String, serde_json::Value>>,
    pub(crate) body: Option<String>,
    pub(crate) body_truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleStreamContext {
    pub(crate) sequence: Option<u64>,
    pub(crate) chunk: Option<String>,
    pub(crate) chunk_truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GatewayVisibleLogContext {
    pub(crate) message: Option<String>,
    pub(crate) message_truncated: bool,
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

fn visible_string_with_limit(bytes: &Bytes, limit: usize) -> (String, bool) {
    if bytes.len() <= limit {
        return (bytes_to_visible_string(bytes), false);
    }
    let capped = &bytes.as_ref()[..limit];
    let boundary = std::str::from_utf8(capped)
        .map(|_| limit)
        .unwrap_or_else(|err| err.valid_up_to());
    (
        String::from_utf8_lossy(&bytes.as_ref()[..boundary]).into_owned(),
        true,
    )
}

fn text_with_limit(text: &str, limit: usize) -> (String, bool) {
    if text.len() <= limit {
        return (text.to_string(), false);
    }
    let boundary = (0..=limit)
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0);
    (text[..boundary].to_string(), true)
}

fn normalized_messages_from_body_with_budget(
    body: &str,
    message_limit: usize,
    text_limit: usize,
) -> (Vec<GatewayNormalizedMessage>, bool) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(body) else {
        return (Vec::new(), false);
    };
    let mut out = Vec::new();
    let mut truncated = false;

    if let Some(messages) = root.get("messages").and_then(serde_json::Value::as_array) {
        for message in messages {
            collect_message_content(
                message,
                "messages.content",
                &mut out,
                message_limit,
                text_limit,
                &mut truncated,
            );
        }
    }

    if let Some(input) = root.get("input") {
        match input {
            serde_json::Value::String(text) => push_normalized_message(
                &mut out,
                "user",
                text,
                "openai.responses.input",
                message_limit,
                text_limit,
                &mut truncated,
            ),
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_responses_input_item(
                        item,
                        &mut out,
                        message_limit,
                        text_limit,
                        &mut truncated,
                    );
                }
            }
            _ => {}
        }
    }

    (out, truncated)
}

fn collect_message_content(
    message: &serde_json::Value,
    source_prefix: &'static str,
    out: &mut Vec<GatewayNormalizedMessage>,
    message_limit: usize,
    text_limit: usize,
    truncated: &mut bool,
) {
    let role = message_role(message, "user");
    match message.get("content") {
        Some(serde_json::Value::String(text)) => {
            push_normalized_message(
                out,
                &role,
                text,
                source_prefix,
                message_limit,
                text_limit,
                truncated,
            );
        }
        Some(serde_json::Value::Array(parts)) => {
            for part in parts {
                if let Some(text) = part.get("text").and_then(serde_json::Value::as_str) {
                    push_normalized_message(
                        out,
                        &role,
                        text,
                        "messages.content.text",
                        message_limit,
                        text_limit,
                        truncated,
                    );
                }
            }
        }
        _ => {}
    }
}

fn collect_responses_input_item(
    item: &serde_json::Value,
    out: &mut Vec<GatewayNormalizedMessage>,
    message_limit: usize,
    text_limit: usize,
    truncated: &mut bool,
) {
    let role = message_role(item, "user");
    match item.get("content") {
        Some(serde_json::Value::String(text)) => {
            push_normalized_message(
                out,
                &role,
                text,
                "openai.responses.content",
                message_limit,
                text_limit,
                truncated,
            );
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
                push_normalized_message(
                    out,
                    &role,
                    text,
                    source,
                    message_limit,
                    text_limit,
                    truncated,
                );
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
    message_limit: usize,
    text_limit: usize,
    truncated: &mut bool,
) {
    if text.is_empty() {
        return;
    }
    if out.len() >= message_limit {
        *truncated = true;
        return;
    }
    let (text, text_truncated) = text_with_limit(text, text_limit);
    *truncated |= text_truncated;
    out.push(GatewayNormalizedMessage {
        role: role.to_string(),
        text,
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
        // Provider-owned secrets remain private even when the hook contract exposes
        // standard sensitive headers (that access is currently granted implicitly).
        if value.is_sensitive() || (!include_sensitive && is_sensitive_header(&key)) {
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

    #[test]
    fn hook_name_from_str_maps_all_known_hook_names() {
        let hooks = [
            GatewayPluginHookName::RequestReceived,
            GatewayPluginHookName::RequestAfterBodyRead,
            GatewayPluginHookName::RequestBeforeProviderResolution,
            GatewayPluginHookName::RequestBeforeSend,
            GatewayPluginHookName::ResponseHeaders,
            GatewayPluginHookName::ResponseChunk,
            GatewayPluginHookName::ResponseAfter,
            GatewayPluginHookName::Error,
            GatewayPluginHookName::LogBeforePersist,
        ];

        for hook in hooks {
            assert_eq!(GatewayPluginHookName::from_str(hook.as_str()), Some(hook));
        }
        assert_eq!(GatewayPluginHookName::from_str("gateway.unknown"), None);
    }

    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        headers.insert("x-public", HeaderValue::from_static("visible"));
        let (_, secret) =
            crate::shared::provider_headers::parse("x-tenant", "synthetic-secret").unwrap();
        headers.insert("x-tenant", secret);
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
        assert!(!visible.contains_key("x-tenant"));
        let debug = crate::gateway::util::redacted_headers_for_debug(&input.headers);
        assert!(!format!("{debug:?}").contains("synthetic-secret"));
        let allowed = input.visible_context(&[
            "request.header.read".into(),
            "request.header.readSensitive".into(),
        ]);
        assert!(!allowed.request.headers.unwrap().contains_key("x-tenant"));

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
    fn gateway_plugin_context_truncates_request_body_and_normalized_messages_by_budget() {
        let body = format!(
            "{{\"messages\":[{}]}}",
            (0..5)
                .map(|index| format!(
                    "{{\"role\":\"user\",\"content\":\"message-{index}-{}\"}}",
                    "x".repeat(64)
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-budget".to_string(),
            cli_key: "codex".to_string(),
            method: Method::POST,
            path: "/v1/messages".to_string(),
            query: None,
            headers: HeaderMap::new(),
            body: Bytes::from(body),
            requested_model: None,
        };
        let budget = GatewayPluginContextBudget {
            body_bytes: 48,
            normalized_messages: 2,
            normalized_message_text_bytes: 16,
            ..GatewayPluginContextBudget::default()
        };

        let visible = input.visible_context_with_budget(&["request.body.read".to_string()], budget);

        assert!(visible.request.body.as_deref().unwrap().len() <= 48 + 64);
        assert!(visible.request.body_truncated);
        assert!(visible.request.normalized_messages.len() <= 2);
        assert!(visible
            .request
            .normalized_messages
            .iter()
            .all(|message| message.text.len() <= 16 + 32));
    }

    #[test]
    fn gateway_plugin_context_truncates_stream_and_log_by_budget() {
        let stream = GatewayStreamHookInput {
            trace_id: "trace-stream-budget".to_string(),
            chunk: Bytes::from("s".repeat(128)),
            sequence: 1,
        };
        let log = GatewayLogHookInput {
            trace_id: "trace-log-budget".to_string(),
            message: "l".repeat(128),
        };
        let budget = GatewayPluginContextBudget {
            stream_bytes: 16,
            log_bytes: 24,
            ..GatewayPluginContextBudget::default()
        };

        let visible_stream =
            stream.visible_context_with_budget(&["stream.inspect".to_string()], budget);
        let visible_log = log.visible_context_with_budget(&["log.redact".to_string()], budget);

        assert!(visible_stream.stream.chunk.as_deref().unwrap().len() <= 80);
        assert!(visible_stream.stream.chunk_truncated);
        assert!(visible_log.log.message.as_deref().unwrap().len() <= 88);
        assert!(visible_log.log.message_truncated);
    }

    #[test]
    fn gateway_plugin_context_truncates_response_body_by_budget() {
        let response = GatewayResponseHookInput {
            hook_name: GatewayPluginHookName::ResponseAfter,
            trace_id: "trace-response-budget".to_string(),
            status: 200,
            headers: HeaderMap::new(),
            body: Bytes::from_static(b"abcdefghij"),
        };
        let budget = GatewayPluginContextBudget {
            body_bytes: 4,
            ..GatewayPluginContextBudget::default()
        };

        let visible =
            response.visible_context_with_budget(&["response.body.read".to_string()], budget);

        assert_eq!(visible.response.status, Some(200));
        assert_eq!(visible.response.body.as_deref(), Some("abcd"));
        assert!(visible.response.body_truncated);
    }

    #[test]
    fn gateway_plugin_context_truncates_multibyte_text_without_replacement_characters() {
        let body = "你好🙂abc";
        let normalized_body = "{\"messages\":[{\"role\":\"user\",\"content\":\"你好🙂abc\"}]}";
        let input = GatewayRequestHookInput {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead,
            trace_id: "trace-multibyte-budget".to_string(),
            cli_key: "codex".to_string(),
            method: Method::POST,
            path: "/v1/messages".to_string(),
            query: None,
            headers: HeaderMap::new(),
            body: Bytes::from(body),
            requested_model: None,
        };
        let normalized_input = GatewayRequestHookInput {
            body: Bytes::from(normalized_body),
            ..input.clone()
        };
        let stream = GatewayStreamHookInput {
            trace_id: "trace-stream-multibyte-budget".to_string(),
            chunk: Bytes::from("你好🙂abc"),
            sequence: 1,
        };
        let log = GatewayLogHookInput {
            trace_id: "trace-log-multibyte-budget".to_string(),
            message: "你好🙂abc".to_string(),
        };
        let budget = GatewayPluginContextBudget {
            body_bytes: 11,
            stream_bytes: 5,
            log_bytes: 5,
            normalized_messages: 1,
            normalized_message_text_bytes: 8,
        };

        let visible_request =
            input.visible_context_with_budget(&["request.body.read".to_string()], budget);
        let visible_normalized_request = normalized_input.visible_context_with_budget(
            &["request.body.read".to_string()],
            GatewayPluginContextBudget {
                body_bytes: normalized_body.len(),
                ..budget
            },
        );
        let visible_stream =
            stream.visible_context_with_budget(&["stream.inspect".to_string()], budget);
        let visible_log = log.visible_context_with_budget(&["log.redact".to_string()], budget);

        let request_body = visible_request.request.body.as_deref().unwrap();
        let stream_chunk = visible_stream.stream.chunk.as_deref().unwrap();
        let log_message = visible_log.log.message.as_deref().unwrap();
        assert!(visible_request.request.body_truncated);
        assert!(visible_stream.stream.chunk_truncated);
        assert!(visible_log.log.message_truncated);
        assert!(!request_body.contains('\u{FFFD}'));
        assert!(!stream_chunk.contains('\u{FFFD}'));
        assert!(!log_message.contains('\u{FFFD}'));
        assert!(request_body.len() <= 11);
        assert!(stream_chunk.len() <= 5);
        assert!(log_message.len() <= 5);
        let normalized_message = visible_normalized_request
            .request
            .normalized_messages
            .first()
            .unwrap();
        assert!(
            visible_normalized_request
                .request
                .normalized_messages_truncated
        );
        assert!(!normalized_message.text.contains('\u{FFFD}'));
        assert!(normalized_message.text.len() <= 8);
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
