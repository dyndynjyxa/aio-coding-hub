use crate::{blocking, claude_model_validation_history, usage};
use reqwest::header::HeaderMap;
use serde::Serialize;
use std::time::{Duration, Instant};

mod masking;
mod provider;
mod request;
mod response;

const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_EXCERPT_BYTES: usize = 16 * 1024;
const MAX_TEXT_PREVIEW_CHARS: usize = 4000;
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeModelValidationResult {
    pub ok: bool,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub target_url: String,
    pub status: Option<u16>,
    pub duration_ms: i64,
    pub requested_model: Option<String>,
    pub responded_model: Option<String>,
    pub stream: bool,
    pub output_text_chars: i64,
    pub output_text_preview: String,
    pub checks: serde_json::Value,
    pub signals: serde_json::Value,
    pub response_headers: serde_json::Value,
    pub usage: Option<serde_json::Value>,
    pub error: Option<String>,
    pub raw_excerpt: String,
    pub request: serde_json::Value,
}

#[derive(Debug, Clone)]
struct ProviderForValidation {
    id: i64,
    cli_key: String,
    name: String,
    base_urls: Vec<String>,
    api_key_plaintext: String,
}

#[derive(Debug, Clone)]
struct ParsedRequest {
    request_value: serde_json::Value,
    headers: serde_json::Map<String, serde_json::Value>,
    body: serde_json::Value,
    expect_max_output_chars: Option<usize>,
    expect_exact_output_chars: Option<usize>,
    forwarded_path: String,
    forwarded_query: Option<String>,
    roundtrip: Option<RoundtripConfig>,
}

