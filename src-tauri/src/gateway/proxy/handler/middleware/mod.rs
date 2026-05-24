//! Middleware chain for gateway proxy handler.
//!
//! Each middleware processes a `ProxyContext` and returns a `MiddlewareAction`:
//! - `Continue(ctx)`: pass the (possibly enriched) context to the next middleware.
//! - `ShortCircuit(Response)`: return a response immediately, skipping remaining middlewares.

pub(super) mod billing_header_rectifier;
pub(super) mod body_reader;
pub(super) mod cli_proxy_guard;
pub(super) mod codex_session_completion;
pub(super) mod model_inference;
pub(super) mod probe_interceptor;
pub(super) mod provider_resolution;
pub(super) mod recursion_guard;
pub(super) mod request_fingerprint;
pub(super) mod runtime_settings_reader;
pub(super) mod warmup_interceptor;

pub(super) use billing_header_rectifier::BillingHeaderRectifierMiddleware;
pub(super) use body_reader::BodyReaderMiddleware;
pub(super) use cli_proxy_guard::CliProxyGuardMiddleware;
pub(super) use codex_session_completion::CodexSessionCompletionMiddleware;
pub(super) use model_inference::ModelInferenceMiddleware;
pub(super) use probe_interceptor::ProbeInterceptorMiddleware;
pub(super) use provider_resolution::ProviderResolutionMiddleware;
pub(super) use recursion_guard::RecursionGuardMiddleware;
pub(super) use request_fingerprint::RequestFingerprintMiddleware;
pub(super) use runtime_settings_reader::RuntimeSettingsMiddleware;
pub(super) use warmup_interceptor::WarmupInterceptorMiddleware;

use crate::gateway::proxy::request_context::RequestContextParts;
use crate::gateway::runtime::GatewayAppState;
use crate::gateway::util::RequestedModelLocation;
use crate::providers;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method};
use axum::response::Response;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Result of a middleware step: continue processing or return early.
pub(super) enum MiddlewareAction<R: tauri::Runtime = tauri::Wry> {
    Continue(Box<ProxyContext<R>>),
    ShortCircuit(Response),
}

/// Accumulated state that flows through the middleware chain.
///
/// Fields are progressively populated by each middleware. The context starts
/// with only the minimal request information and gains richer data as it passes
/// through the chain.
pub(super) struct ProxyContext<R: tauri::Runtime = tauri::Wry> {
    // -- immutable request metadata (set at construction) --
    pub(super) state: GatewayAppState<R>,
    pub(super) cli_key: String,
    pub(super) forwarded_path: String,
    pub(super) req_method: Method,
    pub(super) method_hint: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) is_claude_count_tokens: bool,

    // -- mutable request data (enriched by middlewares) --
    pub(super) request_body: Option<Body>,
    pub(super) headers: HeaderMap,
    pub(super) body_bytes: Bytes,
    pub(super) introspection_json: Option<serde_json::Value>,
    pub(super) observe_request: bool,
    pub(super) strip_request_content_encoding_seed: bool,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,

    // -- model inference results --
    pub(super) requested_model: Option<String>,
    pub(super) requested_model_location: Option<RequestedModelLocation>,

    // -- runtime settings (populated after settings read) --
    pub(super) runtime_settings: Option<super::runtime_settings::HandlerRuntimeSettings>,

    // -- session routing --
    pub(super) session_id: Option<String>,
    pub(super) allow_session_reuse: bool,

    // -- provider resolution --
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) providers: Vec<providers::ProviderForGateway>,
    pub(super) session_bound_provider_id: Option<i64>,
    pub(super) forced_provider_id: Option<i64>,

    // -- request fingerprinting --
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
}

impl<R: tauri::Runtime> ProxyContext<R> {
    /// Build the `RequestContextParts` needed by the forwarder, consuming this context.
    pub(super) fn into_request_context_parts(self) -> RequestContextParts<R> {
        let rs = self
            .runtime_settings
            .expect("runtime_settings must be populated before forwarding");

        RequestContextParts {
            state: self.state,
            cli_key: self.cli_key,
            forwarded_path: self.forwarded_path,
            observe_request: self.observe_request,
            req_method: self.req_method,
            method_hint: self.method_hint,
            query: self.query,
            trace_id: self.trace_id,
            started: self.started,
            created_at_ms: self.created_at_ms,
            created_at: self.created_at,
            session_id: self.session_id,
            requested_model: self.requested_model,
            requested_model_location: self.requested_model_location,
            effective_sort_mode_id: self.effective_sort_mode_id,
            providers: self.providers,
            session_bound_provider_id: self.session_bound_provider_id,
            headers: self.headers,
            body_bytes: self.body_bytes,
            introspection_json: self.introspection_json,
            strip_request_content_encoding_seed: self.strip_request_content_encoding_seed,
            special_settings: self.special_settings,
            provider_base_url_ping_cache_ttl_seconds: rs.provider_base_url_ping_cache_ttl_seconds,
            verbose_provider_error: rs.verbose_provider_error,
            enable_codex_session_id_completion: rs.enable_codex_session_id_completion,
            max_attempts_per_provider: rs.max_attempts_per_provider,
            max_providers_to_try: rs.max_providers_to_try,
            provider_cooldown_secs: rs.provider_cooldown_secs,
            upstream_first_byte_timeout_secs: rs.upstream_first_byte_timeout_secs,
            upstream_stream_idle_timeout_secs: rs.upstream_stream_idle_timeout_secs,
            upstream_request_timeout_non_streaming_secs: rs
                .upstream_request_timeout_non_streaming_secs,
            fingerprint_key: self.fingerprint_key,
            fingerprint_debug: self.fingerprint_debug,
            unavailable_fingerprint_key: self.unavailable_fingerprint_key,
            unavailable_fingerprint_debug: self.unavailable_fingerprint_debug,
            enable_thinking_signature_rectifier: rs.enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier: rs.enable_thinking_budget_rectifier,
            enable_claude_metadata_user_id_injection: rs.enable_claude_metadata_user_id_injection,
            cx2cc_settings: rs.cx2cc_settings,
            enable_response_fixer: rs.enable_response_fixer,
            response_fixer_stream_config: rs.response_fixer_stream_config,
            response_fixer_non_stream_config: rs.response_fixer_non_stream_config,
        }
    }
}
