//! Usage: Gemini OAuth request/response translation for Code Assist upstreams.

use crate::gateway::oauth::adapters::gemini::{
    resolve_project_id_for_access_token, GEMINI_CODE_ASSIST_API_VERSION,
    GEMINI_CODE_ASSIST_BASE_URL,
};
use axum::body::Bytes;
use futures_core::Stream;
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

const MAX_GEMINI_OAUTH_SSE_EVENT_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GeminiOAuthResponseMode {
    GenerateContent,
    StreamGenerateContent,
    CountTokens,
}

#[derive(Debug, Clone)]
pub(super) struct GeminiOAuthPreparedRequest {
    pub(super) base_url: String,
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) body_bytes: Bytes,
    pub(super) strip_request_content_encoding: bool,
    pub(super) response_mode: GeminiOAuthResponseMode,
}

pub(super) async fn prepare_upstream_request(
    client: &reqwest::Client,
    access_token: &str,
    forwarded_path: &str,
    query: Option<&str>,
    body_json: Option<&Value>,
    body_bytes: &Bytes,
    requested_model: Option<&str>,
) -> Result<GeminiOAuthPreparedRequest, String> {
    let body_value = request_body_value(body_json, body_bytes)?;
    let parsed = parse_model_and_action(forwarded_path, requested_model, &body_value)?;
    let project_id = if matches!(parsed.response_mode, GeminiOAuthResponseMode::CountTokens) {
        None
    } else {
        Some(resolve_project_id_for_access_token(client, access_token).await?)
    };

    prepare_upstream_request_with_project(
        forwarded_path,
        query,
        body_value,
        requested_model,
        project_id.as_deref(),
    )
}

fn prepare_upstream_request_with_project(
    forwarded_path: &str,
    query: Option<&str>,
    body_value: Value,
    requested_model: Option<&str>,
    project_id: Option<&str>,
) -> Result<GeminiOAuthPreparedRequest, String> {
    let ParsedGeminiMethod {
        model,
        response_mode,
    } = parse_model_and_action(forwarded_path, requested_model, &body_value)?;

    let payload = match response_mode {
        GeminiOAuthResponseMode::CountTokens => {
            let request = build_count_tokens_request(body_value, &model)?;
            Value::Object({
                let mut map = Map::new();
                map.insert("request".to_string(), Value::Object(request));
                map
            })
        }
        GeminiOAuthResponseMode::GenerateContent
        | GeminiOAuthResponseMode::StreamGenerateContent => {
            let Some(project_id) = project_id.map(str::trim).filter(|value| !value.is_empty())
            else {
                return Err("gemini oauth could not resolve a Code Assist project id".to_string());
            };
            let request = build_generate_content_request(body_value)?;
            Value::Object({
                let mut map = Map::new();
                map.insert("model".to_string(), Value::String(model));
                map.insert("project".to_string(), Value::String(project_id.to_string()));
                map.insert("request".to_string(), Value::Object(request));
                map
            })
        }
    };

    let payload_bytes = serde_json::to_vec(&payload)
        .map(Bytes::from)
        .map_err(|e| format!("gemini oauth payload encode failed: {e}"))?;

    Ok(GeminiOAuthPreparedRequest {
        base_url: GEMINI_CODE_ASSIST_BASE_URL.to_string(),
        forwarded_path: format!(
            "/{GEMINI_CODE_ASSIST_API_VERSION}:{}",
            action_name(response_mode)
        ),
        query: translate_query_string(query, response_mode),
        body_bytes: payload_bytes,
        strip_request_content_encoding: true,
        response_mode,
    })
}

