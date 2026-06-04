//! Unified streaming translation wrapper.
//!
//! `BridgeStream<S>` wraps an upstream byte stream and translates SSE events
//! through the Outbound → IR → Inbound pipeline.  When `active` is false the
//! stream is a zero-cost passthrough.

use super::traits::{BridgeContext, BridgeError};
use axum::body::Bytes;
use futures_core::Stream;
use serde_json::Value;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

const MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES: usize = 1024 * 1024;
const BRIDGE_SSE_FRAME_TOO_LARGE: &[u8] = concat!(
    "event: error\n",
    "data: {\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"bridge_sse_frame_too_large\"}}\n\n"
)
.as_bytes();

/// Generic stream wrapper that translates upstream SSE events via IR.
pub(crate) struct BridgeStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    active: bool,
    translator: Option<StreamTranslatorOwned>,
    ctx: BridgeContext,
    /// Buffered output frames ready to be yielded.
    buffer: VecDeque<Bytes>,
    /// Accumulator for partial SSE lines from the upstream.
    line_buf: Vec<u8>,
    terminated: bool,
}

/// Owned version of StreamTranslator that doesn't borrow from Bridge.
///
/// Because the Bridge is consumed when creating the stream pipeline, we need
/// to own the Inbound/Outbound trait objects directly.
pub(crate) struct StreamTranslatorOwned {
    pub inbound: Box<dyn super::traits::Inbound>,
    pub outbound: Box<dyn super::traits::Outbound>,
    pub state: super::traits::StreamState,
}

impl StreamTranslatorOwned {
    /// Translate a single upstream SSE event into client-facing SSE bytes.
    pub fn translate_event(
        &mut self,
        event_type: &str,
        data: &Value,
        ctx: &BridgeContext,
    ) -> Result<Vec<Bytes>, BridgeError> {
        self.state.enable_reasoning_to_thinking = ctx.cx2cc_settings.enable_reasoning_to_thinking;
        let ir_chunks = self
            .outbound
            .sse_event_to_ir(event_type, data, &mut self.state)?;
        let mut output = Vec::new();
        for chunk in &ir_chunks {
            let mut frames = self.inbound.ir_chunk_to_sse(chunk, ctx)?;
            output.append(&mut frames);
        }
        Ok(output)
    }
}

