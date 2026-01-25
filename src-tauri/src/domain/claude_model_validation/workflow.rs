//! Usage: Claude model validation workflow (HTTP execution + roundtrip checks).

use crate::{blocking, claude_model_validation_history, db};
use std::time::Instant;

use super::execute::perform_request;
use super::padding::{
    apply_prompt_cache_padding, force_stream_true, infer_cache_min_tokens_for_model,
};
use super::types::{ClaudeModelValidationResult, RoundtripConfig};
use super::{masking, provider, request, response, HTTP_CONNECT_TIMEOUT, HTTP_TIMEOUT};

/// Helper trait to simplify JSON object insertions.
trait SignalsExt {
    fn insert_bool(&mut self, key: &str, value: bool);
    fn insert_i64(&mut self, key: &str, value: i64);
    fn insert_usize(&mut self, key: &str, value: usize);
    fn insert_str(&mut self, key: &str, value: &str);
    fn insert_opt_str(&mut self, key: &str, value: Option<&str>);
}

impl SignalsExt for serde_json::Map<String, serde_json::Value> {
    fn insert_bool(&mut self, key: &str, value: bool) {
        self.insert(key.to_string(), value.into());
    }

    fn insert_i64(&mut self, key: &str, value: i64) {
        self.insert(key.to_string(), value.into());
    }

    fn insert_usize(&mut self, key: &str, value: usize) {
        self.insert(key.to_string(), (value as i64).into());
    }

    fn insert_str(&mut self, key: &str, value: &str) {
        if !value.is_empty() {
            self.insert(key.to_string(), value.to_string().into());
        }
    }

    fn insert_opt_str(&mut self, key: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.insert_str(key, v);
        }
    }
}