pub(super) fn translate_response_body(
    body_bytes: Bytes,
    response_mode: Option<GeminiOAuthResponseMode>,
) -> Bytes {
    let Some(response_mode) = response_mode else {
        return body_bytes;
    };

    let Ok(value) = serde_json::from_slice::<Value>(body_bytes.as_ref()) else {
        return body_bytes;
    };

    let translated = match response_mode {
        GeminiOAuthResponseMode::GenerateContent => extract_wrapped_response(value),
        GeminiOAuthResponseMode::StreamGenerateContent => extract_wrapped_stream_response(value),
        GeminiOAuthResponseMode::CountTokens => Some(augment_count_tokens_response(value)),
    };

    translated
        .and_then(|value| serde_json::to_vec(&value).ok())
        .map(Bytes::from)
        .unwrap_or(body_bytes)
}

pub(super) struct GeminiOAuthSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    response_mode: Option<GeminiOAuthResponseMode>,
    buffer: Vec<u8>,
    queued: VecDeque<Bytes>,
    pending_error: Option<reqwest::Error>,
    upstream_done: bool,
    passthrough: bool,
}

impl<S> GeminiOAuthSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(super) fn new(upstream: S, response_mode: Option<GeminiOAuthResponseMode>) -> Self {
        Self {
            upstream,
            response_mode,
            buffer: Vec::new(),
            queued: VecDeque::new(),
            pending_error: None,
            upstream_done: false,
            passthrough: false,
        }
    }

    fn should_translate(&self) -> bool {
        matches!(
            self.response_mode,
            Some(GeminiOAuthResponseMode::StreamGenerateContent)
        )
    }

    fn degrade_to_passthrough(&mut self, chunk: Bytes) {
        tracing::warn!(
            max_bytes = MAX_GEMINI_OAUTH_SSE_EVENT_BUFFER_BYTES,
            "Gemini OAuth SSE event buffer exceeded maximum size; switching to passthrough"
        );
        self.passthrough = true;
        let buffered = std::mem::take(&mut self.buffer);
        if !buffered.is_empty() {
            self.queued.push_back(Bytes::from(buffered));
        }
        if !chunk.is_empty() {
            self.queued.push_back(chunk);
        }
    }

    fn queue_buffered_events(&mut self) {
        while let Some(event_end) = find_sse_event_end(&self.buffer) {
            let event = self.buffer.drain(..event_end).collect::<Vec<u8>>();
            self.queued
                .push_back(transform_sse_event(&event, self.response_mode));
        }
    }
}

