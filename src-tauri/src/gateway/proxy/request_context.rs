//! Usage: Request context (SSOT) for gateway proxy forwarding.

use super::abort_guard::RequestAbortGuard;
use crate::gateway::manager::GatewayAppState;
use crate::gateway::response_fixer;
use crate::gateway::util::{strip_hop_headers, RequestedModelLocation};
use crate::infra::settings::MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS;
use crate::providers;
use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue, Method};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) struct RequestContext {
    pub(super) state: GatewayAppState,
    pub(super) cli_key: String,
    pub(super) forwarded_path: String,
    pub(super) observe_request: bool,
    pub(super) req_method: Method,
    pub(super) method_hint: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) requested_model_location: Option<RequestedModelLocation>,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) providers: Vec<providers::ProviderForGateway>,
    pub(super) session_bound_provider_id: Option<i64>,
    pub(super) base_headers: HeaderMap,
    pub(super) body_bytes: Bytes,
    pub(super) introspection_json: Option<serde_json::Value>,
    pub(super) strip_request_content_encoding_seed: bool,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_base_url_ping_cache_ttl_seconds: u32,
    pub(super) verbose_provider_error: bool,
    pub(super) enable_codex_session_id_completion: bool,
    pub(super) max_attempts_per_provider: u32,
    pub(super) max_providers_to_try: u32,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_first_byte_timeout: Option<Duration>,
    pub(super) upstream_stream_idle_timeout: Option<Duration>,
    pub(super) upstream_request_timeout_non_streaming: Option<Duration>,
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
    pub(super) abort_guard: RequestAbortGuard,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) enable_claude_metadata_user_id_injection: bool,
    #[allow(dead_code)]
    pub(super) cx2cc_settings: super::cx2cc::settings::Cx2ccSettings,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
}

impl RequestContext {
    pub(super) fn from_handler_parts(parts: RequestContextParts) -> Self {
        let RequestContextParts {
            state,
            cli_key,
            forwarded_path,
            observe_request,
            req_method,
            method_hint,
            query,
            trace_id,
            started,
            created_at_ms,
            created_at,
            session_id,
            requested_model,
            requested_model_location,
            effective_sort_mode_id,
            providers,
            session_bound_provider_id,
            headers,
            body_bytes,
            introspection_json,
            strip_request_content_encoding_seed,
            special_settings,
            provider_base_url_ping_cache_ttl_seconds,
            verbose_provider_error,
            enable_codex_session_id_completion,
            max_attempts_per_provider,
            max_providers_to_try,
            provider_cooldown_secs,
            upstream_first_byte_timeout_secs,
            upstream_stream_idle_timeout_secs,
            upstream_request_timeout_non_streaming_secs,
            fingerprint_key,
            fingerprint_debug,
            unavailable_fingerprint_key,
            unavailable_fingerprint_debug,
            enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier,
            enable_claude_metadata_user_id_injection,
            cx2cc_settings,
            enable_response_fixer,
            response_fixer_stream_config,
            response_fixer_non_stream_config,
        } = parts;

        let max_attempts_per_provider = Self::normalize_max_attempts_per_provider(
            &cli_key,
            enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier,
            max_attempts_per_provider,
        );
        let (
            upstream_first_byte_timeout,
            upstream_stream_idle_timeout,
            upstream_request_timeout_non_streaming,
        ) = Self::upstream_timeouts_from_secs(
            upstream_first_byte_timeout_secs,
            upstream_stream_idle_timeout_secs,
            upstream_request_timeout_non_streaming_secs,
        );

        let abort_guard = RequestAbortGuard::new(
            state.app.clone(),
            state.db.clone(),
            state.log_tx.clone(),
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            observe_request,
            query.clone(),
            session_id.clone(),
            requested_model.clone(),
            created_at_ms,
            created_at,
            started,
        );

        let base_headers = build_base_headers(headers);

        Self {
            state,
            cli_key,
            forwarded_path,
            observe_request,
            req_method,
            method_hint,
            query,
            trace_id,
            started,
            created_at_ms,
            created_at,
            session_id,
            requested_model,
            requested_model_location,
            effective_sort_mode_id,
            providers,
            session_bound_provider_id,
            base_headers,
            body_bytes,
            introspection_json,
            strip_request_content_encoding_seed,
            special_settings,
            provider_base_url_ping_cache_ttl_seconds,
            verbose_provider_error,
            enable_codex_session_id_completion,
            max_attempts_per_provider,
            max_providers_to_try,
            provider_cooldown_secs,
            upstream_first_byte_timeout_secs,
            upstream_first_byte_timeout,
            upstream_stream_idle_timeout,
            upstream_request_timeout_non_streaming,
            fingerprint_key,
            fingerprint_debug,
            unavailable_fingerprint_key,
            unavailable_fingerprint_debug,
            abort_guard,
            enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier,
            enable_claude_metadata_user_id_injection,
            cx2cc_settings,
            enable_response_fixer,
            response_fixer_stream_config,
            response_fixer_non_stream_config,
        }
    }

