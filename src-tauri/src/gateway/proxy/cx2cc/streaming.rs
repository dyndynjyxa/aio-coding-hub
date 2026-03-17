//! SSE stream translation: OpenAI Responses API → Anthropic Messages API.
//!
//! Wraps an upstream byte stream that emits OpenAI Responses API SSE events and
//! re-emits Anthropic Messages API SSE events.  Each emitted frame uses the format:
//!
//! ```text
//! event: {event_type}
//! data: {json}
//!
//! ```
//!
//! The translation is driven by a state machine that tracks the current Anthropic
//! content-block index, whether a block is currently open, and the active tool-call
//! identity so that `content_block_delta` events are emitted with the correct index.

use axum::body::Bytes;
use futures_core::Stream;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Internal state machine
// ---------------------------------------------------------------------------

/// Tracks the identity of the tool-call block that is currently open.
struct ActiveToolBlock {
    /// The OpenAI `item_id` used to correlate `function_call_arguments.delta` events.
    item_id: String,
    /// The Anthropic content-block index assigned to this tool-use block.
    block_index: u32,
}

struct TranslationState {
    /// Monotonically increasing Anthropic content-block index; advanced when a new
    /// block is opened.
    block_index: u32,
    /// Whether a content block (`text` or `tool_use`) is currently open.
    block_open: bool,
    /// Present when the currently open block is a `tool_use` block.
    active_tool: Option<ActiveToolBlock>,
    /// Tracks whether this response emitted any tool-use block so the final
    /// Anthropic stop_reason can stay `tool_use` even after the block closes.
    saw_tool_use: bool,
    /// Tracks whether the currently open text block already emitted visible text.
    text_emitted_in_current_block: bool,
    /// Tracks whether any visible assistant text has been emitted in this response.
    saw_visible_text: bool,
    /// Synthetic message ID generated once at stream construction.
    message_id: String,
    /// Original Claude model requested by the client when bridging Anthropic to OpenAI.
    requested_model: Option<String>,
    /// Model string forwarded from the upstream `response.created` event.
    model: String,
}

impl TranslationState {
    #[cfg(test)]
    fn new() -> Self {
        Self::with_requested_model(None)
    }

    fn with_requested_model(requested_model: Option<String>) -> Self {
        Self {
            block_index: 0,
            block_open: false,
            active_tool: None,
            saw_tool_use: false,
            text_emitted_in_current_block: false,
            saw_visible_text: false,
            message_id: generate_message_id(),
            requested_model: requested_model.filter(|model| !model.is_empty()),
            model: String::new(),
        }
    }

    /// Return the current block index and advance the counter.
    fn next_index(&mut self) -> u32 {
        let idx = self.block_index;
        self.block_index += 1;
        idx
    }

    /// The index of the most recently opened block (index - 1 after advancing).
    fn current_index(&self) -> u32 {
        self.block_index.saturating_sub(1)
    }
}

// ---------------------------------------------------------------------------
// Stream struct
// ---------------------------------------------------------------------------

pub(crate) struct CX2CCSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    active: bool,
    /// Byte accumulation buffer for incomplete SSE frames.
    buffer: Vec<u8>,
    /// Fully translated SSE frames ready to be yielded.
    queued: VecDeque<Bytes>,
    /// Deferred upstream error, emitted after the queue drains.
    pending_error: Option<reqwest::Error>,
    upstream_done: bool,
    state: TranslationState,
}

impl<S> CX2CCSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(crate) fn new(upstream: S, active: bool, requested_model: Option<String>) -> Self {
        let state = TranslationState::with_requested_model(requested_model);
        Self {
            upstream,
            active,
            buffer: Vec::new(),
            queued: VecDeque::new(),
            pending_error: None,
            upstream_done: false,
            state,
        }
    }

    /// Drain complete SSE frames from the buffer, translate each one, and
    /// append the resulting bytes to `self.queued`.
    fn queue_buffered_events(&mut self) {
        if !self.active {
            if !self.buffer.is_empty() {
                let chunk: Vec<u8> = self.buffer.drain(..).collect();
                self.queued.push_back(Bytes::from(chunk));
            }
            return;
        }
        while let Some(event_end) = find_sse_event_end(&self.buffer) {
            let raw: Vec<u8> = self.buffer.drain(..event_end).collect();
            for bytes in translate_event(&mut self.state, &raw) {
                self.queued.push_back(bytes);
            }
        }
    }
}