impl<S> Stream for GeminiOAuthSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        if !this.should_translate() {
            return Pin::new(&mut this.upstream).poll_next(cx);
        }

        loop {
            if let Some(chunk) = this.queued.pop_front() {
                return Poll::Ready(Some(Ok(chunk)));
            }

            if let Some(err) = this.pending_error.take() {
                return Poll::Ready(Some(Err(err)));
            }

            if this.upstream_done {
                if !this.buffer.is_empty() {
                    if let Some(final_event) =
                        finalize_buffered_sse_tail(&mut this.buffer, this.response_mode)
                    {
                        return Poll::Ready(Some(Ok(final_event)));
                    }
                }
                return Poll::Ready(None);
            }

            if this.passthrough {
                return Pin::new(&mut this.upstream).poll_next(cx);
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
                    if this.buffer.len().saturating_add(chunk.len())
                        > MAX_GEMINI_OAUTH_SSE_EVENT_BUFFER_BYTES
                    {
                        this.degrade_to_passthrough(chunk);
                        continue;
                    }
                    this.buffer.extend_from_slice(chunk.as_ref());
                    this.queue_buffered_events();
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedGeminiMethod {
    model: String,
    response_mode: GeminiOAuthResponseMode,
}

fn parse_model_and_action(
    forwarded_path: &str,
    requested_model: Option<&str>,
    body_value: &Value,
) -> Result<ParsedGeminiMethod, String> {
    let path = forwarded_path.trim();
    let trimmed = path
        .strip_prefix("/v1beta/")
        .or_else(|| path.strip_prefix("/v1/"))
        .or_else(|| path.strip_prefix('/'))
        .unwrap_or(path);
    let Some(rest) = trimmed.strip_prefix("models/") else {
        return Err(format!(
            "gemini oauth only supports /models/{{model}}:* requests, got path={forwarded_path}"
        ));
    };
    let Some((path_model, action)) = rest.split_once(':') else {
        return Err(format!(
            "gemini oauth request path is missing an action suffix, got path={forwarded_path}"
        ));
    };

    let response_mode = match action.trim() {
        "generateContent" => GeminiOAuthResponseMode::GenerateContent,
        "streamGenerateContent" => GeminiOAuthResponseMode::StreamGenerateContent,
        "countTokens" => GeminiOAuthResponseMode::CountTokens,
        other => {
            return Err(format!(
                "gemini oauth does not support action={other} for path={forwarded_path}"
            ));
        }
    };

    let model = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let trimmed = path_model.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .or_else(|| {
            body_value
                .get("model")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .ok_or_else(|| {
            format!("gemini oauth could not resolve a model for path={forwarded_path}")
        })?;

    Ok(ParsedGeminiMethod {
        model,
        response_mode,
    })
}

fn request_body_value(body_json: Option<&Value>, body_bytes: &Bytes) -> Result<Value, String> {
    if let Some(value) = body_json {
        return Ok(value.clone());
    }
    if body_bytes.is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_slice(body_bytes.as_ref())
        .map_err(|e| format!("gemini oauth request JSON parse failed: {e}"))
}

fn build_generate_content_request(body_value: Value) -> Result<Map<String, Value>, String> {
    let mut request = ensure_object(body_value)?;
    request.remove("model");

    if let Some(system_instruction) = request.remove("system_instruction") {
        request
            .entry("systemInstruction".to_string())
            .or_insert(system_instruction);
    }

    normalize_tools(&mut request);
    Ok(request)
}

fn build_count_tokens_request(
    body_value: Value,
    model: &str,
) -> Result<Map<String, Value>, String> {
    let mut request = ensure_object(body_value)?;
    let contents = request
        .remove("contents")
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let mut count_request = Map::new();
    count_request.insert(
        "model".to_string(),
        Value::String(format!("models/{model}")),
    );
    count_request.insert("contents".to_string(), contents);
    Ok(count_request)
}

fn ensure_object(value: Value) -> Result<Map<String, Value>, String> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "gemini oauth expects a JSON object request body".to_string())
}

fn normalize_tools(request: &mut Map<String, Value>) {
    let Some(Value::Array(tools)) = request.get_mut("tools") else {
        return;
    };

    for tool in tools {
        let Some(tool_object) = tool.as_object_mut() else {
            continue;
        };

        if let Some(function_declarations) = tool_object.remove("function_declarations") {
            tool_object
                .entry("functionDeclarations".to_string())
                .or_insert(function_declarations);
        }

        let Some(Value::Array(function_declarations)) = tool_object.get_mut("functionDeclarations")
        else {
            continue;
        };

        for declaration in function_declarations {
            let Some(declaration_object) = declaration.as_object_mut() else {
                continue;
            };

            if let Some(parameters) = declaration_object.remove("parameters") {
                declaration_object
                    .entry("parametersJsonSchema".to_string())
                    .or_insert(parameters);
            }
        }
    }
}

fn action_name(response_mode: GeminiOAuthResponseMode) -> &'static str {
    match response_mode {
        GeminiOAuthResponseMode::GenerateContent => "generateContent",
        GeminiOAuthResponseMode::StreamGenerateContent => "streamGenerateContent",
        GeminiOAuthResponseMode::CountTokens => "countTokens",
    }
}

fn translate_query_string(
    query: Option<&str>,
    response_mode: GeminiOAuthResponseMode,
) -> Option<String> {
    let ensure_stream_alt = matches!(
        response_mode,
        GeminiOAuthResponseMode::StreamGenerateContent
    );
    let mut pairs = Vec::new();
    let mut has_alt = false;

    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        for pair in query.split('&').filter(|part| !part.is_empty()) {
            let key = pair.split_once('=').map(|(key, _)| key).unwrap_or(pair);
            if matches!(key, "key" | "api_key" | "x-goog-api-key") {
                continue;
            }
            if matches!(key, "alt" | "$alt") {
                has_alt = true;
            }
            pairs.push(pair.to_string());
        }
    }

    if ensure_stream_alt && !has_alt {
        pairs.push("alt=sse".to_string());
    }

    (!pairs.is_empty()).then(|| pairs.join("&"))
}

