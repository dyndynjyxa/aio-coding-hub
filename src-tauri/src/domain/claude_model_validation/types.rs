//! Usage: Types for Claude model validation (workflow I/O and internal DTOs).

use serde::Serialize;

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
pub(super) struct ProviderForValidation {
    pub(super) id: i64,
    pub(super) cli_key: String,
    pub(super) name: String,
    pub(super) base_urls: Vec<String>,
    pub(super) api_key_plaintext: String,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedRequest {
    pub(super) request_value: serde_json::Value,
    pub(super) headers: serde_json::Map<String, serde_json::Value>,
    pub(super) body: serde_json::Value,
    pub(super) expect_max_output_chars: Option<usize>,
    pub(super) expect_exact_output_chars: Option<usize>,
    pub(super) forwarded_path: String,
    pub(super) forwarded_query: Option<String>,
    pub(super) roundtrip: Option<RoundtripConfig>,
}

#[derive(Debug, Clone)]
pub(super) struct SignatureRoundtripConfig {
    pub(super) enable_tamper: bool,
    pub(super) step2_user_prompt: Option<String>,
    /// If set, Step3 will be sent to this provider (Step2 remains on the original provider).
    /// This enables cross-provider signature validation.
    pub(super) cross_provider_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct CacheRoundtripConfig {
    pub(super) step2_user_prompt: Option<String>,
    /// If true, force padding to ensure cache creation (min 5000 tokens).
    pub(super) force_padding: bool,
}

#[derive(Debug, Clone)]
pub(super) enum RoundtripConfig {
    Signature(SignatureRoundtripConfig),
    Cache(CacheRoundtripConfig),
}

#[derive(Debug, Clone)]
pub(super) struct StepOutcome {
    pub(super) ok: bool,
    pub(super) status: Option<u16>,
    pub(super) duration_ms: i64,
    pub(super) responded_model: Option<String>,
    pub(super) usage_json_value: Option<serde_json::Value>,
    pub(super) output_text_chars: usize,
    pub(super) output_text_preview: String,
    pub(super) thinking_block_seen: bool,
    pub(super) thinking_chars: usize,
    pub(super) thinking_preview: String,
    pub(super) signature_chars: usize,
    pub(super) thinking_full: String,
    pub(super) signature_full: String,
    pub(super) signature_from_delta: bool,
    pub(super) sse_message_delta_seen: bool,
    pub(super) sse_message_delta_stop_reason: Option<String>,
    pub(super) sse_message_delta_stop_reason_is_max_tokens: bool,
    pub(super) sse_error_event_seen: bool,
    pub(super) sse_error_status: Option<u16>,
    pub(super) sse_error_message: String,
    pub(super) response_id: Option<String>,
    pub(super) service_tier: Option<String>,
    pub(super) response_headers: serde_json::Value,
    pub(super) raw_excerpt: String,
    pub(super) response_parse_mode: String,
    pub(super) response_content_type: String,
    pub(super) response_bytes_truncated: bool,
    pub(super) stream_read_error: Option<String>,
    pub(super) error: Option<String>,
    pub(super) total_read: usize,
}

impl StepOutcome {
    /// Create an error outcome with a parse mode and error message.
    pub(super) fn error(started: std::time::Instant, parse_mode: &str, error: String) -> Self {
        Self {
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
            sse_error_event_seen: false,
            sse_error_status: None,
            sse_error_message: String::new(),
            response_id: None,
            service_tier: None,
            response_headers: serde_json::json!({}),
            raw_excerpt: String::new(),
            response_parse_mode: parse_mode.to_string(),
            response_content_type: String::new(),
            response_bytes_truncated: false,
            stream_read_error: None,
            error: Some(error),
            total_read: 0,
        }
    }
}