#[derive(Debug, Clone)]
struct SignatureRoundtripConfig {
    enable_tamper: bool,
    step2_user_prompt: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheRoundtripConfig {
    step2_user_prompt: Option<String>,
}

#[derive(Debug, Clone)]
enum RoundtripConfig {
    Signature(SignatureRoundtripConfig),
    Cache(CacheRoundtripConfig),
}

#[derive(Debug, Clone)]
struct StepOutcome {
    ok: bool,
    status: Option<u16>,
    duration_ms: i64,
    responded_model: Option<String>,
    usage_json_value: Option<serde_json::Value>,
    output_text_chars: usize,
    output_text_preview: String,
    thinking_block_seen: bool,
    thinking_chars: usize,
    thinking_preview: String,
    signature_chars: usize,
    thinking_full: String,
    signature_full: String,
    signature_from_delta: bool,
    sse_message_delta_seen: bool,
    sse_message_delta_stop_reason: Option<String>,
    sse_message_delta_stop_reason_is_max_tokens: bool,
    response_id: Option<String>,
    service_tier: Option<String>,
    response_headers: serde_json::Value,
    raw_excerpt: String,
    response_parse_mode: String,
    response_content_type: String,
    response_bytes_truncated: bool,
    stream_read_error: Option<String>,
    error: Option<String>,
    total_read: usize,
}

fn force_stream_true(body: &mut serde_json::Value) -> bool {
    let Some(obj) = body.as_object_mut() else {
        return false;
    };
    match obj.get("stream").and_then(|v| v.as_bool()) {
        Some(true) => false,
        _ => {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
            true
        }
    }
}

fn infer_cache_min_tokens_for_model(model: Option<&str>) -> usize {
    let m = model.unwrap_or("").trim().to_lowercase();
    if m.contains("opus-4-5") || m.contains("haiku-4-5") {
        return 4096;
    }
    if m.contains("sonnet-4-5") || m.contains("sonnet-4") || m.contains("opus-4") {
        return 1024;
    }
    1024
}

fn apply_prompt_cache_padding(body: &mut serde_json::Value, min_tokens: usize) -> (bool, usize) {
    // Best-effort padding: repeat a simple token-like word to exceed min_tokens.
    // Keep deterministic to maximize cache hit probability across Step1/Step2.
    let word_count = min_tokens.saturating_add(256);
    let marker_begin = "[AIO_CACHE_PAD_BEGIN]";
    let marker_end = "[AIO_CACHE_PAD_END]";

    let Some(obj) = body.as_object_mut() else {
        return (false, word_count);
    };

    let system = obj.entry("system").or_insert_with(|| serde_json::Value::Array(vec![]));
    if !system.is_array() {
        *system = serde_json::Value::Array(vec![]);
    }
    let system_arr = system.as_array_mut().unwrap();

    let mut applied = false;
    for block in system_arr.iter_mut() {
        let Some(block_obj) = block.as_object_mut() else {
            continue;
        };
        let is_ephemeral = block_obj
            .get("cache_control")
            .and_then(|v| v.as_object())
            .and_then(|cc| cc.get("type"))
            .and_then(|v| v.as_str())
            == Some("ephemeral");
        if !is_ephemeral {
            continue;
        }
        let Some(text) = block_obj.get_mut("text").and_then(|v| v.as_str()) else {
            continue;
        };
        if text.contains(marker_begin) {
            return (false, word_count);
        }

        let mut padded = String::with_capacity(text.len() + word_count * 6);
        padded.push_str(text);
        padded.push('\n');
        padded.push_str(marker_begin);
        padded.push('\n');
        for i in 0..word_count {
            if i > 0 {
                padded.push(' ');
            }
            padded.push_str("cachepad");
        }
        padded.push('\n');
        padded.push_str(marker_end);

        block_obj.insert("text".to_string(), serde_json::Value::String(padded));
        applied = true;
        break;
    }

    if !applied {
        let mut padded = String::with_capacity(word_count * 6 + 64);
        padded.push_str("AIO prompt caching validation (auto padding)\n");
        padded.push_str(marker_begin);
        padded.push('\n');
        for i in 0..word_count {
            if i > 0 {
                padded.push(' ');
            }
            padded.push_str("cachepad");
        }
        padded.push('\n');
        padded.push_str(marker_end);
        system_arr.insert(
            0,
            serde_json::json!({
                "type": "text",
                "text": padded,
                "cache_control": { "type": "ephemeral", "ttl": "5m" }
            }),
        );
        applied = true;
    }

    (applied, word_count)
}

fn build_preserved_assistant_message(thinking: &str, signature: &str, text_fallback: &str) -> serde_json::Value {
    let thinking_trimmed = thinking.trim();
    let signature_trimmed = signature.trim();
    let text = text_fallback.trim();

    let mut blocks = Vec::<serde_json::Value>::new();
    blocks.push(serde_json::json!({
        "type": "thinking",
        "thinking": thinking_trimmed,
        "signature": signature_trimmed,
    }));
    if !text.is_empty() {
        blocks.push(serde_json::json!({
            "type": "text",
            "text": text,
        }));
    }
    serde_json::json!({
        "role": "assistant",
        "content": blocks,
    })
}

fn tamper_signature(signature: &str) -> Option<String> {
    let trimmed = signature.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut chars: Vec<char> = trimmed.chars().collect();
    if chars.len() < 6 {
        return None;
    }
    let idx = chars.len() / 2;
    let orig = chars[idx];
    let replacement = if orig != 'A' { 'A' } else { 'B' };
    chars[idx] = replacement;
    Some(chars.into_iter().collect())
}

async fn perform_request(
    client: &reqwest::Client,
    target_url: &reqwest::Url,
    headers: HeaderMap,
    body: serde_json::Value,
    stream_requested: bool,
) -> StepOutcome {
    let started = Instant::now();

    let body_bytes = match serde_json::to_vec(&body) {
        Ok(v) => v,
        Err(e) => {
            return StepOutcome {
                ok: false,
                status: None,
                duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
                responded_model: None,
                usage_json_value: None,
                output_text_chars: 0,
                output_text_preview: String::new(),
                thinking_block_seen: false,
                thinking_chars: 0,
                thinking_preview: String::new(),
                signature_chars: 0,
                thinking_full: String::new(),
                signature_full: String::new(),
                signature_from_delta: false,
                sse_message_delta_seen: false,
                sse_message_delta_stop_reason: None,
                sse_message_delta_stop_reason_is_max_tokens: false,
                response_id: None,
                service_tier: None,
                response_headers: serde_json::json!({}),
                raw_excerpt: String::new(),
                response_parse_mode: "encode_error".to_string(),
                response_content_type: String::new(),
                response_bytes_truncated: false,
                stream_read_error: None,
                error: Some(format!("SYSTEM_ERROR: failed to encode body JSON: {e}")),
                total_read: 0,
            };
        }
    };

    let send_result = client
        .post(target_url.clone())
        .headers(headers)
        .body(body_bytes)
        .send()
        .await;

    let mut err_out: Option<String> = None;
    let mut resp = match send_result {
        Ok(v) => Some(v),
        Err(e) => {
            err_out = Some(format!("HTTP_ERROR: {e}"));
            None
        }
    };

    if resp.is_none() {
        return StepOutcome {
            ok: false,
            status: None,
            duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
            responded_model: None,
            usage_json_value: None,
            output_text_chars: 0,
            output_text_preview: String::new(),
            thinking_block_seen: false,
            thinking_chars: 0,
            thinking_preview: String::new(),
            signature_chars: 0,
            thinking_full: String::new(),
            signature_full: String::new(),
            signature_from_delta: false,
            sse_message_delta_seen: false,
            sse_message_delta_stop_reason: None,
            sse_message_delta_stop_reason_is_max_tokens: false,
            response_id: None,
            service_tier: None,
            response_headers: serde_json::json!({}),
            raw_excerpt: String::new(),
            response_parse_mode: "send_error".to_string(),
            response_content_type: String::new(),
            response_bytes_truncated: false,
            stream_read_error: None,
            error: err_out,
            total_read: 0,
        };
    }

    let mut resp = resp.take().unwrap();
    let response_headers = response::response_headers_to_json(resp.headers());
    let status = resp.status().as_u16();

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let content_type_lc = content_type.to_lowercase();
    let is_sse_by_header = content_type_lc.contains("text/event-stream");

    let mut raw_excerpt = Vec::<u8>::new();
    let mut total_read = 0usize;
    let mut stream_read_error: Option<String> = None;
    let mut response_parse_mode = if is_sse_by_header { "sse" } else { "json" }.to_string();

    let (
        responded_model,
        usage_json_value,
        output_text_chars,
        output_text_preview,
        thinking_block_seen,
        thinking_chars,
        thinking_preview,
        signature_chars,
        thinking_full,
        signature_full,
        signature_from_delta,
        sse_message_delta_seen,
        sse_message_delta_stop_reason,
        sse_message_delta_stop_reason_is_max_tokens,
        response_id,
        service_tier,
    ) = if is_sse_by_header {
        let mut usage_tracker = usage::SseUsageTracker::new("claude");
        let mut text_tracker = response::SseTextAccumulator::default();

        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    total_read = total_read.saturating_add(chunk.len());

                    if raw_excerpt.len() < MAX_EXCERPT_BYTES {
                        let remaining = MAX_EXCERPT_BYTES.saturating_sub(raw_excerpt.len());
                        raw_excerpt.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    }

                    usage_tracker.ingest_chunk(chunk.as_ref());
                    text_tracker.ingest_chunk(chunk.as_ref());

                    if total_read >= MAX_RESPONSE_BYTES {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    stream_read_error = Some(format!("STREAM_READ_ERROR: {e}"));
                    break;
                }
            }
        }