impl<S> Stream for CX2CCSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        loop {
            if let Some(chunk) = this.queued.pop_front() {
                return Poll::Ready(Some(Ok(chunk)));
            }

            if let Some(err) = this.pending_error.take() {
                return Poll::Ready(Some(Err(err)));
            }

            if this.upstream_done {
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    this.upstream_done = true;
                }
                Poll::Ready(Some(Err(err))) => {
                    this.upstream_done = true;
                    this.pending_error = Some(err);
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    this.buffer.extend_from_slice(chunk.as_ref());
                    this.queue_buffered_events();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Event translation (pure functions over TranslationState)
// ---------------------------------------------------------------------------

/// Translate a single raw SSE frame into zero or more Anthropic SSE frames.
fn translate_event(state: &mut TranslationState, raw: &[u8]) -> Vec<Bytes> {
    let Some((event_name, data)) = parse_sse_frame(raw) else {
        return Vec::new();
    };

    match event_name.as_str() {
        "response.created" => handle_response_created(state, &data),
        "response.output_item.added" => handle_output_item_added(state, &data),
        "response.content_part.added" => handle_content_part_added(state, &data),
        "response.content_part.done" => handle_content_part_done(state, &data),
        "response.output_text.delta" | "response.content_part.delta" => {
            handle_text_delta(state, &data)
        }
        "response.output_text.done" => handle_text_done(state, &data),
        "response.refusal.delta" => handle_refusal_delta(state, &data),
        "response.refusal.done" => handle_refusal_done(state, &data),
        "response.function_call_arguments.delta" => handle_function_args_delta(state, &data),
        "response.output_item.done" => handle_output_item_done(state, &data),
        "response.completed" => handle_response_completed(state, &data),
        _ => Vec::new(),
    }
}

fn handle_response_created(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let response = data.get("response").unwrap_or(data);

    let upstream_model = response.get("model").and_then(Value::as_str).unwrap_or("");
    state.model = state
        .requested_model
        .as_deref()
        .filter(|model| !model.is_empty())
        .unwrap_or(upstream_model)
        .to_string();

    let input_tokens = response
        .pointer("/usage/input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let msg_start = sse_frame(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": state.message_id,
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": state.model,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": input_tokens,
                    "output_tokens": 0
                }
            }
        }),
    );

    let ping = sse_frame("ping", json!({"type": "ping"}));

    // Open the initial text content block at index 0.
    let idx = state.next_index();
    state.block_open = true;
    state.active_tool = None;
    state.text_emitted_in_current_block = false;
    let block_start = sse_frame(
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": idx,
            "content_block": {"type": "text", "text": ""}
        }),
    );

    vec![msg_start, ping, block_start]
}

fn open_text_block(state: &mut TranslationState) -> Bytes {
    let idx = state.next_index();
    state.block_open = true;
    state.active_tool = None;
    state.text_emitted_in_current_block = false;
    sse_frame(
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": idx,
            "content_block": {"type": "text", "text": ""}
        }),
    )
}

fn handle_output_item_added(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let item = match data.get("item") {
        Some(v) => v,
        None => return Vec::new(),
    };

    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");

    match item_type {
        // A message-type item has no new block; the initial text block was opened
        // during `handle_response_created`.
        "message" => {
            if state.block_open && state.active_tool.is_none() {
                Vec::new()
            } else {
                vec![open_text_block(state)]
            }
        }

        "function_call" => {
            let call_id = item
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let item_id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let mut out = Vec::new();

            // Close the currently open block (text block from response.created) if needed.
            if state.block_open {
                out.push(sse_frame(
                    "content_block_stop",
                    json!({"type": "content_block_stop", "index": state.current_index()}),
                ));
                state.block_open = false;
                state.text_emitted_in_current_block = false;
            }

            let idx = state.next_index();
            state.block_open = true;
            state.active_tool = Some(ActiveToolBlock {
                item_id,
                block_index: idx,
            });
            state.saw_tool_use = true;
            state.text_emitted_in_current_block = false;

            out.push(sse_frame(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": idx,
                    "content_block": {
                        "type": "tool_use",
                        "id": call_id,
                        "name": name,
                        "input": {}
                    }
                }),
            ));
            out
        }

        _ => Vec::new(),
    }
}