    fn normalize_max_attempts_per_provider(
        cli_key: &str,
        enable_thinking_signature_rectifier: bool,
        enable_thinking_budget_rectifier: bool,
        max_attempts_per_provider: u32,
    ) -> u32 {
        if cli_key == "claude"
            && (enable_thinking_signature_rectifier || enable_thinking_budget_rectifier)
        {
            max_attempts_per_provider.max(2)
        } else {
            max_attempts_per_provider
        }
    }

    fn upstream_timeouts_from_secs(
        upstream_first_byte_timeout_secs: u32,
        upstream_stream_idle_timeout_secs: u32,
        upstream_request_timeout_non_streaming_secs: u32,
    ) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
        // Values below 60s cause premature stream disconnects during long AI thinking phases.
        let clamped_idle = if upstream_stream_idle_timeout_secs > 0 {
            upstream_stream_idle_timeout_secs.max(MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS)
        } else {
            0
        };
        (
            duration_from_secs(upstream_first_byte_timeout_secs),
            duration_from_secs(clamped_idle),
            duration_from_secs(upstream_request_timeout_non_streaming_secs),
        )
    }
}

fn build_base_headers(mut headers: HeaderMap) -> HeaderMap {
    strip_hop_headers(&mut headers);
    headers.remove(header::HOST);
    headers.remove(header::CONTENT_LENGTH);
    headers.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("identity"),
    );
    headers
}

fn duration_from_secs(secs: u32) -> Option<Duration> {
    if secs == 0 {
        None
    } else {
        Some(Duration::from_secs(secs as u64))
    }
}

pub(super) struct RequestContextParts {
    pub(super) state: GatewayAppState,
    pub(super) cli_key: String,
    pub(super) forwarded_path: String,
    pub(super) observe_request: bool,
    pub(super) req_method: Method,
    pub(super) method_hint: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) requested_model_location: Option<RequestedModelLocation>,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) providers: Vec<providers::ProviderForGateway>,
    pub(super) session_bound_provider_id: Option<i64>,
    pub(super) headers: HeaderMap,
    pub(super) body_bytes: Bytes,
    pub(super) introspection_json: Option<serde_json::Value>,
    pub(super) strip_request_content_encoding_seed: bool,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_base_url_ping_cache_ttl_seconds: u32,
    pub(super) verbose_provider_error: bool,
    pub(super) enable_codex_session_id_completion: bool,
    pub(super) max_attempts_per_provider: u32,
    pub(super) max_providers_to_try: u32,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_stream_idle_timeout_secs: u32,
    pub(super) upstream_request_timeout_non_streaming_secs: u32,
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
    pub(super) enable_thinking_signature_rectifier: bool,
    pub(super) enable_thinking_budget_rectifier: bool,
    pub(super) enable_claude_metadata_user_id_injection: bool,
    pub(super) cx2cc_settings: super::cx2cc::settings::Cx2ccSettings,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
}