fn extract_wrapped_response(value: Value) -> Option<Value> {
    match value {
        Value::Object(mut object) => object.remove("response"),
        _ => None,
    }
}

fn extract_wrapped_stream_response(value: Value) -> Option<Value> {
    match value {
        Value::Array(items) => Some(Value::Array(
            items
                .into_iter()
                .map(|item| extract_wrapped_response(item).unwrap_or(Value::Null))
                .collect(),
        )),
        other => extract_wrapped_response(other),
    }
}

fn augment_count_tokens_response(value: Value) -> Value {
    let Some(object) = value.as_object() else {
        return value;
    };
    if object.contains_key("promptTokensDetails") {
        return value;
    }
    let Some(total_tokens) = object.get("totalTokens").cloned() else {
        return value;
    };

    let mut output = object.clone();
    output.insert(
        "promptTokensDetails".to_string(),
        Value::Array(vec![Value::Object({
            let mut details = Map::new();
            details.insert("modality".to_string(), Value::String("TEXT".to_string()));
            details.insert("tokenCount".to_string(), total_tokens);
            details
        })]),
    );
    Value::Object(output)
}

fn transform_sse_event(event: &[u8], response_mode: Option<GeminiOAuthResponseMode>) -> Bytes {
    transform_sse_event_inner(event, response_mode, false)
        .unwrap_or_else(|| Bytes::copy_from_slice(event))
}

fn finalize_buffered_sse_tail(
    buffer: &mut Vec<u8>,
    response_mode: Option<GeminiOAuthResponseMode>,
) -> Option<Bytes> {
    if buffer.is_empty() {
        return None;
    }

    let tail = std::mem::take(buffer);
    match transform_sse_event_inner(&tail, response_mode, true) {
        Some(bytes) => Some(bytes),
        None => {
            tracing::warn!(
                buffered_bytes = tail.len(),
                "dropping incomplete Gemini OAuth SSE tail at EOF"
            );
            None
        }
    }
}

fn transform_sse_event_inner(
    event: &[u8],
    response_mode: Option<GeminiOAuthResponseMode>,
    allow_eof_without_blank_line: bool,
) -> Option<Bytes> {
    let should_translate = matches!(
        response_mode,
        Some(GeminiOAuthResponseMode::StreamGenerateContent)
    );
    if !should_translate {
        let parsed = allow_eof_without_blank_line
            .then(|| parse_sse_event(event, allow_eof_without_blank_line))
            .flatten();
        return Some(if allow_eof_without_blank_line {
            let parsed = parsed?;
            rebuild_sse_event(&parsed.passthrough_lines, &parsed.data_payload_lines)
        } else {
            Bytes::copy_from_slice(event)
        });
    }

    let parsed = parse_sse_event(event, allow_eof_without_blank_line)?;
    let joined_payload = join_sse_data_payload_lines(&parsed.data_payload_lines);
    if parsed.data_payload_lines.is_empty() || joined_payload == b"[DONE]" {
        return Some(if allow_eof_without_blank_line {
            rebuild_sse_event(&parsed.passthrough_lines, &parsed.data_payload_lines)
        } else {
            Bytes::copy_from_slice(event)
        });
    }

    let value: Value = serde_json::from_slice(joined_payload.as_slice()).ok()?;
    let Some(response) = extract_wrapped_response(value) else {
        return Some(if allow_eof_without_blank_line {
            rebuild_sse_event(&parsed.passthrough_lines, &parsed.data_payload_lines)
        } else {
            Bytes::copy_from_slice(event)
        });
    };
    let response_bytes = serde_json::to_vec(&response).ok()?;
    Some(rebuild_sse_event(
        &parsed.passthrough_lines,
        &[response_bytes],
    ))
}

