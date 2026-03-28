//! Usage: Gateway proxy handler implementation (request forwarding + failover + circuit breaker + logging).
//!
//! Note: this module is being split into smaller submodules under `handler/`.

use super::request_context::{RequestContext, RequestContextParts};
use super::request_end::{
    emit_request_event_and_enqueue_request_log, emit_request_event_and_spawn_request_log,
    RequestEndArgs, RequestEndDeps,
};
use super::{
    cli_proxy_guard::cli_proxy_enabled_cached, errors::error_response,
    is_claude_count_tokens_request, should_observe_request,
};
use super::{ErrorCategory, GatewayErrorCode};
use provider_selection::{
    resolve_session_bound_provider_id, resolve_session_routing_decision,
    select_providers_with_session_binding, ProviderSelection, SessionRoutingDecision,
};

use crate::shared::mutex_ext::MutexExt;
use crate::{settings, usage};
use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::codex_session_id;
use super::super::events::{decision_chain as dc, emit_gateway_log, emit_request_start_event};
use super::super::manager::GatewayAppState;
use super::super::response_fixer;
use super::super::util::{
    body_for_introspection, infer_requested_model_info, new_trace_id, now_unix_millis,
    MAX_REQUEST_BODY_BYTES,
};
use super::super::warmup;
use request_fingerprint::{apply_recent_error_cache_gate, build_request_fingerprints};

mod provider_order;
mod provider_selection;
mod request_fingerprint;

const DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 5;
const DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 5;

type SpecialSettings = Arc<Mutex<Vec<serde_json::Value>>>;

#[derive(Debug, Clone, Copy)]
enum EarlyErrorKind {
    CliProxyDisabled,
    BodyTooLarge,
    InvalidCliKey,
    NoEnabledProvider,
}

#[derive(Debug, Clone, Copy)]
struct EarlyErrorContract {
    status: StatusCode,
    error_code: &'static str,
    error_category: Option<&'static str>,
    excluded_from_stats: bool,
}

