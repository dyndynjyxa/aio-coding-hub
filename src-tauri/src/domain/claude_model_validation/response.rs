use super::MAX_TEXT_PREVIEW_CHARS;
use reqwest::header::HeaderMap;

fn json_map_push_value(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    match map.get_mut(key) {
        None => {
            map.insert(key.to_string(), value);
        }
        Some(existing) => {
            if let Some(arr) = existing.as_array_mut() {
                arr.push(value);
            } else {
                let prev = existing.take();
                *existing = serde_json::Value::Array(vec![prev, value]);
            }
        }
    }
}

pub(super) fn response_headers_to_json(headers: &HeaderMap) -> serde_json::Value {
    let mut out = serde_json::Map::<String, serde_json::Value>::new();
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if name_str.trim().is_empty() {
            continue;
        }
        let Ok(value_str) = value.to_str() else {
            continue;
        };
        json_map_push_value(
            &mut out,
            name_str,
            super::masking::mask_response_header_value(name_str, value_str),
        );
    }
    serde_json::Value::Object(out)
}

pub(super) fn signals_from_text(text: &str) -> serde_json::Value {
    let lower = text.to_lowercase();
    let mentions_bedrock = lower.contains("amazon-bedrock")
        || lower.contains("bedrock-")
        || lower.contains("model group=bedrock");

    let mentions_max_tokens = lower.contains("max_tokens");
    let mentions_tokens_greater = lower.contains("must be greater") && lower.contains("_tokens");
    let mentions_invalid_signature =
        lower.contains("invalid") && lower.contains("signature");

    serde_json::json!({
        "mentions_amazon_bedrock": mentions_bedrock,
        "mentions_max_tokens": mentions_max_tokens,
        "mentions_max_tokens_must_be_greater_than_tokens": mentions_tokens_greater,
        "mentions_invalid_signature": mentions_invalid_signature,
    })
}

pub(super) fn has_cache_creation_detail(usage: Option<&serde_json::Value>) -> bool {
    let Some(obj) = usage.and_then(|v| v.as_object()) else {
        return false;
    };
    obj.contains_key("cache_creation_5m_input_tokens")
        || obj.contains_key("cache_creation_1h_input_tokens")
}

fn take_first_n_chars(s: &str, n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    s.chars().take(n).collect()
}

const MAX_ROUNDTRIP_THINKING_CHARS: usize = 120_000;
const MAX_ROUNDTRIP_SIGNATURE_CHARS: usize = 24_000;

#[derive(Default)]
pub(super) struct SseTextAccumulator {
    buffer: Vec<u8>,
    current_event: Vec<u8>,
    current_data: Vec<u8>,
    pub(super) total_chars: usize,
    pub(super) preview: String,
    pub(super) thinking_chars: usize,
    pub(super) thinking_preview: String,
    pub(super) thinking_block_seen: bool,
    pub(super) signature_chars: usize,
    pub(super) thinking_full: String,
    pub(super) signature_full: String,
    pub(super) signature_from_delta: bool,
    pub(super) message_delta_seen: bool,
    pub(super) message_delta_stop_reason: Option<String>,
    pub(super) message_delta_stop_reason_is_max_tokens: bool,
    pub(super) response_id: String,
    pub(super) service_tier: String,

    current_thinking_block_index: Option<i64>,
    capturing_thinking_block: bool,
    current_thinking_full: String,
    current_signature_full: String,
    current_signature_from_delta: bool,
}

impl SseTextAccumulator {
    pub(super) fn ingest_chunk(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);

        let buf = std::mem::take(&mut self.buffer);
        let mut start = 0usize;
        for (idx, b) in buf.iter().enumerate() {
            if *b != b'\n' {
                continue;
            }

            let mut line = &buf[start..idx];
            if line.last() == Some(&b'\r') {
                line = &line[..line.len().saturating_sub(1)];
            }
            self.ingest_line(line);
            start = idx + 1;
        }

