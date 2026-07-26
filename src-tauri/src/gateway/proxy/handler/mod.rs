//! Usage: Gateway proxy handler implementation (request forwarding + failover + circuit breaker + logging).
//!
//! The handler is organized as a middleware chain. Each middleware in the `middleware/`
//! directory processes a `ProxyContext` and either continues to the next step or
//! short-circuits with a Response.

use super::abort_guard::RequestAbortGuard;
use super::logging::enqueue_request_log_placeholder;
use super::request_context::RequestContext;
use super::{is_claude_count_tokens_request, is_codex_model_discovery_request};

use crate::gateway::active_requests::ActiveRequestStart;
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
    CodexRequestClassifierMiddleware, CodexSessionCompletionMiddleware,
    Cx2ccCountTokensInterceptorMiddleware, MiddlewareAction, ModelInferenceMiddleware,
    ProbeInterceptorMiddleware, ProviderResolutionMiddleware, ProxyContext,
    RecursionGuardMiddleware, RequestFingerprintMiddleware, RuntimeSettingsMiddleware,
    WarmupInterceptorMiddleware,
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
        last_activity_ms: None,
        activity_details_json: None,
        created_at: ctx.created_at,
        usage_metrics: None,
        usage: None,
        provider_chain_json: None,
        error_details_json: None,
    })
}

fn register_active_request_from_proxy_context<R: tauri::Runtime>(
    ctx: &middleware::ProxyContext<R>,
) {
    if !ctx.observe_request {
        return;
    }

    ctx.state.active_requests.register(ActiveRequestStart {
        trace_id: ctx.trace_id.clone(),
        cli_key: ctx.cli_key.clone(),
        method: ctx.method_hint.clone(),
        path: ctx.forwarded_path.clone(),
        query: ctx.query.clone(),
        session_id: ctx.session_id.clone(),
        requested_model: ctx.requested_model.clone(),
        created_at_ms: ctx.created_at_ms,
    });
}