fn early_error_contract(kind: EarlyErrorKind) -> EarlyErrorContract {
    match kind {
        EarlyErrorKind::CliProxyDisabled => EarlyErrorContract {
            status: StatusCode::FORBIDDEN,
            error_code: GatewayErrorCode::CliProxyDisabled.as_str(),
            error_category: Some(ErrorCategory::NonRetryableClientError.as_str()),
            excluded_from_stats: true,
        },
        EarlyErrorKind::BodyTooLarge => EarlyErrorContract {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            error_code: GatewayErrorCode::BodyTooLarge.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
        EarlyErrorKind::InvalidCliKey => EarlyErrorContract {
            status: StatusCode::BAD_REQUEST,
            error_code: GatewayErrorCode::InvalidCliKey.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
        EarlyErrorKind::NoEnabledProvider => EarlyErrorContract {
            status: StatusCode::SERVICE_UNAVAILABLE,
            error_code: GatewayErrorCode::NoEnabledProvider.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
    }
}

fn body_too_large_message(err: &str) -> String {
    format!("failed to read request body: {err}")
}

fn no_enabled_provider_message(cli_key: &str) -> String {
    format!("no enabled provider for cli_key={cli_key}")
}

fn cli_proxy_disabled_message(cli_key: &str, error: Option<&str>) -> String {
    match error {
        Some(err) => format!(
            "CLI 代理状态读取失败（按未开启处理）：{err}；请在首页开启 {cli_key} 的 CLI 代理开关后重试"
        ),
        None => format!("CLI 代理未开启：请在首页开启 {cli_key} 的 CLI 代理开关后重试"),
    }
}

fn extract_forced_provider_id(headers: &axum::http::HeaderMap) -> Option<i64> {
    let raw = headers.get("x-aio-provider-id")?.to_str().ok()?.trim();
    let provider_id = raw.parse::<i64>().ok()?;
    (provider_id > 0).then_some(provider_id)
}

fn force_provider_if_requested(
    providers: &mut Vec<crate::providers::ProviderForGateway>,
    provider_id: Option<i64>,
    special_settings: &SpecialSettings,
) {
    let Some(provider_id) = provider_id else {
        return;
    };

    if let Some(index) = providers.iter().position(|p| p.id == provider_id) {
        if index > 0 {
            providers.rotate_left(index);
        }

        providers.truncate(1);

        push_special_setting(
            special_settings,
            serde_json::json!({
                "type": "provider_lock",
                "scope": "request",
                "hit": true,
                "providerId": provider_id,
            }),
        );
    } else {
        providers.clear();
    }
}

fn cli_proxy_guard_special_settings_json(
    cache_hit: bool,
    cache_ttl_ms: i64,
    error: Option<&str>,
) -> String {
    serde_json::json!([{
        "type": "cli_proxy_guard",
        "scope": "request",
        "hit": true,
        "enabled": false,
        "cacheHit": cache_hit,
        "cacheTtlMs": cache_ttl_ms,
        "error": error,
    }])
    .to_string()
}

fn new_special_settings() -> SpecialSettings {
    Arc::new(Mutex::new(Vec::new()))
}

fn push_special_setting(special_settings: &SpecialSettings, setting: serde_json::Value) {
    let mut settings = special_settings.lock_or_recover();
    settings.push(setting);
}

struct EarlyErrorLogCtx<'a> {
    state: &'a GatewayAppState,
    trace_id: &'a str,
    cli_key: &'a str,
    method_hint: &'a str,
    forwarded_path: &'a str,
    query: Option<&'a str>,
    duration_ms: u128,
    created_at_ms: i64,
    created_at: i64,
}

impl<'a> EarlyErrorLogCtx<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        state: &'a GatewayAppState,
        trace_id: &'a str,
        cli_key: &'a str,
        method_hint: &'a str,
        forwarded_path: &'a str,
        query: Option<&'a str>,
        duration_ms: u128,
        created_at_ms: i64,
        created_at: i64,
    ) -> Self {
        Self {
            state,
            trace_id,
            cli_key,
            method_hint,
            forwarded_path,
            query,
            duration_ms,
            created_at_ms,
            created_at,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_early_error_log_ctx<'a>(
    state: &'a GatewayAppState,
    started: &Instant,
    trace_id: &'a str,
    cli_key: &'a str,
    method_hint: &'a str,
    forwarded_path: &'a str,
    query: Option<&'a str>,
    created_at_ms: i64,
    created_at: i64,
) -> EarlyErrorLogCtx<'a> {
    EarlyErrorLogCtx::new(
        state,
        trace_id,
        cli_key,
        method_hint,
        forwarded_path,
        query,
        started.elapsed().as_millis(),
        created_at_ms,
        created_at,
    )
}

fn build_early_error_response(
    trace_id: &str,
    contract: EarlyErrorContract,
    message: String,
) -> Response {
    error_response(
        contract.status,
        trace_id.to_string(),
        contract.error_code,
        message,
        vec![],
    )
}

fn early_error_request_end_args<'a>(
    ctx: &'a EarlyErrorLogCtx<'a>,
    contract: EarlyErrorContract,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> RequestEndArgs<'a> {
    RequestEndArgs {
        deps: RequestEndDeps::new(&ctx.state.app, &ctx.state.db, &ctx.state.log_tx),
        trace_id: ctx.trace_id,
        cli_key: ctx.cli_key,
        method: ctx.method_hint,
        path: ctx.forwarded_path,
        query: ctx.query,
        excluded_from_stats: contract.excluded_from_stats,
        status: Some(contract.status.as_u16()),
        error_category: contract.error_category,
        error_code: Some(contract.error_code),
        duration_ms: ctx.duration_ms,
        event_ttfb_ms: None,
        log_ttfb_ms: None,
        attempts: &[],
        special_settings_json,
        session_id,
        requested_model,
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
        usage_metrics: None,
        log_usage_metrics: None,
        usage: None,
    }
}