        if start < buf.len() {
            self.buffer.extend_from_slice(&buf[start..]);
        }
    }

    pub(super) fn finalize(&mut self) {
        if !self.buffer.is_empty() {
            let mut tail = std::mem::take(&mut self.buffer);
            if tail.last() == Some(&b'\r') {
                tail.pop();
            }
            self.ingest_line(&tail);
        }
        self.flush_event();
        self.finalize_thinking_capture_best_effort();
    }

    fn ingest_line(&mut self, line: &[u8]) {
        if line.is_empty() {
            self.flush_event();
            return;
        }
        if line[0] == b':' {
            return;
        }
        if let Some(rest) = line.strip_prefix(b"event:") {
            let rest = trim_ascii(rest);
            self.current_event.clear();
            self.current_event.extend_from_slice(rest);
            return;
        }
        if let Some(rest) = line.strip_prefix(b"data:") {
            let mut rest = rest;
            if rest.first() == Some(&b' ') {
                rest = &rest[1..];
            }
            if rest == b"[DONE]" {
                return;
            }
            if !self.current_data.is_empty() {
                self.current_data.push(b'\n');
            }
            self.current_data.extend_from_slice(rest);
        }
    }

    fn flush_event(&mut self) {
        if self.current_data.is_empty() {
            self.current_event.clear();
            return;
        }

        let event_name = if self.current_event.is_empty() {
            b"message".to_vec()
        } else {
            self.current_event.clone()
        };
        let data_json: serde_json::Value = match serde_json::from_slice(&self.current_data) {
            Ok(v) => v,
            Err(_) => {
                self.current_event.clear();
                self.current_data.clear();
                return;
            }
        };

        self.ingest_event(&event_name, &data_json);
        self.current_event.clear();
        self.current_data.clear();
    }

    fn append_text(&mut self, text: &str) {
        self.total_chars = self.total_chars.saturating_add(text.chars().count());
        if self.preview.chars().count() >= MAX_TEXT_PREVIEW_CHARS {
            return;
        }
        let remaining = MAX_TEXT_PREVIEW_CHARS.saturating_sub(self.preview.chars().count());
        self.preview.push_str(&take_first_n_chars(text, remaining));
    }

    fn append_thinking(&mut self, text: &str) {
        self.thinking_block_seen = true;
        self.thinking_chars = self.thinking_chars.saturating_add(text.chars().count());
        if self.capturing_thinking_block && !text.is_empty() {
            if self.current_thinking_full.chars().count() < MAX_ROUNDTRIP_THINKING_CHARS {
                let remaining = MAX_ROUNDTRIP_THINKING_CHARS
                    .saturating_sub(self.current_thinking_full.chars().count());
                self.current_thinking_full
                    .push_str(&take_first_n_chars(text, remaining));
            }
        }
        if self.thinking_preview.chars().count() >= MAX_TEXT_PREVIEW_CHARS {
            return;
        }
        let remaining =
            MAX_TEXT_PREVIEW_CHARS.saturating_sub(self.thinking_preview.chars().count());
        self.thinking_preview
            .push_str(&take_first_n_chars(text, remaining));
    }

    fn ingest_signature(&mut self, signature: &str) {
        let trimmed = signature.trim();
        if trimmed.is_empty() {
            return;
        }
        self.thinking_block_seen = true;
        let chars = trimmed.chars().count();
        if chars > self.signature_chars {
            self.signature_chars = chars;
        }
        if self.capturing_thinking_block && !self.current_signature_from_delta {
            if trimmed.chars().count() > self.current_signature_full.chars().count() {
                self.current_signature_full =
                    take_first_n_chars(trimmed, MAX_ROUNDTRIP_SIGNATURE_CHARS);
            }
        }
    }

    fn begin_thinking_capture(&mut self, index: Option<i64>) {
        if self.capturing_thinking_block {
            if self.current_thinking_block_index == index {
                return;
            }
        }
        self.capturing_thinking_block = true;
        self.current_thinking_block_index = index;
        self.current_thinking_full.clear();
        self.current_signature_full.clear();
        self.current_signature_from_delta = false;
    }

    fn append_signature_delta(&mut self, signature_part: &str) {
        let part = signature_part.trim();
        if part.is_empty() {
            return;
        }
        self.thinking_block_seen = true;
        self.current_signature_from_delta = true;

        if self.current_signature_full.chars().count() < MAX_ROUNDTRIP_SIGNATURE_CHARS {
            let remaining = MAX_ROUNDTRIP_SIGNATURE_CHARS
                .saturating_sub(self.current_signature_full.chars().count());
            self.current_signature_full
                .push_str(&take_first_n_chars(part, remaining));
        }

        let chars = self.current_signature_full.trim().chars().count();
        if chars > self.signature_chars {
            self.signature_chars = chars;
        }
    }

    fn finalize_thinking_capture_best_effort(&mut self) {
        if !self.capturing_thinking_block {
            return;
        }
        if !self.current_signature_full.trim().is_empty() {
            self.thinking_full = self.current_thinking_full.clone();
            self.signature_full = self.current_signature_full.clone();
            self.signature_from_delta = self.current_signature_from_delta;
        }
        self.capturing_thinking_block = false;
        self.current_thinking_block_index = None;
    }

    fn ingest_response_meta(&mut self, value: &serde_json::Value) {
        let (id, service_tier) = extract_response_meta_from_message_json(value);
        if self.response_id.is_empty() {
            if let Some(v) = id {
                self.response_id = v;
            }
        }
        if self.service_tier.is_empty() {
            if let Some(v) = service_tier {
                self.service_tier = v;
            }
        }
    }

    fn ingest_event(&mut self, event: &[u8], data: &serde_json::Value) {
        let event_name = std::str::from_utf8(event).unwrap_or("").trim();
        let data_type = data
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let block_index = data.get("index").and_then(|v| v.as_i64());

        if data_type == "content_block_start" {
            if let Some(block) = data.get("content_block").and_then(|v| v.as_object()) {
                let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if block_type == "thinking" || block_type == "redacted_thinking" {
                    self.begin_thinking_capture(block_index);
                    if let Some(thinking) = block
                        .get("thinking")
                        .and_then(|v| v.as_str())
                        .or_else(|| block.get("text").and_then(|v| v.as_str()))
                    {
                        if !thinking.is_empty() {
                            self.append_thinking(thinking);
                        }
                    }
                    if let Some(signature) = block.get("signature").and_then(|v| v.as_str()) {
                        self.ingest_signature(signature);
                    }
                }
            }
        }
        if data_type == "content_block_stop" {
            if self.capturing_thinking_block {
                if self.current_thinking_block_index.is_none()
                    || self.current_thinking_block_index == block_index
                {
                    self.finalize_thinking_capture_best_effort();
                }
            }
        }

        if event_name == "message_delta" || data_type == "message_delta" {
            self.message_delta_seen = true;
            if let Some(stop_reason) = data
                .get("delta")
                .and_then(|v| v.get("stop_reason"))
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                let is_max_tokens = stop_reason == "max_tokens";
                self.message_delta_stop_reason_is_max_tokens =
                    self.message_delta_stop_reason_is_max_tokens || is_max_tokens;
                if is_max_tokens || self.message_delta_stop_reason.is_none() {
                    self.message_delta_stop_reason = Some(stop_reason);
                }
            }
        }

        // 先尽可能从 message 或根对象里提取结构字段（不会影响 text/thinking 的计数口径）。
        if let Some(message) = data.get("message") {
            self.ingest_response_meta(message);
        }
        self.ingest_response_meta(data);

        // Prefer deltas: { delta: { type: "text_delta", text: "..." } }
        if let Some(delta) = data.get("delta").and_then(|v| v.as_object()) {
            let delta_type = delta
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if delta_type == "text_delta" || delta_type == "text" {
                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                    self.append_text(text);
                    return;
                }
            }

            // thinking_delta: { delta: { type:"thinking_delta", thinking:"..." } }
            if delta_type == "thinking_delta" || delta_type == "thinking" {
                if let Some(text) = delta.get("thinking").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        self.begin_thinking_capture(block_index);
                        self.append_thinking(text);
                        return;
                    }
                }
                // Best-effort fallback: some variants might use `text` for thinking.
                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        self.begin_thinking_capture(block_index);
                        self.append_thinking(text);
                        return;
                    }
                }
            }

            // signature_delta: { delta: { type:"signature_delta", signature:"..." } }
            if delta_type == "signature_delta" {
                if let Some(sig) = delta.get("signature").and_then(|v| v.as_str()) {
                    self.begin_thinking_capture(block_index);
                    self.append_signature_delta(sig);
                    return;
                }
            }

            // Best-effort: signature might appear on delta/message shapes.
            if let Some(signature) = delta.get("signature").and_then(|v| v.as_str()) {
                self.begin_thinking_capture(block_index);
                self.ingest_signature(signature);
            }
            if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                if !thinking.is_empty() {
                    self.append_thinking(thinking);
                    return;
                }
            }

            // Best-effort fallback: some variants might include "text" without explicit type.
            if delta_type.is_empty() {
                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                    self.append_text(text);
                    return;
                }
            }

            // Best-effort fallback: some proxies might embed message content in delta.content.
            if let Some(content) = delta.get("content") {
                if let Some(text) = content.as_str() {
                    self.append_text(text);
                    return;
                }
                if let Some(arr) = content.as_array() {
                    for block in arr {
                        let Some(obj) = block.as_object() else {
                            continue;
                        };
                        let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if block_type == "text" {
                            let Some(text) = obj.get("text").and_then(|v| v.as_str()) else {
                                continue;
                            };
                            if !text.is_empty() {
                                self.append_text(text);
                            }
                            continue;
                        }
                        if block_type == "thinking" || block_type == "redacted_thinking" {
                            self.thinking_block_seen = true;
                            if let Some(thinking) = obj
                                .get("thinking")
                                .and_then(|v| v.as_str())
                                .or_else(|| obj.get("text").and_then(|v| v.as_str()))
                            {
                                if !thinking.is_empty() {
                                    self.append_thinking(thinking);
                                }
                            }
                            if let Some(signature) = obj.get("signature").and_then(|v| v.as_str()) {
                                self.ingest_signature(signature);
                            }
                        }
                    }
                    if self.total_chars > 0 {
                        return;
                    }
                }
            }
        }

        // content_block_start fallback: { content_block: { type:"text"/"thinking", ... } }
        // If we already handled a canonical content_block_start above, avoid double counting.
        if data_type != "content_block_start" {
            if let Some(block) = data.get("content_block").and_then(|v| v.as_object()) {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            self.append_text(text);
                            return;
                        }
                    }
                }
                if let Some(block_type) = block.get("type").and_then(|v| v.as_str()) {
                    if block_type == "thinking" || block_type == "redacted_thinking" {
                        self.thinking_block_seen = true;
                        if let Some(thinking) = block
                            .get("thinking")
                            .and_then(|v| v.as_str())
                            .or_else(|| block.get("text").and_then(|v| v.as_str()))
                        {
                            if !thinking.is_empty() {
                                self.append_thinking(thinking);
                            }
                        }
                        if let Some(signature) = block.get("signature").and_then(|v| v.as_str()) {
                            self.ingest_signature(signature);
                        }
                    }
                }
            }
        }

        // Last resort: if no delta was captured, try to extract from a full message/content shape
        // (some proxies may not preserve Anthropic SSE event structure).
        if self.total_chars == 0 {
            if let Some(message) = data.get("message") {
                let (chars, preview) = extract_text_from_message_json(message);
                if chars > 0 {
                    self.total_chars = chars;
                    self.preview = preview;
                    return;
                }
            }

            let (chars, preview) = extract_text_from_message_json(data);
            if chars > 0 {
                self.total_chars = chars;
                self.preview = preview;
            }
        }

        // Thinking/signature：同样做 best-effort 兜底提取（只在尚未拿到时尝试，避免重复计数）。
        if !self.thinking_block_seen && self.thinking_chars == 0 && self.signature_chars == 0 {
            if let Some(message) = data.get("message") {
                let (has_block, chars, preview, signature_chars) =
                    extract_thinking_from_message_json(message);
                if has_block {
                    self.thinking_block_seen = true;
                }
                if chars > 0 {
                    self.thinking_chars = chars;
                    self.thinking_preview = preview;
                }
                if signature_chars > self.signature_chars {
                    self.signature_chars = signature_chars;
                }
                if self.thinking_block_seen || self.thinking_chars > 0 || self.signature_chars > 0 {
                    return;
                }
            }

            let (has_block, chars, preview, signature_chars) =
                extract_thinking_from_message_json(data);
            if has_block {
                self.thinking_block_seen = true;
            }
            if chars > 0 {
                self.thinking_chars = chars;
                self.thinking_preview = preview;
            }
            if signature_chars > self.signature_chars {
                self.signature_chars = signature_chars;
            }
        }
    }
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

