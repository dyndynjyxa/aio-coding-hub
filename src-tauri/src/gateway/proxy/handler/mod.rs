//! Usage: Gateway proxy handler implementation (request forwarding + failover + circuit breaker + logging).
//!
//! The handler is organized as a middleware chain. Each middleware in the `middleware/`
//! directory processes a `ProxyContext` and either continues to the next step or
//! short-circuits with a Response.

use super::is_claude_count_tokens_request;
use super::logging::enqueue_request_log_placeholder;
use super::request_context::RequestContext;

use crate::gateway::events::{emit_gateway_debug_log_lazy, emit_request_start_event};
use crate::gateway::proxy::should_seed_in_progress_request_log;
use crate::gateway::response_fixer;
use crate::gateway::util::{
    lossy_utf8_preview, new_trace_id, now_unix_millis, redacted_headers_for_debug,
    MAX_DEBUG_BODY_PREVIEW_BYTES,
};
use axum::{
    body::{Body, Bytes},
    http::Request,
    response::Response,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod early_error;
mod middleware;
mod provider_order;
mod provider_selection;
mod request_fingerprint;
mod runtime_settings;

use early_error::extract_forced_provider_id;
use middleware::{
    BillingHeaderRectifierMiddleware, BodyReaderMiddleware, CliProxyGuardMiddleware,
    CodexSessionCompletionMiddleware, Cx2ccCountTokensInterceptorMiddleware, MiddlewareAction,
    ModelInferenceMiddleware, ProbeInterceptorMiddleware, ProviderResolutionMiddleware,
    ProxyContext, RecursionGuardMiddleware, RequestFingerprintMiddleware,
    RuntimeSettingsMiddleware, WarmupInterceptorMiddleware,
};

type SpecialSettings = Arc<Mutex<Vec<serde_json::Value>>>;

fn new_special_settings() -> SpecialSettings {
    Arc::new(Mutex::new(Vec::new()))
}

// ---------------------------------------------------------------------------
// In-progress request log placeholder
// ---------------------------------------------------------------------------

fn build_in_progress_request_log_args<R: tauri::Runtime>(
    ctx: &middleware::ProxyContext<R>,
) -> Option<super::RequestLogEnqueueArgs> {
    if !should_seed_in_progress_request_log(&ctx.cli_key, &ctx.forwarded_path, ctx.observe_request)
    {
        return None;
    }

    Some(super::RequestLogEnqueueArgs {
        trace_id: ctx.trace_id.to_string(),
        cli_key: ctx.cli_key.to_string(),
        session_id: ctx.session_id.as_deref().map(str::to_string),
        method: ctx.method_hint.to_string(),
        path: ctx.forwarded_path.to_string(),
        query: ctx.query.as_deref().map(str::to_string),
        excluded_from_stats: false,
        special_settings_json: response_fixer::special_settings_json(&ctx.special_settings),
        status: None,
        error_code: None,
        duration_ms: 0,
        ttfb_ms: None,
        attempts_json: "[]".to_string(),
        requested_model: ctx.requested_model.as_deref().map(str::to_string),
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
        usage_metrics: None,
        usage: None,
        provider_chain_json: None,
        error_details_json: None,
    })
}

// ---------------------------------------------------------------------------
// Main entry point: middleware chain orchestrator
// ---------------------------------------------------------------------------

pub(in crate::gateway) async fn proxy_impl<R>(
    state: crate::gateway::runtime::GatewayAppState<R>,
    cli_key: String,
    forwarded_path: String,
    req: Request<Body>,
) -> Response
where
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let started = Instant::now();
    let trace_id = new_trace_id();
    let created_at_ms = now_unix_millis() as i64;
    let created_at = (created_at_ms / 1000).max(0);
    let method = req.method().clone();
    let method_hint = method.to_string();
    let query = req.uri().query().map(str::to_string);
    let is_claude_count_tokens = is_claude_count_tokens_request(&cli_key, &forwarded_path);

    let (headers, body) = {
        let (parts, b) = req.into_parts();
        (parts.headers, b)
    };

    let forced_provider_id = extract_forced_provider_id(&headers);

    // Build the initial context.
    let ctx = ProxyContext {
        state,
        cli_key,
        forwarded_path,
        req_method: method,
        method_hint,
        query,
        trace_id,
        started,
        created_at_ms,
        created_at,
        is_claude_count_tokens,
        request_body: Some(body),
        headers,
        body_bytes: Bytes::new(),
        request_body_state: None,
        introspection_json: None,
        observe_request: false,
        strip_request_content_encoding_seed: false,
        special_settings: new_special_settings(),
        requested_model: None,
        requested_model_location: None,
        runtime_settings: None,
        session_id: None,
        allow_session_reuse: false,
        effective_sort_mode_id: None,
        providers: vec![],
        session_bound_provider_id: None,
        forced_provider_id,
        fingerprint_key: 0,
        fingerprint_debug: String::new(),
        unavailable_fingerprint_key: 0,
        unavailable_fingerprint_debug: String::new(),
    };

    // --- Middleware chain ---
    // 1. Recursion guard (blocks recursive loops).
    let ctx = match RecursionGuardMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 2. CLI proxy guard (checks enable/disable per CLI key).
    let ctx = match CliProxyGuardMiddleware::run(ctx).await {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 3. Body reader + size validator.
    let ctx = match BodyReaderMiddleware::run(ctx).await {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 4. Model inference (from path/query/JSON).
    let ctx = match ModelInferenceMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 5. Probe interceptor (Claude probe requests).
    let ctx = match ProbeInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 6. Runtime settings reader.
    let ctx = match RuntimeSettingsMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 7. Warmup interceptor (requires runtime_settings).
    let ctx = match WarmupInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 8. Codex session ID completion.
    let ctx = match CodexSessionCompletionMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 9. Billing header rectifier.
    let ctx = match BillingHeaderRectifierMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 10. Provider resolution (session routing + provider selection).
    let ctx = match ProviderResolutionMiddleware::run(ctx).await {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 11. CX2CC count_tokens compatibility.
    let ctx = match Cx2ccCountTokensInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 12. Request fingerprinting + recent error cache gate.
    let ctx = match RequestFingerprintMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // --- Post-chain: emit start event, seed in-progress log, then forward ---
    if ctx.observe_request {
        emit_request_start_event(
            &ctx.state.app,
            ctx.trace_id.clone(),
            ctx.cli_key.clone(),
            ctx.session_id.clone(),
            ctx.method_hint.clone(),
            ctx.forwarded_path.clone(),
            ctx.query.clone(),
            ctx.requested_model.clone(),
            ctx.created_at,
        );
    }

    emit_gateway_debug_log_lazy(&ctx.state.app, || {
        format!(
            "[REQ] trace_id={} cli_key={} method={} path={} model={}\n  headers={}\n  body({} bytes)={}",
            ctx.trace_id,
            ctx.cli_key,
            ctx.method_hint,
            ctx.forwarded_path,
            ctx.requested_model.as_deref().unwrap_or("-"),
            redacted_headers_for_debug(&ctx.headers),
            ctx.body_bytes.len(),
            lossy_utf8_preview(&ctx.body_bytes, MAX_DEBUG_BODY_PREVIEW_BYTES),
        )
    });

    if let Some(args) = build_in_progress_request_log_args(&ctx) {
        enqueue_request_log_placeholder(&ctx.state.app, &ctx.state.log_tx, args).await;
    }

    super::forwarder::forward(RequestContext::from_handler_parts(
        ctx.into_request_context_parts(),
    ))
    .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::early_error::early_error_contract;
    use super::middleware::body_reader::body_too_large_message;
    use super::middleware::cli_proxy_guard::{
        cli_proxy_disabled_message, cli_proxy_guard_special_settings_json,
    };
    use super::middleware::provider_resolution::no_enabled_provider_message;
    use super::middleware::warmup_interceptor::{
        should_intercept_warmup_request, warmup_intercept_special_settings_json,
        warmup_log_usage_metrics,
    };
    use super::provider_selection::resolve_session_routing_decision;
    use super::request_fingerprint::build_request_fingerprints;
    use super::runtime_settings::handler_runtime_settings;
    use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};
    use crate::settings;
    use axum::body::Bytes;
    use axum::http::{HeaderMap, HeaderValue, StatusCode};

    fn provider(id: i64) -> crate::providers::ProviderForGateway {
        crate::providers::ProviderForGateway {
            id,
            name: format!("p{id}"),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: String::new(),
            claude_models: crate::providers::ClaudeModels::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: crate::providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
        }
    }

    fn provider_ids(items: &[crate::providers::ProviderForGateway]) -> Vec<i64> {
        items.iter().map(|item| item.id).collect()
    }

    #[test]
    fn cli_proxy_disabled_message_without_error_is_actionable() {
        let message = cli_proxy_disabled_message("claude", None);
        assert!(message.contains("CLI 代理未开启"));
        assert!(message.contains("claude"));
        assert!(message.contains("首页开启"));
    }

    #[test]
    fn cli_proxy_disabled_message_with_error_preserves_context() {
        let message = cli_proxy_disabled_message("codex", Some("manifest read failed"));
        assert!(message.contains("CLI 代理状态读取失败"));
        assert!(message.contains("manifest read failed"));
        assert!(message.contains("codex"));
    }

    #[test]
    fn cli_proxy_guard_special_settings_json_has_expected_shape() {
        let encoded = cli_proxy_guard_special_settings_json(false, 5000, Some("boom"));
        let value: serde_json::Value =
            serde_json::from_str(&encoded).expect("special settings should be valid json");

        let row = value
            .as_array()
            .and_then(|rows| rows.first())
            .expect("special settings should contain one object");

        assert_eq!(
            row.get("type").and_then(|v| v.as_str()),
            Some("cli_proxy_guard")
        );
        assert_eq!(row.get("scope").and_then(|v| v.as_str()), Some("request"));
        assert_eq!(row.get("hit").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(row.get("enabled").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(row.get("cacheHit").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(row.get("cacheTtlMs").and_then(|v| v.as_i64()), Some(5000));
        assert_eq!(row.get("error").and_then(|v| v.as_str()), Some("boom"));
    }

    #[test]
    fn early_error_contracts_match_expected_status_and_codes() {
        use super::early_error::EarlyErrorKind;

        let cli_proxy = early_error_contract(EarlyErrorKind::CliProxyDisabled);
        assert_eq!(cli_proxy.status, StatusCode::FORBIDDEN);
        assert_eq!(
            cli_proxy.error_code,
            GatewayErrorCode::CliProxyDisabled.as_str()
        );
        assert_eq!(
            cli_proxy.error_category,
            Some(ErrorCategory::NonRetryableClientError.as_str())
        );
        assert!(cli_proxy.excluded_from_stats);

        let body_too_large = early_error_contract(EarlyErrorKind::BodyTooLarge);
        assert_eq!(body_too_large.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            body_too_large.error_code,
            GatewayErrorCode::BodyTooLarge.as_str()
        );
        assert_eq!(body_too_large.error_category, None);
        assert!(!body_too_large.excluded_from_stats);

        let large_body_missing_model = early_error_contract(EarlyErrorKind::LargeBodyMissingModel);
        assert_eq!(large_body_missing_model.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            large_body_missing_model.error_code,
            GatewayErrorCode::LargeBodyMissingModel.as_str()
        );
        assert_eq!(large_body_missing_model.error_category, None);
        assert!(!large_body_missing_model.excluded_from_stats);

        let invalid_cli = early_error_contract(EarlyErrorKind::InvalidCliKey);
        assert_eq!(invalid_cli.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            invalid_cli.error_code,
            GatewayErrorCode::InvalidCliKey.as_str()
        );
        assert_eq!(invalid_cli.error_category, None);
        assert!(!invalid_cli.excluded_from_stats);

        let no_provider = early_error_contract(EarlyErrorKind::NoEnabledProvider);
        assert_eq!(no_provider.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            no_provider.error_code,
            GatewayErrorCode::NoEnabledProvider.as_str()
        );
        assert_eq!(no_provider.error_category, None);
        assert!(!no_provider.excluded_from_stats);
    }

    #[test]
    fn body_too_large_message_includes_prefix_and_error() {
        let message = body_too_large_message("stream exceeded limit", 64 * 1024 * 1024);
        assert!(message.contains("failed to read request body:"));
        assert!(message.contains("stream exceeded limit"));
        assert!(message.contains("64 MB"));
    }

    #[test]
    fn no_enabled_provider_message_preserves_cli_key() {
        let message = no_enabled_provider_message("codex");
        assert_eq!(message, "no enabled provider for cli_key=codex");
    }

    #[test]
    fn handler_runtime_settings_defaults_match_expected() {
        let runtime = handler_runtime_settings(None, false);

        assert!(runtime.verbose_provider_error);
        assert!(!runtime.intercept_warmup);
        assert!(runtime.enable_thinking_signature_rectifier);
        assert_eq!(runtime.cx2cc_settings.fallback_model_main, "gpt-5.4");
        assert!(runtime.cx2cc_settings.disable_response_storage);
        assert!(runtime.enable_response_fixer);
        assert_eq!(
            runtime.provider_base_url_ping_cache_ttl_seconds,
            settings::DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS
        );
        assert_eq!(runtime.max_attempts_per_provider, 5);
        assert_eq!(runtime.max_providers_to_try, 5);
        assert_eq!(
            runtime.provider_cooldown_secs,
            settings::DEFAULT_PROVIDER_COOLDOWN_SECONDS as i64
        );
        assert!(runtime.response_fixer_stream_config.fix_sse_format);
        assert!(!runtime.response_fixer_non_stream_config.fix_sse_format);
    }

    #[test]
    fn handler_runtime_settings_respects_count_tokens_override() {
        let cfg = settings::AppSettings {
            enable_thinking_signature_rectifier: true,
            failover_max_attempts_per_provider: 9,
            failover_max_providers_to_try: 7,
            cx2cc_fallback_model_main: "custom-main".to_string(),
            cx2cc_service_tier: "priority".to_string(),
            ..Default::default()
        };

        let runtime = handler_runtime_settings(Some(&cfg), true);

        assert!(!runtime.enable_thinking_signature_rectifier);
        assert_eq!(runtime.max_attempts_per_provider, 1);
        assert_eq!(runtime.max_providers_to_try, 1);
        assert_eq!(runtime.cx2cc_settings.fallback_model_main, "custom-main");
        assert_eq!(
            runtime.cx2cc_settings.service_tier.as_deref(),
            Some("priority")
        );
    }

    #[test]
    fn apply_session_reuse_binding_noop_when_reuse_disabled() {
        let mut providers = vec![provider(11), provider(22), provider(33)];

        let selected = super::provider_selection::apply_session_reuse_provider_binding(
            false,
            &mut providers,
            Some(22),
            Some(&[11, 22, 33]),
        );

        assert_eq!(selected, None);
        assert_eq!(provider_ids(&providers), vec![11, 22, 33]);
    }

    #[test]
    fn apply_session_reuse_binding_rotates_from_bound_provider_when_allowed() {
        let mut providers = vec![provider(11), provider(22), provider(33)];

        let selected = super::provider_selection::apply_session_reuse_provider_binding(
            true,
            &mut providers,
            Some(22),
            Some(&[11, 22, 33]),
        );

        assert_eq!(selected, Some(22));
        assert_eq!(provider_ids(&providers), vec![22, 33, 11]);
    }

    #[test]
    fn apply_session_reuse_binding_rotates_to_next_when_bound_missing() {
        let mut providers = vec![provider(10), provider(20), provider(30)];

        let selected = super::provider_selection::apply_session_reuse_provider_binding(
            true,
            &mut providers,
            Some(99),
            Some(&[99, 30, 20]),
        );

        assert_eq!(selected, None);
        assert_eq!(provider_ids(&providers), vec![30, 10, 20]);
    }

    #[test]
    fn warmup_intercept_special_settings_json_has_expected_shape() {
        let encoded = warmup_intercept_special_settings_json();
        let value: serde_json::Value =
            serde_json::from_str(&encoded).expect("warmup special settings should be valid json");

        let row = value
            .as_array()
            .and_then(|rows| rows.first())
            .expect("warmup special settings should contain one object");

        assert_eq!(
            row.get("type").and_then(|v| v.as_str()),
            Some("warmup_intercept")
        );
        assert_eq!(row.get("scope").and_then(|v| v.as_str()), Some("request"));
        assert_eq!(row.get("hit").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            row.get("reason").and_then(|v| v.as_str()),
            Some("anthropic_warmup_intercepted")
        );
    }

    #[test]
    fn warmup_log_usage_metrics_sets_all_zero_tokens() {
        let usage = warmup_log_usage_metrics();

        assert_eq!(usage.input_tokens, Some(0));
        assert_eq!(usage.output_tokens, Some(0));
        assert_eq!(usage.total_tokens, Some(0));
        assert_eq!(usage.cache_read_input_tokens, Some(0));
        assert_eq!(usage.cache_creation_input_tokens, Some(0));
        assert_eq!(usage.cache_creation_5m_input_tokens, Some(0));
        assert_eq!(usage.cache_creation_1h_input_tokens, Some(0));
    }

    #[test]
    fn should_intercept_warmup_request_detects_valid_claude_warmup() {
        let body = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "warmup",
                            "cache_control": {"type": "ephemeral"}
                        }
                    ]
                }
            ]
        });

        let hit = should_intercept_warmup_request("claude", true, "/v1/messages", Some(&body));

        assert!(hit);
    }

    #[test]
    fn should_intercept_warmup_request_rejects_non_claude_cli() {
        let body = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "warmup",
                            "cache_control": {"type": "ephemeral"}
                        }
                    ]
                }
            ]
        });

        let hit = should_intercept_warmup_request("codex", true, "/v1/messages", Some(&body));

        assert!(!hit);
    }

    #[test]
    fn resolve_session_routing_decision_disables_for_count_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "session_id",
            HeaderValue::from_static("sess-count-token-123"),
        );
        let body = serde_json::json!({
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": "hello"}
            ]
        });

        let decision = resolve_session_routing_decision(&headers, Some(&body), true);

        assert_eq!(decision.session_id, None);
        assert!(!decision.allow_session_reuse);
    }

    #[test]
    fn resolve_session_routing_decision_extracts_session_and_reuse() {
        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", HeaderValue::from_static("sess-normal-456"));
        let body = serde_json::json!({
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": "hello"}
            ]
        });

        let decision = resolve_session_routing_decision(&headers, Some(&body), false);

        assert_eq!(decision.session_id.as_deref(), Some("sess-normal-456"));
        assert!(decision.allow_session_reuse);
    }

    #[test]
    fn extract_forced_provider_id_reads_positive_integer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-aio-provider-id", HeaderValue::from_static("12"));
        assert_eq!(
            super::early_error::extract_forced_provider_id(&headers),
            Some(12)
        );
    }

    #[test]
    fn extract_forced_provider_id_rejects_invalid_or_non_positive_values() {
        let mut headers = HeaderMap::new();
        headers.insert("x-aio-provider-id", HeaderValue::from_static("0"));
        assert_eq!(
            super::early_error::extract_forced_provider_id(&headers),
            None
        );

        headers.insert("x-aio-provider-id", HeaderValue::from_static("-1"));
        assert_eq!(
            super::early_error::extract_forced_provider_id(&headers),
            None
        );

        headers.insert("x-aio-provider-id", HeaderValue::from_static("abc"));
        assert_eq!(
            super::early_error::extract_forced_provider_id(&headers),
            None
        );
    }

    #[test]
    fn force_provider_if_requested_keeps_only_selected_provider() {
        let mut providers = vec![provider(1), provider(2), provider(3)];
        let special_settings = super::new_special_settings();

        super::early_error::force_provider_if_requested(&mut providers, Some(2), &special_settings);

        assert_eq!(provider_ids(&providers), vec![2]);
    }

    #[test]
    fn force_provider_if_requested_clears_when_selected_provider_missing() {
        let mut providers = vec![provider(1), provider(2), provider(3)];
        let special_settings = super::new_special_settings();

        super::early_error::force_provider_if_requested(
            &mut providers,
            Some(99),
            &special_settings,
        );

        assert!(providers.is_empty());
    }

    #[test]
    fn request_fingerprint_ignores_session_when_idempotency_key_present() {
        let mut headers = HeaderMap::new();
        headers.insert("idempotency-key", HeaderValue::from_static("idem-123"));
        let body = Bytes::from_static(br#"{"model":"claude-3-5-sonnet"}"#);

        let left = build_request_fingerprints(
            "claude",
            Some(11),
            "POST",
            "/v1/messages",
            Some("stream=true&model=claude-3-5-sonnet"),
            Some("session-a"),
            Some("claude-3-5-sonnet"),
            &headers,
            &body,
        );
        let right = build_request_fingerprints(
            "claude",
            Some(11),
            "POST",
            "/v1/messages",
            Some("model=claude-3-5-sonnet&stream=true"),
            Some("session-b"),
            Some("claude-3-5-sonnet"),
            &headers,
            &body,
        );

        assert_eq!(left.fingerprint_key, right.fingerprint_key);
        assert_eq!(left.fingerprint_debug, right.fingerprint_debug);
        assert_eq!(
            left.unavailable_fingerprint_key,
            right.unavailable_fingerprint_key
        );
        assert_eq!(
            left.unavailable_fingerprint_debug,
            right.unavailable_fingerprint_debug
        );
    }
}