async fn respond_early_error_with_enqueue(
    ctx: &EarlyErrorLogCtx<'_>,
    contract: EarlyErrorContract,
    message: String,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> Response {
    let resp = build_early_error_response(ctx.trace_id, contract, message);
    emit_request_event_and_enqueue_request_log(early_error_request_end_args(
        ctx,
        contract,
        special_settings_json,
        session_id,
        requested_model,
    ))
    .await;
    resp
}

fn respond_early_error_with_spawn(
    ctx: &EarlyErrorLogCtx<'_>,
    contract: EarlyErrorContract,
    message: String,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> Response {
    let resp = build_early_error_response(ctx.trace_id, contract, message);
    emit_request_event_and_spawn_request_log(early_error_request_end_args(
        ctx,
        contract,
        special_settings_json,
        session_id,
        requested_model,
    ));
    resp
}

fn respond_invalid_cli_key_with_spawn(
    ctx: &EarlyErrorLogCtx<'_>,
    session_id: Option<String>,
    requested_model: Option<String>,
    err: String,
) -> Response {
    let contract = early_error_contract(EarlyErrorKind::InvalidCliKey);
    respond_early_error_with_spawn(ctx, contract, err, None, session_id, requested_model)
}

#[derive(Debug, Clone)]
struct HandlerRuntimeSettings {
    verbose_provider_error: bool,
    intercept_warmup: bool,
    enable_thinking_signature_rectifier: bool,
    enable_thinking_budget_rectifier: bool,
    cx2cc_settings: super::cx2cc::settings::Cx2ccSettings,
    enable_response_fixer: bool,
    response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
    provider_base_url_ping_cache_ttl_seconds: u32,
    enable_codex_session_id_completion: bool,
    enable_claude_metadata_user_id_injection: bool,
    max_attempts_per_provider: u32,
    max_providers_to_try: u32,
    provider_cooldown_secs: i64,
    upstream_first_byte_timeout_secs: u32,
    upstream_stream_idle_timeout_secs: u32,
    upstream_request_timeout_non_streaming_secs: u32,
}

fn handler_runtime_settings(
    settings_cfg: Option<&settings::AppSettings>,
    is_claude_count_tokens: bool,
) -> HandlerRuntimeSettings {
    let verbose_provider_error = settings_cfg
        .map(|cfg| cfg.verbose_provider_error)
        .unwrap_or(true);

    let enable_thinking_signature_rectifier = settings_cfg
        .map(|cfg| cfg.enable_thinking_signature_rectifier)
        .unwrap_or(true)
        && !is_claude_count_tokens;

    let enable_thinking_budget_rectifier = settings_cfg
        .map(|cfg| cfg.enable_thinking_budget_rectifier)
        .unwrap_or(true)
        && !is_claude_count_tokens;
    let cx2cc_settings = settings_cfg
        .map(super::cx2cc::settings::Cx2ccSettings::from_app_settings)
        .unwrap_or_default();

    let enable_response_fixer = settings_cfg
        .map(|cfg| cfg.enable_response_fixer)
        .unwrap_or(true);
    let response_fixer_fix_encoding = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_encoding)
        .unwrap_or(true);
    let response_fixer_fix_sse_format = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_sse_format)
        .unwrap_or(true);
    let response_fixer_fix_truncated_json = settings_cfg
        .map(|cfg| cfg.response_fixer_fix_truncated_json)
        .unwrap_or(true);
    let response_fixer_max_json_depth = settings_cfg
        .map(|cfg| cfg.response_fixer_max_json_depth)
        .unwrap_or(response_fixer::DEFAULT_MAX_JSON_DEPTH as u32);
    let response_fixer_max_fix_size = settings_cfg
        .map(|cfg| cfg.response_fixer_max_fix_size)
        .unwrap_or(response_fixer::DEFAULT_MAX_FIX_SIZE as u32);

    let mut max_attempts_per_provider = settings_cfg
        .map(|cfg| cfg.failover_max_attempts_per_provider.max(1))
        .unwrap_or(DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER);
    let mut max_providers_to_try = settings_cfg
        .map(|cfg| cfg.failover_max_providers_to_try.max(1))
        .unwrap_or(DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY);

    if is_claude_count_tokens {
        max_attempts_per_provider = 1;
        max_providers_to_try = 1;
    }

    HandlerRuntimeSettings {
        verbose_provider_error,
        intercept_warmup: settings_cfg
            .map(|cfg| cfg.intercept_anthropic_warmup_requests)
            .unwrap_or(false),
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        cx2cc_settings,
        enable_response_fixer,
        response_fixer_stream_config: response_fixer::ResponseFixerConfig {
            fix_encoding: response_fixer_fix_encoding,
            fix_sse_format: response_fixer_fix_sse_format,
            fix_truncated_json: response_fixer_fix_truncated_json,
            max_json_depth: response_fixer_max_json_depth as usize,
            max_fix_size: response_fixer_max_fix_size as usize,
        },
        response_fixer_non_stream_config: response_fixer::ResponseFixerConfig {
            fix_encoding: response_fixer_fix_encoding,
            fix_sse_format: false,
            fix_truncated_json: response_fixer_fix_truncated_json,
            max_json_depth: response_fixer_max_json_depth as usize,
            max_fix_size: response_fixer_max_fix_size as usize,
        },
        provider_base_url_ping_cache_ttl_seconds: settings_cfg
            .map(|cfg| cfg.provider_base_url_ping_cache_ttl_seconds)
            .unwrap_or(settings::DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS),
        enable_codex_session_id_completion: settings_cfg
            .map(|cfg| cfg.enable_codex_session_id_completion)
            .unwrap_or(true),
        enable_claude_metadata_user_id_injection: settings_cfg
            .map(|cfg| cfg.enable_claude_metadata_user_id_injection)
            .unwrap_or(true)
            && !is_claude_count_tokens,
        max_attempts_per_provider,
        max_providers_to_try,
        provider_cooldown_secs: settings_cfg
            .map(|cfg| cfg.provider_cooldown_seconds as i64)
            .unwrap_or(settings::DEFAULT_PROVIDER_COOLDOWN_SECONDS as i64),
        upstream_first_byte_timeout_secs: settings_cfg
            .map(|cfg| cfg.upstream_first_byte_timeout_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS),
        upstream_stream_idle_timeout_secs: settings_cfg
            .map(|cfg| cfg.upstream_stream_idle_timeout_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS),
        upstream_request_timeout_non_streaming_secs: settings_cfg
            .map(|cfg| cfg.upstream_request_timeout_non_streaming_seconds)
            .unwrap_or(settings::DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS),
    }
}

