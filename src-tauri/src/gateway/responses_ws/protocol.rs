//! Responses event boundaries shared by the WebSocket ingress and content gate.

use axum::body::Bytes;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(in crate::gateway) const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
pub(in crate::gateway) const MAX_PREFIX_BYTES: usize = 1024 * 1024;
pub(in crate::gateway) const MAX_PREFIX_EVENTS: usize = 256;
pub(in crate::gateway) const TURN_STATE_HEADER: &str = "x-codex-turn-state";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gateway) enum EventKind {
    Metadata,
    Content,
    Completed,
    Incomplete,
    Failed,
}

pub(in crate::gateway) fn event_kind(event: &Value) -> Result<EventKind, &'static str> {
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .ok_or("missing event type")?;
    match kind {
        "error" | "response.failed" => Ok(EventKind::Failed),
        "response.completed" | "response.incomplete" => {
            let response = event
                .get("response")
                .and_then(Value::as_object)
                .ok_or("missing terminal response")?;
            if response.get("id").and_then(Value::as_str).is_none() {
                return Err("missing terminal response id");
            }
            Ok(if kind == "response.completed" {
                EventKind::Completed
            } else {
                EventKind::Incomplete
            })
        }
        "response.created" | "response.in_progress" => {
            // A provider may include a completed item even in an initial snapshot.
            Ok(
                if event
                    .pointer("/response/output")
                    .and_then(Value::as_array)
                    .is_some_and(|items| !items.is_empty())
                {
                    EventKind::Content
                } else {
                    EventKind::Metadata
                },
            )
        }
        "response.metadata" | "codex.response.metadata" | "codex.rate_limits" => {
            Ok(EventKind::Metadata)
        }
        _ => Ok(EventKind::Content),
    }
}

pub(in crate::gateway) fn error_event(code: &str, message: &str) -> Value {
    serde_json::json!({"type":"error","error":{"type":"invalid_request_error","code":code,"message":message}})
}

pub(in crate::gateway) fn metadata_event(nonce: &str) -> Value {
    serde_json::json!({"type":"response.metadata","headers":{TURN_STATE_HEADER:nonce}})
}

pub(in crate::gateway) fn sse_bytes(value: &Value) -> Bytes {
    Bytes::from(format!("data: {value}\n\n"))
}

/// Preserve event boundaries across arbitrary byte chunks; do not treat EOF or DONE as success.
#[derive(Default)]
pub(in crate::gateway) struct EventDecoder {
    pending: Vec<u8>,
}

impl EventDecoder {
    pub(in crate::gateway) fn push(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if self.pending.len().saturating_add(bytes.len()) > MAX_MESSAGE_BYTES {
            return Err("Responses event exceeds byte limit");
        }
        let required = self.pending.len() + bytes.len();
        if self.pending.capacity() < required {
            let capacity = required.next_power_of_two().min(MAX_MESSAGE_BYTES);
            self.pending.reserve_exact(capacity - self.pending.len());
        }
        self.pending.extend_from_slice(bytes);
        Ok(())
    }

    pub(in crate::gateway) fn next(&mut self) -> Result<Option<Value>, &'static str> {
        loop {
            let Some(end) =
                crate::gateway::proxy::protocol_bridge::stream::find_sse_event_end(&self.pending)
            else {
                return Ok(None);
            };
            let frame =
                std::str::from_utf8(&self.pending[..end]).map_err(|_| "invalid event UTF-8")?;
            let mut data = String::with_capacity(frame.len());
            for line in frame.lines() {
                if let Some(value) = line.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(value.strip_prefix(' ').unwrap_or(value));
                }
            }
            self.pending.drain(..end);
            if data.is_empty() || data.trim() == "[DONE]" {
                continue;
            }
            let value: Value =
                serde_json::from_str(&data).map_err(|_| "invalid Responses event JSON")?;
            event_kind(&value)?;
            return Ok(Some(value));
        }
    }

    #[cfg(test)]
    pub(in crate::gateway) fn has_partial_event(&self) -> bool {
        self.pending.iter().any(|byte| !byte.is_ascii_whitespace())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::gateway) struct Owner {
    pub(in crate::gateway) session: String,
    pub(in crate::gateway) thread: String,
    pub(in crate::gateway) window: String,
    pub(in crate::gateway) context_window: String,
    pub(in crate::gateway) turn: String,
}