fn content_part_text(part: &Value) -> Option<String> {
    match part.get("type").and_then(Value::as_str).unwrap_or("") {
        "output_text" => part.get("text").and_then(Value::as_str),
        "refusal" => part
            .get("refusal")
            .and_then(Value::as_str)
            .or_else(|| part.get("text").and_then(Value::as_str)),
        _ => None,
    }
    .map(str::trim)
    .filter(|text| !text.is_empty())
    .map(str::to_string)
}

fn emit_text_delta_if_needed(state: &mut TranslationState, text: &str) -> Vec<Bytes> {
    let mut out = Vec::new();
    if !state.block_open || state.active_tool.is_some() {
        out.push(open_text_block(state));
    }

    if text.is_empty() {
        return out;
    }

    state.text_emitted_in_current_block = true;
    state.saw_visible_text = true;
    out.push(sse_frame(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": state.current_index(),
            "delta": {"type": "text_delta", "text": text}
        }),
    ));
    out
}

fn extract_response_text_parts(response: &Value) -> Vec<String> {
    let Some(items) = response.get("output").and_then(Value::as_array) else {
        return Vec::new();
    };

    items
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(extract_message_item_text_parts)
        .collect()
}

fn handle_content_part_added(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let Some(part) = data.get("part") else {
        return Vec::new();
    };

    if !state.block_open || state.active_tool.is_some() {
        return vec![open_text_block(state)];
    }

    if state.text_emitted_in_current_block {
        return Vec::new();
    }

    content_part_text(part)
        .map(|text| emit_text_delta_if_needed(state, &text))
        .unwrap_or_default()
}

fn handle_text_delta(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let text = data
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default();

    emit_text_delta_if_needed(state, text)
}

fn extract_message_item_text_parts(item: &Value) -> Vec<String> {
    let Some(blocks) = item.get("content").and_then(Value::as_array) else {
        return Vec::new();
    };

    blocks
        .iter()
        .filter_map(
            |block| match block.get("type").and_then(Value::as_str).unwrap_or("") {
                "output_text" => block.get("text").and_then(Value::as_str),
                "refusal" => block.get("refusal").and_then(Value::as_str),
                _ => None,
            },
        )
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .collect()
}

fn handle_content_part_done(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    if state.text_emitted_in_current_block {
        return Vec::new();
    }
    let Some(part) = data.get("part") else {
        return Vec::new();
    };
    content_part_text(part)
        .map(|text| emit_text_delta_if_needed(state, &text))
        .unwrap_or_default()
}

fn handle_text_done(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    if state.text_emitted_in_current_block {
        return Vec::new();
    }
    let text = data
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    text.map(|value| emit_text_delta_if_needed(state, value))
        .unwrap_or_default()
}

fn handle_refusal_delta(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let text = data
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default();
    emit_text_delta_if_needed(state, text)
}

fn handle_refusal_done(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    if state.text_emitted_in_current_block {
        return Vec::new();
    }
    let refusal = data
        .get("refusal")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    refusal
        .map(|value| emit_text_delta_if_needed(state, value))
        .unwrap_or_default()
}

fn handle_function_args_delta(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let partial_json = data
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let item_id = data.get("item_id").and_then(Value::as_str).unwrap_or("");

    // Resolve the block index from the tracked active tool block.  Fall back to
    // `output_index` from the event, and ultimately to the current block index.
    let idx = state
        .active_tool
        .as_ref()
        .filter(|t| t.item_id == item_id || item_id.is_empty())
        .map(|t| t.block_index)
        .or_else(|| {
            data.get("output_index")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
        })
        .unwrap_or_else(|| state.current_index());

    vec![sse_frame(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": idx,
            "delta": {"type": "input_json_delta", "partial_json": partial_json}
        }),
    )]
}

fn handle_output_item_done(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let item = match data.get("item") {
        Some(v) => v,
        None => return Vec::new(),
    };

    let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");

    if state
        .active_tool
        .as_ref()
        .map(|t| t.item_id == item_id)
        .unwrap_or(false)
    {
        let tool = state.active_tool.take().unwrap();
        state.block_open = false;
        state.text_emitted_in_current_block = false;
        return vec![sse_frame(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": tool.block_index}),
        )];
    }

    let mut out = Vec::new();
    if !state.text_emitted_in_current_block {
        for text in extract_message_item_text_parts(item) {
            out.extend(emit_text_delta_if_needed(state, &text));
        }
    }

    state.block_open = false;
    state.text_emitted_in_current_block = false;
    out.push(sse_frame(
        "content_block_stop",
        json!({"type": "content_block_stop", "index": state.current_index()}),
    ));
    out
}