struct WarmupInterceptCtx<'a> {
    state: &'a GatewayAppState,
    trace_id: &'a str,
    cli_key: &'a str,
    method_hint: &'a str,
    forwarded_path: &'a str,
    query: Option<&'a str>,
    requested_model: Option<&'a str>,
    created_at_ms: i64,
    created_at: i64,
    duration_ms: u128,
}

fn warmup_intercept_special_settings_json() -> String {
    serde_json::json!([{
        "type": "warmup_intercept",
        "scope": "request",
        "hit": true,
        "reason": "anthropic_warmup_intercepted",
        "note": "已由 aio-coding-hub 抢答，未转发上游；写入日志但排除统计",
    }])
    .to_string()
}

fn warmup_log_usage_metrics() -> usage::UsageMetrics {
    usage::UsageMetrics {
        input_tokens: Some(0),
        output_tokens: Some(0),
        total_tokens: Some(0),
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cache_creation_5m_input_tokens: Some(0),
        cache_creation_1h_input_tokens: Some(0),
    }
}

fn respond_warmup_intercept(ctx: &WarmupInterceptCtx<'_>) -> Response {
    let response_body = warmup::build_warmup_response_body(ctx.requested_model, ctx.trace_id);
    let special_settings_json = warmup_intercept_special_settings_json();

    emit_request_start_event(
        &ctx.state.app,
        ctx.trace_id.to_string(),
        ctx.cli_key.to_string(),
        ctx.method_hint.to_string(),
        ctx.forwarded_path.to_string(),
        ctx.query.map(str::to_string),
        ctx.requested_model.map(str::to_string),
        ctx.created_at,
    );

    let warmup_attempts = [super::super::events::FailoverAttempt {
        provider_id: 0,
        provider_name: "Warmup".to_string(),
        base_url: "/__aio__/warmup".to_string(),
        outcome: "success".to_string(),
        status: Some(StatusCode::OK.as_u16()),
        provider_index: None,
        retry_index: None,
        session_reuse: Some(false),
        error_category: None,
        error_code: None,
        decision: Some("success"),
        reason: None,
        selection_method: None,
        reason_code: Some(dc::REASON_REQUEST_SUCCESS),
        attempt_started_ms: None,
        attempt_duration_ms: None,
        circuit_state_before: None,
        circuit_state_after: None,
        circuit_failure_count: None,
        circuit_failure_threshold: None,
    }];

    emit_request_event_and_spawn_request_log(RequestEndArgs {
        deps: RequestEndDeps::new(&ctx.state.app, &ctx.state.db, &ctx.state.log_tx),
        trace_id: ctx.trace_id,
        cli_key: ctx.cli_key,
        method: ctx.method_hint,
        path: ctx.forwarded_path,
        query: ctx.query,
        excluded_from_stats: true,
        status: Some(StatusCode::OK.as_u16()),
        error_category: None,
        error_code: None,
        duration_ms: ctx.duration_ms,
        event_ttfb_ms: Some(ctx.duration_ms),
        log_ttfb_ms: Some(ctx.duration_ms),
        attempts: &warmup_attempts,
        special_settings_json: Some(special_settings_json),
        session_id: None,
        requested_model: ctx.requested_model.map(str::to_string),
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
        usage_metrics: Some(usage::UsageMetrics::default()),
        log_usage_metrics: Some(warmup_log_usage_metrics()),
        usage: None,
    });

    let mut resp = (StatusCode::OK, Json(response_body)).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    resp.headers_mut()
        .insert("x-aio-intercepted", HeaderValue::from_static("warmup"));
    resp.headers_mut().insert(
        "x-aio-intercepted-by",
        HeaderValue::from_static("aio-coding-hub"),
    );
    if let Ok(v) = HeaderValue::from_str(ctx.trace_id) {
        resp.headers_mut().insert("x-trace-id", v);
    }
    resp.headers_mut().insert(
        "x-aio-upstream-meta-url",
        HeaderValue::from_static("/__aio__/warmup"),
    );
    resp
}