impl Owner {
    pub(in crate::gateway) fn parse(metadata: &str) -> Option<Self> {
        if metadata.len() > 16 * 1024 {
            return None;
        }
        let value: Value = serde_json::from_str(metadata).ok()?;
        let field = |name| {
            value
                .get(name)
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty() && text.len() <= 256)
                .map(str::to_owned)
        };
        Some(Self {
            session: field("session_id")?,
            thread: field("thread_id")?,
            window: field("window_id")?,
            context_window: field("context_window_id")?,
            turn: field("turn_id")?,
        })
    }
}

/// Only hashes and a count survive a generation; no prompt/output history is retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gateway) struct HistoryDigest {
    hash: [u8; 32],
    pub(in crate::gateway) count: usize,
    recoverable: bool,
}

impl Default for HistoryDigest {
    fn default() -> Self {
        Self {
            hash: [0; 32],
            count: 0,
            recoverable: true,
        }
    }
}

impl HistoryDigest {
    pub(in crate::gateway) fn append(&mut self, items: &[Value]) {
        for item in items {
            self.count = self.count.saturating_add(1);
            match canonical_history_item(item) {
                Some(item) => self.hash_value(item),
                None => self.recoverable = false,
            }
        }
    }

    fn hash_value(&mut self, mut item: Value) {
        item.sort_all_objects();
        let mut hash = Sha256::new();
        hash.update(self.hash);
        hash.update(item.to_string().as_bytes());
        self.hash = hash.finalize().into();
    }

    pub(in crate::gateway) fn is_recoverable(&self) -> bool {
        self.recoverable
    }

    pub(in crate::gateway) fn disable_recovery(&mut self) {
        self.recoverable = false;
    }

    pub(in crate::gateway) fn from_items(items: &[Value]) -> Self {
        let mut result = Self::default();
        result.append(items);
        result
    }

    pub(in crate::gateway) fn from_value(value: &Value) -> Self {
        let mut result = Self::default();
        result.hash_value(value.clone());
        result.count = 1;
        result
    }

    pub(in crate::gateway) fn is_strict_prefix_of(&self, input: &[Value]) -> bool {
        self.recoverable
            && input.len() > self.count
            && Self::from_items(&input[..self.count]) == *self
    }
}