fn build_preserved_assistant_message(
    thinking: &str,
    signature: &str,
    text_fallback: &str,
) -> serde_json::Value {
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

pub(super) async fn validate_provider_model(
    db: db::Db,
    provider_id: i64,
    base_url: &str,
    request_json: &str,
) -> Result<ClaudeModelValidationResult, String> {
    let started = Instant::now();

    let provider = provider::load_provider(db.clone(), provider_id).await?;
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
    let force_padding = matches!(
        parsed.roundtrip,
        Some(RoundtripConfig::Cache(ref cfg)) if cfg.force_padding
    );

    let mut cache_pad_applied: Option<bool> = None;
    let mut cache_pad_word_count: Option<usize> = None;
    let mut cache_pad_force_mode: Option<bool> = None;
    if wants_cache_padding {
        let base_min_tokens = infer_cache_min_tokens_for_model(
            step1_body
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty()),
        );
        // When force_padding is enabled, use a much larger padding to guarantee cache creation.
        // 5000 tokens is well above the max threshold (4096 for Haiku 4.5).
        let min_tokens = if force_padding {
            base_min_tokens.max(5000)
        } else {
            base_min_tokens
        };
        let (applied, word_count) = apply_prompt_cache_padding(&mut step1_body, min_tokens);
        cache_pad_applied = Some(applied);
        cache_pad_word_count = Some(word_count);
        cache_pad_force_mode = Some(force_padding);
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

    let step1 = perform_request(
        &client,
        &target_url,
        headers.clone(),
        step1_body.clone(),
        stream,
    );
    let step1 = step1.await;

    let raw_excerpt_text = step1.raw_excerpt.clone();
    let mut signals = response::signals_from_text(&raw_excerpt_text);
    if let Some(obj) = signals.as_object_mut() {
        obj.insert_bool("stream_forced_true", stream_forced);
        obj.insert_i64("step1_duration_ms", step1.duration_ms);
        obj.insert_usize("step1_total_read", step1.total_read);

        if let Some(applied) = cache_pad_applied {
            obj.insert_bool("cache_pad_applied", applied);
        }
        if let Some(word_count) = cache_pad_word_count {
            obj.insert_usize("cache_pad_word_count", word_count);
        }
        if let Some(force_mode) = cache_pad_force_mode {
            obj.insert_bool("cache_pad_force_mode", force_mode);
        }

        obj.insert_bool(
            "has_cache_creation_detail",
            response::has_cache_creation_detail(step1.usage_json_value.as_ref()),
        );
        obj.insert_bool("thinking_block_seen", step1.thinking_block_seen);
        obj.insert_usize("thinking_chars", step1.thinking_chars);
        obj.insert_str("thinking_preview", &step1.thinking_preview);
        obj.insert_usize("signature_chars", step1.signature_chars);
        obj.insert_bool("signature_from_delta", step1.signature_from_delta);
        obj.insert_opt_str("response_id", step1.response_id.as_deref());
        obj.insert_opt_str("service_tier", step1.service_tier.as_deref());
        obj.insert_bool("response_bytes_truncated", step1.response_bytes_truncated);
        obj.insert_str("response_content_type", &step1.response_content_type);
        obj.insert_str("response_parse_mode", &step1.response_parse_mode);
        obj.insert_bool("stream_read_error", step1.stream_read_error.is_some());
        obj.insert_opt_str("stream_read_error_message", step1.stream_read_error.as_deref());
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
        "sse_error_event_seen": step1.sse_error_event_seen,
        "sse_error_status": step1.sse_error_status.map(|s| s as i64),
        "sse_error_message": step1.sse_error_message,
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

                    // Record cross-provider configuration
                    let has_cross_provider = cfg.cross_provider_id.is_some();
                    obj.insert(
                        "roundtrip_cross_provider_enabled".to_string(),
                        serde_json::Value::Bool(has_cross_provider),
                    );
                    if let Some(cross_id) = cfg.cross_provider_id {
                        obj.insert(
                            "roundtrip_cross_provider_id".to_string(),
                            serde_json::Value::Number(cross_id.into()),
                        );
                    }

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
                                "MISSING_STEP1_THINKING_SIGNATURE: cannot run roundtrip"
                                    .to_string(),
                            ),
                        );
                    } else {
                        let step2_prompt = cfg.step2_user_prompt.clone().unwrap_or_else(|| {
                            "第一行原样输出暗号：AIO_MULTI_TURN_OK（不要解释）。\n第二行输出 OK。"
                                .to_string()
                        });
                        let assistant_message = build_preserved_assistant_message(
                            thinking,
                            signature,
                            &step1.output_text_preview,
                        );

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
                                    assistant_message.clone(),
                                    serde_json::json!({
                                        "role": "user",
                                        "content": step2_prompt.clone(),
                                    }),
                                ]),
                            );
                        }

                        // Step2: always validate on the original provider (non-tampered).
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
                        // Step2 thinking info for cross-step thinking preservation check
                        if step2.thinking_chars > 0 {
                            obj.insert(
                                "roundtrip_step2_thinking_chars".to_string(),
                                serde_json::Value::Number((step2.thinking_chars as i64).into()),
                            );
                        }
                        if !step2.thinking_preview.trim().is_empty() {
                            obj.insert(
                                "roundtrip_step2_thinking_preview".to_string(),
                                serde_json::Value::String(step2.thinking_preview.clone()),
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

                        if let Some(cross_id) = cfg.cross_provider_id {
                            // Step3: cross-provider positive verification (non-tampered signature).
                            let mut step3_cross_error: Option<String> = None;
                            let (step3_target_url, step3_headers) =
                                match provider::load_provider(db.clone(), cross_id).await {
                                    Ok(cross_provider) => {
                                        obj.insert(
                                            "roundtrip_cross_provider_name".to_string(),
                                            serde_json::Value::String(cross_provider.name.clone()),
                                        );
                                        let cross_base_url = cross_provider
                                            .base_urls
                                            .first()
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                "https://api.anthropic.com".to_string()
                                            });
                                        obj.insert(
                                            "roundtrip_cross_provider_base_url".to_string(),
                                            serde_json::Value::String(cross_base_url.clone()),
                                        );

                                        match request::build_target_url(
                                            &cross_base_url,
                                            &parsed.forwarded_path,
                                            parsed.forwarded_query.as_deref(),
                                        ) {
                                            Ok(url) => {
                                                let hdrs = request::header_map_from_json(
                                                    &parsed.headers,
                                                    &cross_provider.api_key_plaintext,
                                                );
                                                (Some(url), Some(hdrs))
                                            }
                                            Err(e) => {
                                                step3_cross_error =
                                                    Some(format!("CROSS_PROVIDER_URL_ERROR: {e}"));
                                                (None, None)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        step3_cross_error =
                                            Some(format!("CROSS_PROVIDER_LOAD_ERROR: {e}"));
                                        (None, None)
                                    }
                                };

                            if let (Some(step3_target_url), Some(step3_headers)) =
                                (step3_target_url, step3_headers)
                            {
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
                                            assistant_message,
                                            serde_json::json!({
                                                "role": "user",
                                                "content": step2_prompt.clone(),
                                            }),
                                        ]),
                                    );
                                }

                                let step3 = perform_request(
                                    &client,
                                    &step3_target_url,
                                    step3_headers,
                                    step3_body,
                                    true,
                                )
                                .await;

                                obj.insert(
                                    "roundtrip_step3_cross_status".to_string(),
                                    step3
                                        .status
                                        .map(|s| serde_json::Value::Number((s as i64).into()))
                                        .unwrap_or(serde_json::Value::Null),
                                );
                                obj.insert(
                                    "roundtrip_step3_cross_ok".to_string(),
                                    serde_json::Value::Bool(step3.ok),
                                );
                                if !step3.output_text_preview.trim().is_empty() {
                                    obj.insert(
                                        "roundtrip_step3_cross_output_preview".to_string(),
                                        serde_json::Value::String(
                                            step3.output_text_preview.clone(),
                                        ),
                                    );
                                }
                                if step3.thinking_chars > 0 {
                                    obj.insert(
                                        "roundtrip_step3_cross_thinking_chars".to_string(),
                                        serde_json::Value::Number(
                                            (step3.thinking_chars as i64).into(),
                                        ),
                                    );
                                }
                                if !step3.thinking_preview.trim().is_empty() {
                                    obj.insert(
                                        "roundtrip_step3_cross_thinking_preview".to_string(),
                                        serde_json::Value::String(step3.thinking_preview.clone()),
                                    );
                                }
                                obj.insert(
                                    "roundtrip_step3_cross_response_parse_mode".to_string(),
                                    serde_json::Value::String(step3.response_parse_mode.clone()),
                                );
                                if let Some(err) = step3.error.as_ref() {
                                    obj.insert(
                                        "roundtrip_step3_cross_error".to_string(),
                                        serde_json::Value::String(err.clone()),
                                    );
                                }
                            } else {
                                obj.insert(
                                    "roundtrip_step3_cross_ok".to_string(),
                                    serde_json::Value::Bool(false),
                                );
                            }

                            if let Some(err) = step3_cross_error {
                                obj.insert(
                                    "roundtrip_step3_cross_error".to_string(),
                                    serde_json::Value::String(err),
                                );
                            }
                        } else {
                            // Step3: optional tamper negative verification (same provider).
                            obj.insert(
                                "roundtrip_step3_enabled".to_string(),
                                serde_json::Value::Bool(cfg.enable_tamper),
                            );

                            if cfg.enable_tamper {
                                let tampered = tamper_signature(signature);
                                if tampered.is_none() {
                                    obj.insert(
                                        "roundtrip_step3_error".to_string(),
                                        serde_json::Value::String(
                                            "TAMPER_NOT_POSSIBLE".to_string(),
                                        ),
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

                                    // Step3 (tamper verification) always goes to original provider
                                    let step3 = perform_request(
                                        &client,
                                        &target_url,
                                        headers.clone(),
                                        step3_body,
                                        true,
                                    )
                                    .await;

                                    let step3_signals =
                                        response::signals_from_text(&step3.raw_excerpt);
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

                    let step2 =
                        perform_request(&client, &target_url, headers.clone(), step2_body, true)
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
                    if let Some(usage_obj) =
                        step2.usage_json_value.as_ref().and_then(|v| v.as_object())
                    {
                        if let Some(v) = usage_obj
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_i64())
                        {
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
    let db_for_history = db.clone();
    let provider_id_for_history = provider.id;
    let request_json_text = sanitized_request_text;
    let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
    let _ = blocking::run("claude_validation_history_insert", move || {
        claude_model_validation_history::insert_run_and_prune(
            &db_for_history,
            provider_id_for_history,
            &request_json_text,
            &result_json,
            Some(50),
        )?;
        Ok(())
    })
    .await;

    Ok(result)
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
        assert_eq!(
            thinking.get("type").and_then(|v| v.as_str()),
            Some("thinking")
        );
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