pub(super) fn extract_text_from_message_json(value: &serde_json::Value) -> (usize, String) {
    let mut total = 0usize;
    let mut preview = String::new();

    let Some(content) = value.get("content") else {
        return (0, String::new());
    };

    if let Some(s) = content.as_str() {
        total = s.chars().count();
        preview = take_first_n_chars(s, MAX_TEXT_PREVIEW_CHARS);
        return (total, preview);
    }

    let Some(arr) = content.as_array() else {
        return (0, String::new());
    };

    for block in arr {
        let Some(obj) = block.as_object() else {
            continue;
        };
        if obj.get("type").and_then(|v| v.as_str()) != Some("text") {
            continue;
        }
        let Some(text) = obj.get("text").and_then(|v| v.as_str()) else {
            continue;
        };
        total = total.saturating_add(text.chars().count());
        if preview.chars().count() < MAX_TEXT_PREVIEW_CHARS {
            let remaining = MAX_TEXT_PREVIEW_CHARS.saturating_sub(preview.chars().count());
            preview.push_str(&take_first_n_chars(text, remaining));
        }
    }

    (total, preview)
}

pub(super) fn extract_response_meta_from_message_json(
    value: &serde_json::Value,
) -> (Option<String>, Option<String>) {
    // Support both:
    // - message JSON: { id, service_tier, usage, content, ... }
    // - SSE wrapper: { message: { ... } }
    if let Some(message) = value.get("message") {
        return extract_response_meta_from_message_json(message);
    }

    let Some(obj) = value.as_object() else {
        return (None, None);
    };

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Prefer top-level service_tier; best-effort fallback to usage.service_tier.
    let service_tier = obj
        .get("service_tier")
        .and_then(|v| v.as_str())
        .or_else(|| {
            obj.get("usage")
                .and_then(|u| u.get("service_tier"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    (id, service_tier)
}

pub(super) fn extract_thinking_from_message_json(
    value: &serde_json::Value,
) -> (bool, usize, String, usize) {
    // Returns:
    // - has_thinking_block: whether "thinking"/"redacted_thinking" blocks were observed
    // - thinking_chars: accumulated char count of thinking text (best-effort)
    // - thinking_preview: first N chars of thinking (best-effort)
    // - signature_chars: max signature length found (best-effort)

    // Support SSE wrapper: { message: { ... } }
    if let Some(message) = value.get("message") {
        return extract_thinking_from_message_json(message);
    }

    let mut has_block = false;
    let mut total = 0usize;
    let mut preview = String::new();
    let mut signature_chars = 0usize;

    // Some variants may flatten thinking to a top-level field.
    if let Some(t) = value.get("thinking").and_then(|v| v.as_str()) {
        let text = t.trim();
        if !text.is_empty() {
            has_block = true;
            total = total.saturating_add(text.chars().count());
            if preview.chars().count() < MAX_TEXT_PREVIEW_CHARS {
                let remaining = MAX_TEXT_PREVIEW_CHARS.saturating_sub(preview.chars().count());
                preview.push_str(&take_first_n_chars(text, remaining));
            }
        }
    }

    let Some(content) = value.get("content") else {
        return (has_block, total, preview, signature_chars);
    };

    let Some(arr) = content.as_array() else {
        return (has_block, total, preview, signature_chars);
    };

    for block in arr {
        let Some(obj) = block.as_object() else {
            continue;
        };
        let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if block_type == "thinking" || block_type == "redacted_thinking" {
            has_block = true;

            if let Some(sig) = obj.get("signature").and_then(|v| v.as_str()) {
                let trimmed = sig.trim();
                if !trimmed.is_empty() {
                    signature_chars = signature_chars.max(trimmed.chars().count());
                }
            }

            // `thinking` blocks usually provide the text under `thinking`.
            // Some proxies might store it under `text`; treat that as best-effort.
            if let Some(t) = obj
                .get("thinking")
                .and_then(|v| v.as_str())
                .or_else(|| obj.get("text").and_then(|v| v.as_str()))
            {
                let text = t.trim();
                if !text.is_empty() {
                    total = total.saturating_add(text.chars().count());
                    if preview.chars().count() < MAX_TEXT_PREVIEW_CHARS {
                        let remaining =
                            MAX_TEXT_PREVIEW_CHARS.saturating_sub(preview.chars().count());
                        preview.push_str(&take_first_n_chars(text, remaining));
                    }
                }
            }
        }
    }

    (has_block, total, preview, signature_chars)
}

pub(super) fn extract_thinking_full_and_signature_from_message_json(
    value: &serde_json::Value,
) -> (bool, String, String) {
    // Returns:
    // - has_thinking_block: whether "thinking"/"redacted_thinking" blocks were observed
    // - thinking_full: best-effort concatenated thinking text (truncated)
    // - signature_full: best-effort signature text (truncated; prefers the longest found)

    // Support SSE wrapper: { message: { ... } }
    if let Some(message) = value.get("message") {
        return extract_thinking_full_and_signature_from_message_json(message);
    }

    let mut has_block = false;
    let mut thinking_full = String::new();
    let mut signature_full = String::new();

    // Some variants may flatten thinking to a top-level field.
    if let Some(t) = value.get("thinking").and_then(|v| v.as_str()) {
        let text = t.trim();
        if !text.is_empty() {
            has_block = true;
            thinking_full = take_first_n_chars(text, MAX_ROUNDTRIP_THINKING_CHARS);
        }
    }

    let Some(content) = value.get("content") else {
        return (has_block, thinking_full, signature_full);
    };
    let Some(arr) = content.as_array() else {
        return (has_block, thinking_full, signature_full);
    };

    for block in arr {
        let Some(obj) = block.as_object() else {
            continue;
        };
        let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if block_type == "thinking" || block_type == "redacted_thinking" {
            has_block = true;

            if let Some(sig) = obj.get("signature").and_then(|v| v.as_str()) {
                let trimmed = sig.trim();
                if !trimmed.is_empty() && trimmed.chars().count() > signature_full.chars().count()
                {
                    signature_full = take_first_n_chars(trimmed, MAX_ROUNDTRIP_SIGNATURE_CHARS);
                }
            }

            if let Some(t) = obj
                .get("thinking")
                .and_then(|v| v.as_str())
                .or_else(|| obj.get("text").and_then(|v| v.as_str()))
            {
                let text = t.trim();
                if !text.is_empty()
                    && thinking_full.chars().count() < MAX_ROUNDTRIP_THINKING_CHARS
                {
                    if !thinking_full.is_empty() {
                        thinking_full.push('\n');
                    }
                    let remaining = MAX_ROUNDTRIP_THINKING_CHARS
                        .saturating_sub(thinking_full.chars().count());
                    thinking_full.push_str(&take_first_n_chars(text, remaining));
                }
            }
        }
    }

    (has_block, thinking_full, signature_full)
}

#[cfg(test)]
mod tests {
    use super::SseTextAccumulator;

    #[test]
    fn sse_signature_delta_is_accumulated() {
        let mut acc = SseTextAccumulator::default();
        let sse = concat!(
            "event: message\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"THINK1\",\"signature\":\"\"}}\n",
            "\n",
            "event: message\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"THINK2\"}}\n",
            "\n",
            "event: message\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"SIG_PART_1\"}}\n",
            "\n",
            "event: message\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"SIG_PART_2\"}}\n",
            "\n",
            "event: message\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n",
            "\n",
        );

        acc.ingest_chunk(sse.as_bytes());
        acc.finalize();

        assert!(acc.thinking_block_seen);
        assert_eq!(acc.thinking_full, "THINK1THINK2");
        assert_eq!(acc.signature_full, "SIG_PART_1SIG_PART_2");
        assert!(acc.signature_from_delta);
        assert_eq!(acc.signature_chars, "SIG_PART_1SIG_PART_2".chars().count());
    }
}