impl<S> BridgeStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    /// Convenience constructor for CX2CC translation.
    ///
    /// When `active` is false the stream is a zero-cost passthrough.
    /// When `active` is true a fresh CX2CC bridge translator is created.
    pub fn for_cx2cc(
        upstream: S,
        active: bool,
        requested_model: Option<String>,
        cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
    ) -> Self {
        if !active {
            let dummy_ctx = BridgeContext {
                claude_models: crate::domain::providers::ClaudeModels::default(),
                cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
                requested_model: None,
                mapped_model: None,
                stream_requested: false,
                is_chatgpt_backend: false,
            };
            return Self::new(upstream, false, None, dummy_ctx);
        }

        let bridge = match super::registry::get_bridge("cx2cc") {
            Some(b) => b,
            None => {
                tracing::error!("cx2cc bridge not found in registry; falling back to passthrough");
                let dummy_ctx = BridgeContext {
                    claude_models: crate::domain::providers::ClaudeModels::default(),
                    cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(
                    ),
                    requested_model: None,
                    mapped_model: None,
                    stream_requested: false,
                    is_chatgpt_backend: false,
                };
                return Self::new(upstream, false, None, dummy_ctx);
            }
        };
        let translator = StreamTranslatorOwned {
            inbound: bridge.inbound,
            outbound: bridge.outbound,
            state: super::traits::StreamState::default(),
        };
        let ctx = BridgeContext {
            claude_models: crate::domain::providers::ClaudeModels::default(),
            cx2cc_settings,
            requested_model,
            mapped_model: None,
            stream_requested: true,
            is_chatgpt_backend: false,
        };
        Self::new(upstream, true, Some(translator), ctx)
    }

    /// Create a new bridge stream.
    ///
    /// When `active` is false, the stream simply forwards upstream bytes
    /// without any processing.
    pub fn new(
        upstream: S,
        active: bool,
        translator: Option<StreamTranslatorOwned>,
        ctx: BridgeContext,
    ) -> Self {
        Self {
            upstream,
            active,
            translator,
            ctx,
            buffer: VecDeque::new(),
            line_buf: Vec::new(),
            terminated: false,
        }
    }

    fn terminate_oversized_frame(&mut self) {
        tracing::warn!(
            max_bytes = MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES,
            "bridge stream SSE frame exceeded maximum buffered size"
        );
        self.line_buf.clear();
        self.buffer
            .push_back(Bytes::from_static(BRIDGE_SSE_FRAME_TOO_LARGE));
        self.terminated = true;
    }

    /// Process a raw byte chunk from upstream: split into SSE frames, translate
    /// each, and push the results into `self.buffer`.
    fn process_chunk(&mut self, bytes: &[u8]) {
        if self.translator.is_none() || self.terminated {
            return;
        }

        let mut remaining = bytes;
        while !remaining.is_empty() && !self.terminated {
            let available = MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES.saturating_sub(self.line_buf.len());
            if available == 0 {
                self.terminate_oversized_frame();
                return;
            }

            let take = remaining.len().min(available);
            self.line_buf.extend_from_slice(&remaining[..take]);
            remaining = &remaining[take..];
            self.process_complete_frames();

            if !remaining.is_empty() && self.line_buf.len() >= MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES {
                self.terminate_oversized_frame();
                return;
            }
        }

        if self.line_buf.len() >= MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES {
            self.terminate_oversized_frame();
        }
    }

    fn process_complete_frames(&mut self) {
        // SSE frames are delimited by `\n\n` or `\r\n\r\n`.
        while let Some(end) = find_sse_event_end(&self.line_buf) {
            let frame_bytes: Vec<u8> = self.line_buf.drain(..end).collect();
            let frame_str = match std::str::from_utf8(&frame_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Some((event_type, data)) = parse_sse_frame(frame_str) {
                let Some(translator) = self.translator.as_mut() else {
                    return;
                };
                match translator.translate_event(&event_type, &data, &self.ctx) {
                    Ok(frames) => {
                        for f in frames {
                            self.buffer.push_back(f);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("bridge stream translation error: {e}");
                    }
                }
            }
        }
    }
}

impl<S> Stream for BridgeStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if !this.active {
            return Pin::new(&mut this.upstream).poll_next(cx);
        }

        loop {
            // Yield buffered frames first.
            if let Some(frame) = this.buffer.pop_front() {
                return Poll::Ready(Some(Ok(frame)));
            }

            if this.terminated {
                return Poll::Ready(None);
            }

            // Poll upstream for more data.
            match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.process_chunk(&bytes);
                    // Loop back to check buffer — avoids spurious wakeup.
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// ─── SSE parsing helpers ────────────────────────────────────────────────────

/// Find the byte offset immediately after the first complete SSE event,
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

/// Parse a single SSE frame string into (event_type, data_json).
///
/// Supports both `event: xxx\ndata: {...}\n\n` and `data: {...}\n\n` formats.
/// In the latter case, the event type is inferred from the `type` field of the
/// JSON data.
fn parse_sse_frame(frame: &str) -> Option<(String, Value)> {
    let mut event_type = None;
    let mut data_parts: Vec<&str> = Vec::new();

    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            event_type = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            let payload = rest.trim_start();
            if payload == "[DONE]" {
                return None;
            }
            data_parts.push(payload);
        }
    }

    if data_parts.is_empty() {
        return None;
    }
    let data_str = data_parts.join("\n");
    let data: Value = serde_json::from_str(&data_str).ok()?;

    // Infer event type from data.type if not explicitly provided.
    let event_type = event_type.unwrap_or_else(|| {
        data.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string()
    });

    Some((event_type, data))
}