#[derive(Debug, Default)]
struct ParsedSseEvent {
    passthrough_lines: Vec<Vec<u8>>,
    data_payload_lines: Vec<Vec<u8>>,
}

fn parse_sse_event(event: &[u8], allow_eof_without_blank_line: bool) -> Option<ParsedSseEvent> {
    let mut parsed = ParsedSseEvent::default();
    let mut index = 0;

    while index < event.len() {
        let (line, next_index) = next_sse_line(event, index, allow_eof_without_blank_line)?;
        if line.is_empty() {
            return Some(parsed);
        }

        let trimmed = trim_ascii_prefix(line);
        if let Some(payload_bytes) = trimmed.strip_prefix(b"data:") {
            parsed
                .data_payload_lines
                .push(trim_ascii(payload_bytes).to_vec());
        } else {
            parsed.passthrough_lines.push(line.to_vec());
        }

        index = next_index;
    }

    allow_eof_without_blank_line.then_some(parsed)
}

fn find_sse_event_end(buffer: &[u8]) -> Option<usize> {
    let mut index = 0;
    while index < buffer.len() {
        let (line, next_index) = next_sse_line(buffer, index, false)?;
        if line.is_empty() {
            return Some(next_index);
        }
        index = next_index;
    }
    None
}

fn next_sse_line(
    bytes: &[u8],
    start: usize,
    allow_eof_without_line_ending: bool,
) -> Option<(&[u8], usize)> {
    if start >= bytes.len() {
        return Some((&[], start));
    }

    let mut index = start;
    while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
        index += 1;
    }

    if index >= bytes.len() {
        return allow_eof_without_line_ending.then_some((&bytes[start..], bytes.len()));
    }

    let line = &bytes[start..index];
    if bytes[index] == b'\r' {
        index += 1;
        if index < bytes.len() && bytes[index] == b'\n' {
            index += 1;
        }
    } else {
        index += 1;
    }

    Some((line, index))
}

fn rebuild_sse_event(passthrough_lines: &[Vec<u8>], data_payload_lines: &[Vec<u8>]) -> Bytes {
    let capacity = passthrough_lines
        .iter()
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + data_payload_lines
            .iter()
            .map(|line| 6 + line.len() + 1)
            .sum::<usize>()
        + 1;
    let mut output = Vec::with_capacity(capacity);

    for line in passthrough_lines {
        output.extend_from_slice(line);
        output.push(b'\n');
    }
    for line in data_payload_lines {
        output.extend_from_slice(b"data: ");
        output.extend_from_slice(line);
        output.push(b'\n');
    }
    output.push(b'\n');
    Bytes::from(output)
}

fn join_sse_data_payload_lines(lines: &[Vec<u8>]) -> Vec<u8> {
    if lines.is_empty() {
        return Vec::new();
    }

    let capacity = lines.iter().map(Vec::len).sum::<usize>() + lines.len().saturating_sub(1);
    let mut joined = Vec::with_capacity(capacity);
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            joined.push(b'\n');
        }
        joined.extend_from_slice(line);
    }
    joined
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let bytes = trim_ascii_prefix(bytes);
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(0);
    &bytes[..end]
}