// Armed guard for the post-chain stretch: it must exist BEFORE the active
// request is registered and before any await, so that cancellation at any
// later await point (e.g. the placeholder enqueue) still finishes the
// registry entry and persists a client_abort terminal row via Drop.
fn abort_guard_from_proxy_context<R: tauri::Runtime>(
    ctx: &middleware::ProxyContext<R>,
) -> RequestAbortGuard<R> {
    RequestAbortGuard::new(
        ctx.state.app.clone(),
        ctx.state.db.clone(),
        ctx.state.log_tx.clone(),
        ctx.state.plugin_pipeline.clone(),
        ctx.state.active_requests.clone(),
        ctx.trace_id.clone(),
        ctx.cli_key.clone(),
        ctx.method_hint.clone(),
        ctx.forwarded_path.clone(),
        ctx.observe_request,
        ctx.query.clone(),
        ctx.session_id.clone(),
        ctx.requested_model.clone(),
        Arc::clone(&ctx.special_settings),
        ctx.created_at_ms,
        ctx.created_at,
        ctx.started,
    )
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
    let is_codex_model_discovery =
        is_codex_model_discovery_request(&cli_key, &method, &forwarded_path);

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
        is_codex_model_discovery,
        request_body: Some(body),
        headers,
        body_bytes: Bytes::new(),
        request_body_state: None,
        introspection_json: None,
        observe_request: false,
        strip_request_content_encoding_seed: false,
        special_settings: new_special_settings(),
        provider_health_neutral: is_codex_model_discovery,
        requested_model: None,
        requested_model_location: None,
        is_compact_request: false,
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

    // 4. Codex request-origin classification.
    let ctx = match CodexRequestClassifierMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 5. Model inference (from path/query/JSON).
    let ctx = match ModelInferenceMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 6. Probe interceptor (Claude probe requests).
    let ctx = match ProbeInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 7. Runtime settings reader.
    let ctx = match RuntimeSettingsMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 8. Warmup interceptor (requires runtime_settings).
    let ctx = match WarmupInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 9. Codex session ID completion.
    let ctx = match CodexSessionCompletionMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 10. Billing header rectifier.
    let ctx = match BillingHeaderRectifierMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 11. Provider resolution (session routing + provider selection).
    let ctx = match ProviderResolutionMiddleware::run(ctx).await {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 12. CX2CC count_tokens compatibility.
    let ctx = match Cx2ccCountTokensInterceptorMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // 13. Request fingerprinting + recent error cache gate.
    let ctx = match RequestFingerprintMiddleware::run(ctx) {
        MiddlewareAction::Continue(ctx) => *ctx,
        MiddlewareAction::ShortCircuit(resp) => return resp,
    };

    // --- Post-chain: emit start event, seed in-progress log, then forward ---
    // 顺序契约：先武装 abort guard，再登记活跃注册表，之后才允许出现 await。
    // guard 未武装时登记后被取消（future 被丢弃）会让注册表条目永久泄漏，
    // 前端将永远显示一张"进行中"合成卡片。
    let abort_guard = abort_guard_from_proxy_context(&ctx);
    if ctx.observe_request {
        register_active_request_from_proxy_context(&ctx);
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
            "[REQ] trace_id={} cli_key={} method={} path={} model={}{}\n  headers={}\n  body({} bytes)={}",
            ctx.trace_id,
            ctx.cli_key,
            ctx.method_hint,
            ctx.forwarded_path,
            ctx.requested_model.as_deref().unwrap_or("-"),
            if ctx.is_compact_request {
                " kind=compact"
            } else {
                ""
            },
            redacted_headers_for_debug(&ctx.headers),
            ctx.body_bytes.len(),
            lossy_utf8_preview(&ctx.body_bytes, MAX_DEBUG_BODY_PREVIEW_BYTES),
        )
    });

    if let Some(args) = build_in_progress_request_log_args(&ctx) {
        enqueue_request_log_placeholder(&ctx.state.app, &ctx.state.db, &ctx.state.log_tx, args)
            .await;
    }

    super::forwarder::forward(RequestContext::from_handler_parts(
        ctx.into_request_context_parts(),
        abort_guard,
    ))
    .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::abort_guard_from_proxy_context;
    use super::early_error::{
        build_early_error_log_ctx, early_error_contract,
        respond_provider_selection_failed_with_spawn,
    };
    use super::middleware;
    use super::middleware::body_reader::body_too_large_message;
    use super::middleware::cli_proxy_guard::{
        cli_proxy_disabled_message, cli_proxy_guard_special_settings_json,
    };
    use super::middleware::provider_resolution::no_enabled_provider_message;
    use super::middleware::warmup_interceptor::{
        should_intercept_warmup_request, warmup_intercept_special_settings_json,
        warmup_log_usage_metrics,
    };
    use super::middleware::{CodexRequestClassifierMiddleware, MiddlewareAction};
    use super::provider_selection::resolve_session_routing_decision;
    use super::register_active_request_from_proxy_context;
    use super::request_fingerprint::build_request_fingerprints;
    use super::runtime_settings::handler_runtime_settings;
    use crate::gateway::active_requests::ActiveRequestRegistry;
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
    use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};
    use crate::gateway::proxy::{ProviderBaseUrlPingCache, RecentErrorCache};
    use crate::gateway::response_fixer;
    use crate::gateway::runtime::GatewayAppState;
    use crate::{circuit_breaker, db, session_manager, settings};
    use axum::body::{Body, Bytes};
    use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

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
            extension_values: vec![],
        }
    }

    fn provider_ids(items: &[crate::providers::ProviderForGateway]) -> Vec<i64> {
        items.iter().map(|item| item.id).collect()
    }

    fn active_request_test_state(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<crate::request_logs::RequestLogInsert>,
        active_requests: Arc<ActiveRequestRegistry>,
    ) -> GatewayAppState<tauri::test::MockRuntime> {
        GatewayAppState {
            app,
            db,
            log_tx,
            circuit: Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            session: Arc::new(session_manager::SessionManager::new()),
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: Arc::new(Mutex::new(RecentErrorCache::default())),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline: GatewayPluginPipeline::empty_shared(),
            active_requests,
        }
    }

    #[test]
    fn observed_proxy_context_registers_active_request() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db =
            crate::db::init_for_tests(&db_dir.path().join("handler-active.db")).expect("init db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(1);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        let ctx = middleware::ProxyContext {
            state: active_request_test_state(
                app.handle().clone(),
                db,
                log_tx,
                active_requests.clone(),
            ),
            cli_key: "claude".to_string(),
            forwarded_path: "/v1/messages".to_string(),
            req_method: Method::POST,
            method_hint: "POST".to_string(),
            query: Some("beta=1".to_string()),
            trace_id: "trace-start-active".to_string(),
            started: Instant::now(),
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            is_claude_count_tokens: false,
            is_codex_model_discovery: false,
            request_body: Some(Body::empty()),
            headers: HeaderMap::new(),
            body_bytes: Bytes::new(),
            request_body_state: None,
            introspection_json: None,
            observe_request: true,
            strip_request_content_encoding_seed: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            provider_health_neutral: false,
            requested_model: Some("claude-sonnet-4".to_string()),
            requested_model_location: None,
            is_compact_request: false,
            runtime_settings: None,
            session_id: Some("session-start".to_string()),
            allow_session_reuse: false,
            effective_sort_mode_id: None,
            providers: vec![],
            session_bound_provider_id: None,
            forced_provider_id: None,
            fingerprint_key: 0,
            fingerprint_debug: String::new(),
            unavailable_fingerprint_key: 0,
            unavailable_fingerprint_debug: String::new(),
        };

        register_active_request_from_proxy_context(&ctx);

        let snapshot = active_requests.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].trace_id, "trace-start-active");
        assert_eq!(snapshot[0].session_id.as_deref(), Some("session-start"));
        assert_eq!(
            snapshot[0].requested_model.as_deref(),
            Some("claude-sonnet-4")
        );
    }

    #[test]
    fn unobserved_proxy_context_does_not_register_active_request() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("handler-unobserved.db"))
            .expect("init db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(1);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        let mut ctx = middleware::ProxyContext {
            state: active_request_test_state(
                app.handle().clone(),
                db,
                log_tx,
                active_requests.clone(),
            ),
            cli_key: "claude".to_string(),
            forwarded_path: "/v1/messages".to_string(),
            req_method: Method::POST,
            method_hint: "POST".to_string(),
            query: None,
            trace_id: "trace-unobserved".to_string(),
            started: Instant::now(),
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            is_claude_count_tokens: false,
            is_codex_model_discovery: false,
            request_body: Some(Body::empty()),
            headers: HeaderMap::new(),
            body_bytes: Bytes::new(),
            request_body_state: None,
            introspection_json: None,
            observe_request: false,
            strip_request_content_encoding_seed: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            provider_health_neutral: false,
            requested_model: Some("claude-sonnet-4".to_string()),
            requested_model_location: None,
            is_compact_request: false,
            runtime_settings: None,
            session_id: None,
            allow_session_reuse: false,
            effective_sort_mode_id: None,
            providers: vec![],
            session_bound_provider_id: None,
            forced_provider_id: None,
            fingerprint_key: 0,
            fingerprint_debug: String::new(),
            unavailable_fingerprint_key: 0,
            unavailable_fingerprint_debug: String::new(),
        };

        register_active_request_from_proxy_context(&ctx);
        assert!(active_requests.snapshot().is_empty());

        ctx.observe_request = true;
        register_active_request_from_proxy_context(&ctx);

        assert_eq!(active_requests.snapshot().len(), 1);
    }

    fn observed_guard_test_proxy_context(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: crate::db::Db,
        log_tx: tokio::sync::mpsc::Sender<crate::request_logs::RequestLogInsert>,
        active_requests: Arc<ActiveRequestRegistry>,
        trace_id: &str,
    ) -> middleware::ProxyContext<tauri::test::MockRuntime> {
        middleware::ProxyContext {
            state: active_request_test_state(app, db, log_tx, active_requests),
            cli_key: "claude".to_string(),
            forwarded_path: "/v1/messages".to_string(),
            req_method: Method::POST,
            method_hint: "POST".to_string(),
            query: None,
            trace_id: trace_id.to_string(),
            started: Instant::now(),
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            is_claude_count_tokens: false,
            is_codex_model_discovery: false,
            request_body: Some(Body::empty()),
            headers: HeaderMap::new(),
            body_bytes: Bytes::new(),
            request_body_state: None,
            introspection_json: None,
            observe_request: true,
            strip_request_content_encoding_seed: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            provider_health_neutral: false,
            requested_model: Some("claude-sonnet-4".to_string()),
            requested_model_location: None,
            is_compact_request: false,
            runtime_settings: None,
            session_id: Some("session-guard".to_string()),
            allow_session_reuse: false,
            effective_sort_mode_id: None,
            providers: vec![],
            session_bound_provider_id: None,
            forced_provider_id: None,
            fingerprint_key: 0,
            fingerprint_debug: String::new(),
            unavailable_fingerprint_key: 0,
            unavailable_fingerprint_debug: String::new(),
        }
    }

    #[tokio::test]
    async fn codex_system_metadata_flows_to_terminal_and_provider_failure_logs() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("handler-system-marker.db"))
            .expect("init db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(1);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        let mut ctx = observed_guard_test_proxy_context(
            app.handle().clone(),
            db,
            log_tx,
            active_requests,
            "trace-system-marker",
        );
        let body = serde_json::json!({
            "model": "gpt-5.4-mini",
            "client_metadata": {
                "x-codex-turn-metadata": serde_json::json!({
                    "thread_source": "system"
                }).to_string()
            }
        });
        ctx.cli_key = "codex".to_string();
        ctx.forwarded_path = "/v1/responses".to_string();
        ctx.req_method = Method::POST;
        ctx.method_hint = "POST".to_string();
        ctx.body_bytes = Bytes::from(serde_json::to_vec(&body).expect("request body json"));
        ctx.introspection_json = Some(body);
        ctx.requested_model = Some("gpt-5.4-mini".to_string());
        let original_body = ctx.body_bytes.clone();

        let ctx = match CodexRequestClassifierMiddleware::run(ctx) {
            MiddlewareAction::Continue(ctx) => *ctx,
            MiddlewareAction::ShortCircuit(_) => panic!("classifier must not short circuit"),
        };
        assert_eq!(ctx.body_bytes, original_body);
        assert!(ctx.provider_health_neutral);

        let special_settings_json =
            response_fixer::special_settings_json(&ctx.special_settings).expect("system marker");
        let (log_args, _) =
            crate::gateway::proxy::RequestLogEnqueueArgs::from_proxy_request_end_parts(
                &ctx.trace_id,
                &ctx.cli_key,
                ctx.session_id.clone(),
                &ctx.method_hint,
                &ctx.forwarded_path,
                ctx.query.as_deref(),
                false,
                Some(special_settings_json),
                Some(200),
                None,
                10,
                Some(1),
                &[],
                Some("gpt-5.4-mini".to_string()),
                ctx.created_at_ms,
                ctx.created_at,
                None,
                None,
            );
        let settings: serde_json::Value = serde_json::from_str(
            log_args
                .special_settings_json
                .as_deref()
                .expect("terminal special settings"),
        )
        .expect("valid terminal special settings");

        assert!(!log_args.excluded_from_stats);
        assert!(settings.as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.get("type").and_then(serde_json::Value::as_str)
                    == Some("codex_system_request")
                    && entry
                        .get("threadSource")
                        .and_then(serde_json::Value::as_str)
                        == Some("system")
            })
        }));

        let log_ctx = build_early_error_log_ctx(&ctx);
        let response = respond_provider_selection_failed_with_spawn(
            &log_ctx,
            response_fixer::special_settings_json(&ctx.special_settings),
            ctx.session_id.clone(),
            ctx.requested_model.clone(),
            "provider selection failed".to_string(),
        );
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let failure_log = tokio::time::timeout(std::time::Duration::from_secs(5), log_rx.recv())
            .await
            .expect("provider failure log timeout")
            .expect("provider failure log");
        assert_eq!(failure_log.status, Some(500));
        assert_eq!(
            failure_log.error_code.as_deref(),
            Some(GatewayErrorCode::InternalError.as_str())
        );
        assert!(!failure_log.excluded_from_stats);
        assert_eq!(failure_log.requested_model.as_deref(), Some("gpt-5.4-mini"));
        assert_eq!(
            failure_log.special_settings_json,
            log_args.special_settings_json
        );
    }

    // 取消安全契约：guard 在登记前武装。handler future 在之后任意 await 点被
    // 取消（drop）时，guard 的 Drop 必须终结注册表条目，否则条目永久泄漏、
    // 前端永远显示"进行中"合成卡片。
    #[tokio::test]
    async fn dropping_armed_abort_guard_finishes_request_and_preserves_special_settings() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("handler-guard-drop.db"))
            .expect("init db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        let ctx = observed_guard_test_proxy_context(
            app.handle().clone(),
            db,
            log_tx,
            active_requests.clone(),
            "trace-guard-drop",
        );
        response_fixer::push_special_setting(
            &ctx.special_settings,
            serde_json::json!({
                "type": "codex_system_request",
                "threadSource": "system",
            }),
        );
        let expected_special_settings =
            response_fixer::special_settings_json(&ctx.special_settings);

        let guard = abort_guard_from_proxy_context(&ctx);
        register_active_request_from_proxy_context(&ctx);
        assert_eq!(active_requests.snapshot().len(), 1);

        drop(guard);

        assert!(active_requests.snapshot().is_empty());
        let abort_log = tokio::time::timeout(std::time::Duration::from_secs(5), log_rx.recv())
            .await
            .expect("abort log timeout")
            .expect("abort log");
        assert_eq!(
            abort_log.error_code.as_deref(),
            Some(GatewayErrorCode::RequestAborted.as_str())
        );
        assert_eq!(abort_log.special_settings_json, expected_special_settings);
    }

    #[test]
    fn dropping_disarmed_abort_guard_keeps_active_request_for_request_end() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&db_dir.path().join("handler-guard-disarm.db"))
            .expect("init db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        let ctx = observed_guard_test_proxy_context(
            app.handle().clone(),
            db,
            log_tx,
            active_requests.clone(),
            "trace-guard-disarm",
        );

        let mut guard = abort_guard_from_proxy_context(&ctx);
        register_active_request_from_proxy_context(&ctx);
        guard.disarm();

        drop(guard);

        // 正常完成路径由 request_end 负责终结，disarm 后的 guard 不得抢先移除。
        assert_eq!(active_requests.snapshot().len(), 1);
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

        let selection_failed = early_error_contract(EarlyErrorKind::ProviderSelectionFailed);
        assert_eq!(selection_failed.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            selection_failed.error_code,
            GatewayErrorCode::InternalError.as_str()
        );
        assert_eq!(
            selection_failed.error_category,
            Some(ErrorCategory::SystemError.as_str())
        );
        assert!(!selection_failed.excluded_from_stats);
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
        let runtime = handler_runtime_settings(None, false, false);

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

        let runtime = handler_runtime_settings(Some(&cfg), true, false);

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
    fn handler_runtime_settings_limits_codex_model_discovery_per_provider_only() {
        let cfg = settings::AppSettings {
            failover_max_attempts_per_provider: 9,
            failover_max_providers_to_try: 7,
            ..Default::default()
        };

        let runtime = handler_runtime_settings(Some(&cfg), false, true);

        assert_eq!(runtime.max_attempts_per_provider, 1);
        assert_eq!(runtime.max_providers_to_try, 7);
        assert!(runtime.enable_thinking_signature_rectifier);
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