fn handle_response_completed(state: &mut TranslationState, data: &Value) -> Vec<Bytes> {
    let response = data.get("response").unwrap_or(data);

    let output_tokens = response
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let status = response.get("status").and_then(Value::as_str).unwrap_or("");
    let stop_reason = stop_reason_from_status(status, state.saw_tool_use);

    let mut out = Vec::new();

    if !state.saw_visible_text {
        for text in extract_response_text_parts(response) {
            out.extend(emit_text_delta_if_needed(state, &text));
        }
    }

    if state.block_open {
        let idx = state.current_index();
        out.push(sse_frame(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": idx}),
        ));
        state.block_open = false;
        state.active_tool = None;
        state.text_emitted_in_current_block = false;
    }

    out.push(sse_frame(
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": stop_reason,
                "stop_sequence": null
            },
            "usage": {
                "output_tokens": output_tokens
            }
        }),
    ));

    out.push(sse_frame("message_stop", json!({"type": "message_stop"})));

    out
}

// ---------------------------------------------------------------------------
// SSE frame parsing
// ---------------------------------------------------------------------------

/// Returns the byte offset immediately after the first complete SSE event,
/// terminated by `\n\n` or `\r\n\r\n`.
fn find_sse_event_end(buffer: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < buffer.len() {
        if buffer[i] == b'\n' {
            if i + 1 < buffer.len() && buffer[i + 1] == b'\n' {
                return Some(i + 2);
            }
        } else if buffer[i] == b'\r'
            && i + 3 < buffer.len()
            && buffer[i + 1] == b'\n'
            && buffer[i + 2] == b'\r'
            && buffer[i + 3] == b'\n'
        {
            return Some(i + 4);
        }
        i += 1;
    }
    None
}

/// Parse a raw SSE frame into `(event_name, data_json)`.
///
/// Returns `None` for frames that carry no actionable data — e.g. `[DONE]`
/// sentinels, comment-only frames, or frames missing an `event:` field.
fn parse_sse_frame(raw: &[u8]) -> Option<(String, Value)> {
    let text = std::str::from_utf8(raw).ok()?;

    let mut event_name: Option<String> = None;
    let mut data_parts: Vec<&str> = Vec::new();

    for line in text.lines() {
        let line = line.trim_end_matches('\r');

        if let Some(rest) = line.strip_prefix("event:") {
            event_name = Some(rest.trim_start().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            let payload = rest.trim_start();
            if payload != "[DONE]" {
                data_parts.push(payload);
            }
        }
        // Ignore comment lines (`:`) and unknown field names.
    }

    if data_parts.is_empty() {
        return None;
    }

    let joined = data_parts.join("\n");
    let value: Value = serde_json::from_str(&joined).ok()?;
    let name = event_name.or_else(|| {
        value
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    })?;
    Some((name, value))
}

// ---------------------------------------------------------------------------
// SSE frame formatting
// ---------------------------------------------------------------------------

fn sse_frame(event_type: &str, payload: Value) -> Bytes {
    let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    Bytes::from(format!("event: {event_type}\ndata: {data}\n\n").into_bytes())
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn generate_message_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("msg_cx2cc_{millis}")
}

fn stop_reason_from_status(status: &str, has_tool: bool) -> &'static str {
    match status {
        "incomplete" => "max_tokens",
        "completed" if has_tool => "tool_use",
        _ => "end_turn",
    }
}