/// Aggregate an OpenAI Responses SSE stream into a single JSON response.
pub(crate) fn aggregate_responses_event_stream(raw: &[u8]) -> Result<Value, String> {
    let mut response: Option<Value> = None;
    let mut output: Vec<Value> = Vec::new();
    let mut cursor = 0usize;

    while let Some(relative_end) = find_sse_event_end(&raw[cursor..]) {
        let event_end = cursor + relative_end;
        let frame = &raw[cursor..event_end];
        cursor = event_end;
        let text =
            std::str::from_utf8(frame).map_err(|e| format!("invalid utf-8 in SSE frame: {e}"))?;
        let Some((event_name, data)) = parse_sse_frame(text) else {
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

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::task::Wake;

    #[test]
    fn find_sse_event_end_basic() {
        assert_eq!(find_sse_event_end(b"abc\n\ndef"), Some(5));
        assert_eq!(find_sse_event_end(b"abc\ndef"), None);
        assert_eq!(find_sse_event_end(b"\n\n"), Some(2));
        assert_eq!(find_sse_event_end(b"abc\r\n\r\ndef"), Some(7));
    }

    #[test]
    fn parse_sse_frame_with_event() {
        let frame = "event: response.created\ndata: {\"id\":\"r1\"}\n\n";
        let (evt, data) = parse_sse_frame(frame).unwrap();
        assert_eq!(evt, "response.created");
        assert_eq!(data.get("id").unwrap().as_str().unwrap(), "r1");
    }

    #[test]
    fn parse_sse_frame_data_only_infers_type() {
        let frame = "data: {\"type\":\"response.completed\",\"id\":\"r2\"}\n\n";
        let (evt, data) = parse_sse_frame(frame).unwrap();
        assert_eq!(evt, "response.completed");
        assert_eq!(data.get("id").unwrap().as_str().unwrap(), "r2");
    }

    #[test]
    fn parse_sse_frame_done_returns_none() {
        let frame = "data: [DONE]\n\n";
        assert!(parse_sse_frame(frame).is_none());
    }

    #[test]
    fn parse_sse_frame_comment_lines_ignored() {
        let frame = ": keepalive\ndata: {\"type\":\"ping\"}\n\n";
        let (evt, _) = parse_sse_frame(frame).unwrap();
        assert_eq!(evt, "ping");
    }

    #[test]
    fn aggregate_responses_event_stream_handles_many_frames_with_trailing_partial() {
        let mut raw = String::from(
            "event: response.created\n\
             data: {\"response\":{\"id\":\"resp_many\",\"status\":\"in_progress\"}}\n\n",
        );
        for index in 0..128 {
            raw.push_str(&format!(
                "event: response.output_item.done\n\
                 data: {{\"item\":{{\"id\":\"msg_{index}\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[]}}}}\n\n"
            ));
        }
        raw.push_str(
            "event: response.completed\n\
             data: {\"response\":{\"id\":\"resp_many\",\"status\":\"completed\"}}\n\n\
             event: response.output_item.done\n\
             data: {\"item\":{\"id\":\"partial\"",
        );

        let aggregated = aggregate_responses_event_stream(raw.as_bytes()).expect("aggregate");

        assert_eq!(aggregated["id"], "resp_many");
        assert_eq!(aggregated["status"], "completed");
        assert_eq!(
            aggregated
                .get("output")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(128)
        );
    }

    struct MockStream {
        items: VecDeque<Result<Bytes, reqwest::Error>>,
    }

    impl MockStream {
        fn new(items: Vec<Result<Bytes, reqwest::Error>>) -> Self {
            Self {
                items: items.into_iter().collect(),
            }
        }
    }

    impl Stream for MockStream {
        type Item = Result<Bytes, reqwest::Error>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.items.pop_front())
        }
    }

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    #[test]
    fn bridge_stream_emits_error_and_stops_when_frame_buffer_exceeds_limit() {
        let oversized = Bytes::from(vec![b'a'; MAX_BRIDGE_SSE_FRAME_BUFFER_BYTES + 1]);
        let mut stream = BridgeStream::for_cx2cc(
            MockStream::new(vec![Ok(oversized)]),
            true,
            None,
            crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
        );
        let waker = std::task::Waker::from(Arc::new(NoopWaker));
        let mut cx = Context::from_waker(&waker);

        let first = Pin::new(&mut stream).poll_next(&mut cx);
        let Poll::Ready(Some(Ok(frame))) = first else {
            panic!("expected bridge error frame, got {first:?}");
        };
        let text = std::str::from_utf8(frame.as_ref()).expect("utf-8 error frame");
        assert!(text.contains("event: error"));
        assert!(text.contains("bridge_sse_frame_too_large"));

        assert!(matches!(
            Pin::new(&mut stream).poll_next(&mut cx),
            Poll::Ready(None)
        ));
    }
}
