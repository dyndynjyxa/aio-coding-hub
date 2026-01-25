//! Usage: Execute a validation request and parse the response into a `StepOutcome`.

use crate::usage;
use reqwest::header::HeaderMap;
use std::time::Instant;

use super::response;
use super::types::StepOutcome;
use super::{MAX_EXCERPT_BYTES, MAX_RESPONSE_BYTES};

pub(super) async fn perform_request(
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
            return StepOutcome::error(
                started,
                "encode_error",
                format!("SYSTEM_ERROR: failed to encode body JSON: {e}"),
            );
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
        return StepOutcome::error(
            started,
            "send_error",
            err_out.unwrap_or_else(|| "UNKNOWN_ERROR".to_string()),
        );
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
        sse_error_event_seen,
        sse_error_status,
        sse_error_message,
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
            text_tracker.error_event_seen,
            text_tracker.error_status,
            text_tracker.error_message.clone(),
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
                false,
                None,
                String::new(),
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
                text_tracker.error_event_seen,
                text_tracker.error_status,
                text_tracker.error_message.clone(),
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
                false,
                None,
                String::new(),
                None,
                None,
            )
        }
    };

    let raw_excerpt_text = String::from_utf8_lossy(&raw_excerpt).to_string();

    let mut status_out = status;
    if sse_error_event_seen && (200..300).contains(&status_out) {
        if let Some(s) = sse_error_status {
            status_out = s;
        }
    }

    if sse_error_event_seen {
        let sse_msg = if sse_error_message.trim().is_empty() {
            if let Some(s) = sse_error_status {
                format!("UPSTREAM_SSE_ERROR: status={s}")
            } else {
                "UPSTREAM_SSE_ERROR".to_string()
            }
        } else {
            sse_error_message.clone()
        };

        err_out = match err_out {
            Some(existing) if !existing.trim().is_empty() && existing.trim() != sse_msg => {
                Some(format!("{existing}; {sse_msg}"))
            }
            _ => Some(sse_msg),
        };
    }

    let http_ok = (200..300).contains(&status_out);
    let has_body_bytes = total_read > 0;
    let no_stream_read_error = stream_read_error.is_none();
    let ok = http_ok && has_body_bytes && no_stream_read_error && !sse_error_event_seen;

    if err_out.is_none() {
        err_out = stream_read_error.clone();
    }
    if http_ok && !has_body_bytes && err_out.is_none() {
        err_out = Some("EMPTY_RESPONSE_BODY".to_string());
    }
    if !http_ok && err_out.is_none() {
        err_out = Some(format!("UPSTREAM_ERROR: status={status_out}"));
    }

    StepOutcome {
        ok,
        status: Some(status_out),
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
        sse_error_event_seen,
        sse_error_status,
        sse_error_message,
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