pub(crate) fn aggregate_responses_event_stream(raw: &[u8]) -> Result<Value, String> {
    let mut buffer = raw.to_vec();
    let mut response: Option<Value> = None;
    let mut output: Vec<Value> = Vec::new();

    while let Some(event_end) = find_sse_event_end(&buffer) {
        let frame: Vec<u8> = buffer.drain(..event_end).collect();
        let Some((event_name, data)) = parse_sse_frame(&frame) else {
            continue;
        };

        match event_name.as_str() {
            "response.created" => {
                let created = data.get("response").cloned().unwrap_or(data);
                response = Some(created);
            }
            "response.output_item.done" => {
                let item = data
                    .get("item")
                    .cloned()
                    .ok_or_else(|| "missing item in response.output_item.done".to_string())?;
                upsert_output_item(&mut output, item);
            }
            "response.completed" => {
                let completed = data.get("response").cloned().unwrap_or(data);
                if let Some(existing) = response.as_mut() {
                    merge_response_object(existing, &completed);
                } else {
                    response = Some(completed);
                }
            }
            "error" => {
                let detail = data
                    .get("detail")
                    .and_then(Value::as_str)
                    .or_else(|| data.get("message").and_then(Value::as_str))
                    .unwrap_or("unknown SSE error");
                return Err(detail.to_string());
            }
            _ => {}
        }
    }

    let mut response =
        response.ok_or_else(|| "missing response.created/response.completed".to_string())?;
    let obj = response
        .as_object_mut()
        .ok_or_else(|| "aggregated response is not an object".to_string())?;
    obj.insert("output".to_string(), Value::Array(output));
    Ok(response)
}

pub(crate) fn responses_json_to_anthropic_sse(response: &Value) -> Result<Bytes, String> {
    responses_json_to_anthropic_sse_with_model_override(response, None)
}

pub(crate) fn responses_json_to_anthropic_sse_with_model_override(
    response: &Value,
    model_override: Option<&str>,
) -> Result<Bytes, String> {
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing response.output".to_string())?;

    let mut state = TranslationState::with_requested_model(model_override.map(str::to_string));
    let mut translated = Vec::new();
    let translated_model = model_override
        .filter(|model| !model.is_empty())
        .map(|model| json!(model))
        .unwrap_or_else(|| response.get("model").cloned().unwrap_or_else(|| json!("")));

    let created = json!({
        "response": {
            "id": response.get("id").cloned().unwrap_or_else(|| json!("")),
            "model": translated_model,
            "status": "in_progress",
            "output": [],
            "usage": {
                "input_tokens": response
                    .pointer("/usage/input_tokens")
                    .cloned()
                    .unwrap_or_else(|| json!(0)),
                "output_tokens": 0
            }
        }
    });
    append_translated_event(&mut state, &mut translated, "response.created", created);

    for item in output {
        match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "message" => {
                append_translated_event(
                    &mut state,
                    &mut translated,
                    "response.output_item.done",
                    json!({ "item": item }),
                );
            }
            "function_call" => {
                append_translated_event(
                    &mut state,
                    &mut translated,
                    "response.output_item.added",
                    json!({ "item": item }),
                );
                if let Some(arguments) = item
                    .get("arguments")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                {
                    append_translated_event(
                        &mut state,
                        &mut translated,
                        "response.function_call_arguments.delta",
                        json!({
                            "item_id": item.get("id").and_then(Value::as_str).unwrap_or(""),
                            "delta": arguments
                        }),
                    );
                }
                append_translated_event(
                    &mut state,
                    &mut translated,
                    "response.output_item.done",
                    json!({ "item": item }),
                );
            }
            _ => {}
        }
    }

    append_translated_event(
        &mut state,
        &mut translated,
        "response.completed",
        json!({ "response": response.clone() }),
    );

    Ok(Bytes::from(translated))
}

fn merge_response_object(base: &mut Value, update: &Value) {
    let (Some(base_obj), Some(update_obj)) = (base.as_object_mut(), update.as_object()) else {
        *base = update.clone();
        return;
    };

    for (key, value) in update_obj {
        if key == "output" {
            continue;
        }
        base_obj.insert(key.clone(), value.clone());
    }
}

fn upsert_output_item(output: &mut Vec<Value>, item: Value) {
    let item_id = item.get("id").and_then(Value::as_str);
    if let Some(item_id) = item_id {
        if let Some(existing) = output
            .iter_mut()
            .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(item_id))
        {
            *existing = item;
            return;
        }
    }
    output.push(item);
}

