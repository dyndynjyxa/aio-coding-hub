use crate::{blocking, claude_model_validation_history, usage};
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

    let stream = parsed
        .body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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
    let body_bytes = serde_json::to_vec(&parsed.body)
        .map_err(|e| format!("SYSTEM_ERROR: failed to encode body JSON: {e}"))?;

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "aio-coding-hub-validate/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP_CLIENT_INIT: {e}"))?;

    let mut raw_excerpt = Vec::<u8>::new();

    let mut err_out: Option<String> = None;

    let send_result = client
        .post(target_url.clone())
        .headers(headers)
        .body(body_bytes)
        .send()
        .await;

    let resp = match send_result {
        Ok(v) => Some(v),
        Err(e) => {
            err_out = Some(format!("HTTP_ERROR: {e}"));
            None
        }
    };

    if resp.is_none() {
        let result = ClaudeModelValidationResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url: base_url.to_string(),
            target_url: target_url.to_string(),
            status: None,
            duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
            requested_model,
            responded_model: None,
            stream,
            output_text_chars: 0,
            output_text_preview: String::new(),
            checks: serde_json::json!({}),
            signals: serde_json::json!({}),
            response_headers: serde_json::json!({}),
            usage: None,
            error: err_out,
            raw_excerpt: String::new(),
            request: sanitized_request,
        };

        return Ok(result);
    }

    let mut resp = resp.unwrap();
    let response_headers = response::response_headers_to_json(resp.headers());

    let status = resp.status().as_u16();
    let mut total_read = 0usize;

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let content_type_lc = content_type.to_lowercase();
    let is_sse_by_header = content_type_lc.contains("text/event-stream");

    let mut stream_read_error: Option<String> = None;
    let mut response_parse_mode = if is_sse_by_header { "sse" } else { "json" };

    let (
        responded_model,
        usage_json_value,
        output_text_chars,
        output_text_preview,
        thinking_block_seen,
        thinking_chars,
        thinking_preview,
        signature_chars,
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
        // Non-SSE by header: read up to MAX_RESPONSE_BYTES and parse as JSON; if parse fails and
        // caller requested stream=true, fall back to best-effort SSE parsing.
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
            let (resp_id, service_tier) = response::extract_response_meta_from_message_json(&value);
            (
                responded_model,
                usage_json_value,
                chars,
                preview,
                thinking_block,
                thinking_chars,
                thinking_preview,
                signature_chars,
                false,
                None,
                false,
                resp_id,
                service_tier,
            )
        } else if stream {
            response_parse_mode = "sse_fallback";
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
                false,
                None,
                false,
                None,
                None,
            )
        }
    };

    let raw_excerpt_text = String::from_utf8_lossy(&raw_excerpt).to_string();
    let mut signals = response::signals_from_text(&raw_excerpt_text);
    if let Some(obj) = signals.as_object_mut() {
        obj.insert(
            "has_cache_creation_detail".to_string(),
            serde_json::Value::Bool(response::has_cache_creation_detail(
                usage_json_value.as_ref(),
            )),
        );
        obj.insert(
            "thinking_block_seen".to_string(),
            serde_json::Value::Bool(thinking_block_seen),
        );
        obj.insert(
            "thinking_chars".to_string(),
            serde_json::Value::Number((thinking_chars as i64).into()),
        );
        if !thinking_preview.trim().is_empty() {
            obj.insert(
                "thinking_preview".to_string(),
                serde_json::Value::String(thinking_preview.clone()),
            );
        }
        obj.insert(
            "signature_chars".to_string(),
            serde_json::Value::Number((signature_chars as i64).into()),
        );
        if let Some(v) = response_id.as_ref() {
            obj.insert(
                "response_id".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        if let Some(v) = service_tier.as_ref() {
            obj.insert(
                "service_tier".to_string(),
                serde_json::Value::String(v.clone()),
            );
        }
        obj.insert(
            "response_bytes_truncated".to_string(),
            serde_json::Value::Bool(total_read >= MAX_RESPONSE_BYTES),
        );
        obj.insert(
            "response_content_type".to_string(),
            serde_json::Value::String(content_type),
        );
        obj.insert(
            "response_parse_mode".to_string(),
            serde_json::Value::String(response_parse_mode.to_string()),
        );
        obj.insert(
            "stream_read_error".to_string(),
            serde_json::Value::Bool(stream_read_error.is_some()),
        );
        if let Some(err) = &stream_read_error {
            obj.insert(
                "stream_read_error_message".to_string(),
                serde_json::Value::String(err.clone()),
            );
        }
    }

    let mut checks = serde_json::json!({
        "output_text_chars": output_text_chars as i64,
        "thinking_chars": thinking_chars as i64,
        "signature_chars": signature_chars as i64,
        "has_response_id": response_id.is_some(),
        "has_service_tier": service_tier.is_some(),
        "sse_message_delta_seen": sse_message_delta_seen,
        "sse_message_delta_stop_reason": sse_message_delta_stop_reason,
        "sse_message_delta_stop_reason_is_max_tokens": sse_message_delta_stop_reason_is_max_tokens,
    });
    if let Some(max_chars) = parsed.expect_max_output_chars {
        if let Some(obj) = checks.as_object_mut() {
            obj.insert(
                "expect_max_output_chars".to_string(),
                serde_json::Value::Number((max_chars as i64).into()),
            );
            obj.insert(
                "output_text_chars_le_max".to_string(),
                serde_json::Value::Bool(output_text_chars <= max_chars),
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
                serde_json::Value::Bool(output_text_chars == exact_chars),
            );
        }
    }

    // “请求成功”口径（用于 ok 与落库）：HTTP 2xx + 有响应数据 + 无 stream 读取错误。
    //
    // 背景：曾出现 HTTP 2xx 但无响应数据（total_read==0）的场景，前端会误判为成功并写入历史。
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

    let result = ClaudeModelValidationResult {
        ok,
        provider_id: provider.id,
        provider_name: provider.name,
        base_url: base_url.to_string(),
        target_url: target_url.to_string(),
        status: Some(status),
        duration_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
        requested_model,
        responded_model,
        stream,
        output_text_chars: output_text_chars.min(i64::MAX as usize) as i64,
        output_text_preview,
        checks,
        signals,
        response_headers,
        usage: usage_json_value,
        error: err_out,
        raw_excerpt: raw_excerpt_text,
        request: sanitized_request,
    };

    // 仅记录“请求成功”的验证：HTTP 2xx + 有响应数据 + 无 stream 读取错误。
    // 例如 HTTP=503、空响应、stream read error 均不写入历史。
    if result.ok {
        let app_handle = app.clone();
        let request_json_text = request_json.to_string();
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
    }

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