fn trim_ascii_prefix(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct VecBytesStream {
        items: VecDeque<Result<Bytes, reqwest::Error>>,
    }

    impl VecBytesStream {
        fn new(items: Vec<Result<Bytes, reqwest::Error>>) -> Self {
            Self {
                items: items.into_iter().collect(),
            }
        }
    }

    impl Stream for VecBytesStream {
        type Item = Result<Bytes, reqwest::Error>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.items.pop_front())
        }
    }

    async fn next_item<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
        std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
    }

    #[test]
    fn prepare_upstream_request_with_project_wraps_generate_content() {
        let prepared = prepare_upstream_request_with_project(
            "/v1beta/models/gemini-2.5-flash-lite:generateContent",
            None,
            serde_json::json!({
                "model": "ignored-body-model",
                "contents": [{"role": "user", "parts": [{"text": "hello"}]}],
                "system_instruction": {"parts": [{"text": "system"}]},
                "tools": [{
                    "functionDeclarations": [{
                        "name": "sum",
                        "parameters": {"type": "object"}
                    }]
                }]
            }),
            None,
            Some("projects/test-project"),
        )
        .expect("prepare request");

        assert_eq!(prepared.base_url, GEMINI_CODE_ASSIST_BASE_URL);
        assert_eq!(prepared.forwarded_path, "/v1internal:generateContent");
        assert_eq!(prepared.query, None);

        let payload: Value =
            serde_json::from_slice(prepared.body_bytes.as_ref()).expect("payload json");
        assert_eq!(
            payload.get("model").and_then(Value::as_str),
            Some("gemini-2.5-flash-lite")
        );
        assert_eq!(
            payload.get("project").and_then(Value::as_str),
            Some("projects/test-project")
        );
        assert!(payload
            .get("request")
            .and_then(|v| v.get("model"))
            .is_none());
        assert!(payload
            .get("request")
            .and_then(|v| v.get("systemInstruction"))
            .is_some());
        assert!(payload
            .get("request")
            .and_then(|v| v.get("tools"))
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("functionDeclarations"))
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("parametersJsonSchema"))
            .is_some());
    }

    #[test]
    fn prepare_upstream_request_with_project_builds_count_tokens_payload() {
        let prepared = prepare_upstream_request_with_project(
            "/v1beta/models/gemini-2.5-flash-lite:countTokens",
            None,
            serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": "hello"}]}],
                "safetySettings": [{"category": "HARM_CATEGORY_HARASSMENT"}]
            }),
            None,
            None,
        )
        .expect("prepare request");

        assert_eq!(prepared.forwarded_path, "/v1internal:countTokens");
        let payload: Value =
            serde_json::from_slice(prepared.body_bytes.as_ref()).expect("payload json");
        assert_eq!(
            payload
                .get("request")
                .and_then(|v| v.get("model"))
                .and_then(Value::as_str),
            Some("models/gemini-2.5-flash-lite")
        );
        assert!(payload
            .get("request")
            .and_then(|v| v.get("safetySettings"))
            .is_none());
    }

    #[test]
    fn translate_response_body_extracts_wrapped_generate_content_response() {
        let body = Bytes::from(
            serde_json::to_vec(&serde_json::json!({
                "response": {"candidates": [{"content": {"parts": [{"text": "ok"}]}}]},
                "traceId": "trace-1"
            }))
            .expect("serialize"),
        );

        let translated =
            translate_response_body(body, Some(GeminiOAuthResponseMode::GenerateContent));
        let value: Value = serde_json::from_slice(translated.as_ref()).expect("json");

        assert!(value.get("candidates").is_some());
        assert!(value.get("traceId").is_none());
    }

    #[test]
    fn translate_response_body_extracts_wrapped_stream_array_response() {
        let body = Bytes::from(
            serde_json::to_vec(&serde_json::json!([
                {"response": {"candidates": [{"content": {"parts": [{"text": "a"}]}}]}},
                {"response": {"candidates": [{"content": {"parts": [{"text": "b"}]}}]}}
            ]))
            .expect("serialize"),
        );

        let translated =
            translate_response_body(body, Some(GeminiOAuthResponseMode::StreamGenerateContent));
        let value: Value = serde_json::from_slice(translated.as_ref()).expect("json");

        assert_eq!(value.as_array().map(|items| items.len()), Some(2));
        assert!(value
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item.get("candidates"))
            .is_some());
    }

    #[test]
    fn translate_response_body_augments_count_tokens_response() {
        let body = Bytes::from(
            serde_json::to_vec(&serde_json::json!({
                "totalTokens": 42
            }))
            .expect("serialize"),
        );

        let translated = translate_response_body(body, Some(GeminiOAuthResponseMode::CountTokens));
        let value: Value = serde_json::from_slice(translated.as_ref()).expect("json");

        assert_eq!(value.get("totalTokens").and_then(Value::as_i64), Some(42));
        assert!(value
            .get("promptTokensDetails")
            .and_then(Value::as_array)
            .is_some());
    }

    #[test]
    fn transform_sse_event_extracts_wrapped_response() {
        let event =
            b"data: {\"response\":{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]},\"traceId\":\"trace-1\"}\n\n";
        let transformed =
            transform_sse_event(event, Some(GeminiOAuthResponseMode::StreamGenerateContent));

        assert_eq!(
            String::from_utf8(transformed.to_vec()).expect("utf8"),
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}\n\n"
        );
    }

    #[test]
    fn transform_sse_event_joins_multiple_data_lines_before_parsing() {
        let event = concat!(
            "data: {\"response\":\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}}\n",
            "\n"
        )
        .as_bytes();
        let transformed =
            transform_sse_event(event, Some(GeminiOAuthResponseMode::StreamGenerateContent));

        assert_eq!(
            String::from_utf8(transformed.to_vec()).expect("utf8"),
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}\n\n"
        );
    }

    #[test]
    fn finalize_buffered_sse_tail_emits_completed_event_without_trailing_blank_line() {
        let mut tail = b"data: {\"response\":{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}}".to_vec();
        let transformed = finalize_buffered_sse_tail(
            &mut tail,
            Some(GeminiOAuthResponseMode::StreamGenerateContent),
        )
        .expect("finalized tail");

        assert!(tail.is_empty());
        assert_eq!(
            String::from_utf8(transformed.to_vec()).expect("utf8"),
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}\n\n"
        );
    }

    #[test]
    fn finalize_buffered_sse_tail_drops_incomplete_json() {
        let mut tail = b"data: {\"response\":".to_vec();
        let transformed = finalize_buffered_sse_tail(
            &mut tail,
            Some(GeminiOAuthResponseMode::StreamGenerateContent),
        );

        assert!(transformed.is_none());
        assert!(tail.is_empty());
    }

    #[test]
    fn find_sse_event_end_waits_for_blank_line() {
        assert_eq!(find_sse_event_end(b"data: one\n"), None);
        assert_eq!(find_sse_event_end(b"data: one\n\n"), Some(11));
        assert_eq!(find_sse_event_end(b"data: one\r\n\r\n"), Some(13));
    }

    #[tokio::test]
    async fn gemini_oauth_sse_stream_degrades_to_passthrough_when_event_buffer_exceeds_limit() {
        let buffered = Bytes::from_static(b"data: ");
        let oversized = Bytes::from(vec![b'a'; MAX_GEMINI_OAUTH_SSE_EVENT_BUFFER_BYTES]);
        let mut stream = GeminiOAuthSseStream::new(
            VecBytesStream::new(vec![Ok(buffered.clone()), Ok(oversized.clone())]),
            Some(GeminiOAuthResponseMode::StreamGenerateContent),
        );

        let first = next_item(&mut stream)
            .await
            .expect("buffered output")
            .expect("buffered ok");
        let second = next_item(&mut stream)
            .await
            .expect("oversized output")
            .expect("oversized ok");

        assert_eq!(first, buffered);
        assert_eq!(second, oversized);
        assert!(next_item(&mut stream).await.is_none());
    }

    #[test]
    fn translate_query_string_adds_alt_for_stream_requests() {
        assert_eq!(
            translate_query_string(None, GeminiOAuthResponseMode::StreamGenerateContent).as_deref(),
            Some("alt=sse")
        );
        assert_eq!(
            translate_query_string(
                Some("foo=bar&alt=json"),
                GeminiOAuthResponseMode::StreamGenerateContent
            )
            .as_deref(),
            Some("foo=bar&alt=json")
        );
    }
}