fn append_translated_event(
    state: &mut TranslationState,
    translated: &mut Vec<u8>,
    event_name: &str,
    payload: Value,
) {
    let raw = sse_frame(event_name, payload);
    for frame in translate_event(state, raw.as_ref()) {
        translated.extend_from_slice(frame.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_responses_event_stream, parse_sse_frame, responses_json_to_anthropic_sse,
        responses_json_to_anthropic_sse_with_model_override, translate_event, TranslationState,
    };
    use serde_json::json;

    #[test]
    fn function_call_stream_completes_with_tool_use_stop_reason() {
        let mut state = TranslationState::new();
        let mut translated = Vec::new();

        for raw in [
            concat!(
                "event: response.created\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n"
            ),
            concat!(
                "event: response.output_item.added\n",
                "data: {\"item\":{\"id\":\"fc_1\",\"type\":\"function_call\",\"call_id\":\"call_1\",\"name\":\"shell\",\"arguments\":\"\"}}\n\n"
            ),
            concat!(
                "event: response.function_call_arguments.delta\n",
                "data: {\"item_id\":\"fc_1\",\"delta\":\"{\\\"cmd\\\":\\\"pwd\\\"}\"}\n\n"
            ),
            concat!(
                "event: response.output_item.done\n",
                "data: {\"item\":{\"id\":\"fc_1\",\"type\":\"function_call\",\"call_id\":\"call_1\",\"name\":\"shell\",\"arguments\":\"{\\\"cmd\\\":\\\"pwd\\\"}\"}}\n\n"
            ),
            concat!(
                "event: response.completed\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
            ),
        ] {
            translated.extend(translate_event(&mut state, raw.as_bytes()));
        }

        let message_delta = translated
            .iter()
            .filter_map(|frame| parse_sse_frame(frame.as_ref()))
            .find(|(event, _)| event == "message_delta")
            .map(|(_, data)| data)
            .expect("message_delta frame");

        assert_eq!(message_delta["delta"]["stop_reason"], "tool_use");
    }

    #[test]
    fn output_item_done_message_content_becomes_text_delta() {
        let mut state = TranslationState::new();
        let mut translated = Vec::new();

        for raw in [
            concat!(
                "event: response.created\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n"
            ),
            concat!(
                "event: response.output_item.done\n",
                "data: {\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello from done\"}]}}\n\n"
            ),
            concat!(
                "event: response.completed\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
            ),
        ] {
            translated.extend(translate_event(&mut state, raw.as_bytes()));
        }

        let text_delta = translated
            .iter()
            .filter_map(|frame| parse_sse_frame(frame.as_ref()))
            .find(|(event, data)| {
                event == "content_block_delta"
                    && data["delta"]["type"] == "text_delta"
                    && data["delta"]["text"] == "Hello from done"
            })
            .map(|(_, data)| data)
            .expect("text delta from output_item.done");

        assert_eq!(text_delta["index"], 0);
    }

    #[test]
    fn output_text_done_becomes_text_delta_when_no_incremental_delta_arrives() {
        let mut state = TranslationState::new();
        let mut translated = Vec::new();

        for raw in [
            concat!(
                "event: response.created\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n"
            ),
            concat!(
                "event: response.content_part.added\n",
                "data: {\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\",\"annotations\":[]}}\n\n"
            ),
            concat!(
                "event: response.output_text.done\n",
                "data: {\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello from output_text.done\"}\n\n"
            ),
            concat!(
                "event: response.completed\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
            ),
        ] {
            translated.extend(translate_event(&mut state, raw.as_bytes()));
        }

        let text_delta = translated
            .iter()
            .filter_map(|frame| parse_sse_frame(frame.as_ref()))
            .find(|(event, data)| {
                event == "content_block_delta"
                    && data["delta"]["type"] == "text_delta"
                    && data["delta"]["text"] == "Hello from output_text.done"
            })
            .map(|(_, data)| data)
            .expect("text delta from output_text.done");

        assert_eq!(text_delta["index"], 0);
    }

    #[test]
    fn response_completed_output_becomes_text_delta_when_no_item_events_arrive() {
        let mut state = TranslationState::new();
        let mut translated = Vec::new();

        for raw in [
            concat!(
                "event: response.created\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n"
            ),
            concat!(
                "event: response.completed\n",
                "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"output\":[{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello from response.completed\"}]}],\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
            ),
        ] {
            translated.extend(translate_event(&mut state, raw.as_bytes()));
        }

        let text_delta = translated
            .iter()
            .filter_map(|frame| parse_sse_frame(frame.as_ref()))
            .find(|(event, data)| {
                event == "content_block_delta"
                    && data["delta"]["type"] == "text_delta"
                    && data["delta"]["text"] == "Hello from response.completed"
            })
            .map(|(_, data)| data)
            .expect("text delta from response.completed");

        assert_eq!(text_delta["index"], 0);
    }

    #[test]
    fn aggregates_responses_sse_into_openai_response_json() {
        let raw = concat!(
            "event: response.created\n",
            "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "event: response.output_item.done\n",
            "data: {\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n",
            "event: response.completed\n",
            "data: {\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
        );

        let response = aggregate_responses_event_stream(raw.as_bytes()).unwrap();

        assert_eq!(response["id"], "resp_123");
        assert_eq!(response["model"], "gpt-5");
        assert_eq!(response["status"], "completed");
        assert_eq!(response["usage"]["input_tokens"], 11);
        assert_eq!(response["usage"]["output_tokens"], 7);
        assert_eq!(response["output"][0]["type"], "message");
        assert_eq!(response["output"][0]["content"][0]["type"], "output_text");
        assert_eq!(response["output"][0]["content"][0]["text"], "Hello");
    }

    #[test]
    fn responses_json_can_be_synthesized_into_anthropic_sse() {
        let response = json!({
            "id": "resp_123",
            "status": "completed",
            "model": "gpt-5",
            "output": [
                {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello from synthesized SSE"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7
            }
        });

        let sse = responses_json_to_anthropic_sse(&response).unwrap();
        let text = std::str::from_utf8(sse.as_ref()).unwrap();

        assert!(text.contains("event: message_start"));
        assert!(text.contains("event: content_block_delta"));
        assert!(text.contains("Hello from synthesized SSE"));
        assert!(text.contains("event: message_stop"));
    }

    #[test]
    fn synthesized_anthropic_sse_uses_requested_model_override() {
        let response = json!({
            "id": "resp_123",
            "status": "completed",
            "model": "gpt-5",
            "output": [
                {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello from synthesized SSE"
                        }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7
            }
        });

        let sse = responses_json_to_anthropic_sse_with_model_override(
            &response,
            Some("claude-sonnet-4-5"),
        )
        .unwrap();

        let message_start = sse
            .split(|byte| *byte == b'\n')
            .collect::<Vec<_>>()
            .windows(2)
            .find_map(|window| {
                let event_line = std::str::from_utf8(window[0]).ok()?.trim_end_matches('\r');
                let data_line = std::str::from_utf8(window[1]).ok()?.trim_end_matches('\r');
                if event_line == "event: message_start" {
                    data_line
                        .strip_prefix("data: ")
                        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
                } else {
                    None
                }
            })
            .expect("message_start event");

        assert_eq!(message_start["message"]["model"], "claude-sonnet-4-5");
    }

    #[test]
    fn data_only_responses_events_use_payload_type_for_translation() {
        let mut state = TranslationState::new();
        let mut translated = Vec::new();

        for raw in [
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello from data-only event\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n",
        ] {
            translated.extend(translate_event(&mut state, raw.as_bytes()));
        }

        let text_delta = translated
            .iter()
            .filter_map(|frame| parse_sse_frame(frame.as_ref()))
            .find(|(event, data)| {
                event == "content_block_delta"
                    && data["delta"]["type"] == "text_delta"
                    && data["delta"]["text"] == "Hello from data-only event"
            })
            .map(|(_, data)| data)
            .expect("text delta from data-only response event");

        assert_eq!(text_delta["index"], 0);
    }

    #[test]
    fn aggregates_data_only_responses_sse_into_openai_response_json() {
        let raw = concat!(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[],\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}\n\n",
            "data: {\"type\":\"response.output_item.done\",\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello from data-only aggregate\"}]}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_123\",\"model\":\"gpt-5\",\"status\":\"completed\",\"usage\":{\"input_tokens\":11,\"output_tokens\":7}}}\n\n"
        );

        let response = aggregate_responses_event_stream(raw.as_bytes()).unwrap();

        assert_eq!(response["id"], "resp_123");
        assert_eq!(response["status"], "completed");
        assert_eq!(
            response["output"][0]["content"][0]["text"],
            "Hello from data-only aggregate"
        );
    }
}
