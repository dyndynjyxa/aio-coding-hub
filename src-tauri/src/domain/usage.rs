//! Usage: Parse upstream usage/model information from JSON and SSE streams.

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct UsageMetrics {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_creation_5m_input_tokens: Option<i64>,
    pub cache_creation_1h_input_tokens: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UsageExtract {
    pub metrics: UsageMetrics,
    pub usage_json: String,
}

fn as_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|v| i64::try_from(v).ok())),
        _ => None,
    }
}

fn has_any_metric(metrics: &UsageMetrics) -> bool {
    metrics.input_tokens.is_some()
        || metrics.output_tokens.is_some()
        || metrics.total_tokens.is_some()
        || metrics.cache_read_input_tokens.is_some()
        || metrics.cache_creation_input_tokens.is_some()
        || metrics.cache_creation_5m_input_tokens.is_some()
        || metrics.cache_creation_1h_input_tokens.is_some()
}

fn normalize_usage_json(metrics: &UsageMetrics) -> String {
    let mut obj = serde_json::Map::new();

    if let Some(v) = metrics.input_tokens {
        obj.insert("input_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.output_tokens {
        obj.insert("output_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.total_tokens {
        obj.insert("total_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.cache_read_input_tokens {
        obj.insert("cache_read_input_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.cache_creation_input_tokens {
        obj.insert("cache_creation_input_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.cache_creation_5m_input_tokens {
        obj.insert("cache_creation_5m_input_tokens".to_string(), json!(v));
    }
    if let Some(v) = metrics.cache_creation_1h_input_tokens {
        obj.insert("cache_creation_1h_input_tokens".to_string(), json!(v));
    }

    Value::Object(obj).to_string()
}

fn sanitize_model(model: &str) -> Option<String> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    let model = if model.len() > 200 {
        model[..200].to_string()
    } else {
        model.to_string()
    };
    Some(model)
}

fn extract_model_from_json_value(value: &Value) -> Option<String> {
    if let Some(model) = value.get("model").and_then(|v| v.as_str()) {
        return sanitize_model(model);
    }

    if let Some(model) = value
        .get("message")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("model"))
        .and_then(|v| v.as_str())
    {
        return sanitize_model(model);
    }

    if let Some(model) = value
        .get("response")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("model"))
        .and_then(|v| v.as_str())
    {
        return sanitize_model(model);
    }

    None
}

pub fn parse_model_from_json_bytes(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;

    // The input `value` might be a full response, a partial wrapper, or an SSE data payload.
    if let Some(model) = extract_model_from_json_value(&value) {
        return Some(model);
    }

    // Object root: try common containers.
    if let Some(obj) = value.as_object() {
        if let Some(model) = obj.get("message").and_then(extract_model_from_json_value) {
            return Some(model);
        }
        if let Some(model) = obj.get("response").and_then(extract_model_from_json_value) {
            return Some(model);
        }
    }

    None
}

fn extract_usage_metrics(value: &Value) -> Option<UsageMetrics> {
    let obj = value.as_object()?;

    let mut metrics = UsageMetrics::default();

    // OpenAI ChatCompletions: {prompt_tokens, completion_tokens, total_tokens}
    metrics.input_tokens = metrics
        .input_tokens
        .or_else(|| as_i64(obj.get("prompt_tokens")));
    metrics.output_tokens = metrics
        .output_tokens
        .or_else(|| as_i64(obj.get("completion_tokens")));
    metrics.total_tokens = metrics
        .total_tokens
        .or_else(|| as_i64(obj.get("total_tokens")));

    // OpenAI Responses API: {input_tokens, output_tokens, total_tokens}
    metrics.input_tokens = metrics
        .input_tokens
        .or_else(|| as_i64(obj.get("input_tokens")));
    metrics.output_tokens = metrics
        .output_tokens
        .or_else(|| as_i64(obj.get("output_tokens")));
    metrics.total_tokens = metrics
        .total_tokens
        .or_else(|| as_i64(obj.get("total_tokens")));

    // OpenAI detail: input_tokens_details.cached_tokens OR prompt_tokens_details.cached_tokens
    metrics.cache_read_input_tokens = metrics.cache_read_input_tokens.or_else(|| {
        obj.get("input_tokens_details")
            .and_then(|v| v.as_object())
            .and_then(|m| as_i64(m.get("cached_tokens")))
    });
    metrics.cache_read_input_tokens = metrics.cache_read_input_tokens.or_else(|| {
        obj.get("prompt_tokens_details")
            .and_then(|v| v.as_object())
            .and_then(|m| as_i64(m.get("cached_tokens")))
    });

    // Claude: cache_creation fields may be top-level or nested under cache_creation
    metrics.cache_read_input_tokens = metrics
        .cache_read_input_tokens
        .or_else(|| as_i64(obj.get("cache_read_input_tokens")));

    metrics.cache_creation_input_tokens = metrics
        .cache_creation_input_tokens
        .or_else(|| as_i64(obj.get("cache_creation_input_tokens")));

    metrics.cache_creation_5m_input_tokens = metrics.cache_creation_5m_input_tokens.or_else(|| {
        as_i64(obj.get("cache_creation_5m_input_tokens"))
            .or_else(|| as_i64(obj.get("claude_cache_creation_5_m_tokens")))
    });
    metrics.cache_creation_1h_input_tokens = metrics.cache_creation_1h_input_tokens.or_else(|| {
        as_i64(obj.get("cache_creation_1h_input_tokens"))
            .or_else(|| as_i64(obj.get("claude_cache_creation_1_h_tokens")))
    });

    if let Some(cache_creation) = obj.get("cache_creation").and_then(|v| v.as_object()) {
        metrics.cache_creation_5m_input_tokens = metrics
            .cache_creation_5m_input_tokens
            .or_else(|| as_i64(cache_creation.get("ephemeral_5m_input_tokens")));
        metrics.cache_creation_1h_input_tokens = metrics
            .cache_creation_1h_input_tokens
            .or_else(|| as_i64(cache_creation.get("ephemeral_1h_input_tokens")));
    }

    if metrics.cache_creation_input_tokens.is_none() {
        let summed = match (
            metrics.cache_creation_5m_input_tokens,
            metrics.cache_creation_1h_input_tokens,
        ) {
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        metrics.cache_creation_input_tokens = summed;
    }

    // Gemini usageMetadata
    metrics.input_tokens = metrics
        .input_tokens
        .or_else(|| as_i64(obj.get("promptTokenCount")));
    let candidates = as_i64(obj.get("candidatesTokenCount"));
    let thoughts = as_i64(obj.get("thoughtsTokenCount")).unwrap_or(0);
    metrics.output_tokens = metrics
        .output_tokens
        .or_else(|| candidates.map(|v| v.saturating_add(thoughts)));
    metrics.total_tokens = metrics
        .total_tokens
        .or_else(|| as_i64(obj.get("totalTokenCount")));
    metrics.cache_read_input_tokens = metrics
        .cache_read_input_tokens
        .or_else(|| as_i64(obj.get("cachedContentTokenCount")));

    if has_any_metric(&metrics) {
        Some(metrics)
    } else {
        None
    }
}

fn extract_from_json_value(value: &Value) -> Option<UsageMetrics> {
    // The input `value` might be a full response, a partial wrapper, or already a usage object.
    if let Some(metrics) = extract_usage_metrics(value) {
        return Some(metrics);
    }

    // Object root: prioritize well-known usage containers.
    if let Some(obj) = value.as_object() {
        if let Some(usage) = obj.get("usage").and_then(extract_usage_metrics) {
            return Some(usage);
        }
        if let Some(usage_meta) = obj.get("usageMetadata").and_then(extract_usage_metrics) {
            return Some(usage_meta);
        }

        if let Some(resp) = obj.get("response") {
            if let Some(usage) = resp.get("usage").and_then(extract_usage_metrics) {
                return Some(usage);
            }
            if let Some(usage_meta) = resp.get("usageMetadata").and_then(extract_usage_metrics) {
                return Some(usage_meta);
            }
        }

        if let Some(output) = obj.get("output").and_then(|v| v.as_array()) {
            for item in output {
                if let Some(usage) = item.get("usage").and_then(extract_usage_metrics) {
                    return Some(usage);
                }
            }
        }
    }

    // Array root: scan items (best-effort).
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(usage) = item.get("usage").and_then(extract_usage_metrics) {
                return Some(usage);
            }
            if let Some(data_usage) = item
                .get("data")
                .and_then(|v| v.get("usage"))
                .and_then(extract_usage_metrics)
            {
                return Some(data_usage);
            }
        }
    }

    None
}

pub fn parse_usage_from_json_bytes(body: &[u8]) -> Option<UsageExtract> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let metrics = extract_from_json_value(&value)?;
    Some(UsageExtract {
        usage_json: normalize_usage_json(&metrics),
        metrics,
    })
}