#[allow(clippy::too_many_arguments)]
fn complete_codex_session_ids_if_needed(
    state: &GatewayAppState,
    cli_key: &str,
    enabled: bool,
    created_at: i64,
    created_at_ms: i64,
    headers: &mut axum::http::HeaderMap,
    introspection_json: &mut Option<serde_json::Value>,
    body_bytes: &mut Bytes,
    strip_request_content_encoding_seed: &mut bool,
    special_settings: &SpecialSettings,
) {
    if cli_key != "codex" || !enabled {
        return;
    }

    let result = {
        let mut cache = state.codex_session_cache.lock_or_recover();
        codex_session_id::complete_codex_session_identifiers(
            &mut cache,
            created_at,
            created_at_ms,
            headers,
            introspection_json.as_mut(),
        )
    };

    if result.changed_body {
        if let Some(root) = introspection_json.as_ref() {
            if let Ok(next) = serde_json::to_vec(root) {
                *body_bytes = Bytes::from(next);
                *strip_request_content_encoding_seed = true;
            }
        }
    }

    push_special_setting(
        special_settings,
        serde_json::json!({
            "type": "codex_session_id_completion",
            "scope": "request",
            "hit": result.applied,
            "sessionId": result.session_id,
            "action": result.action,
            "source": result.source,
            "changedHeader": result.changed_headers,
            "changedBody": result.changed_body,
        }),
    );
}

struct RuntimeWarmupDecision {
    runtime_settings: HandlerRuntimeSettings,
    is_warmup_request: bool,
}

fn should_intercept_warmup_request(
    cli_key: &str,
    intercept_warmup: bool,
    forwarded_path: &str,
    introspection_json: Option<&serde_json::Value>,
) -> bool {
    if cli_key != "claude" || !intercept_warmup {
        return false;
    }

    warmup::is_anthropic_warmup_request(forwarded_path, introspection_json)
}