        text_tracker.finalize();
        let usage_extract = usage_tracker.finalize();
        let responded_model = usage_tracker.best_effort_model();
        let usage_json_value = usage_extract
            .as_ref()
            .and_then(|u| serde_json::from_str::<serde_json::Value>(&u.usage_json).ok());

        (
            responded_model,
            usage_json_value,
            text_tracker.total_chars,
            text_tracker.preview,
            text_tracker.thinking_block_seen,
            text_tracker.thinking_chars,
            text_tracker.thinking_preview,
            text_tracker.signature_chars,
            text_tracker.thinking_full,
            text_tracker.signature_full,
            text_tracker.signature_from_delta,
            text_tracker.message_delta_seen,
            text_tracker.message_delta_stop_reason.clone(),
            text_tracker.message_delta_stop_reason_is_max_tokens,
            if text_tracker.response_id.trim().is_empty() {
                None
            } else {
                Some(text_tracker.response_id)
            },
            if text_tracker.service_tier.trim().is_empty() {
                None
            } else {
                Some(text_tracker.service_tier)
            },
        )
    } else {
        let mut buf = Vec::<u8>::new();
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    total_read = total_read.saturating_add(chunk.len());

                    if buf.len() < MAX_RESPONSE_BYTES {
                        let remaining = MAX_RESPONSE_BYTES.saturating_sub(buf.len());
                        buf.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    }
                    if raw_excerpt.len() < MAX_EXCERPT_BYTES {
                        let remaining = MAX_EXCERPT_BYTES.saturating_sub(raw_excerpt.len());
                        raw_excerpt.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                    }

                    if total_read >= MAX_RESPONSE_BYTES {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    stream_read_error = Some(format!("STREAM_READ_ERROR: {e}"));
                    break;
                }
            }
        }

        let responded_model = usage::parse_model_from_json_bytes(&buf);
        let usage_json_value = usage::parse_usage_from_json_bytes(&buf)
            .and_then(|u| serde_json::from_str::<serde_json::Value>(&u.usage_json).ok());

        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&buf) {
            let (chars, preview) = response::extract_text_from_message_json(&value);
            let (thinking_block, thinking_chars, thinking_preview, signature_chars) =
                response::extract_thinking_from_message_json(&value);
            let (thinking_block2, thinking_full, signature_full) =
                response::extract_thinking_full_and_signature_from_message_json(&value);
            let (resp_id, service_tier) = response::extract_response_meta_from_message_json(&value);
            (
                responded_model,
                usage_json_value,
                chars,
                preview,
                thinking_block || thinking_block2,
                thinking_chars,
                thinking_preview,
                signature_chars,
                thinking_full,
                signature_full,
                false,
                false,
                None,
                false,
                resp_id,
                service_tier,
            )
        } else if stream_requested {
            response_parse_mode = "sse_fallback".to_string();
            let mut usage_tracker = usage::SseUsageTracker::new("claude");
            let mut text_tracker = response::SseTextAccumulator::default();
            usage_tracker.ingest_chunk(&buf);
            text_tracker.ingest_chunk(&buf);
            text_tracker.finalize();
            let usage_extract = usage_tracker.finalize();
            let responded_model = usage_tracker.best_effort_model().or(responded_model);
            let usage_json_value = usage_extract
                .as_ref()
                .and_then(|u| serde_json::from_str::<serde_json::Value>(&u.usage_json).ok())
                .or(usage_json_value);
            (
                responded_model,
                usage_json_value,
                text_tracker.total_chars,
                text_tracker.preview,
                text_tracker.thinking_block_seen,
                text_tracker.thinking_chars,
                text_tracker.thinking_preview,
                text_tracker.signature_chars,
                text_tracker.thinking_full,
                text_tracker.signature_full,
                text_tracker.signature_from_delta,
                text_tracker.message_delta_seen,
                text_tracker.message_delta_stop_reason.clone(),
                text_tracker.message_delta_stop_reason_is_max_tokens,
                if text_tracker.response_id.trim().is_empty() {
                    None
                } else {
                    Some(text_tracker.response_id)
                },
                if text_tracker.service_tier.trim().is_empty() {
                    None
                } else {
                    Some(text_tracker.service_tier)
                },
            )
        } else {
            (
                responded_model,
                usage_json_value,
                0usize,
                String::new(),
                false,
                0usize,
                String::new(),
                0usize,
                String::new(),
                String::new(),
                false,
                false,
                None,
                false,
                None,
                None,
            )
        }
    };

    let raw_excerpt_text = String::from_utf8_lossy(&raw_excerpt).to_string();

    let http_ok = (200..300).contains(&status);
    let has_body_bytes = total_read > 0;
    let no_stream_read_error = stream_read_error.is_none();
    let ok = http_ok && has_body_bytes && no_stream_read_error;

    if err_out.is_none() {
        err_out = stream_read_error.clone();
    }
    if http_ok && !has_body_bytes && err_out.is_none() {
        err_out = Some("EMPTY_RESPONSE_BODY".to_string());
    }
    if !http_ok && err_out.is_none() {
        err_out = Some(format!("UPSTREAM_ERROR: status={status}"));
    }

    StepOutcome {
        ok,
        status: Some(status),
        duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
        responded_model,
        usage_json_value,
        output_text_chars,
        output_text_preview,
        thinking_block_seen,
        thinking_chars,
        thinking_preview,
        signature_chars,
        thinking_full,
        signature_full,
        signature_from_delta,
        sse_message_delta_seen,
        sse_message_delta_stop_reason,
        sse_message_delta_stop_reason_is_max_tokens,
        response_id,
        service_tier,
        response_headers,
        raw_excerpt: raw_excerpt_text,
        response_parse_mode,
        response_content_type: content_type,
        response_bytes_truncated: total_read >= MAX_RESPONSE_BYTES,
        stream_read_error,
        error: err_out,
        total_read,
    }
}