pub fn parse_usage_from_json_or_sse_bytes(cli_key: &str, body: &[u8]) -> Option<UsageExtract> {
    parse_usage_from_json_bytes(body).or_else(|| {
        let mut tracker = SseUsageTracker::new(cli_key);
        tracker.ingest_chunk(body);
        tracker.finalize()
    })
}

pub fn parse_model_from_json_or_sse_bytes(cli_key: &str, body: &[u8]) -> Option<String> {
    parse_model_from_json_bytes(body).or_else(|| {
        let mut tracker = SseUsageTracker::new(cli_key);
        tracker.ingest_chunk(body);
        let _ = tracker.finalize();
        tracker.best_effort_model()
    })
}

fn merge_metrics(base: &UsageMetrics, patch: &UsageMetrics) -> UsageMetrics {
    UsageMetrics {
        input_tokens: patch.input_tokens.or(base.input_tokens),
        output_tokens: patch.output_tokens.or(base.output_tokens),
        total_tokens: patch.total_tokens.or(base.total_tokens),
        cache_read_input_tokens: patch
            .cache_read_input_tokens
            .or(base.cache_read_input_tokens),
        cache_creation_input_tokens: patch
            .cache_creation_input_tokens
            .or(base.cache_creation_input_tokens),
        cache_creation_5m_input_tokens: patch
            .cache_creation_5m_input_tokens
            .or(base.cache_creation_5m_input_tokens),
        cache_creation_1h_input_tokens: patch
            .cache_creation_1h_input_tokens
            .or(base.cache_creation_1h_input_tokens),
    }
}