fn resolve_runtime_warmup_decision(
    state: &GatewayAppState,
    is_claude_count_tokens: bool,
    cli_key: &str,
    forwarded_path: &str,
    introspection_json: Option<&serde_json::Value>,
) -> RuntimeWarmupDecision {
    let settings_cfg = match settings::read(&state.app) {
        Ok(cfg) => Some(cfg),
        Err(err) => {
            tracing::warn!(
                "using default handler runtime settings because settings read failed: {err}"
            );
            None
        }
    };
    let runtime_settings = handler_runtime_settings(settings_cfg.as_ref(), is_claude_count_tokens);
    let is_warmup_request = should_intercept_warmup_request(
        cli_key,
        runtime_settings.intercept_warmup,
        forwarded_path,
        introspection_json,
    );

    RuntimeWarmupDecision {
        runtime_settings,
        is_warmup_request,
    }
}

pub(in crate::gateway) async fn proxy_impl(
    state: GatewayAppState,
    cli_key: String,
    forwarded_path: String,
    req: Request<Body>,
) -> Response {
    let started = Instant::now();
    let mut trace_id = new_trace_id();
    let created_at_ms = now_unix_millis() as i64;
    let created_at = (created_at_ms / 1000).max(0);
    let method = req.method().clone();
    let method_hint = method.to_string();
    let query = req.uri().query().map(str::to_string);
    let is_claude_count_tokens = is_claude_count_tokens_request(&cli_key, &forwarded_path);

    let (mut headers, body) = {
        let (parts, body) = req.into_parts();
        (parts.headers, body)
    };

    let forced_provider_id = extract_forced_provider_id(&headers);
    let bypass_cli_proxy_guard = forced_provider_id.is_some();

    if crate::shared::cli_key::is_supported_cli_key(cli_key.as_str()) && !bypass_cli_proxy_guard {
        let enabled_snapshot = cli_proxy_enabled_cached(&state.app, &cli_key);
        if !enabled_snapshot.enabled {
            if !enabled_snapshot.cache_hit {
                if let Some(err) = enabled_snapshot.error.as_deref() {
                    emit_gateway_log(
                        &state.app,
                        "warn",
                        GatewayErrorCode::CliProxyGuardError.as_str(),
                        format!(
                            "CLI 代理开关状态读取失败（按未开启处理）cli={cli_key} trace_id={trace_id} err={err}"
                        ),
                    );
                }
            }

            let contract = early_error_contract(EarlyErrorKind::CliProxyDisabled);
            let message = cli_proxy_disabled_message(&cli_key, enabled_snapshot.error.as_deref());
            let special_settings_json = cli_proxy_guard_special_settings_json(
                enabled_snapshot.cache_hit,
                enabled_snapshot.cache_ttl_ms,
                enabled_snapshot.error.as_deref(),
            );
            let log_ctx = build_early_error_log_ctx(
                &state,
                &started,
                trace_id.as_str(),
                cli_key.as_str(),
                method_hint.as_str(),
                forwarded_path.as_str(),
                query.as_deref(),
                created_at_ms,
                created_at,
            );

            return respond_early_error_with_enqueue(
                &log_ctx,
                contract,
                message,
                Some(special_settings_json),
                None,
                None,
            )
            .await;
        }
    }

    headers.remove("x-aio-provider-id");

    let mut body_bytes = match to_bytes(body, MAX_REQUEST_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(err) => {
            let contract = early_error_contract(EarlyErrorKind::BodyTooLarge);
            let log_ctx = build_early_error_log_ctx(
                &state,
                &started,
                trace_id.as_str(),
                cli_key.as_str(),
                method_hint.as_str(),
                forwarded_path.as_str(),
                query.as_deref(),
                created_at_ms,
                created_at,
            );

            return respond_early_error_with_enqueue(
                &log_ctx,
                contract,
                body_too_large_message(&err.to_string()),
                None,
                None,
                None,
            )
            .await;
        }
    };

    let mut introspection_json = {
        let introspection_body = body_for_introspection(&headers, &body_bytes);
        serde_json::from_slice::<serde_json::Value>(introspection_body.as_ref()).ok()
    };
    let requested_model_info = infer_requested_model_info(
        &forwarded_path,
        query.as_deref(),
        introspection_json.as_ref(),
    );
    let requested_model = requested_model_info.model;
    let requested_model_location = requested_model_info.location;

    let RuntimeWarmupDecision {
        runtime_settings,
        is_warmup_request,
    } = resolve_runtime_warmup_decision(
        &state,
        is_claude_count_tokens,
        &cli_key,
        &forwarded_path,
        introspection_json.as_ref(),
    );

    if is_warmup_request {
        let duration_ms = started.elapsed().as_millis();
        let warmup_ctx = WarmupInterceptCtx {
            state: &state,
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method_hint: method_hint.as_str(),
            forwarded_path: forwarded_path.as_str(),
            query: query.as_deref(),
            requested_model: requested_model.as_deref(),
            created_at_ms,
            created_at,
            duration_ms,
        };
        return respond_warmup_intercept(&warmup_ctx);
    }

    let special_settings = new_special_settings();

    let mut strip_request_content_encoding_seed = false;
    complete_codex_session_ids_if_needed(
        &state,
        &cli_key,
        runtime_settings.enable_codex_session_id_completion,
        created_at,
        created_at_ms,
        &mut headers,
        &mut introspection_json,
        &mut body_bytes,
        &mut strip_request_content_encoding_seed,
        &special_settings,
    );

    let SessionRoutingDecision {
        session_id,
        allow_session_reuse,
    } = resolve_session_routing_decision(
        &headers,
        introspection_json.as_ref(),
        is_claude_count_tokens,
    );

    let ProviderSelection {
        effective_sort_mode_id,
        mut providers,
        bound_provider_order,
    } = match select_providers_with_session_binding(
        &state,
        &cli_key,
        session_id.as_deref(),
        created_at,
    ) {
        Ok(selection) => selection,
        Err(err) => {
            let log_ctx = build_early_error_log_ctx(
                &state,
                &started,
                trace_id.as_str(),
                cli_key.as_str(),
                method_hint.as_str(),
                forwarded_path.as_str(),
                query.as_deref(),
                created_at_ms,
                created_at,
            );
            return respond_invalid_cli_key_with_spawn(
                &log_ctx,
                session_id.clone(),
                requested_model.clone(),
                err.to_string(),
            );
        }
    };

    force_provider_if_requested(&mut providers, forced_provider_id, &special_settings);

    // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

    let session_bound_provider_id = resolve_session_bound_provider_id(
        state.session.as_ref(),
        state.circuit.as_ref(),
        &cli_key,
        session_id.as_deref(),
        created_at,
        allow_session_reuse,
        forced_provider_id,
        &mut providers,
        bound_provider_order.as_deref(),
    );

    if providers.is_empty() {
        let contract = early_error_contract(EarlyErrorKind::NoEnabledProvider);
        let message = no_enabled_provider_message(&cli_key);
        let log_ctx = build_early_error_log_ctx(
            &state,
            &started,
            trace_id.as_str(),
            cli_key.as_str(),
            method_hint.as_str(),
            forwarded_path.as_str(),
            query.as_deref(),
            created_at_ms,
            created_at,
        );

        return respond_early_error_with_enqueue(
            &log_ctx,
            contract,
            message,
            None,
            session_id,
            requested_model,
        )
        .await;
    }

    let fingerprints = build_request_fingerprints(
        &cli_key,
        effective_sort_mode_id,
        &method_hint,
        &forwarded_path,
        query.as_deref(),
        session_id.as_deref(),
        requested_model.as_deref(),
        &headers,
        &body_bytes,
    );

    trace_id = match apply_recent_error_cache_gate(&state.recent_errors, &fingerprints, trace_id) {
        Ok(next_trace_id) => next_trace_id,
        Err(resp) => return *resp,
    };

    if should_observe_request(&cli_key, &forwarded_path) {
        emit_request_start_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            requested_model.clone(),
            created_at,
        );
    }

    super::forwarder::forward(RequestContext::from_handler_parts(RequestContextParts {
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
        provider_base_url_ping_cache_ttl_seconds: runtime_settings
            .provider_base_url_ping_cache_ttl_seconds,
        verbose_provider_error: runtime_settings.verbose_provider_error,
        enable_codex_session_id_completion: runtime_settings.enable_codex_session_id_completion,
        max_attempts_per_provider: runtime_settings.max_attempts_per_provider,
        max_providers_to_try: runtime_settings.max_providers_to_try,
        provider_cooldown_secs: runtime_settings.provider_cooldown_secs,
        upstream_first_byte_timeout_secs: runtime_settings.upstream_first_byte_timeout_secs,
        upstream_stream_idle_timeout_secs: runtime_settings.upstream_stream_idle_timeout_secs,
        upstream_request_timeout_non_streaming_secs: runtime_settings
            .upstream_request_timeout_non_streaming_secs,
        fingerprint_key: fingerprints.fingerprint_key,
        fingerprint_debug: fingerprints.fingerprint_debug,
        unavailable_fingerprint_key: fingerprints.unavailable_fingerprint_key,
        unavailable_fingerprint_debug: fingerprints.unavailable_fingerprint_debug,
        enable_thinking_signature_rectifier: runtime_settings.enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier: runtime_settings.enable_thinking_budget_rectifier,
        enable_claude_metadata_user_id_injection: runtime_settings
            .enable_claude_metadata_user_id_injection,
        cx2cc_settings: runtime_settings.cx2cc_settings.clone(),
        enable_response_fixer: runtime_settings.enable_response_fixer,
        response_fixer_stream_config: runtime_settings.response_fixer_stream_config,
        response_fixer_non_stream_config: runtime_settings.response_fixer_non_stream_config,
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        body_too_large_message, build_request_fingerprints, cli_proxy_disabled_message,
        cli_proxy_guard_special_settings_json, early_error_contract, handler_runtime_settings,
        no_enabled_provider_message, resolve_session_routing_decision,
        should_intercept_warmup_request, warmup_intercept_special_settings_json,
        warmup_log_usage_metrics, EarlyErrorKind,
    };
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
        let message = body_too_large_message("stream exceeded limit");
        assert!(message.contains("failed to read request body:"));
        assert!(message.contains("stream exceeded limit"));
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
    fn apply_session_reuse_binding_promotes_bound_provider_when_allowed() {
        let mut providers = vec![provider(11), provider(22), provider(33)];

        let selected = super::provider_selection::apply_session_reuse_provider_binding(
            true,
            &mut providers,
            Some(22),
            Some(&[11, 22, 33]),
        );

        assert_eq!(selected, Some(22));
        assert_eq!(provider_ids(&providers), vec![22, 11, 33]);
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
        assert_eq!(super::extract_forced_provider_id(&headers), Some(12));
    }

    #[test]
    fn extract_forced_provider_id_rejects_invalid_or_non_positive_values() {
        let mut headers = HeaderMap::new();
        headers.insert("x-aio-provider-id", HeaderValue::from_static("0"));
        assert_eq!(super::extract_forced_provider_id(&headers), None);

        headers.insert("x-aio-provider-id", HeaderValue::from_static("-1"));
        assert_eq!(super::extract_forced_provider_id(&headers), None);

        headers.insert("x-aio-provider-id", HeaderValue::from_static("abc"));
        assert_eq!(super::extract_forced_provider_id(&headers), None);
    }

    #[test]
    fn force_provider_if_requested_keeps_only_selected_provider() {
        let mut providers = vec![provider(1), provider(2), provider(3)];
        let special_settings = super::new_special_settings();

        super::force_provider_if_requested(&mut providers, Some(2), &special_settings);

        assert_eq!(provider_ids(&providers), vec![2]);
    }

    #[test]
    fn force_provider_if_requested_clears_when_selected_provider_missing() {
        let mut providers = vec![provider(1), provider(2), provider(3)];
        let special_settings = super::new_special_settings();

        super::force_provider_if_requested(&mut providers, Some(99), &special_settings);

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