pub async fn validate_provider_model(
    app: &tauri::AppHandle,
    provider_id: i64,
    base_url: &str,
    request_json: &str,
) -> Result<ClaudeModelValidationResult, String> {
    let started = Instant::now();

    let provider = provider::load_provider(app.clone(), provider_id).await?;
    if provider.cli_key != "claude" {
        return Err("SEC_INVALID_INPUT: only cli_key=claude is supported".to_string());
    }

    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err("SEC_INVALID_INPUT: base_url is required".to_string());
    }

    if !provider.base_urls.iter().any(|u| u == base_url) {
        return Err("SEC_INVALID_INPUT: base_url must be one of provider.base_urls".to_string());
    }

    let parsed = request::parse_request_json(request_json)?;

    let requested_model = parsed
        .body
        .get("model")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let target_url = request::build_target_url(
        base_url,
        &parsed.forwarded_path,
        parsed.forwarded_query.as_deref(),
    )?;

    let mut sanitized_request = parsed.request_value.clone();
    if let Some(obj) = sanitized_request.as_object_mut() {
        if let Some(headers) = obj.get_mut("headers").and_then(|v| v.as_object_mut()) {
            let mut next = serde_json::Map::new();
            for (k, v) in headers.iter() {
                if let Some(s) = v.as_str() {
                    next.insert(k.clone(), masking::mask_header_value(k, s));
                }
            }
            // Ensure x-api-key is always masked (even if user did not include it).
            next.insert(
                "x-api-key".to_string(),
                serde_json::Value::String("***".to_string()),
            );
            *headers = next;
        }
    }

    let headers = request::header_map_from_json(&parsed.headers, &provider.api_key_plaintext);

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "aio-coding-hub-validate/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP_CLIENT_INIT: {e}"))?;

    let mut step1_body = parsed.body.clone();
    let stream_forced = force_stream_true(&mut step1_body);
    let stream = true;

    // IMPORTANT: cache padding is ONLY for the explicit caching validation template.
    // Applying padding to other templates (e.g. max_tokens=5 probes) can drastically increase
    // request size and cause previously-stable providers to time out or reject.
    let wants_cache_padding = matches!(parsed.roundtrip, Some(RoundtripConfig::Cache(_)));

    let mut cache_pad_applied: Option<bool> = None;
    let mut cache_pad_word_count: Option<usize> = None;
    if wants_cache_padding {
        let min_tokens = infer_cache_min_tokens_for_model(
            step1_body
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty()),
        );
        let (applied, word_count) = apply_prompt_cache_padding(&mut step1_body, min_tokens);
        cache_pad_applied = Some(applied);
        cache_pad_word_count = Some(word_count);
    }

    // Reflect the actually-sent Step1 body back into the request field (so the UI/diagnostics
    // can see stream=true and any auto padding decisions). Keep headers masked.
    if let Some(obj) = sanitized_request.as_object_mut() {
        if obj.contains_key("body") {
            obj.insert("body".to_string(), step1_body.clone());
        } else {
            sanitized_request = step1_body.clone();
        }
    }

    let step1 = perform_request(&client, &target_url, headers.clone(), step1_body.clone(), stream);
    let step1 = step1.await;

    let raw_excerpt_text = step1.raw_excerpt.clone();
    let mut signals = response::signals_from_text(&raw_excerpt_text);
    if let Some(obj) = signals.as_object_mut() {
        obj.insert(
            "stream_forced_true".to_string(),
            serde_json::Value::Bool(stream_forced),
        );
        obj.insert(
            "step1_duration_ms".to_string(),
            serde_json::Value::Number(step1.duration_ms.into()),
        );
        obj.insert(
            "step1_total_read".to_string(),
            serde_json::Value::Number((step1.total_read as i64).into()),
        );
        if let Some(applied) = cache_pad_applied {
            obj.insert(
                "cache_pad_applied".to_string(),
                serde_json::Value::Bool(applied),
            );
        }
        if let Some(word_count) = cache_pad_word_count {
            obj.insert(
                "cache_pad_word_count".to_string(),
                serde_json::Value::Number((word_count as i64).into()),
            );
        }
        obj.insert(
            "has_cache_creation_detail".to_string(),
            serde_json::Value::Bool(response::has_cache_creation_detail(
                step1.usage_json_value.as_ref(),
            )),
        );
        obj.insert(
            "thinking_block_seen".to_string(),
            serde_json::Value::Bool(step1.thinking_block_seen),
        );
        obj.insert(
            "thinking_chars".to_string(),
            serde_json::Value::Number((step1.thinking_chars as i64).into()),
        );
        if !step1.thinking_preview.trim().is_empty() {
            obj.insert(
                "thinking_preview".to_string(),
                serde_json::Value::String(step1.thinking_preview.clone()),
            );
        }
        obj.insert(
            "signature_chars".to_string(),
            serde_json::Value::Number((step1.signature_chars as i64).into()),
        );
        obj.insert(
            "signature_from_delta".to_string(),
            serde_json::Value::Bool(step1.signature_from_delta),
        );
        if let Some(v) = step1.response_id.as_ref() {
            obj.insert(
                "response_id".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        if let Some(v) = step1.service_tier.as_ref() {
            obj.insert(
                "service_tier".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        obj.insert(
            "response_bytes_truncated".to_string(),
            serde_json::Value::Bool(step1.response_bytes_truncated),
        );
        obj.insert(
            "response_content_type".to_string(),
            serde_json::Value::String(step1.response_content_type.clone()),
        );
        obj.insert(
            "response_parse_mode".to_string(),
            serde_json::Value::String(step1.response_parse_mode.clone()),
        );
        obj.insert(
            "stream_read_error".to_string(),
            serde_json::Value::Bool(step1.stream_read_error.is_some()),
        );
        if let Some(err) = &step1.stream_read_error {
            obj.insert(
                "stream_read_error_message".to_string(),
                serde_json::Value::String(err.clone()),
            );
        }
    }

    let mut checks = serde_json::json!({
        "output_text_chars": step1.output_text_chars as i64,
        "thinking_chars": step1.thinking_chars as i64,
        "signature_chars": step1.signature_chars as i64,
        "has_response_id": step1.response_id.is_some(),
        "has_service_tier": step1.service_tier.is_some(),
        "sse_message_delta_seen": step1.sse_message_delta_seen,
        "sse_message_delta_stop_reason": step1.sse_message_delta_stop_reason,
        "sse_message_delta_stop_reason_is_max_tokens": step1.sse_message_delta_stop_reason_is_max_tokens,
    });
    if let Some(max_chars) = parsed.expect_max_output_chars {
        if let Some(obj) = checks.as_object_mut() {
            obj.insert(
                "expect_max_output_chars".to_string(),
                serde_json::Value::Number((max_chars as i64).into()),
            );
            obj.insert(
                "output_text_chars_le_max".to_string(),
                serde_json::Value::Bool(step1.output_text_chars <= max_chars),
            );
        }
    }
    if let Some(exact_chars) = parsed.expect_exact_output_chars {
        if let Some(obj) = checks.as_object_mut() {
            obj.insert(
                "expect_exact_output_chars".to_string(),
                serde_json::Value::Number((exact_chars as i64).into()),
            );
            obj.insert(
                "output_text_chars_eq_expected".to_string(),
                serde_json::Value::Bool(step1.output_text_chars == exact_chars),
            );
        }
    }

    if let Some(obj) = signals.as_object_mut() {
        if let Some(roundtrip) = parsed.roundtrip.as_ref() {
            match roundtrip {
                RoundtripConfig::Signature(cfg) => {
                    obj.insert(
                        "roundtrip_kind".to_string(),
                        serde_json::Value::String("signature".to_string()),
                    );

                    let signature = step1.signature_full.trim();
                    let thinking = step1.thinking_full.trim();
                    if signature.is_empty() || thinking.is_empty() {
                        obj.insert(
                            "roundtrip_step2_ok".to_string(),
                            serde_json::Value::Bool(false),
                        );
                        obj.insert(
                            "roundtrip_step2_error".to_string(),
                            serde_json::Value::String(
                                "MISSING_STEP1_THINKING_SIGNATURE: cannot run roundtrip".to_string(),
                            ),
                        );
                    } else {
                        let step2_prompt = cfg.step2_user_prompt.clone().unwrap_or_else(|| {
                            "第一行原样输出暗号：AIO_MULTI_TURN_OK（不要解释）。\n第二行输出 OK。"
                                .to_string()
                        });
                        let assistant_message =
                            build_preserved_assistant_message(thinking, signature, &step1.output_text_preview);

                        let mut step2_body = step1_body.clone();
                        force_stream_true(&mut step2_body);

                        if let Some(body_obj) = step2_body.as_object_mut() {
                            body_obj.insert(
                                "messages".to_string(),
                                serde_json::Value::Array(vec![
                                    serde_json::json!({
                                        "role": "user",
                                        "content": "Continue.",
                                    }),
                                    assistant_message,
                                    serde_json::json!({
                                        "role": "user",
                                        "content": step2_prompt.clone(),
                                    }),
                                ]),
                            );
                        }

                        let step2 = perform_request(
                            &client,
                            &target_url,
                            headers.clone(),
                            step2_body.clone(),
                            true,
                        )
                        .await;

                        obj.insert(
                            "roundtrip_step2_status".to_string(),
                            step2
                                .status
                                .map(|s| serde_json::Value::Number((s as i64).into()))
                                .unwrap_or(serde_json::Value::Null),
                        );
                        obj.insert(
                            "roundtrip_step2_ok".to_string(),
                            serde_json::Value::Bool(step2.ok),
                        );
                        if !step2.output_text_preview.trim().is_empty() {
                            obj.insert(
                                "roundtrip_step2_output_preview".to_string(),
                                serde_json::Value::String(step2.output_text_preview.clone()),
                            );
                        }
                        obj.insert(
                            "roundtrip_step2_response_parse_mode".to_string(),
                            serde_json::Value::String(step2.response_parse_mode.clone()),
                        );
                        if let Some(err) = step2.error.as_ref() {
                            obj.insert(
                                "roundtrip_step2_error".to_string(),
                                serde_json::Value::String(err.clone()),
                            );
                        }

                        obj.insert(
                            "roundtrip_step3_enabled".to_string(),
                            serde_json::Value::Bool(cfg.enable_tamper),
                        );

                        if cfg.enable_tamper {
                            let tampered = tamper_signature(signature);
                            if tampered.is_none() {
                                obj.insert(
                                    "roundtrip_step3_error".to_string(),
                                    serde_json::Value::String("TAMPER_NOT_POSSIBLE".to_string()),
                                );
                            } else {
                                let tampered_assistant = build_preserved_assistant_message(
                                    thinking,
                                    tampered.as_deref().unwrap_or(signature),
                                    &step1.output_text_preview,
                                );
                                let mut step3_body = step1_body.clone();
                                force_stream_true(&mut step3_body);
                                if let Some(body_obj) = step3_body.as_object_mut() {
                                    body_obj.insert(
                                        "messages".to_string(),
                                        serde_json::Value::Array(vec![
                                            serde_json::json!({
                                                "role": "user",
                                                "content": "Continue.",
                                            }),
                                            tampered_assistant,
                                            serde_json::json!({
                                                "role": "user",
                                                "content": step2_prompt,
                                            }),
                                        ]),
                                    );
                                }

                                let step3 = perform_request(
                                    &client,
                                    &target_url,
                                    headers.clone(),
                                    step3_body,
                                    true,
                                )
                                .await;

                                let step3_signals = response::signals_from_text(&step3.raw_excerpt);
                                let mentions_invalid_signature = step3_signals
                                    .get("mentions_invalid_signature")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let rejected = match step3.status {
                                    Some(400) => true,
                                    Some(s) if (200..300).contains(&s) => false,
                                    _ => mentions_invalid_signature,
                                };

                                obj.insert(
                                    "roundtrip_step3_status".to_string(),
                                    step3
                                        .status
                                        .map(|s| serde_json::Value::Number((s as i64).into()))
                                        .unwrap_or(serde_json::Value::Null),
                                );
                                obj.insert(
                                    "roundtrip_step3_mentions_invalid_signature".to_string(),
                                    serde_json::Value::Bool(mentions_invalid_signature),
                                );
                                obj.insert(
                                    "roundtrip_step3_rejected".to_string(),
                                    serde_json::Value::Bool(rejected),
                                );
                                if let Some(err) = step3.error.as_ref() {
                                    obj.insert(
                                        "roundtrip_step3_error".to_string(),
                                        serde_json::Value::String(err.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
                RoundtripConfig::Cache(cfg) => {
                    obj.insert(
                        "roundtrip_kind".to_string(),
                        serde_json::Value::String("cache".to_string()),
                    );

                    let step2_prompt = cfg
                        .step2_user_prompt
                        .clone()
                        .unwrap_or_else(|| "Step2：请只回复 OK2（不要输出其他内容）。".to_string());

                    let mut step2_body = step1_body.clone();
                    force_stream_true(&mut step2_body);
                    if let Some(body_obj) = step2_body.as_object_mut() {
                        body_obj.insert(
                            "messages".to_string(),
                            serde_json::Value::Array(vec![serde_json::json!({
                                "role": "user",
                                "content": step2_prompt,
                            })]),
                        );
                    }

                    let step2 = perform_request(
                        &client,
                        &target_url,
                        headers.clone(),
                        step2_body,
                        true,
                    )
                    .await;

                    obj.insert(
                        "roundtrip_step2_status".to_string(),
                        step2
                            .status
                            .map(|s| serde_json::Value::Number((s as i64).into()))
                            .unwrap_or(serde_json::Value::Null),
                    );
                    obj.insert(
                        "roundtrip_step2_ok".to_string(),
                        serde_json::Value::Bool(step2.ok),
                    );
                    if !step2.output_text_preview.trim().is_empty() {
                        obj.insert(
                            "roundtrip_step2_output_preview".to_string(),
                            serde_json::Value::String(step2.output_text_preview.clone()),
                        );
                    }
                    if let Some(err) = step2.error.as_ref() {
                        obj.insert(
                            "roundtrip_step2_error".to_string(),
                            serde_json::Value::String(err.clone()),
                        );
                    }
                    if let Some(usage_obj) = step2.usage_json_value.as_ref().and_then(|v| v.as_object()) {
                        if let Some(v) = usage_obj.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
                            obj.insert(
                                "roundtrip_step2_cache_read_input_tokens".to_string(),
                                serde_json::Value::Number(v.into()),
                            );
                        }
                    }
                }
            }
        }
    }

    let sanitized_request_text =
        serde_json::to_string_pretty(&sanitized_request).unwrap_or_else(|_| "{}".to_string());

    let result = ClaudeModelValidationResult {
        ok: step1.ok,
        provider_id: provider.id,
        provider_name: provider.name,
        base_url: base_url.to_string(),
        target_url: target_url.to_string(),
        status: step1.status,
        duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
        requested_model,
        responded_model: step1.responded_model,
        stream,
        output_text_chars: step1.output_text_chars.min(i64::MAX as usize) as i64,
        output_text_preview: step1.output_text_preview,
        checks,
        signals,
        response_headers: step1.response_headers,
        usage: step1.usage_json_value,
        error: step1.error,
        raw_excerpt: raw_excerpt_text,
        request: sanitized_request,
    };

    // 用户要求：历史需要保留失败步骤用于诊断与回溯（suite 每一步都可查看）。
    //
    // 安全要求：request_json 不得落库明文 key，因此入库的 request_json 使用后端构造的 sanitized_request
    //（headers 已统一 mask 为 "***"，且回显的是“实际发送”的 Step1 body：包含 stream=true 与 auto padding 决策）。
    let app_handle = app.clone();
    let request_json_text = sanitized_request_text;
    let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
    let _ = blocking::run("claude_validation_history_insert", move || {
        claude_model_validation_history::insert_run_and_prune(
            &app_handle,
            provider.id,
            &request_json_text,
            &result_json,
            Some(50),
        )?;
        Ok(())
    })
    .await;

    Ok(result)
}

pub async fn get_provider_api_key_plaintext(
    app: &tauri::AppHandle,
    provider_id: i64,
) -> Result<String, String> {
    let provider = provider::load_provider(app.clone(), provider_id).await?;
    if provider.cli_key != "claude" {
        return Err("SEC_INVALID_INPUT: only cli_key=claude is supported".to_string());
    }
    Ok(provider.api_key_plaintext)
}

#[cfg(test)]
mod tests {
    use super::build_preserved_assistant_message;

    #[test]
    fn preserved_assistant_message_contains_thinking_signature_and_optional_text() {
        let msg = build_preserved_assistant_message("  THINK  ", "  SIG  ", "  hello  ");
        assert_eq!(msg.get("role").and_then(|v| v.as_str()), Some("assistant"));

        let content = msg.get("content").and_then(|v| v.as_array()).unwrap();
        assert_eq!(content.len(), 2);

        let thinking = content[0].as_object().unwrap();
        assert_eq!(thinking.get("type").and_then(|v| v.as_str()), Some("thinking"));
        assert_eq!(
            thinking.get("thinking").and_then(|v| v.as_str()),
            Some("THINK")
        );
        assert_eq!(
            thinking.get("signature").and_then(|v| v.as_str()),
            Some("SIG")
        );

        let text = content[1].as_object().unwrap();
        assert_eq!(text.get("type").and_then(|v| v.as_str()), Some("text"));
        assert_eq!(text.get("text").and_then(|v| v.as_str()), Some("hello"));
    }

    #[test]
    fn preserved_assistant_message_omits_text_block_when_empty() {
        let msg = build_preserved_assistant_message("THINK", "SIG", "   ");
        let content = msg.get("content").and_then(|v| v.as_array()).unwrap();
        assert_eq!(content.len(), 1);
    }
}