#[derive(Debug)]
pub struct SseUsageTracker {
    is_claude: bool,
    buffer: Vec<u8>,
    current_event: Vec<u8>,
    current_data: Vec<u8>,

    claude_message_start: Option<UsageMetrics>,
    claude_message_delta: Option<UsageMetrics>,
    last_generic: Option<UsageMetrics>,
    last_model: Option<String>,
    completion_seen: bool,
    terminal_error_seen: bool,
    fake_200_detected: bool,
}

const MAX_SSE_USAGE_TRACKER_PENDING_BYTES: usize = 1024 * 1024;

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

fn eq_ignore_ascii_case_bytes(left: &[u8], right: &[u8]) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn normalize_ascii_lower(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn is_completion_event_name(event: &[u8]) -> bool {
    let event = trim_ascii(event);
    [
        b"done".as_slice(),
        b"completed".as_slice(),
        b"message_stop".as_slice(),
        b"response.completed".as_slice(),
        b"response.done".as_slice(),
        b"message.completed".as_slice(),
    ]
    .iter()
    .any(|candidate| eq_ignore_ascii_case_bytes(event, candidate))
}

fn is_terminal_error_event_name(event: &[u8]) -> bool {
    let event = trim_ascii(event);
    eq_ignore_ascii_case_bytes(event, b"error")
        || eq_ignore_ascii_case_bytes(event, b"response.error")
}

fn is_completion_event_type(event_type: &str) -> bool {
    let normalized = normalize_ascii_lower(event_type);
    matches!(
        normalized.as_str(),
        "done"
            | "completed"
            | "response.done"
            | "response.completed"
            | "message.done"
            | "message.completed"
            | "message.stop"
    ) || normalized.ends_with(".completed")
}

fn is_terminal_error_event_type(event_type: &str) -> bool {
    let normalized = normalize_ascii_lower(event_type);
    matches!(normalized.as_str(), "error" | "response.error") || normalized.ends_with(".error")
}

fn is_completion_status(status: &str) -> bool {
    matches!(
        normalize_ascii_lower(status).as_str(),
        "done" | "completed" | "finished_successfully" | "succeeded" | "success"
    )
}

fn is_terminal_error_status(status: &str) -> bool {
    matches!(
        normalize_ascii_lower(status).as_str(),
        "error" | "failed" | "cancelled" | "canceled" | "aborted" | "timed_out" | "timeout"
    )
}

impl SseUsageTracker {
    pub fn new(cli_key: &str) -> Self {
        Self {
            is_claude: cli_key == "claude",
            buffer: Vec::new(),
            current_event: Vec::new(),
            current_data: Vec::new(),
            claude_message_start: None,
            claude_message_delta: None,
            last_generic: None,
            last_model: None,
            completion_seen: false,
            terminal_error_seen: false,
            fake_200_detected: false,
        }
    }

    pub fn completion_seen(&self) -> bool {
        self.completion_seen
    }

    pub fn terminal_error_seen(&self) -> bool {
        self.terminal_error_seen
    }

    pub fn fake_200_detected(&self) -> bool {
        self.fake_200_detected
    }

    pub fn ingest_chunk(&mut self, chunk: &[u8]) {
        let mut start = 0usize;

        for (idx, b) in chunk.iter().enumerate() {
            if *b != b'\n' {
                continue;
            }

            self.ingest_complete_line(&chunk[start..idx]);
            start = idx + 1;
        }

        if start < chunk.len() {
            self.append_pending_line_fragment(&chunk[start..]);
        }
    }

    fn clear_pending_event(&mut self) {
        self.buffer.clear();
        self.current_event.clear();
        self.current_data.clear();
    }

    fn append_pending_line_fragment(&mut self, fragment: &[u8]) -> bool {
        if fragment.is_empty() {
            return true;
        }

        if self.buffer.len().saturating_add(fragment.len()) > MAX_SSE_USAGE_TRACKER_PENDING_BYTES {
            self.clear_pending_event();
            return false;
        }

        self.buffer.extend_from_slice(fragment);
        true
    }

    fn ingest_complete_line(&mut self, fragment: &[u8]) {
        if self.buffer.is_empty() {
            if fragment.len() > MAX_SSE_USAGE_TRACKER_PENDING_BYTES {
                self.clear_pending_event();
                return;
            }

            let mut line = fragment;
            if line.last() == Some(&b'\r') {
                line = &line[..line.len().saturating_sub(1)];
            }
            self.ingest_line(line);
            return;
        }

        if !self.append_pending_line_fragment(fragment) {
            return;
        }

        let mut line = std::mem::take(&mut self.buffer);
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        self.ingest_line(&line);
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
            if rest.len() > MAX_SSE_USAGE_TRACKER_PENDING_BYTES {
                self.clear_pending_event();
                return;
            }
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
                self.completion_seen = true;
                return;
            }

            let separator_len = usize::from(!self.current_data.is_empty());
            if self
                .current_data
                .len()
                .saturating_add(separator_len)
                .saturating_add(rest.len())
                > MAX_SSE_USAGE_TRACKER_PENDING_BYTES
            {
                self.clear_pending_event();
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

        let data_json: Value = match serde_json::from_slice(&self.current_data) {
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

    fn ingest_event(&mut self, event: &[u8], data: &Value) {
        if is_completion_event_name(event) {
            self.completion_seen = true;
        }
        if is_terminal_error_event_name(event) {
            self.terminal_error_seen = true;
            // Fake 200: upstream returned HTTP 200 but body contains an error event.
            // Detect patterns: SSE `event: error` with a JSON body containing "error" object
            // or `"type":"error"` in the data payload.
            if data.get("error").is_some()
                || data.get("type").and_then(|v| v.as_str()) == Some("error")
            {
                self.fake_200_detected = true;
            }
        }

        if let Some(event_type) = data.get("type").and_then(|v| v.as_str()) {
            if is_completion_event_type(event_type) {
                self.completion_seen = true;
            }
            if is_terminal_error_event_type(event_type) {
                self.terminal_error_seen = true;
                // Also detect fake 200 from data.type == "error" with an error object
                if data.get("error").is_some() {
                    self.fake_200_detected = true;
                }
            }
        }

        let status_fields = [
            data.get("status").and_then(|v| v.as_str()),
            data.get("response")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str()),
            data.get("message")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str()),
        ];
        for status in status_fields.into_iter().flatten() {
            if is_completion_status(status) {
                self.completion_seen = true;
            }
            if is_terminal_error_status(status) {
                self.terminal_error_seen = true;
            }
        }

        let done_like = [
            data.get("done").and_then(|v| v.as_bool()),
            data.get("is_done").and_then(|v| v.as_bool()),
            data.get("is_final").and_then(|v| v.as_bool()),
            data.get("response")
                .and_then(|v| v.get("done"))
                .and_then(|v| v.as_bool()),
            data.get("message")
                .and_then(|v| v.get("done"))
                .and_then(|v| v.as_bool()),
        ];
        if done_like.into_iter().flatten().any(|v| v) {
            self.completion_seen = true;
        }

        if let Some(model) = extract_model_from_json_value(data) {
            self.last_model = Some(model);
        }

        // Claude SSE: merge message_start + message_delta usage
        if self.is_claude {
            if event == b"message_start" {
                let usage_value = data
                    .get("message")
                    .and_then(|m| m.get("usage"))
                    .or_else(|| data.get("usage"));
                if let Some(metrics) = usage_value.and_then(extract_usage_metrics) {
                    self.claude_message_start = Some(match &self.claude_message_start {
                        Some(prev) => merge_metrics(prev, &metrics),
                        None => metrics,
                    });
                }
                return;
            }

            if event == b"message_delta" {
                let usage_value = data
                    .get("usage")
                    .or_else(|| data.get("delta").and_then(|d| d.get("usage")));
                if let Some(metrics) = usage_value.and_then(extract_usage_metrics) {
                    self.claude_message_delta = Some(match &self.claude_message_delta {
                        Some(prev) => merge_metrics(prev, &metrics),
                        None => metrics,
                    });
                }
                return;
            }

            // Best-effort fallback: some proxies omit the `event:` field and only stream `data: ...`.
            // In that case we may still see a Claude-shaped payload with `message.usage` or `delta.usage`.
            let usage_value = data
                .get("message")
                .and_then(|m| m.get("usage"))
                .or_else(|| data.get("usage"))
                .or_else(|| data.get("delta").and_then(|d| d.get("usage")));
            if let Some(metrics) = usage_value.and_then(extract_usage_metrics) {
                self.last_generic = Some(match &self.last_generic {
                    Some(prev) => merge_metrics(prev, &metrics),
                    None => metrics,
                });
                return;
            }
        }

        // Generic SSE: attempt to extract usage/usageMetadata from the event payload.
        if let Some(metrics) = extract_from_json_value(data) {
            self.last_generic = Some(metrics);
        }
    }

    pub fn best_effort_model(&self) -> Option<String> {
        self.last_model.clone()
    }

    pub fn finalize(&mut self) -> Option<UsageExtract> {
        // Best-effort: handle a trailing line without '\n'.
        if !self.buffer.is_empty() {
            let mut tail = std::mem::take(&mut self.buffer);
            if tail.last() == Some(&b'\r') {
                tail.pop();
            }
            self.ingest_line(&tail);
        }

        // Flush any trailing buffered event if the stream ended without a blank line.
        self.flush_event();

        let merged = if self.is_claude {
            match (&self.claude_message_start, &self.claude_message_delta) {
                (Some(start), Some(delta)) => Some(merge_metrics(start, delta)),
                (Some(start), None) => Some(start.clone()),
                (None, Some(delta)) => Some(delta.clone()),
                (None, None) => self.last_generic.clone(),
            }
        } else {
            self.last_generic.clone()
        }?;

        Some(UsageExtract {
            usage_json: normalize_usage_json(&merged),
            metrics: merged,
        })
    }
}

#[cfg(test)]
mod tests;
