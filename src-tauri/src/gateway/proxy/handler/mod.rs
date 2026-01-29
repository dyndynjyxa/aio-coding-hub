//! Usage: Gateway proxy handler implementation (request forwarding + failover + circuit breaker + logging).
//!
//! Note: this module is being split into smaller submodules under `handler/`.

use super::caches::RECENT_TRACE_DEDUP_TTL_SECS;
use super::request_context::{RequestContext, RequestContextParts};
use super::request_end::{
    emit_request_event_and_enqueue_request_log, emit_request_event_and_spawn_request_log,
    RequestEndArgs, RequestEndDeps,
};
use super::ErrorCategory;
use super::{
    cli_proxy_guard::cli_proxy_enabled_cached,
    errors::{error_response, error_response_with_retry_after},
    failover::{select_next_provider_id_from_order, should_reuse_provider},
    is_claude_count_tokens_request,
};

use crate::{providers, session_manager, settings, usage};
use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::codex_session_id;
use super::super::events::{emit_gateway_log, emit_request_start_event};
use super::super::manager::GatewayAppState;
use super::super::response_fixer;
use super::super::util::{
    body_for_introspection, compute_all_providers_unavailable_fingerprint,
    compute_request_fingerprint, extract_idempotency_key_hash, infer_requested_model_info,
    new_trace_id, now_unix_millis, now_unix_seconds, MAX_REQUEST_BODY_BYTES,
};
use super::super::warmup;

const DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 5;
const DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 5;

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

    if crate::shared::cli_key::is_supported_cli_key(cli_key.as_str()) {
        let enabled_snapshot = cli_proxy_enabled_cached(&state.app, &cli_key);
        if !enabled_snapshot.enabled {
            if !enabled_snapshot.cache_hit {
                if let Some(err) = enabled_snapshot.error.as_deref() {
                    emit_gateway_log(
                        &state.app,
                        "warn",
                        "GW_CLI_PROXY_GUARD_ERROR",
                        format!(
                            "CLI 代理开关状态读取失败（按未开启处理）cli={cli_key} trace_id={trace_id} err={err}"
                        ),
                    );
                }
            }

            let message = match enabled_snapshot.error.as_deref() {
                Some(err) => format!(
                    "CLI 代理状态读取失败（按未开启处理）：{err}；请在首页开启 {cli_key} 的 CLI 代理开关后重试"
                ),
                None => format!("CLI 代理未开启：请在首页开启 {cli_key} 的 CLI 代理开关后重试"),
            };
            let resp = error_response(
                StatusCode::FORBIDDEN,
                trace_id.clone(),
                "GW_CLI_PROXY_DISABLED",
                message,
                vec![],
            );

            let special_settings_json = serde_json::json!([{
                "type": "cli_proxy_guard",
                "scope": "request",
                "hit": true,
                "enabled": false,
                "cacheHit": enabled_snapshot.cache_hit,
                "cacheTtlMs": enabled_snapshot.cache_ttl_ms,
                "error": enabled_snapshot.error.as_deref(),
            }])
            .to_string();

            let duration_ms = started.elapsed().as_millis();
            emit_request_event_and_enqueue_request_log(RequestEndArgs {
                deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                trace_id: trace_id.as_str(),
                cli_key: cli_key.as_str(),
                method: method_hint.as_str(),
                path: forwarded_path.as_str(),
                query: query.as_deref(),
                excluded_from_stats: true,
                status: Some(StatusCode::FORBIDDEN.as_u16()),
                error_category: Some(ErrorCategory::NonRetryableClientError.as_str()),
                error_code: Some("GW_CLI_PROXY_DISABLED"),
                duration_ms,
                event_ttfb_ms: None,
                log_ttfb_ms: None,
                attempts: &[],
                special_settings_json: Some(special_settings_json),
                session_id: None,
                requested_model: None,
                created_at_ms,
                created_at,
                usage_metrics: None,
                log_usage_metrics: None,
                usage: None,
            })
            .await;

            return resp;
        }
    }

    let (mut headers, body) = {
        let (parts, body) = req.into_parts();
        (parts.headers, body)
    };

    let mut body_bytes = match to_bytes(body, MAX_REQUEST_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(err) => {
            let resp = error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                trace_id.clone(),
                "GW_BODY_TOO_LARGE",
                format!("failed to read request body: {err}"),
                vec![],
            );

            let duration_ms = started.elapsed().as_millis();
            emit_request_event_and_enqueue_request_log(RequestEndArgs {
                deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
                trace_id: trace_id.as_str(),
                cli_key: cli_key.as_str(),
                method: method_hint.as_str(),
                path: forwarded_path.as_str(),
                query: query.as_deref(),
                excluded_from_stats: false,
                status: Some(StatusCode::PAYLOAD_TOO_LARGE.as_u16()),
                error_category: None,
                error_code: Some("GW_BODY_TOO_LARGE"),
                duration_ms,
                event_ttfb_ms: None,
                log_ttfb_ms: None,
                attempts: &[],
                special_settings_json: None,
                session_id: None,
                requested_model: None,
                created_at_ms,
                created_at,
                usage_metrics: None,
                log_usage_metrics: None,
                usage: None,
            })
            .await;
            return resp;
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

    let settings_cfg = settings::read(&state.app).ok();
    let intercept_warmup = settings_cfg
        .as_ref()
        .map(|cfg| cfg.intercept_anthropic_warmup_requests)
        .unwrap_or(false);
    let enable_thinking_signature_rectifier = settings_cfg
        .as_ref()
        .map(|cfg| cfg.enable_thinking_signature_rectifier)
        .unwrap_or(true);
    let enable_thinking_signature_rectifier =
        enable_thinking_signature_rectifier && !is_claude_count_tokens;
    let enable_response_fixer = settings_cfg
        .as_ref()
        .map(|cfg| cfg.enable_response_fixer)
        .unwrap_or(true);
    let response_fixer_fix_encoding = settings_cfg
        .as_ref()
        .map(|cfg| cfg.response_fixer_fix_encoding)
        .unwrap_or(true);
    let response_fixer_fix_sse_format = settings_cfg
        .as_ref()
        .map(|cfg| cfg.response_fixer_fix_sse_format)
        .unwrap_or(true);
    let response_fixer_fix_truncated_json = settings_cfg
        .as_ref()
        .map(|cfg| cfg.response_fixer_fix_truncated_json)
        .unwrap_or(true);
    let response_fixer_max_json_depth = settings_cfg
        .as_ref()
        .map(|cfg| cfg.response_fixer_max_json_depth)
        .unwrap_or(response_fixer::DEFAULT_MAX_JSON_DEPTH as u32);
    let response_fixer_max_fix_size = settings_cfg
        .as_ref()
        .map(|cfg| cfg.response_fixer_max_fix_size)
        .unwrap_or(response_fixer::DEFAULT_MAX_FIX_SIZE as u32);
    let provider_base_url_ping_cache_ttl_seconds = settings_cfg
        .as_ref()
        .map(|cfg| cfg.provider_base_url_ping_cache_ttl_seconds)
        .unwrap_or(settings::DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS);
    let enable_codex_session_id_completion = settings_cfg
        .as_ref()
        .map(|cfg| cfg.enable_codex_session_id_completion)
        .unwrap_or(true);

    let response_fixer_stream_config = response_fixer::ResponseFixerConfig {
        fix_encoding: response_fixer_fix_encoding,
        fix_sse_format: response_fixer_fix_sse_format,
        fix_truncated_json: response_fixer_fix_truncated_json,
        max_json_depth: response_fixer_max_json_depth as usize,
        max_fix_size: response_fixer_max_fix_size as usize,
    };
    let response_fixer_non_stream_config = response_fixer::ResponseFixerConfig {
        fix_encoding: response_fixer_fix_encoding,
        fix_sse_format: false,
        fix_truncated_json: response_fixer_fix_truncated_json,
        max_json_depth: response_fixer_max_json_depth as usize,
        max_fix_size: response_fixer_max_fix_size as usize,
    };

    let is_warmup_request = if cli_key == "claude" && intercept_warmup {
        let introspection_body = body_for_introspection(&headers, &body_bytes);
        warmup::is_anthropic_warmup_request(&forwarded_path, introspection_body.as_ref())
    } else {
        false
    };

    if is_warmup_request {
        let duration_ms = started.elapsed().as_millis();
        let response_body =
            warmup::build_warmup_response_body(requested_model.as_deref(), &trace_id);

        let special_settings_json = serde_json::json!([{
            "type": "warmup_intercept",
            "scope": "request",
            "hit": true,
            "reason": "anthropic_warmup_intercepted",
            "note": "已由 aio-coding-hub 抢答，未转发上游；写入日志但排除统计",
        }])
        .to_string();

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
            decision: None,
            reason: None,
            attempt_started_ms: None,
            attempt_duration_ms: None,
            circuit_state_before: None,
            circuit_state_after: None,
            circuit_failure_count: None,
            circuit_failure_threshold: None,
        }];

        emit_request_event_and_spawn_request_log(RequestEndArgs {
            deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method: method_hint.as_str(),
            path: forwarded_path.as_str(),
            query: query.as_deref(),
            excluded_from_stats: true,
            status: Some(StatusCode::OK.as_u16()),
            error_category: None,
            error_code: None,
            duration_ms,
            event_ttfb_ms: Some(duration_ms),
            log_ttfb_ms: Some(duration_ms),
            attempts: &warmup_attempts,
            special_settings_json: Some(special_settings_json),
            session_id: None,
            requested_model: requested_model.clone(),
            created_at_ms,
            created_at,
            usage_metrics: Some(usage::UsageMetrics::default()),
            log_usage_metrics: Some(usage::UsageMetrics {
                input_tokens: Some(0),
                output_tokens: Some(0),
                total_tokens: Some(0),
                cache_read_input_tokens: Some(0),
                cache_creation_input_tokens: Some(0),
                cache_creation_5m_input_tokens: Some(0),
                cache_creation_1h_input_tokens: Some(0),
            }),
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
        if let Ok(v) = HeaderValue::from_str(&trace_id) {
            resp.headers_mut().insert("x-trace-id", v);
        }
        resp.headers_mut().insert(
            "x-aio-upstream-meta-url",
            HeaderValue::from_static("/__aio__/warmup"),
        );
        return resp;
    }

    let special_settings: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let mut strip_request_content_encoding_seed = false;
    if cli_key == "codex" && enable_codex_session_id_completion {
        let mut cache = state
            .codex_session_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let result = codex_session_id::complete_codex_session_identifiers(
            &mut cache,
            created_at,
            created_at_ms,
            &mut headers,
            introspection_json.as_mut(),
        );

        if result.changed_body {
            if let Some(root) = introspection_json.as_ref() {
                if let Ok(next) = serde_json::to_vec(root) {
                    body_bytes = Bytes::from(next);
                    strip_request_content_encoding_seed = true;
                }
            }
        }

        if let Ok(mut settings) = special_settings.lock() {
            settings.push(serde_json::json!({
                "type": "codex_session_id_completion",
                "scope": "request",
                "hit": result.applied,
                "sessionId": result.session_id,
                "action": result.action,
                "source": result.source,
                "changedHeader": result.changed_headers,
                "changedBody": result.changed_body,
            }));
        }
    }

    let session_id = session_manager::SessionManager::extract_session_id_from_json(
        &headers,
        introspection_json.as_ref(),
    );
    let session_id = if is_claude_count_tokens {
        None
    } else {
        session_id
    };
    let allow_session_reuse = if is_claude_count_tokens {
        false
    } else {
        should_reuse_provider(introspection_json.as_ref())
    };

    let respond_invalid_cli_key = |err: String| -> Response {
        let resp = error_response(
            StatusCode::BAD_REQUEST,
            trace_id.clone(),
            "GW_INVALID_CLI_KEY",
            err,
            vec![],
        );

        let duration_ms = started.elapsed().as_millis();
        emit_request_event_and_spawn_request_log(RequestEndArgs {
            deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method: method_hint.as_str(),
            path: forwarded_path.as_str(),
            query: query.as_deref(),
            excluded_from_stats: false,
            status: Some(StatusCode::BAD_REQUEST.as_u16()),
            error_category: None,
            error_code: Some("GW_INVALID_CLI_KEY"),
            duration_ms,
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            attempts: &[],
            special_settings_json: None,
            session_id: session_id.clone(),
            requested_model: requested_model.clone(),
            created_at_ms,
            created_at,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        });

        resp
    };

    let bound_sort_mode_id = session_id.as_deref().and_then(|sid| {
        state
            .session
            .get_bound_sort_mode_id(&cli_key, sid, created_at)
    });

    let (effective_sort_mode_id, mut providers) = match bound_sort_mode_id {
        Some(sort_mode_id) => {
            let providers = match providers::list_enabled_for_gateway_in_mode(
                &state.db,
                &cli_key,
                sort_mode_id,
            ) {
                Ok(v) => v,
                Err(err) => return respond_invalid_cli_key(err),
            };
            (sort_mode_id, providers)
        }
        None => {
            let selection =
                match providers::list_enabled_for_gateway_using_active_mode(&state.db, &cli_key) {
                    Ok(v) => v,
                    Err(err) => return respond_invalid_cli_key(err),
                };
            (selection.sort_mode_id, selection.providers)
        }
    };

    let mut bound_provider_order: Option<Vec<i64>> = None;
    if let Some(sid) = session_id.as_deref() {
        let provider_order: Vec<i64> = providers.iter().map(|p| p.id).collect();
        state.session.bind_sort_mode(
            &cli_key,
            sid,
            effective_sort_mode_id,
            Some(provider_order),
            created_at,
        );

        bound_provider_order = state
            .session
            .get_bound_provider_order(&cli_key, sid, created_at);

        if let Some(order) = bound_provider_order.as_ref() {
            if !order.is_empty() && providers.len() > 1 {
                let mut by_id: HashMap<i64, providers::ProviderForGateway> =
                    HashMap::with_capacity(providers.len());
                let mut original_ids: Vec<i64> = Vec::with_capacity(providers.len());
                for item in providers.drain(..) {
                    original_ids.push(item.id);
                    by_id.insert(item.id, item);
                }

                let mut reordered: Vec<providers::ProviderForGateway> =
                    Vec::with_capacity(original_ids.len());
                for provider_id in order {
                    if let Some(item) = by_id.remove(provider_id) {
                        reordered.push(item);
                    }
                }
                for provider_id in original_ids {
                    if let Some(item) = by_id.remove(&provider_id) {
                        reordered.push(item);
                    }
                }
                providers = reordered;
            }
        }
    }

    if providers.is_empty() {
        let message = format!("no enabled provider for cli_key={cli_key}");
        let resp = error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            trace_id.clone(),
            "GW_NO_ENABLED_PROVIDER",
            message,
            vec![],
        );
        let duration_ms = started.elapsed().as_millis();
        emit_request_event_and_enqueue_request_log(RequestEndArgs {
            deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method: method_hint.as_str(),
            path: forwarded_path.as_str(),
            query: query.as_deref(),
            excluded_from_stats: false,
            status: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            error_category: None,
            error_code: Some("GW_NO_ENABLED_PROVIDER"),
            duration_ms,
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            attempts: &[],
            special_settings_json: None,
            session_id,
            requested_model,
            created_at_ms,
            created_at,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        })
        .await;
        return resp;
    }

    // NOTE: model whitelist filtering removed (Claude uses slot-based model mapping).

    let mut session_bound_provider_id: Option<i64> = None;
    if allow_session_reuse {
        if let Some(bound_provider_id) = session_id
            .as_deref()
            .and_then(|sid| state.session.get_bound_provider(&cli_key, sid, created_at))
        {
            if let Some(idx) = providers.iter().position(|p| p.id == bound_provider_id) {
                session_bound_provider_id = Some(bound_provider_id);
                if idx > 0 {
                    let chosen = providers.remove(idx);
                    providers.insert(0, chosen);
                }
            } else if let Some(order) = bound_provider_order.as_deref() {
                if !order.is_empty() && providers.len() > 1 {
                    let current_provider_ids: HashSet<i64> =
                        providers.iter().map(|p| p.id).collect();
                    if let Some(next_provider_id) = select_next_provider_id_from_order(
                        bound_provider_id,
                        order,
                        &current_provider_ids,
                    ) {
                        if let Some(idx) = providers.iter().position(|p| p.id == next_provider_id) {
                            if idx > 0 {
                                providers.rotate_left(idx);
                            }
                        }
                    }
                }
            }
        }
    }

    let (unavailable_fingerprint_key, unavailable_fingerprint_debug) =
        compute_all_providers_unavailable_fingerprint(
            &cli_key,
            effective_sort_mode_id,
            &method_hint,
            &forwarded_path,
        );

    let idempotency_key_hash = extract_idempotency_key_hash(&headers);

    let introspection_body = body_for_introspection(&headers, &body_bytes);
    let (fingerprint_key, fingerprint_debug) = compute_request_fingerprint(
        &cli_key,
        &method_hint,
        &forwarded_path,
        query.as_deref(),
        session_id.as_deref(),
        requested_model.as_deref(),
        idempotency_key_hash,
        introspection_body.as_ref(),
    );

    if let Ok(mut cache) = state.recent_errors.lock() {
        let now_unix = now_unix_seconds() as i64;
        let cached_error = cache
            .get_error(now_unix, fingerprint_key, &fingerprint_debug)
            .or_else(|| {
                cache.get_error(
                    now_unix,
                    unavailable_fingerprint_key,
                    &unavailable_fingerprint_debug,
                )
            });

        if let Some(entry) = cached_error {
            let any_allowed = providers
                .iter()
                .any(|p| state.circuit.should_allow(p.id, now_unix).allow);
            if !any_allowed {
                trace_id = entry.trace_id.clone();
                cache.upsert_trace_id(
                    now_unix,
                    fingerprint_key,
                    trace_id.clone(),
                    fingerprint_debug.clone(),
                    RECENT_TRACE_DEDUP_TTL_SECS,
                );
                return error_response_with_retry_after(
                    entry.status,
                    entry.trace_id,
                    entry.error_code,
                    entry.message,
                    vec![],
                    entry.retry_after_seconds,
                );
            }

            cache.remove_error(fingerprint_key);
            cache.remove_error(unavailable_fingerprint_key);
        } else if let Some(existing) =
            cache.get_trace_id(now_unix, fingerprint_key, &fingerprint_debug)
        {
            trace_id = existing;
        }

        cache.upsert_trace_id(
            now_unix,
            fingerprint_key,
            trace_id.clone(),
            fingerprint_debug.clone(),
            RECENT_TRACE_DEDUP_TTL_SECS,
        );
    }

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

    let (
        mut max_attempts_per_provider,
        mut max_providers_to_try,
        provider_cooldown_secs,
        upstream_first_byte_timeout_secs,
        upstream_stream_idle_timeout_secs,
        upstream_request_timeout_non_streaming_secs,
    ) = match settings_cfg.as_ref() {
        Some(cfg) => (
            cfg.failover_max_attempts_per_provider.max(1),
            cfg.failover_max_providers_to_try.max(1),
            cfg.provider_cooldown_seconds as i64,
            cfg.upstream_first_byte_timeout_seconds,
            cfg.upstream_stream_idle_timeout_seconds,
            cfg.upstream_request_timeout_non_streaming_seconds,
        ),
        None => (
            DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER,
            DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY,
            settings::DEFAULT_PROVIDER_COOLDOWN_SECONDS as i64,
            settings::DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS,
            settings::DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS,
            settings::DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS,
        ),
    };

    if is_claude_count_tokens {
        max_attempts_per_provider = 1;
        max_providers_to_try = 1;
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
        provider_base_url_ping_cache_ttl_seconds,
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
        enable_response_fixer,
        response_fixer_stream_config,
        response_fixer_non_stream_config,
    }))
    .await
}