/// Project only verified Responses history fields. Unknown item/content fields
/// disable history rebuilds, not initial requests or socket-bound continuations.
/// Internal server metadata is omitted when the client rebuilds non-OpenAI history;
/// semantic IDs, tool names and payloads remain exact.
fn canonical_history_item(item: &Value) -> Option<Value> {
    let mut object = item.as_object()?.clone();
    let kind = object.get("type")?.as_str()?.to_owned();
    let fields: &[&str] = match kind.as_str() {
        "message" => &["role", "content", "phase", "status"],
        "function_call" => &[
            "name",
            "namespace",
            "arguments",
            "call_id",
            "encrypted_function_args",
            "status",
        ],
        "function_call_output" => &["call_id", "name", "namespace", "output"],
        "custom_tool_call" => &["call_id", "name", "namespace", "input", "status"],
        "custom_tool_call_output" => &["call_id", "name", "output"],
        "reasoning" => &["summary", "content", "encrypted_content", "status"],
        "additional_tools" => &["role", "tools"],
        _ => return None,
    };
    if object.keys().any(|key| {
        !["type", "id", "internal_chat_message_metadata_passthrough"].contains(&key.as_str())
            && !fields.contains(&key.as_str())
    }) {
        return None;
    }
    if object
        .get("internal_chat_message_metadata_passthrough")
        .is_some_and(|value| !value.is_null() && !value.is_object())
    {
        return None;
    }
    object.remove("internal_chat_message_metadata_passthrough");
    if let Some(id) = object.get("id") {
        if id.is_null()
            || !id
                .as_str()?
                .split_once('_')
                .is_some_and(|(prefix, suffix)| !prefix.is_empty() && !suffix.is_empty())
        {
            object.remove("id");
        }
    }
    for field in ["phase", "namespace", "name", "call_id", "status"] {
        if object.get(field).is_some_and(Value::is_null) {
            object.remove(field);
        }
    }
    let string = |name: &str| object.get(name).is_some_and(Value::is_string);
    match kind.as_str() {
        "message" => {
            if !string("role") {
                return None;
            }
            if object
                .get("phase")
                .is_some_and(|v| !matches!(v.as_str(), Some("commentary" | "final_answer")))
            {
                return None;
            }
            let items = object.get("content")?.as_array()?;
            if !content_types_match(
                items,
                &["input_text", "input_image", "input_audio", "output_text"],
            ) {
                return None;
            }
            let content = items
                .iter()
                .map(canonical_content)
                .collect::<Option<Vec<_>>>()?;
            object.insert("content".into(), Value::Array(content));
            object.remove("status");
        }
        "function_call" => {
            if !string("name") || !string("arguments") || !string("call_id") {
                return None;
            }
            object.remove("status");
            if object.get("encrypted_function_args").is_some_and(|value| {
                !value.is_null()
                    && !value
                        .as_array()
                        .is_some_and(|items| items.iter().all(Value::is_string))
            }) {
                return None;
            }
            object.remove("encrypted_function_args");
        }
        "custom_tool_call" => {
            if !string("name") || !string("input") || !string("call_id") {
                return None;
            }
        }
        "function_call_output" | "custom_tool_call_output" => {
            if kind == "custom_tool_call_output" && !string("call_id") {
                return None;
            }
            let output = object.get("output")?;
            if let Some(items) = output.as_array() {
                if !content_types_match(
                    items,
                    &[
                        "input_text",
                        "input_image",
                        "input_audio",
                        "encrypted_content",
                    ],
                ) {
                    return None;
                }
                let items = items
                    .iter()
                    .map(canonical_content)
                    .collect::<Option<Vec<_>>>()?;
                object.insert("output".into(), Value::Array(items));
            } else if !output.is_string() {
                return None;
            }
        }
        "reasoning" => {
            let items = object.get("summary")?.as_array()?;
            if !content_types_match(items, &["summary_text"]) {
                return None;
            }
            let summary = items
                .iter()
                .map(canonical_content)
                .collect::<Option<Vec<_>>>()?;
            object.insert("summary".into(), Value::Array(summary));
            match object.get("content") {
                None | Some(Value::Null) => {
                    object.insert("content".into(), Value::Null);
                }
                Some(Value::Array(items)) => {
                    if !content_types_match(items, &["reasoning_text", "text"]) {
                        return None;
                    }
                    let items = items
                        .iter()
                        .map(canonical_content)
                        .collect::<Option<Vec<_>>>()?;
                    if items.iter().any(|item| {
                        item.get("type").and_then(Value::as_str) == Some("reasoning_text")
                    }) {
                        object.insert("content".into(), Value::Array(items));
                    } else {
                        object.remove("content");
                    }
                }
                _ => return None,
            }
            object.entry("encrypted_content").or_insert(Value::Null);
            object.remove("status");
        }
        "additional_tools" => {
            if !string("role") || !object.get("tools")?.is_array() {
                return None;
            }
        }
        _ => return None,
    }
    for field in [
        "namespace",
        "name",
        "call_id",
        "status",
        "encrypted_content",
    ] {
        if object
            .get(field)
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            return None;
        }
    }
    Some(Value::Object(object))
}

fn content_types_match(items: &[Value], allowed: &[&str]) -> bool {
    items.iter().all(|item| {
        item.get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| allowed.contains(&kind))
    })
}

fn canonical_content(item: &Value) -> Option<Value> {
    let mut object = item.as_object()?.clone();
    let kind = object.get("type")?.as_str()?;
    let fields: &[&str] = match kind {
        "input_text" | "output_text" | "summary_text" | "reasoning_text" | "text" => {
            &["text", "annotations"]
        }
        "input_image" => &["image_url", "file_id", "detail"],
        "input_audio" => &["audio_url"],
        "encrypted_content" => &["encrypted_content"],
        _ => return None,
    };
    if object
        .keys()
        .any(|key| key != "type" && !fields.contains(&key.as_str()))
    {
        return None;
    }
    match kind {
        "input_image" => {
            if ["image_url", "file_id"]
                .iter()
                .any(|name| object.get(*name).is_some_and(|value| !value.is_string()))
            {
                return None;
            }
            if object.get("image_url").is_some_and(Value::is_string)
                == object.get("file_id").is_some_and(Value::is_string)
            {
                return None;
            }
            if object.get("detail").is_some_and(|v| {
                !v.is_null() && !matches!(v.as_str(), Some("auto" | "low" | "high" | "original"))
            }) {
                return None;
            }
            if object.get("detail").is_some_and(Value::is_null) {
                object.remove("detail");
            }
        }
        "input_audio" => {
            if !object.get("audio_url")?.is_string() {
                return None;
            }
        }
        "encrypted_content" => {
            if !object.get("encrypted_content")?.is_string() {
                return None;
            }
        }
        _ => {
            if !object.get("text")?.is_string() {
                return None;
            }
            object.remove("annotations");
        }
    }
    Some(Value::Object(object))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_handles_split_utf8_crlf_and_multiple_data_lines() {
        let wire = "event: x\r\ndata: {\"type\":\"response.output_text.delta\",\r\ndata: \"delta\":\"你好\"}\r\n\r\ndata: [DONE]\n\n";
        let mut decoder = EventDecoder::default();
        let mut events = vec![];
        for byte in wire.as_bytes() {
            decoder.push(&[*byte]).unwrap();
            while let Some(event) = decoder.next().unwrap() {
                events.push(event);
            }
        }
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["delta"], "你好");
        assert!(!decoder.has_partial_event());
    }

    #[test]
    fn unknown_events_commit_and_empty_completion_is_terminal() {
        assert_eq!(
            event_kind(&serde_json::json!({"type":"future.event"})),
            Ok(EventKind::Content)
        );
        assert_eq!(
            event_kind(
                &serde_json::json!({"type":"response.completed","response":{"id":"resp_empty","output":[]}})
            ),
            Ok(EventKind::Completed)
        );
        assert!(event_kind(&serde_json::json!({"type":"response.completed"})).is_err());
        assert_eq!(
            event_kind(&metadata_event("synthetic")),
            Ok(EventKind::Metadata)
        );
    }

    #[test]
    fn decoder_rejects_invalid_json_and_large_pending_event() {
        let mut decoder = EventDecoder::default();
        decoder.push(b"data: invalid\n\n").unwrap();
        assert!(decoder.next().is_err());
        let mut decoder = EventDecoder::default();
        assert!(decoder.push(&vec![b'x'; MAX_MESSAGE_BYTES + 1]).is_err());
    }

    #[test]
    fn history_matches_full_replay_but_not_an_older_tool_round() {
        let input = serde_json::json!({"type":"message","role":"user","content":[{"type":"input_text","text":"synthetic"}]});
        let call = serde_json::json!({"type":"function_call","name":"exec","call_id":"call_a","arguments":"{}"});
        let output =
            serde_json::json!({"type":"function_call_output","call_id":"call_a","output":"ok"});
        let mut history = HistoryDigest::from_items(std::slice::from_ref(&input));
        history.append(std::slice::from_ref(&call));
        history.append(std::slice::from_ref(&output));
        assert_eq!(
            history,
            HistoryDigest::from_items(&[input.clone(), call, output.clone()])
        );
        assert_ne!(history, HistoryDigest::from_items(&[input, output]));
        let reordered: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        assert_eq!(
            HistoryDigest::from_value(&reordered),
            HistoryDigest::from_value(&serde_json::json!({"a":1,"b":2}))
        );
    }
    #[test]
    fn history_projection_matches_retained_tool_fields_without_erasing_identity() {
        let output = serde_json::json!({"type":"function_call","id":"fc_one","name":"exec","arguments":"{}","call_id":"call_one","status":"completed","encrypted_function_args":["private"],"internal_chat_message_metadata_passthrough":{"turn_id":"internal"}});
        let input = serde_json::json!({"type":"function_call","id":"fc_one","name":"exec","arguments":"{}","call_id":"call_one"});
        let expected = HistoryDigest::from_items(&[output]);
        assert!(expected.is_recoverable());
        assert_eq!(
            expected,
            HistoryDigest::from_items(std::slice::from_ref(&input))
        );
        for field in ["id", "call_id", "name", "arguments"] {
            let mut changed = input.clone();
            changed[field] = Value::String("changed_value".into());
            assert_ne!(expected, HistoryDigest::from_items(&[changed]));
        }
        let mut legacy = input.clone();
        legacy["id"] = Value::String("legacy".into());
        let mut missing = input;
        missing.as_object_mut().unwrap().remove("id");
        assert_eq!(
            HistoryDigest::from_items(&[legacy]),
            HistoryDigest::from_items(&[missing])
        );
    }

    #[test]
    fn unknown_or_malformed_history_cannot_be_matched_for_recovery() {
        for item in [
            serde_json::json!({"type":"future_item","content":"secret"}),
            serde_json::json!({"type":"function_call","name":"exec","arguments":"{}","call_id":"call_one","future_semantic_state":"value"}),
            serde_json::json!({"type":"message","role":"user","content":[{"type":"summary_text","text":"invalid context"}]}),
            serde_json::json!({"type":"function_call_output","output":42}),
        ] {
            assert!(!HistoryDigest::from_items(&[item]).is_recoverable());
        }
        assert_ne!(
            HistoryDigest::from_value(&serde_json::json!({"model":"a"})),
            HistoryDigest::from_value(&serde_json::json!({"model":"b"}))
        );
    }
}
