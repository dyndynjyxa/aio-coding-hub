//! Usage: Gateway proxy core (request forwarding + failover + circuit breaker + logging).

mod caches;
mod logging;
mod types;

use caches::{CachedGatewayError, RECENT_TRACE_DEDUP_TTL_SECS};
pub(super) use caches::{ProviderBaseUrlPingCache, RecentErrorCache};
pub(super) use logging::spawn_enqueue_request_log_with_backpressure;
use logging::{enqueue_attempt_log_with_backpressure, enqueue_request_log_with_backpressure};
pub(super) use types::ErrorCategory;

use crate::{
    circuit_breaker, cli_proxy, providers, request_logs, session_manager, settings, usage,
};
use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::codex_session_id;
use super::events::{
    emit_attempt_event, emit_circuit_event, emit_circuit_transition, emit_gateway_log,
    emit_request_event, emit_request_start_event, FailoverAttempt, GatewayAttemptEvent,
    GatewayCircuitEvent,
};
use super::manager::GatewayAppState;
use super::response_fixer;
use super::streams::{
    spawn_usage_sse_relay_body, FirstChunkStream, GunzipStream, StreamFinalizeCtx,
    TimingOnlyTeeStream, UsageBodyBufferTeeStream, UsageSseTeeStream,
};
use super::thinking_signature_rectifier;
use super::util::{
    body_for_introspection, build_target_url, compute_all_providers_unavailable_fingerprint,
    compute_request_fingerprint, encode_url_component, extract_idempotency_key_hash,
    infer_requested_model_info, inject_provider_auth, new_trace_id, now_unix_millis,
    now_unix_seconds, strip_hop_headers, RequestedModelLocation, MAX_REQUEST_BODY_BYTES,
};
use super::warmup;

#[derive(Debug, Serialize)]
struct GatewayErrorResponse {
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

const MAX_NON_SSE_BODY_BYTES: usize = 20 * 1024 * 1024;

const DEFAULT_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER: u32 = 5;
const DEFAULT_FAILOVER_MAX_PROVIDERS_TO_TRY: u32 = 5;

const CLI_PROXY_ENABLED_CACHE_TTL_MS_OK: i64 = 500;
const CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR: i64 = 5_000;

#[derive(Debug, Clone, Copy)]
enum FailoverDecision {
    RetrySameProvider,
    SwitchProvider,
    Abort,
}

impl FailoverDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::RetrySameProvider => "retry",
            Self::SwitchProvider => "switch",
            Self::Abort => "abort",
        }
    }
}

fn classify_reqwest_error(err: &reqwest::Error) -> (ErrorCategory, &'static str) {
    if err.is_timeout() {
        return (ErrorCategory::SystemError, "GW_UPSTREAM_TIMEOUT");
    }
    if err.is_connect() {
        return (ErrorCategory::SystemError, "GW_UPSTREAM_CONNECT_FAILED");
    }
    (ErrorCategory::SystemError, "GW_INTERNAL_ERROR")
}

fn classify_upstream_status(
    status: reqwest::StatusCode,
) -> (ErrorCategory, &'static str, FailoverDecision) {
    if status.is_server_error() {
        return (
            ErrorCategory::ProviderError,
            "GW_UPSTREAM_5XX",
            FailoverDecision::RetrySameProvider,
        );
    }

    match status.as_u16() {
        401 | 403 => (
            ErrorCategory::ProviderError,
            "GW_UPSTREAM_4XX",
            FailoverDecision::SwitchProvider,
        ),
        408 | 429 => (
            ErrorCategory::ProviderError,
            "GW_UPSTREAM_4XX",
            FailoverDecision::RetrySameProvider,
        ),
        404 => (
            ErrorCategory::ResourceNotFound,
            "GW_UPSTREAM_4XX",
            FailoverDecision::Abort,
        ),
        _ if status.is_client_error() => (
            ErrorCategory::NonRetryableClientError,
            "GW_UPSTREAM_4XX",
            FailoverDecision::Abort,
        ),
        _ => (
            ErrorCategory::ProviderError,
            "GW_INTERNAL_ERROR",
            FailoverDecision::Abort,
        ),
    }
}

fn retry_backoff_delay(status: reqwest::StatusCode, retry_index: u32) -> Option<Duration> {
    if !matches!(status.as_u16(), 408 | 429) {
        return None;
    }

    let retry_index = retry_index.max(1);
    let base_ms = 80u64;
    let max_ms = 800u64;
    let ms = base_ms.saturating_mul(retry_index as u64).min(max_ms);
    Some(Duration::from_millis(ms))
}

fn should_reuse_provider(body_json: Option<&serde_json::Value>) -> bool {
    let Some(value) = body_json else {
        return false;
    };

    let len = value
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .or_else(|| {
            value
                .get("input")
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .or_else(|| {
            value
                .get("contents")
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .or_else(|| {
            value
                .get("request")
                .and_then(|v| v.get("contents"))
                .and_then(|v| v.as_array())
                .map(|v| v.len())
        })
        .unwrap_or(0);

    len > 1
}

fn select_next_provider_id_from_order(
    bound_provider_id: i64,
    provider_order: &[i64],
    current_provider_ids: &HashSet<i64>,
) -> Option<i64> {
    if provider_order.is_empty() || current_provider_ids.is_empty() {
        return None;
    }

    let start = match provider_order
        .iter()
        .position(|provider_id| *provider_id == bound_provider_id)
    {
        Some(idx) => idx.saturating_add(1),
        None => 0,
    };

    for offset in 0..provider_order.len() {
        let idx = (start + offset) % provider_order.len();
        let candidate = provider_order[idx];
        if current_provider_ids.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn error_response(
    status: StatusCode,
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
) -> Response {
    error_response_with_retry_after(status, trace_id, error_code, message, attempts, None)
}

fn error_response_with_retry_after(
    status: StatusCode,
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
    retry_after_seconds: Option<u64>,
) -> Response {
    let payload = GatewayErrorResponse {
        trace_id: trace_id.clone(),
        error_code,
        message,
        attempts,
        retry_after_seconds,
    };

    let mut resp = (status, Json(payload)).into_response();

    if let Ok(v) = HeaderValue::from_str(&trace_id) {
        resp.headers_mut().insert("x-trace-id", v);
    }

    if let Some(seconds) = retry_after_seconds.filter(|v| *v > 0) {
        let value = seconds.to_string();
        if let Ok(v) = HeaderValue::from_str(&value) {
            resp.headers_mut().insert(header::RETRY_AFTER, v);
        }
    }

    resp
}

struct RequestAbortGuard {
    app: tauri::AppHandle,
    log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    trace_id: String,
    cli_key: String,
    method: String,
    path: String,
    query: Option<String>,
    created_at_ms: i64,
    created_at: i64,
    started: Instant,
    armed: bool,
}

impl RequestAbortGuard {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: tauri::AppHandle,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        trace_id: String,
        cli_key: String,
        method: String,
        path: String,
        query: Option<String>,
        created_at_ms: i64,
        created_at: i64,
        started: Instant,
    ) -> Self {
        Self {
            app,
            log_tx,
            trace_id,
            cli_key,
            method,
            path,
            query,
            created_at_ms,
            created_at,
            started,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RequestAbortGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let duration_ms = self.started.elapsed().as_millis();
        emit_request_event(
            &self.app,
            self.trace_id.clone(),
            self.cli_key.clone(),
            self.method.clone(),
            self.path.clone(),
            self.query.clone(),
            None,
            Some(ErrorCategory::ClientAbort.as_str()),
            Some("GW_REQUEST_ABORTED"),
            duration_ms,
            None,
            vec![],
            None,
        );

        spawn_enqueue_request_log_with_backpressure(
            self.app.clone(),
            self.log_tx.clone(),
            RequestLogEnqueueArgs {
                trace_id: self.trace_id.clone(),
                cli_key: self.cli_key.clone(),
                session_id: None,
                method: self.method.clone(),
                path: self.path.clone(),
                query: self.query.clone(),
                excluded_from_stats: false,
                special_settings_json: None,
                status: None,
                error_code: Some("GW_REQUEST_ABORTED"),
                duration_ms,
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                requested_model: None,
                created_at_ms: self.created_at_ms,
                created_at: self.created_at,
                usage: None,
            },
        );
    }
}

pub(super) struct RequestLogEnqueueArgs {
    pub(super) trace_id: String,
    pub(super) cli_key: String,
    pub(super) session_id: Option<String>,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) query: Option<String>,
    pub(super) excluded_from_stats: bool,
    pub(super) special_settings_json: Option<String>,
    pub(super) status: Option<u16>,
    pub(super) error_code: Option<&'static str>,
    pub(super) duration_ms: u128,
    pub(super) ttfb_ms: Option<u128>,
    pub(super) attempts_json: String,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) usage: Option<usage::UsageExtract>,
}

fn is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

fn has_gzip_content_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|enc| !enc.is_empty())
                .any(|enc| enc.eq_ignore_ascii_case("gzip"))
        })
        .unwrap_or(false)
}

fn has_non_identity_content_encoding(headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .any(|enc| !enc.eq_ignore_ascii_case("identity"))
}

fn maybe_gunzip_response_body_bytes_with_limit(
    body: Bytes,
    headers: &mut HeaderMap,
    max_output_bytes: usize,
) -> Bytes {
    if !has_gzip_content_encoding(headers) {
        return body;
    }

    if body.is_empty() {
        headers.remove(header::CONTENT_ENCODING);
        headers.remove(header::CONTENT_LENGTH);
        return body;
    }

    let mut decoder = flate2::read::GzDecoder::new(body.as_ref());
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    let mut had_any_output = false;
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                had_any_output = true;
                if out.len().saturating_add(n) > max_output_bytes {
                    // 保护性降级：输出过大时，不解压，避免把巨大响应读入内存。
                    return body;
                }
                out.extend_from_slice(&buf[..n]);
            }
            Err(_) => {
                // 容错：忽略解压错误（例如 gzip 流被提前截断），尽可能返回已产出的部分数据。
                if !had_any_output {
                    return body;
                }
                break;
            }
        }
    }

    headers.remove(header::CONTENT_ENCODING);
    headers.remove(header::CONTENT_LENGTH);
    Bytes::from(out)
}

fn build_response(status: StatusCode, headers: &HeaderMap, trace_id: &str, body: Body) -> Response {
    let mut builder = Response::builder().status(status);
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    builder = builder.header("x-trace-id", trace_id);

    match builder.body(body) {
        Ok(r) => r,
        Err(_) => {
            let mut fallback =
                (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR").into_response();
            fallback.headers_mut().insert(
                "x-trace-id",
                HeaderValue::from_str(trace_id).unwrap_or(HeaderValue::from_static("unknown")),
            );
            fallback
        }
    }
}

fn replace_model_in_query(query: &str, model: &str) -> String {
    let encoded = encode_url_component(model);
    let mut changed = false;
    let mut out: Vec<String> = Vec::new();

    for part in query.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            out.push(part.to_string());
            continue;
        };
        if key == "model" {
            out.push(format!("model={encoded}"));
            changed = changed || value != encoded;
        } else {
            out.push(part.to_string());
        }
    }

    if !changed {
        return query.to_string();
    }
    out.join("&")
}

fn replace_model_in_path(path: &str, model: &str) -> Option<String> {
    let needle = "/models/";
    let idx = path.find(needle)?;
    let start = idx + needle.len();
    let rest = &path[start..];
    if rest.is_empty() {
        return None;
    }
    let end_rel = rest.find(['/', ':', '?']).unwrap_or(rest.len());
    let end = start + end_rel;

    let mut out = String::with_capacity(path.len().saturating_add(model.len()));
    out.push_str(&path[..start]);
    out.push_str(&encode_url_component(model));
    out.push_str(&path[end..]);
    Some(out)
}

fn replace_model_in_body_json(root: &mut serde_json::Value, model: &str) -> bool {
    let Some(obj) = root.as_object_mut() else {
        return false;
    };

    let replacement = serde_json::Value::String(model.to_string());
    match obj.get_mut("model") {
        Some(current) => match current {
            serde_json::Value::String(_) => {
                *current = replacement;
                true
            }
            serde_json::Value::Object(m) => {
                if m.get("name").and_then(|v| v.as_str()).is_some() {
                    m.insert("name".to_string(), replacement);
                    return true;
                }
                if m.get("id").and_then(|v| v.as_str()).is_some() {
                    m.insert("id".to_string(), replacement);
                    return true;
                }

                *current = replacement;
                true
            }
            _ => {
                *current = replacement;
                true
            }
        },
        None => {
            obj.insert("model".to_string(), replacement);
            true
        }
    }
}

const PROVIDER_BASE_URL_PING_TIMEOUT_MS: u64 = 2000;

async fn select_provider_base_url_for_request(
    state: &GatewayAppState,
    provider: &providers::ProviderForGateway,
    cache_ttl_seconds: u32,
) -> String {
    let primary = provider
        .base_urls
        .first()
        .cloned()
        .unwrap_or_else(String::new);

    if !matches!(provider.base_url_mode, providers::ProviderBaseUrlMode::Ping) {
        return primary;
    }

    if provider.base_urls.len() <= 1 {
        return primary;
    }

    let now_unix_ms = now_unix_millis();
    if let Ok(mut cache) = state.latency_cache.lock() {
        if let Some(best) =
            cache.get_valid_best_base_url(provider.id, now_unix_ms, &provider.base_urls)
        {
            return best;
        }
    }

    let ttl_ms = (cache_ttl_seconds.max(1) as u64).saturating_mul(1000);
    let expires_at_unix_ms = now_unix_ms.saturating_add(ttl_ms);
    let timeout = Duration::from_millis(PROVIDER_BASE_URL_PING_TIMEOUT_MS);

    let mut join_set = tokio::task::JoinSet::new();
    for base_url in provider.base_urls.iter().cloned() {
        let client = state.client.clone();
        join_set.spawn(async move {
            let result =
                crate::base_url_probe::probe_base_url_ms(&client, &base_url, timeout).await;
            (base_url, result)
        });
    }

    let mut best: Option<(String, u64)> = None;
    while let Some(joined) = join_set.join_next().await {
        let Ok((base_url, result)) = joined else {
            continue;
        };
        let Ok(ms) = result else {
            continue;
        };

        match best.as_ref() {
            Some((_, best_ms)) if ms >= *best_ms => {}
            _ => best = Some((base_url, ms)),
        }
    }

    let Some((best_base_url, _best_latency_ms)) = best else {
        return primary;
    };

    if let Ok(mut cache) = state.latency_cache.lock() {
        cache.put_best_base_url(provider.id, best_base_url.clone(), expires_at_unix_ms);
    }

    best_base_url
}

#[derive(Debug, Clone)]
struct CliProxyEnabledCacheEntry {
    enabled: bool,
    error: Option<String>,
    expires_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
struct CliProxyEnabledSnapshot {
    enabled: bool,
    error: Option<String>,
    cache_hit: bool,
    cache_ttl_ms: i64,
}

fn cli_proxy_enabled_cached(app: &tauri::AppHandle, cli_key: &str) -> CliProxyEnabledSnapshot {
    static CLI_PROXY_ENABLED_CACHE: OnceLock<Mutex<HashMap<String, CliProxyEnabledCacheEntry>>> =
        OnceLock::new();

    let now_unix_ms = now_unix_millis().min(i64::MAX as u64) as i64;
    let cache = CLI_PROXY_ENABLED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let cache = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = cache.get(cli_key) {
            if entry.expires_at_unix_ms > now_unix_ms {
                let cache_ttl_ms = if entry.error.is_some() {
                    CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR
                } else {
                    CLI_PROXY_ENABLED_CACHE_TTL_MS_OK
                };
                return CliProxyEnabledSnapshot {
                    enabled: entry.enabled,
                    error: entry.error.clone(),
                    cache_hit: true,
                    cache_ttl_ms,
                };
            }
        }
    }

    let (enabled, error) = match cli_proxy::is_enabled(app, cli_key) {
        Ok(v) => (v, None),
        Err(err) => (false, Some(err)),
    };
    let cache_ttl_ms = if error.is_some() {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_ERR
    } else {
        CLI_PROXY_ENABLED_CACHE_TTL_MS_OK
    };

    {
        let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.insert(
            cli_key.to_string(),
            CliProxyEnabledCacheEntry {
                enabled,
                error: error.clone(),
                expires_at_unix_ms: now_unix_ms.saturating_add(cache_ttl_ms.max(1)),
            },
        );
    }

    CliProxyEnabledSnapshot {
        enabled,
        error,
        cache_hit: false,
        cache_ttl_ms,
    }
}

pub(super) async fn proxy_impl(
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

    if matches!(cli_key.as_str(), "claude" | "codex" | "gemini") {
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

            emit_request_event(
                &state.app,
                trace_id.clone(),
                cli_key.clone(),
                method_hint.clone(),
                forwarded_path.clone(),
                query.clone(),
                Some(StatusCode::FORBIDDEN.as_u16()),
                Some(ErrorCategory::NonRetryableClientError.as_str()),
                Some("GW_CLI_PROXY_DISABLED"),
                started.elapsed().as_millis(),
                None,
                vec![],
                None,
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

            enqueue_request_log_with_backpressure(
                &state.app,
                &state.log_tx,
                RequestLogEnqueueArgs {
                    trace_id,
                    cli_key,
                    session_id: None,
                    method: method_hint,
                    path: forwarded_path,
                    query,
                    excluded_from_stats: true,
                    special_settings_json: Some(special_settings_json),
                    status: Some(StatusCode::FORBIDDEN.as_u16()),
                    error_code: Some("GW_CLI_PROXY_DISABLED"),
                    duration_ms: started.elapsed().as_millis(),
                    ttfb_ms: None,
                    attempts_json: "[]".to_string(),
                    requested_model: None,
                    created_at_ms,
                    created_at,
                    usage: None,
                },
            )
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
            emit_request_event(
                &state.app,
                trace_id.clone(),
                cli_key.clone(),
                method_hint.clone(),
                forwarded_path.clone(),
                query.clone(),
                Some(StatusCode::PAYLOAD_TOO_LARGE.as_u16()),
                None,
                Some("GW_BODY_TOO_LARGE"),
                started.elapsed().as_millis(),
                None,
                vec![],
                None,
            );

            enqueue_request_log_with_backpressure(
                &state.app,
                &state.log_tx,
                RequestLogEnqueueArgs {
                    trace_id,
                    cli_key,
                    session_id: None,
                    method: method_hint,
                    path: forwarded_path,
                    query,
                    excluded_from_stats: false,
                    special_settings_json: None,
                    status: Some(StatusCode::PAYLOAD_TOO_LARGE.as_u16()),
                    error_code: Some("GW_BODY_TOO_LARGE"),
                    duration_ms: started.elapsed().as_millis(),
                    ttfb_ms: None,
                    attempts_json: "[]".to_string(),
                    requested_model: None,
                    created_at_ms,
                    created_at,
                    usage: None,
                },
            )
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
        .unwrap_or(false);
    let enable_response_fixer = settings_cfg
        .as_ref()
        .map(|cfg| cfg.enable_response_fixer)
        .unwrap_or(false);
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
    let provider_base_url_ping_cache_ttl_seconds = settings_cfg
        .as_ref()
        .map(|cfg| cfg.provider_base_url_ping_cache_ttl_seconds)
        .unwrap_or(settings::DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS);
    let enable_codex_session_id_completion = settings_cfg
        .as_ref()
        .map(|cfg| cfg.enable_codex_session_id_completion)
        .unwrap_or(false);

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
        emit_request_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            Some(StatusCode::OK.as_u16()),
            None,
            None,
            duration_ms,
            Some(duration_ms),
            vec![],
            Some(usage::UsageMetrics::default()),
        );

        let attempts_json = serde_json::json!([{
            "provider_id": 0,
            "provider_name": "Warmup",
            "outcome": "success",
            "session_reuse": false,
        }])
        .to_string();

        let insert = request_logs::RequestLogInsert {
            trace_id: trace_id.clone(),
            cli_key: cli_key.clone(),
            session_id: None,
            method: method_hint.clone(),
            path: forwarded_path.clone(),
            query: query.clone(),
            excluded_from_stats: true,
            special_settings_json: Some(special_settings_json),
            status: Some(StatusCode::OK.as_u16() as i64),
            error_code: None,
            duration_ms: duration_ms.min(i64::MAX as u128) as i64,
            ttfb_ms: Some(duration_ms.min(i64::MAX as u128) as i64),
            attempts_json,
            input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(0),
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
            cache_creation_5m_input_tokens: Some(0),
            cache_creation_1h_input_tokens: Some(0),
            usage_json: None,
            requested_model: requested_model.clone(),
            created_at_ms,
            created_at,
        };

        let _ = state.log_tx.try_send(insert);

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
    let allow_session_reuse = should_reuse_provider(introspection_json.as_ref());

    let respond_invalid_cli_key = |err: String| -> Response {
        let resp = error_response(
            StatusCode::BAD_REQUEST,
            trace_id.clone(),
            "GW_INVALID_CLI_KEY",
            err,
            vec![],
        );

        emit_request_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            Some(StatusCode::BAD_REQUEST.as_u16()),
            None,
            Some("GW_INVALID_CLI_KEY"),
            started.elapsed().as_millis(),
            None,
            vec![],
            None,
        );

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
                &state.app,
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
                match providers::list_enabled_for_gateway_using_active_mode(&state.app, &cli_key) {
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
                for provider_id in order.iter().copied() {
                    if let Some(item) = by_id.remove(&provider_id) {
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
        emit_request_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            None,
            Some("GW_NO_ENABLED_PROVIDER"),
            started.elapsed().as_millis(),
            None,
            vec![],
            None,
        );

        enqueue_request_log_with_backpressure(
            &state.app,
            &state.log_tx,
            RequestLogEnqueueArgs {
                trace_id,
                cli_key,
                session_id: session_id.clone(),
                method: method_hint,
                path: forwarded_path,
                query,
                excluded_from_stats: false,
                special_settings_json: None,
                status: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                error_code: Some("GW_NO_ENABLED_PROVIDER"),
                duration_ms: started.elapsed().as_millis(),
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                requested_model,
                created_at_ms,
                created_at,
                usage: None,
            },
        )
        .await;
        return resp;
    }

    if let Some(model) = requested_model.as_deref() {
        let candidates_before = providers.len();
        let mut filtered_total = 0usize;
        let mut filtered_preview: Vec<serde_json::Value> = Vec::new();
        providers.retain(|p| {
            let ok = p.is_model_supported(model);
            if !ok {
                filtered_total = filtered_total.saturating_add(1);
                if filtered_preview.len() < 50 {
                    filtered_preview.push(serde_json::json!({
                        "id": p.id,
                        "name": p.name.clone(),
                    }));
                }
            }
            ok
        });

        if providers.is_empty() {
            let message =
                format!("no provider supports requested_model={model} (candidates={candidates_before}, filtered={filtered_total})");

            let location = requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
            if let Ok(mut settings) = special_settings.lock() {
                settings.push(serde_json::json!({
                    "type": "model_filter",
                    "scope": "request",
                    "hit": true,
                    "requestedModel": model,
                    "location": match location {
                        RequestedModelLocation::BodyJson => "body",
                        RequestedModelLocation::Query => "query",
                        RequestedModelLocation::Path => "path",
                    },
                    "candidatesCount": candidates_before,
                    "filteredCount": filtered_total,
                    "filteredProvidersPreview": filtered_preview,
                }));
            }
            let special_settings_json = response_fixer::special_settings_json(&special_settings);
            let resp = error_response(
                StatusCode::NOT_FOUND,
                trace_id.clone(),
                "GW_NO_PROVIDER_FOR_MODEL",
                message,
                vec![],
            );

            emit_request_event(
                &state.app,
                trace_id.clone(),
                cli_key.clone(),
                method_hint.clone(),
                forwarded_path.clone(),
                query.clone(),
                Some(StatusCode::NOT_FOUND.as_u16()),
                None,
                Some("GW_NO_PROVIDER_FOR_MODEL"),
                started.elapsed().as_millis(),
                None,
                vec![],
                None,
            );

            enqueue_request_log_with_backpressure(
                &state.app,
                &state.log_tx,
                RequestLogEnqueueArgs {
                    trace_id,
                    cli_key,
                    session_id: session_id.clone(),
                    method: method_hint,
                    path: forwarded_path,
                    query,
                    excluded_from_stats: false,
                    special_settings_json,
                    status: Some(StatusCode::NOT_FOUND.as_u16()),
                    error_code: Some("GW_NO_PROVIDER_FOR_MODEL"),
                    duration_ms: started.elapsed().as_millis(),
                    ttfb_ms: None,
                    attempts_json: "[]".to_string(),
                    requested_model,
                    created_at_ms,
                    created_at,
                    usage: None,
                },
            )
            .await;

            return resp;
        }
    }

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

        if let Some(entry) = cached_error.as_ref() {
            trace_id = entry.trace_id.clone();
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

        if let Some(entry) = cached_error {
            let any_allowed = providers
                .iter()
                .any(|p| state.circuit.should_allow(p.id, now_unix).allow);
            if !any_allowed {
                return error_response_with_retry_after(
                    entry.status,
                    entry.trace_id,
                    entry.error_code,
                    entry.message,
                    vec![],
                    entry.retry_after_seconds,
                );
            }
        }
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

    let mut abort_guard = RequestAbortGuard::new(
        state.app.clone(),
        state.log_tx.clone(),
        trace_id.clone(),
        cli_key.clone(),
        method_hint.clone(),
        forwarded_path.clone(),
        query.clone(),
        created_at_ms,
        created_at,
        started,
    );

    let mut base_headers = headers;
    strip_hop_headers(&mut base_headers);
    base_headers.remove(header::HOST);
    base_headers.remove(header::CONTENT_LENGTH);
    base_headers.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("identity"),
    );

    let (
        mut max_attempts_per_provider,
        max_providers_to_try,
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

    if cli_key == "claude" && enable_thinking_signature_rectifier {
        max_attempts_per_provider = max_attempts_per_provider.max(2);
    }

    let upstream_first_byte_timeout = if upstream_first_byte_timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(upstream_first_byte_timeout_secs as u64))
    };
    let upstream_stream_idle_timeout = if upstream_stream_idle_timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(
            upstream_stream_idle_timeout_secs as u64,
        ))
    };
    let upstream_request_timeout_non_streaming = if upstream_request_timeout_non_streaming_secs == 0
    {
        None
    } else {
        Some(Duration::from_secs(
            upstream_request_timeout_non_streaming_secs as u64,
        ))
    };

    let mut attempts: Vec<FailoverAttempt> = Vec::new();
    let mut failed_provider_ids: HashSet<i64> = HashSet::new();
    let mut last_error_category: Option<&'static str> = None;
    let mut last_error_code: Option<&'static str> = None;

    let max_providers_to_try = (max_providers_to_try as usize).max(1);
    let mut providers_tried: usize = 0;
    let mut earliest_available_unix: Option<i64> = None;
    let mut skipped_open: usize = 0;
    let mut skipped_cooldown: usize = 0;

    for provider in providers.iter() {
        if providers_tried >= max_providers_to_try {
            break;
        }

        let provider_id = provider.id;
        let provider_name_base = provider.name.clone();
        let provider_base_url_display = provider
            .base_urls
            .first()
            .cloned()
            .unwrap_or_else(String::new);

        if failed_provider_ids.contains(&provider_id) {
            continue;
        }

        let now_unix = now_unix_seconds() as i64;
        let allow = state.circuit.should_allow(provider_id, now_unix);
        if let Some(t) = allow.transition.as_ref() {
            emit_circuit_transition(
                &state.app,
                &trace_id,
                &cli_key,
                provider_id,
                &provider_name_base,
                &provider_base_url_display,
                t,
                now_unix,
            );
        }
        if !allow.allow {
            let snap = allow.after;
            let reason = if snap.state == circuit_breaker::CircuitState::Open {
                skipped_open = skipped_open.saturating_add(1);
                "SKIP_OPEN"
            } else {
                skipped_cooldown = skipped_cooldown.saturating_add(1);
                "SKIP_COOLDOWN"
            };

            if let Some(until) = snap.cooldown_until.or(snap.open_until) {
                if until > now_unix {
                    earliest_available_unix = Some(match earliest_available_unix {
                        Some(cur) => cur.min(until),
                        None => until,
                    });
                }
            }

            emit_circuit_event(
                &state.app,
                GatewayCircuitEvent {
                    trace_id: trace_id.clone(),
                    cli_key: cli_key.clone(),
                    provider_id,
                    provider_name: provider_name_base.clone(),
                    base_url: provider_base_url_display.clone(),
                    prev_state: snap.state.as_str(),
                    next_state: snap.state.as_str(),
                    failure_count: snap.failure_count,
                    failure_threshold: snap.failure_threshold,
                    open_until: snap.open_until,
                    cooldown_until: snap.cooldown_until,
                    reason,
                    ts: now_unix,
                },
            );
            continue;
        }

        let provider_base_url_base = select_provider_base_url_for_request(
            &state,
            provider,
            provider_base_url_ping_cache_ttl_seconds,
        )
        .await;

        let mut circuit_snapshot = allow.after.clone();

        providers_tried = providers_tried.saturating_add(1);
        let provider_index = providers_tried as u32;
        let session_reuse = match session_bound_provider_id {
            Some(id) => (id == provider_id && provider_index == 1).then_some(true),
            None => None,
        };

        let mut upstream_forwarded_path = forwarded_path.clone();
        let mut upstream_query = query.clone();
        let mut upstream_body_bytes = body_bytes.clone();
        let mut strip_request_content_encoding = strip_request_content_encoding_seed;
        let mut thinking_signature_rectifier_retried = false;

        if let Some(requested_model) = requested_model.as_deref() {
            let effective_model = provider.get_effective_model(requested_model);
            if effective_model != requested_model {
                let location = requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
                match location {
                    RequestedModelLocation::BodyJson => {
                        if let Some(root) = introspection_json.as_ref() {
                            let mut next = root.clone();
                            let replaced = replace_model_in_body_json(&mut next, &effective_model);
                            if replaced {
                                if let Ok(bytes) = serde_json::to_vec(&next) {
                                    upstream_body_bytes = Bytes::from(bytes);
                                    strip_request_content_encoding = true;
                                }
                            }
                        }
                    }
                    RequestedModelLocation::Query => {
                        if let Some(q) = upstream_query.as_deref() {
                            upstream_query = Some(replace_model_in_query(q, &effective_model));
                        }
                    }
                    RequestedModelLocation::Path => {
                        if let Some(next_path) =
                            replace_model_in_path(&upstream_forwarded_path, &effective_model)
                        {
                            upstream_forwarded_path = next_path;
                        }
                    }
                }

                if let Ok(mut settings) = special_settings.lock() {
                    settings.push(serde_json::json!({
                        "type": "model_mapping",
                        "scope": "attempt",
                        "hit": true,
                        "providerId": provider_id,
                        "providerName": provider_name_base.clone(),
                        "requestedModel": requested_model,
                        "effectiveModel": effective_model,
                        "location": match location {
                            RequestedModelLocation::BodyJson => "body",
                            RequestedModelLocation::Query => "query",
                            RequestedModelLocation::Path => "path",
                        },
                    }));
                }
            }
        }

        for retry_index in 1..=max_attempts_per_provider {
            let attempt_index = attempts.len().saturating_add(1) as u32;
            let attempt_started_ms = started.elapsed().as_millis();
            let attempt_started = Instant::now();
            let circuit_before = circuit_snapshot.clone();

            let url = match build_target_url(
                &provider_base_url_base,
                &upstream_forwarded_path,
                upstream_query.as_deref(),
            ) {
                Ok(u) => u,
                Err(err) => {
                    let category = ErrorCategory::SystemError;
                    let error_code = "GW_INTERNAL_ERROR";
                    let decision = FailoverDecision::SwitchProvider;

                    let outcome = format!(
                        "build_target_url_error: category={} code={} decision={} err={err}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
                    );

                    attempts.push(FailoverAttempt {
                        provider_id,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: None,
                        provider_index: Some(provider_index),
                        retry_index: Some(retry_index),
                        session_reuse,
                        error_category: Some(category.as_str()),
                        error_code: Some(error_code),
                        decision: Some(decision.as_str()),
                        reason: Some("invalid base_url".to_string()),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    });

                    let attempt_event = GatewayAttemptEvent {
                        trace_id: trace_id.clone(),
                        cli_key: cli_key.clone(),
                        method: method_hint.clone(),
                        path: forwarded_path.clone(),
                        query: query.clone(),
                        attempt_index,
                        provider_id,
                        session_reuse,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome,
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    failed_provider_ids.insert(provider_id);
                    break;
                }
            };

            // Realtime routing UX: emit an attempt event as soon as a provider is selected (before awaiting upstream).
            // This enables the Home page to display the current routed provider immediately, similar to claude-code-hub.
            //
            // Note: do NOT enqueue attempt_logs for this "started" event (avoid DB noise/IO); completion events still get persisted.
            emit_attempt_event(
                &state.app,
                GatewayAttemptEvent {
                    trace_id: trace_id.clone(),
                    cli_key: cli_key.clone(),
                    method: method_hint.clone(),
                    path: forwarded_path.clone(),
                    query: query.clone(),
                    attempt_index,
                    provider_id,
                    session_reuse,
                    provider_name: provider_name_base.clone(),
                    base_url: provider_base_url_base.clone(),
                    outcome: "started".to_string(),
                    status: None,
                    attempt_started_ms,
                    attempt_duration_ms: 0,
                    circuit_state_before: Some(circuit_before.state.as_str()),
                    circuit_state_after: None,
                    circuit_failure_count: Some(circuit_before.failure_count),
                    circuit_failure_threshold: Some(circuit_before.failure_threshold),
                },
            );

            let mut headers = base_headers.clone();
            inject_provider_auth(&cli_key, &provider.api_key_plaintext, &mut headers);
            if strip_request_content_encoding {
                headers.remove(header::CONTENT_ENCODING);
            }

            enum SendResult {
                Ok(reqwest::Response),
                Err(reqwest::Error),
                Timeout,
            }

            let send_result = if let Some(timeout) = upstream_first_byte_timeout {
                let send = state
                    .client
                    .request(method.clone(), url)
                    .headers(headers)
                    .body(upstream_body_bytes.clone())
                    .send();
                match tokio::time::timeout(timeout, send).await {
                    Ok(Ok(resp)) => SendResult::Ok(resp),
                    Ok(Err(err)) => SendResult::Err(err),
                    Err(_) => SendResult::Timeout,
                }
            } else {
                match state
                    .client
                    .request(method.clone(), url)
                    .headers(headers)
                    .body(upstream_body_bytes.clone())
                    .send()
                    .await
                {
                    Ok(resp) => SendResult::Ok(resp),
                    Err(err) => SendResult::Err(err),
                }
            };

            match send_result {
                SendResult::Ok(resp) => {
                    let status = resp.status();
                    let mut response_headers = resp.headers().clone();

                    if status.is_success() {
                        if is_event_stream(&response_headers) {
                            strip_hop_headers(&mut response_headers);

                            let mut resp = resp;

                            enum FirstChunkProbe {
                                Skipped,
                                Ok(Option<Bytes>, Option<u128>),
                                ReadError(reqwest::Error),
                                Timeout,
                            }

                            let probe = match upstream_first_byte_timeout {
                                Some(total) => {
                                    let elapsed = attempt_started.elapsed();
                                    if elapsed >= total {
                                        FirstChunkProbe::Timeout
                                    } else {
                                        let remaining = total - elapsed;
                                        match tokio::time::timeout(remaining, resp.chunk()).await {
                                            Ok(Ok(Some(chunk))) => FirstChunkProbe::Ok(
                                                Some(chunk),
                                                Some(started.elapsed().as_millis()),
                                            ),
                                            Ok(Ok(None)) => FirstChunkProbe::Ok(None, None),
                                            Ok(Err(err)) => FirstChunkProbe::ReadError(err),
                                            Err(_) => FirstChunkProbe::Timeout,
                                        }
                                    }
                                }
                                None => FirstChunkProbe::Skipped,
                            };
                            let probe_is_empty_event_stream =
                                matches!(probe, FirstChunkProbe::Ok(None, None));

                            let mut first_chunk: Option<Bytes> = None;
                            let mut initial_first_byte_ms: Option<u128> = None;

                            match probe {
                                FirstChunkProbe::Ok(chunk, ttfb_ms) => {
                                    first_chunk = chunk;
                                    initial_first_byte_ms = ttfb_ms;
                                }
                                FirstChunkProbe::ReadError(err) => {
                                    let category = ErrorCategory::SystemError;
                                    let error_code = "GW_STREAM_ERROR";
                                    let decision = if retry_index < max_attempts_per_provider {
                                        FailoverDecision::RetrySameProvider
                                    } else {
                                        FailoverDecision::SwitchProvider
                                    };

                                    let outcome = format!(
                                        "stream_first_chunk_error: category={} code={} decision={} timeout_secs={}",
                                        category.as_str(),
                                        error_code,
                                        decision.as_str(),
                                        upstream_first_byte_timeout_secs,
                                    );

                                    attempts.push(FailoverAttempt {
                                        provider_id,
                                        provider_name: provider_name_base.clone(),
                                        base_url: provider_base_url_base.clone(),
                                        outcome: outcome.clone(),
                                        status: Some(status.as_u16()),
                                        provider_index: Some(provider_index),
                                        retry_index: Some(retry_index),
                                        session_reuse,
                                        error_category: Some(category.as_str()),
                                        error_code: Some(error_code),
                                        decision: Some(decision.as_str()),
                                        reason: Some(format!(
                                            "first chunk read error (event-stream): {err}"
                                        )),
                                        attempt_started_ms: Some(attempt_started_ms),
                                        attempt_duration_ms: Some(
                                            attempt_started.elapsed().as_millis(),
                                        ),
                                        circuit_state_before: Some(circuit_before.state.as_str()),
                                        circuit_state_after: None,
                                        circuit_failure_count: Some(circuit_before.failure_count),
                                        circuit_failure_threshold: Some(
                                            circuit_before.failure_threshold,
                                        ),
                                    });

                                    let attempt_event = GatewayAttemptEvent {
                                        trace_id: trace_id.clone(),
                                        cli_key: cli_key.clone(),
                                        method: method_hint.clone(),
                                        path: forwarded_path.clone(),
                                        query: query.clone(),
                                        attempt_index,
                                        provider_id,
                                        session_reuse,
                                        provider_name: provider_name_base.clone(),
                                        base_url: provider_base_url_base.clone(),
                                        outcome,
                                        status: Some(status.as_u16()),
                                        attempt_started_ms,
                                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                        circuit_state_before: Some(circuit_before.state.as_str()),
                                        circuit_state_after: None,
                                        circuit_failure_count: Some(circuit_before.failure_count),
                                        circuit_failure_threshold: Some(
                                            circuit_before.failure_threshold,
                                        ),
                                    };
                                    emit_attempt_event(&state.app, attempt_event.clone());
                                    enqueue_attempt_log_with_backpressure(
                                        &state.app,
                                        &state.attempt_log_tx,
                                        &attempt_event,
                                        created_at,
                                    )
                                    .await;

                                    last_error_category = Some(category.as_str());
                                    last_error_code = Some(error_code);

                                    if provider_cooldown_secs > 0
                                        && matches!(
                                            decision,
                                            FailoverDecision::SwitchProvider
                                                | FailoverDecision::Abort
                                        )
                                    {
                                        let now_unix = now_unix_seconds() as i64;
                                        let snap = state.circuit.trigger_cooldown(
                                            provider_id,
                                            now_unix,
                                            provider_cooldown_secs,
                                        );
                                        circuit_snapshot = snap;
                                    }

                                    match decision {
                                        FailoverDecision::RetrySameProvider => continue,
                                        FailoverDecision::SwitchProvider => {
                                            failed_provider_ids.insert(provider_id);
                                            break;
                                        }
                                        FailoverDecision::Abort => break,
                                    }
                                }
                                FirstChunkProbe::Timeout => {
                                    let category = ErrorCategory::SystemError;
                                    let error_code = "GW_UPSTREAM_TIMEOUT";
                                    let decision = if retry_index < max_attempts_per_provider {
                                        FailoverDecision::RetrySameProvider
                                    } else {
                                        FailoverDecision::SwitchProvider
                                    };

                                    let outcome = format!(
                                        "stream_first_byte_timeout: category={} code={} decision={} timeout_secs={}",
                                        category.as_str(),
                                        error_code,
                                        decision.as_str(),
                                        upstream_first_byte_timeout_secs,
                                    );

                                    attempts.push(FailoverAttempt {
                                        provider_id,
                                        provider_name: provider_name_base.clone(),
                                        base_url: provider_base_url_base.clone(),
                                        outcome: outcome.clone(),
                                        status: Some(status.as_u16()),
                                        provider_index: Some(provider_index),
                                        retry_index: Some(retry_index),
                                        session_reuse,
                                        error_category: Some(category.as_str()),
                                        error_code: Some(error_code),
                                        decision: Some(decision.as_str()),
                                        reason: Some(
                                            "first byte timeout (event-stream)".to_string(),
                                        ),
                                        attempt_started_ms: Some(attempt_started_ms),
                                        attempt_duration_ms: Some(
                                            attempt_started.elapsed().as_millis(),
                                        ),
                                        circuit_state_before: Some(circuit_before.state.as_str()),
                                        circuit_state_after: None,
                                        circuit_failure_count: Some(circuit_before.failure_count),
                                        circuit_failure_threshold: Some(
                                            circuit_before.failure_threshold,
                                        ),
                                    });

                                    let attempt_event = GatewayAttemptEvent {
                                        trace_id: trace_id.clone(),
                                        cli_key: cli_key.clone(),
                                        method: method_hint.clone(),
                                        path: forwarded_path.clone(),
                                        query: query.clone(),
                                        attempt_index,
                                        provider_id,
                                        session_reuse,
                                        provider_name: provider_name_base.clone(),
                                        base_url: provider_base_url_base.clone(),
                                        outcome,
                                        status: Some(status.as_u16()),
                                        attempt_started_ms,
                                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                        circuit_state_before: Some(circuit_before.state.as_str()),
                                        circuit_state_after: None,
                                        circuit_failure_count: Some(circuit_before.failure_count),
                                        circuit_failure_threshold: Some(
                                            circuit_before.failure_threshold,
                                        ),
                                    };
                                    emit_attempt_event(&state.app, attempt_event.clone());
                                    enqueue_attempt_log_with_backpressure(
                                        &state.app,
                                        &state.attempt_log_tx,
                                        &attempt_event,
                                        created_at,
                                    )
                                    .await;

                                    last_error_category = Some(category.as_str());
                                    last_error_code = Some(error_code);

                                    if provider_cooldown_secs > 0
                                        && matches!(
                                            decision,
                                            FailoverDecision::SwitchProvider
                                                | FailoverDecision::Abort
                                        )
                                    {
                                        let now_unix = now_unix_seconds() as i64;
                                        let snap = state.circuit.trigger_cooldown(
                                            provider_id,
                                            now_unix,
                                            provider_cooldown_secs,
                                        );
                                        circuit_snapshot = snap;
                                    }

                                    match decision {
                                        FailoverDecision::RetrySameProvider => continue,
                                        FailoverDecision::SwitchProvider => {
                                            failed_provider_ids.insert(provider_id);
                                            break;
                                        }
                                        FailoverDecision::Abort => break,
                                    }
                                }
                                FirstChunkProbe::Skipped => {}
                            }

                            if upstream_first_byte_timeout.is_some()
                                && first_chunk.is_none()
                                && initial_first_byte_ms.is_none()
                                && probe_is_empty_event_stream
                            {
                                let category = ErrorCategory::SystemError;
                                let error_code = "GW_STREAM_ERROR";
                                let decision = if retry_index < max_attempts_per_provider {
                                    FailoverDecision::RetrySameProvider
                                } else {
                                    FailoverDecision::SwitchProvider
                                };

                                let outcome = format!(
                                    "stream_first_chunk_eof: category={} code={} decision={} timeout_secs={}",
                                    category.as_str(),
                                    error_code,
                                    decision.as_str(),
                                    upstream_first_byte_timeout_secs,
                                );

                                attempts.push(FailoverAttempt {
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome: outcome.clone(),
                                    status: Some(status.as_u16()),
                                    provider_index: Some(provider_index),
                                    retry_index: Some(retry_index),
                                    session_reuse,
                                    error_category: Some(category.as_str()),
                                    error_code: Some(error_code),
                                    decision: Some(decision.as_str()),
                                    reason: Some(
                                        "upstream returned empty event-stream".to_string(),
                                    ),
                                    attempt_started_ms: Some(attempt_started_ms),
                                    attempt_duration_ms: Some(
                                        attempt_started.elapsed().as_millis(),
                                    ),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                });

                                let attempt_event = GatewayAttemptEvent {
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    attempt_index,
                                    provider_id,
                                    session_reuse,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome,
                                    status: Some(status.as_u16()),
                                    attempt_started_ms,
                                    attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                };
                                emit_attempt_event(&state.app, attempt_event.clone());
                                enqueue_attempt_log_with_backpressure(
                                    &state.app,
                                    &state.attempt_log_tx,
                                    &attempt_event,
                                    created_at,
                                )
                                .await;

                                last_error_category = Some(category.as_str());
                                last_error_code = Some(error_code);

                                if provider_cooldown_secs > 0
                                    && matches!(
                                        decision,
                                        FailoverDecision::SwitchProvider | FailoverDecision::Abort
                                    )
                                {
                                    let now_unix = now_unix_seconds() as i64;
                                    let snap = state.circuit.trigger_cooldown(
                                        provider_id,
                                        now_unix,
                                        provider_cooldown_secs,
                                    );
                                    circuit_snapshot = snap;
                                }

                                match decision {
                                    FailoverDecision::RetrySameProvider => continue,
                                    FailoverDecision::SwitchProvider => {
                                        failed_provider_ids.insert(provider_id);
                                        break;
                                    }
                                    FailoverDecision::Abort => break,
                                }
                            }

                            let outcome = "success".to_string();

                            attempts.push(FailoverAttempt {
                                provider_id,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_base.clone(),
                                outcome: outcome.clone(),
                                status: Some(status.as_u16()),
                                provider_index: Some(provider_index),
                                retry_index: Some(retry_index),
                                session_reuse,
                                error_category: None,
                                error_code: None,
                                decision: Some("success"),
                                reason: None,
                                attempt_started_ms: Some(attempt_started_ms),
                                attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                                circuit_state_before: Some(circuit_before.state.as_str()),
                                circuit_state_after: None,
                                circuit_failure_count: Some(circuit_before.failure_count),
                                circuit_failure_threshold: Some(circuit_before.failure_threshold),
                            });

                            let attempt_event = GatewayAttemptEvent {
                                trace_id: trace_id.clone(),
                                cli_key: cli_key.clone(),
                                method: method_hint.clone(),
                                path: forwarded_path.clone(),
                                query: query.clone(),
                                attempt_index,
                                provider_id,
                                session_reuse,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_base.clone(),
                                outcome,
                                status: Some(status.as_u16()),
                                attempt_started_ms,
                                attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                circuit_state_before: Some(circuit_before.state.as_str()),
                                circuit_state_after: None,
                                circuit_failure_count: Some(circuit_before.failure_count),
                                circuit_failure_threshold: Some(circuit_before.failure_threshold),
                            };
                            emit_attempt_event(&state.app, attempt_event.clone());
                            enqueue_attempt_log_with_backpressure(
                                &state.app,
                                &state.attempt_log_tx,
                                &attempt_event,
                                created_at,
                            )
                            .await;

                            let attempts_json = serde_json::to_string(&attempts)
                                .unwrap_or_else(|_| "[]".to_string());
                            let ctx = StreamFinalizeCtx {
                                app: state.app.clone(),
                                log_tx: state.log_tx.clone(),
                                circuit: state.circuit.clone(),
                                session: state.session.clone(),
                                session_id: session_id.clone(),
                                sort_mode_id: effective_sort_mode_id,
                                trace_id: trace_id.clone(),
                                cli_key: cli_key.clone(),
                                method: method_hint.clone(),
                                path: forwarded_path.clone(),
                                query: query.clone(),
                                excluded_from_stats: false,
                                special_settings: special_settings.clone(),
                                status: status.as_u16(),
                                error_category: None,
                                error_code: None,
                                started,
                                attempts: attempts.clone(),
                                attempts_json,
                                requested_model: requested_model.clone(),
                                created_at_ms,
                                created_at,
                                provider_cooldown_secs,
                                provider_id,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_base.clone(),
                            };

                            let should_gunzip = has_gzip_content_encoding(&response_headers);
                            if should_gunzip {
                                // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                                response_headers.remove(header::CONTENT_ENCODING);
                                response_headers.remove(header::CONTENT_LENGTH);
                            }

                            let enable_response_fixer_for_this_response = enable_response_fixer
                                && !has_non_identity_content_encoding(&response_headers);

                            if enable_response_fixer_for_this_response {
                                response_headers.remove(header::CONTENT_LENGTH);
                                response_headers.insert(
                                    "x-cch-response-fixer",
                                    HeaderValue::from_static("processed"),
                                );
                            }

                            let use_sse_relay =
                                cli_key == "codex" && forwarded_path == "/v1/responses";

                            let body =
                                match (enable_response_fixer_for_this_response, should_gunzip) {
                                    (true, true) => {
                                        let upstream = GunzipStream::new(FirstChunkStream::new(
                                            first_chunk,
                                            resp.bytes_stream(),
                                        ));
                                        let config = response_fixer::ResponseFixerConfig {
                                            fix_encoding: response_fixer_fix_encoding,
                                            fix_sse_format: response_fixer_fix_sse_format,
                                            fix_truncated_json: response_fixer_fix_truncated_json,
                                            max_json_depth: response_fixer::DEFAULT_MAX_JSON_DEPTH,
                                            max_fix_size: response_fixer::DEFAULT_MAX_FIX_SIZE,
                                        };
                                        let upstream = response_fixer::ResponseFixerStream::new(
                                            upstream,
                                            config,
                                            special_settings.clone(),
                                        );
                                        if use_sse_relay {
                                            spawn_usage_sse_relay_body(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            )
                                        } else {
                                            let stream = UsageSseTeeStream::new(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            );
                                            Body::from_stream(stream)
                                        }
                                    }
                                    (true, false) => {
                                        let upstream =
                                            FirstChunkStream::new(first_chunk, resp.bytes_stream());
                                        let config = response_fixer::ResponseFixerConfig {
                                            fix_encoding: response_fixer_fix_encoding,
                                            fix_sse_format: response_fixer_fix_sse_format,
                                            fix_truncated_json: response_fixer_fix_truncated_json,
                                            max_json_depth: response_fixer::DEFAULT_MAX_JSON_DEPTH,
                                            max_fix_size: response_fixer::DEFAULT_MAX_FIX_SIZE,
                                        };
                                        let upstream = response_fixer::ResponseFixerStream::new(
                                            upstream,
                                            config,
                                            special_settings.clone(),
                                        );
                                        if use_sse_relay {
                                            spawn_usage_sse_relay_body(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            )
                                        } else {
                                            let stream = UsageSseTeeStream::new(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            );
                                            Body::from_stream(stream)
                                        }
                                    }
                                    (false, true) => {
                                        let upstream = GunzipStream::new(FirstChunkStream::new(
                                            first_chunk,
                                            resp.bytes_stream(),
                                        ));
                                        if use_sse_relay {
                                            spawn_usage_sse_relay_body(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            )
                                        } else {
                                            let stream = UsageSseTeeStream::new(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            );
                                            Body::from_stream(stream)
                                        }
                                    }
                                    (false, false) => {
                                        let upstream =
                                            FirstChunkStream::new(first_chunk, resp.bytes_stream());
                                        if use_sse_relay {
                                            spawn_usage_sse_relay_body(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            )
                                        } else {
                                            let stream = UsageSseTeeStream::new(
                                                upstream,
                                                ctx,
                                                upstream_stream_idle_timeout,
                                                initial_first_byte_ms,
                                            );
                                            Body::from_stream(stream)
                                        }
                                    }
                                };

                            let mut builder = Response::builder().status(status);
                            for (k, v) in response_headers.iter() {
                                builder = builder.header(k, v);
                            }
                            builder = builder.header("x-trace-id", trace_id.as_str());

                            abort_guard.disarm();
                            return match builder.body(body) {
                                Ok(r) => r,
                                Err(_) => {
                                    let mut fallback = (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        "GW_RESPONSE_BUILD_ERROR",
                                    )
                                        .into_response();
                                    fallback.headers_mut().insert(
                                        "x-trace-id",
                                        HeaderValue::from_str(&trace_id)
                                            .unwrap_or(HeaderValue::from_static("unknown")),
                                    );
                                    fallback
                                }
                            };
                        }

                        strip_hop_headers(&mut response_headers);
                        let attempts_json =
                            serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());

                        let should_gunzip = has_gzip_content_encoding(&response_headers);

                        match resp.content_length() {
                            Some(len) if len > MAX_NON_SSE_BODY_BYTES as u64 => {
                                let outcome = "success".to_string();

                                attempts.push(FailoverAttempt {
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome: outcome.clone(),
                                    status: Some(status.as_u16()),
                                    provider_index: Some(provider_index),
                                    retry_index: Some(retry_index),
                                    session_reuse,
                                    error_category: None,
                                    error_code: None,
                                    decision: Some("success"),
                                    reason: None,
                                    attempt_started_ms: Some(attempt_started_ms),
                                    attempt_duration_ms: Some(
                                        attempt_started.elapsed().as_millis(),
                                    ),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                });

                                let attempt_event = GatewayAttemptEvent {
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    attempt_index,
                                    provider_id,
                                    session_reuse,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome,
                                    status: Some(status.as_u16()),
                                    attempt_started_ms,
                                    attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                };
                                emit_attempt_event(&state.app, attempt_event.clone());
                                enqueue_attempt_log_with_backpressure(
                                    &state.app,
                                    &state.attempt_log_tx,
                                    &attempt_event,
                                    created_at,
                                )
                                .await;

                                let ctx = StreamFinalizeCtx {
                                    app: state.app.clone(),
                                    log_tx: state.log_tx.clone(),
                                    circuit: state.circuit.clone(),
                                    session: state.session.clone(),
                                    session_id: session_id.clone(),
                                    sort_mode_id: effective_sort_mode_id,
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    excluded_from_stats: false,
                                    special_settings: special_settings.clone(),
                                    status: status.as_u16(),
                                    error_category: None,
                                    error_code: None,
                                    started,
                                    attempts: attempts.clone(),
                                    attempts_json,
                                    requested_model: requested_model.clone(),
                                    created_at_ms,
                                    created_at,
                                    provider_cooldown_secs,
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                };

                                if should_gunzip {
                                    // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                                    response_headers.remove(header::CONTENT_ENCODING);
                                    response_headers.remove(header::CONTENT_LENGTH);
                                }

                                if should_gunzip {
                                    let upstream = GunzipStream::new(resp.bytes_stream());
                                    let stream = TimingOnlyTeeStream::new(
                                        upstream,
                                        ctx,
                                        upstream_request_timeout_non_streaming,
                                    );
                                    let body = Body::from_stream(stream);
                                    abort_guard.disarm();
                                    return build_response(
                                        status,
                                        &response_headers,
                                        &trace_id,
                                        body,
                                    );
                                }

                                let stream = TimingOnlyTeeStream::new(
                                    resp.bytes_stream(),
                                    ctx,
                                    upstream_request_timeout_non_streaming,
                                );
                                let body = Body::from_stream(stream);
                                abort_guard.disarm();
                                return build_response(status, &response_headers, &trace_id, body);
                            }
                            None => {
                                let outcome = "success".to_string();

                                attempts.push(FailoverAttempt {
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome: outcome.clone(),
                                    status: Some(status.as_u16()),
                                    provider_index: Some(provider_index),
                                    retry_index: Some(retry_index),
                                    session_reuse,
                                    error_category: None,
                                    error_code: None,
                                    decision: Some("success"),
                                    reason: None,
                                    attempt_started_ms: Some(attempt_started_ms),
                                    attempt_duration_ms: Some(
                                        attempt_started.elapsed().as_millis(),
                                    ),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                });

                                let attempt_event = GatewayAttemptEvent {
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    attempt_index,
                                    provider_id,
                                    session_reuse,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome,
                                    status: Some(status.as_u16()),
                                    attempt_started_ms,
                                    attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                };
                                emit_attempt_event(&state.app, attempt_event.clone());
                                enqueue_attempt_log_with_backpressure(
                                    &state.app,
                                    &state.attempt_log_tx,
                                    &attempt_event,
                                    created_at,
                                )
                                .await;

                                let ctx = StreamFinalizeCtx {
                                    app: state.app.clone(),
                                    log_tx: state.log_tx.clone(),
                                    circuit: state.circuit.clone(),
                                    session: state.session.clone(),
                                    session_id: session_id.clone(),
                                    sort_mode_id: effective_sort_mode_id,
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    excluded_from_stats: false,
                                    special_settings: special_settings.clone(),
                                    status: status.as_u16(),
                                    error_category: None,
                                    error_code: None,
                                    started,
                                    attempts: attempts.clone(),
                                    attempts_json,
                                    requested_model: requested_model.clone(),
                                    created_at_ms,
                                    created_at,
                                    provider_cooldown_secs,
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                };

                                if should_gunzip {
                                    // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                                    response_headers.remove(header::CONTENT_ENCODING);
                                    response_headers.remove(header::CONTENT_LENGTH);
                                }

                                let body = if should_gunzip {
                                    let upstream = GunzipStream::new(resp.bytes_stream());
                                    let stream = UsageBodyBufferTeeStream::new(
                                        upstream,
                                        ctx,
                                        MAX_NON_SSE_BODY_BYTES,
                                        upstream_request_timeout_non_streaming,
                                    );
                                    Body::from_stream(stream)
                                } else {
                                    let stream = UsageBodyBufferTeeStream::new(
                                        resp.bytes_stream(),
                                        ctx,
                                        MAX_NON_SSE_BODY_BYTES,
                                        upstream_request_timeout_non_streaming,
                                    );
                                    Body::from_stream(stream)
                                };

                                let mut builder = Response::builder().status(status);
                                for (k, v) in response_headers.iter() {
                                    builder = builder.header(k, v);
                                }
                                builder = builder.header("x-trace-id", trace_id.as_str());

                                abort_guard.disarm();
                                return match builder.body(body) {
                                    Ok(r) => r,
                                    Err(_) => {
                                        let mut fallback = (
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            "GW_RESPONSE_BUILD_ERROR",
                                        )
                                            .into_response();
                                        fallback.headers_mut().insert(
                                            "x-trace-id",
                                            HeaderValue::from_str(&trace_id)
                                                .unwrap_or(HeaderValue::from_static("unknown")),
                                        );
                                        fallback
                                    }
                                };
                            }
                            _ => {}
                        }

                        let remaining_total = upstream_request_timeout_non_streaming
                            .and_then(|t| t.checked_sub(started.elapsed()));
                        let bytes_result = match remaining_total {
                            Some(remaining) => {
                                if remaining.is_zero() {
                                    Err("timeout")
                                } else {
                                    match tokio::time::timeout(remaining, resp.bytes()).await {
                                        Ok(Ok(b)) => Ok(b),
                                        Ok(Err(_)) => Err("read_error"),
                                        Err(_) => Err("timeout"),
                                    }
                                }
                            }
                            None => match resp.bytes().await {
                                Ok(b) => Ok(b),
                                Err(_) => Err("read_error"),
                            },
                        };

                        let mut body_bytes = match bytes_result {
                            Ok(b) => b,
                            Err(kind) => {
                                let category = ErrorCategory::SystemError;
                                let error_code = if kind == "timeout" {
                                    "GW_UPSTREAM_TIMEOUT"
                                } else {
                                    "GW_UPSTREAM_READ_ERROR"
                                };
                                let decision = if retry_index < max_attempts_per_provider {
                                    FailoverDecision::RetrySameProvider
                                } else {
                                    FailoverDecision::SwitchProvider
                                };

                                let outcome = format!(
                                    "upstream_body_error: category={} code={} decision={} kind={kind}",
                                    category.as_str(),
                                    error_code,
                                    decision.as_str(),
                                );

                                attempts.push(FailoverAttempt {
                                    provider_id,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome: outcome.clone(),
                                    status: Some(status.as_u16()),
                                    provider_index: Some(provider_index),
                                    retry_index: Some(retry_index),
                                    session_reuse,
                                    error_category: Some(category.as_str()),
                                    error_code: Some(error_code),
                                    decision: Some(decision.as_str()),
                                    reason: Some("failed to read upstream body".to_string()),
                                    attempt_started_ms: Some(attempt_started_ms),
                                    attempt_duration_ms: Some(
                                        attempt_started.elapsed().as_millis(),
                                    ),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                });

                                let attempt_event = GatewayAttemptEvent {
                                    trace_id: trace_id.clone(),
                                    cli_key: cli_key.clone(),
                                    method: method_hint.clone(),
                                    path: forwarded_path.clone(),
                                    query: query.clone(),
                                    attempt_index,
                                    provider_id,
                                    session_reuse,
                                    provider_name: provider_name_base.clone(),
                                    base_url: provider_base_url_base.clone(),
                                    outcome,
                                    status: Some(status.as_u16()),
                                    attempt_started_ms,
                                    attempt_duration_ms: attempt_started.elapsed().as_millis(),
                                    circuit_state_before: Some(circuit_before.state.as_str()),
                                    circuit_state_after: None,
                                    circuit_failure_count: Some(circuit_before.failure_count),
                                    circuit_failure_threshold: Some(
                                        circuit_before.failure_threshold,
                                    ),
                                };
                                emit_attempt_event(&state.app, attempt_event.clone());
                                enqueue_attempt_log_with_backpressure(
                                    &state.app,
                                    &state.attempt_log_tx,
                                    &attempt_event,
                                    created_at,
                                )
                                .await;

                                last_error_category = Some(category.as_str());
                                last_error_code = Some(error_code);

                                if provider_cooldown_secs > 0
                                    && matches!(
                                        decision,
                                        FailoverDecision::SwitchProvider | FailoverDecision::Abort
                                    )
                                {
                                    let now_unix = now_unix_seconds() as i64;
                                    let snap = state.circuit.trigger_cooldown(
                                        provider_id,
                                        now_unix,
                                        provider_cooldown_secs,
                                    );
                                    circuit_snapshot = snap;
                                }

                                match decision {
                                    FailoverDecision::RetrySameProvider => continue,
                                    FailoverDecision::SwitchProvider => {
                                        failed_provider_ids.insert(provider_id);
                                        break;
                                    }
                                    FailoverDecision::Abort => break,
                                }
                            }
                        };

                        let outcome = "success".to_string();

                        attempts.push(FailoverAttempt {
                            provider_id,
                            provider_name: provider_name_base.clone(),
                            base_url: provider_base_url_base.clone(),
                            outcome: outcome.clone(),
                            status: Some(status.as_u16()),
                            provider_index: Some(provider_index),
                            retry_index: Some(retry_index),
                            session_reuse,
                            error_category: None,
                            error_code: None,
                            decision: Some("success"),
                            reason: None,
                            attempt_started_ms: Some(attempt_started_ms),
                            attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                            circuit_state_before: Some(circuit_before.state.as_str()),
                            circuit_state_after: None,
                            circuit_failure_count: Some(circuit_before.failure_count),
                            circuit_failure_threshold: Some(circuit_before.failure_threshold),
                        });

                        let attempt_event = GatewayAttemptEvent {
                            trace_id: trace_id.clone(),
                            cli_key: cli_key.clone(),
                            method: method_hint.clone(),
                            path: forwarded_path.clone(),
                            query: query.clone(),
                            attempt_index,
                            provider_id,
                            session_reuse,
                            provider_name: provider_name_base.clone(),
                            base_url: provider_base_url_base.clone(),
                            outcome,
                            status: Some(status.as_u16()),
                            attempt_started_ms,
                            attempt_duration_ms: attempt_started.elapsed().as_millis(),
                            circuit_state_before: Some(circuit_before.state.as_str()),
                            circuit_state_after: None,
                            circuit_failure_count: Some(circuit_before.failure_count),
                            circuit_failure_threshold: Some(circuit_before.failure_threshold),
                        };
                        emit_attempt_event(&state.app, attempt_event.clone());
                        enqueue_attempt_log_with_backpressure(
                            &state.app,
                            &state.attempt_log_tx,
                            &attempt_event,
                            created_at,
                        )
                        .await;

                        body_bytes = maybe_gunzip_response_body_bytes_with_limit(
                            body_bytes,
                            &mut response_headers,
                            MAX_NON_SSE_BODY_BYTES,
                        );

                        let enable_response_fixer_for_this_response = enable_response_fixer
                            && !has_non_identity_content_encoding(&response_headers);
                        if enable_response_fixer_for_this_response {
                            response_headers.remove(header::CONTENT_LENGTH);
                            let config = response_fixer::ResponseFixerConfig {
                                fix_encoding: response_fixer_fix_encoding,
                                fix_sse_format: false,
                                fix_truncated_json: response_fixer_fix_truncated_json,
                                max_json_depth: response_fixer::DEFAULT_MAX_JSON_DEPTH,
                                max_fix_size: response_fixer::DEFAULT_MAX_FIX_SIZE,
                            };
                            let outcome = response_fixer::process_non_stream(body_bytes, config);
                            response_headers.insert(
                                "x-cch-response-fixer",
                                HeaderValue::from_static(outcome.header_value),
                            );
                            if let Some(setting) = outcome.special_setting {
                                if let Ok(mut settings) = special_settings.lock() {
                                    settings.push(setting);
                                }
                            }
                            body_bytes = outcome.body;
                        }

                        let usage = usage::parse_usage_from_json_bytes(&body_bytes);
                        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
                        let requested_model_for_log = requested_model.clone().or_else(|| {
                            if body_bytes.is_empty() {
                                None
                            } else {
                                usage::parse_model_from_json_bytes(&body_bytes)
                            }
                        });

                        let body = Body::from(body_bytes);
                        let mut builder = Response::builder().status(status);
                        for (k, v) in response_headers.iter() {
                            builder = builder.header(k, v);
                        }
                        builder = builder.header("x-trace-id", trace_id.as_str());

                        let out = match builder.body(body) {
                            Ok(r) => r,
                            Err(_) => {
                                let mut fallback =
                                    (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR")
                                        .into_response();
                                fallback.headers_mut().insert(
                                    "x-trace-id",
                                    HeaderValue::from_str(&trace_id)
                                        .unwrap_or(HeaderValue::from_static("unknown")),
                                );
                                fallback
                            }
                        };

                        if out.status() == status {
                            let now_unix = now_unix_seconds() as i64;
                            let change = state.circuit.record_success(provider_id, now_unix);
                            if let Some(t) = change.transition {
                                emit_circuit_transition(
                                    &state.app,
                                    &trace_id,
                                    &cli_key,
                                    provider_id,
                                    &provider_name_base,
                                    &provider_base_url_base,
                                    &t,
                                    now_unix,
                                );
                            }
                            if let Some(last) = attempts.last_mut() {
                                last.circuit_state_after = Some(change.after.state.as_str());
                                last.circuit_failure_count = Some(change.after.failure_count);
                                last.circuit_failure_threshold =
                                    Some(change.after.failure_threshold);
                            }
                            if (200..300).contains(&status.as_u16()) {
                                if let Some(session_id) = session_id.as_deref() {
                                    state.session.bind_success(
                                        &cli_key,
                                        session_id,
                                        provider_id,
                                        effective_sort_mode_id,
                                        now_unix,
                                    );
                                }
                            }
                        }

                        let attempts_json =
                            serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string());
                        let duration_ms = started.elapsed().as_millis();
                        emit_request_event(
                            &state.app,
                            trace_id.clone(),
                            cli_key.clone(),
                            method_hint.clone(),
                            forwarded_path.clone(),
                            query.clone(),
                            Some(status.as_u16()),
                            None,
                            None,
                            duration_ms,
                            Some(duration_ms),
                            attempts.clone(),
                            usage_metrics,
                        );
                        enqueue_request_log_with_backpressure(
                            &state.app,
                            &state.log_tx,
                            RequestLogEnqueueArgs {
                                trace_id,
                                cli_key,
                                session_id: session_id.clone(),
                                method: method_hint,
                                path: forwarded_path,
                                query,
                                excluded_from_stats: false,
                                special_settings_json: response_fixer::special_settings_json(
                                    &special_settings,
                                ),
                                status: Some(status.as_u16()),
                                error_code: None,
                                duration_ms,
                                ttfb_ms: None,
                                attempts_json,
                                requested_model: requested_model_for_log,
                                created_at_ms,
                                created_at,
                                usage,
                            },
                        )
                        .await;
                        abort_guard.disarm();
                        return out;
                    }

                    if cli_key == "claude"
                        && enable_thinking_signature_rectifier
                        && status.as_u16() == 400
                    {
                        let buffered_body = match resp.bytes().await {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                let duration_ms = started.elapsed().as_millis();
                                let resp = error_response(
                                    StatusCode::BAD_GATEWAY,
                                    trace_id.clone(),
                                    "GW_UPSTREAM_BODY_READ_ERROR",
                                    format!("failed to read upstream error body: {err}"),
                                    attempts.clone(),
                                );
                                emit_request_event(
                                    &state.app,
                                    trace_id.clone(),
                                    cli_key.clone(),
                                    method_hint.clone(),
                                    forwarded_path.clone(),
                                    query.clone(),
                                    Some(StatusCode::BAD_GATEWAY.as_u16()),
                                    Some(ErrorCategory::SystemError.as_str()),
                                    Some("GW_UPSTREAM_BODY_READ_ERROR"),
                                    duration_ms,
                                    None,
                                    attempts.clone(),
                                    None,
                                );
                                enqueue_request_log_with_backpressure(
                                    &state.app,
                                    &state.log_tx,
                                    RequestLogEnqueueArgs {
                                        trace_id: trace_id.clone(),
                                        cli_key: cli_key.clone(),
                                        session_id: session_id.clone(),
                                        method: method_hint.clone(),
                                        path: forwarded_path.clone(),
                                        query: query.clone(),
                                        excluded_from_stats: false,
                                        special_settings_json: None,
                                        status: Some(StatusCode::BAD_GATEWAY.as_u16()),
                                        error_code: Some("GW_UPSTREAM_BODY_READ_ERROR"),
                                        duration_ms,
                                        ttfb_ms: None,
                                        attempts_json: serde_json::to_string(&attempts)
                                            .unwrap_or_else(|_| "[]".to_string()),
                                        requested_model: requested_model.clone(),
                                        created_at_ms,
                                        created_at,
                                        usage: None,
                                    },
                                )
                                .await;
                                abort_guard.disarm();
                                return resp;
                            }
                        };

                        let upstream_body_text =
                            String::from_utf8_lossy(buffered_body.as_ref()).to_string();
                        let trigger =
                            thinking_signature_rectifier::detect_trigger(&upstream_body_text);

                        let mut rectified_applied = false;
                        if let Some(trigger) = trigger {
                            if !thinking_signature_rectifier_retried {
                                let mut message_value =
                                    match serde_json::from_slice::<serde_json::Value>(
                                        introspection_body.as_ref(),
                                    ) {
                                        Ok(v) => v,
                                        Err(_) => serde_json::Value::Null,
                                    };

                                let rectified =
                                    thinking_signature_rectifier::rectify_anthropic_request_message(
                                        &mut message_value,
                                    );

                                if let Ok(mut settings) = special_settings.lock() {
                                    settings.push(serde_json::json!({
                                        "type": "thinking_signature_rectifier",
                                        "scope": "request",
                                        "hit": rectified.applied,
                                        "providerId": provider_id,
                                        "providerName": provider_name_base.clone(),
                                        "trigger": trigger,
                                        "attemptNumber": retry_index,
                                        "retryAttemptNumber": retry_index + 1,
                                        "removedThinkingBlocks": rectified.removed_thinking_blocks,
                                        "removedRedactedThinkingBlocks": rectified.removed_redacted_thinking_blocks,
                                        "removedSignatureFields": rectified.removed_signature_fields,
                                        "removedTopLevelThinking": rectified.removed_top_level_thinking,
                                    }));
                                }

                                if rectified.applied {
                                    if let Ok(next) = serde_json::to_vec(&message_value) {
                                        upstream_body_bytes = Bytes::from(next);
                                        strip_request_content_encoding = true;
                                        thinking_signature_rectifier_retried = true;
                                        rectified_applied = true;
                                    }
                                }
                            }
                        }

                        let (category, error_code, _base_decision) =
                            classify_upstream_status(status);
                        let decision = if rectified_applied {
                            FailoverDecision::RetrySameProvider
                        } else {
                            FailoverDecision::Abort
                        };

                        let circuit_state_before = Some(circuit_before.state.as_str());
                        let circuit_state_after: Option<&'static str> = None;
                        let circuit_failure_count = Some(circuit_before.failure_count);
                        let circuit_failure_threshold = Some(circuit_before.failure_threshold);

                        let reason = format!("status={}", status.as_u16());
                        let outcome = format!(
                            "upstream_error: status={} category={} code={} decision={}",
                            status.as_u16(),
                            category.as_str(),
                            error_code,
                            decision.as_str()
                        );

                        attempts.push(FailoverAttempt {
                            provider_id,
                            provider_name: provider_name_base.clone(),
                            base_url: provider_base_url_base.clone(),
                            outcome: outcome.clone(),
                            status: Some(status.as_u16()),
                            provider_index: Some(provider_index),
                            retry_index: Some(retry_index),
                            session_reuse,
                            error_category: Some(category.as_str()),
                            error_code: Some(error_code),
                            decision: Some(decision.as_str()),
                            reason: Some(reason),
                            attempt_started_ms: Some(attempt_started_ms),
                            attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                            circuit_state_before,
                            circuit_state_after,
                            circuit_failure_count,
                            circuit_failure_threshold,
                        });

                        let attempt_event = GatewayAttemptEvent {
                            trace_id: trace_id.clone(),
                            cli_key: cli_key.clone(),
                            method: method_hint.clone(),
                            path: forwarded_path.clone(),
                            query: query.clone(),
                            attempt_index,
                            provider_id,
                            session_reuse,
                            provider_name: provider_name_base.clone(),
                            base_url: provider_base_url_base.clone(),
                            outcome,
                            status: Some(status.as_u16()),
                            attempt_started_ms,
                            attempt_duration_ms: attempt_started.elapsed().as_millis(),
                            circuit_state_before,
                            circuit_state_after,
                            circuit_failure_count,
                            circuit_failure_threshold,
                        };
                        emit_attempt_event(&state.app, attempt_event.clone());
                        enqueue_attempt_log_with_backpressure(
                            &state.app,
                            &state.attempt_log_tx,
                            &attempt_event,
                            created_at,
                        )
                        .await;

                        last_error_category = Some(category.as_str());
                        last_error_code = Some(error_code);

                        match decision {
                            FailoverDecision::RetrySameProvider => {
                                if let Some(delay) = retry_backoff_delay(status, retry_index) {
                                    tokio::time::sleep(delay).await;
                                }
                                continue;
                            }
                            FailoverDecision::SwitchProvider => {
                                failed_provider_ids.insert(provider_id);
                                break;
                            }
                            FailoverDecision::Abort => {
                                strip_hop_headers(&mut response_headers);
                                let mut body_to_return = buffered_body;

                                body_to_return = maybe_gunzip_response_body_bytes_with_limit(
                                    body_to_return,
                                    &mut response_headers,
                                    MAX_NON_SSE_BODY_BYTES,
                                );

                                let enable_response_fixer_for_this_response = enable_response_fixer
                                    && !has_non_identity_content_encoding(&response_headers);
                                if enable_response_fixer_for_this_response {
                                    response_headers.remove(header::CONTENT_LENGTH);
                                    let config = response_fixer::ResponseFixerConfig {
                                        fix_encoding: response_fixer_fix_encoding,
                                        fix_sse_format: false,
                                        fix_truncated_json: response_fixer_fix_truncated_json,
                                        max_json_depth: response_fixer::DEFAULT_MAX_JSON_DEPTH,
                                        max_fix_size: response_fixer::DEFAULT_MAX_FIX_SIZE,
                                    };
                                    let outcome =
                                        response_fixer::process_non_stream(body_to_return, config);
                                    response_headers.insert(
                                        "x-cch-response-fixer",
                                        HeaderValue::from_static(outcome.header_value),
                                    );
                                    if let Some(setting) = outcome.special_setting {
                                        if let Ok(mut settings) = special_settings.lock() {
                                            settings.push(setting);
                                        }
                                    }
                                    body_to_return = outcome.body;
                                }

                                let attempts_json = serde_json::to_string(&attempts)
                                    .unwrap_or_else(|_| "[]".to_string());
                                let special_settings_json =
                                    response_fixer::special_settings_json(&special_settings);
                                let duration_ms = started.elapsed().as_millis();

                                emit_request_event(
                                    &state.app,
                                    trace_id.clone(),
                                    cli_key.clone(),
                                    method_hint.clone(),
                                    forwarded_path.clone(),
                                    query.clone(),
                                    Some(status.as_u16()),
                                    Some(category.as_str()),
                                    Some(error_code),
                                    duration_ms,
                                    Some(duration_ms),
                                    attempts.clone(),
                                    None,
                                );
                                enqueue_request_log_with_backpressure(
                                    &state.app,
                                    &state.log_tx,
                                    RequestLogEnqueueArgs {
                                        trace_id: trace_id.clone(),
                                        cli_key: cli_key.clone(),
                                        session_id: session_id.clone(),
                                        method: method_hint.clone(),
                                        path: forwarded_path.clone(),
                                        query: query.clone(),
                                        excluded_from_stats: false,
                                        special_settings_json,
                                        status: Some(status.as_u16()),
                                        error_code: Some(error_code),
                                        duration_ms,
                                        ttfb_ms: Some(duration_ms),
                                        attempts_json,
                                        requested_model: requested_model.clone(),
                                        created_at_ms,
                                        created_at,
                                        usage: None,
                                    },
                                )
                                .await;

                                abort_guard.disarm();
                                return build_response(
                                    status,
                                    &response_headers,
                                    &trace_id,
                                    Body::from(body_to_return),
                                );
                            }
                        }
                    }

                    let (category, error_code, base_decision) = classify_upstream_status(status);
                    let mut decision = base_decision;
                    if matches!(decision, FailoverDecision::RetrySameProvider)
                        && retry_index >= max_attempts_per_provider
                    {
                        decision = FailoverDecision::SwitchProvider;
                    }

                    let mut circuit_state_before = Some(circuit_before.state.as_str());
                    let mut circuit_state_after: Option<&'static str> = None;
                    let mut circuit_failure_count = Some(circuit_before.failure_count);
                    let circuit_failure_threshold = Some(circuit_before.failure_threshold);

                    let now_unix = now_unix_seconds() as i64;
                    if matches!(category, ErrorCategory::ProviderError) {
                        let change = state.circuit.record_failure(provider_id, now_unix);
                        circuit_snapshot = change.after.clone();
                        circuit_state_before = Some(change.before.state.as_str());
                        circuit_state_after = Some(change.after.state.as_str());
                        circuit_failure_count = Some(change.after.failure_count);

                        if let Some(t) = change.transition {
                            emit_circuit_transition(
                                &state.app,
                                &trace_id,
                                &cli_key,
                                provider_id,
                                &provider_name_base,
                                &provider_base_url_base,
                                &t,
                                now_unix,
                            );
                        }

                        if change.after.state == circuit_breaker::CircuitState::Open {
                            decision = FailoverDecision::SwitchProvider;
                        }
                    }

                    if provider_cooldown_secs > 0
                        && matches!(category, ErrorCategory::ProviderError)
                        && matches!(
                            decision,
                            FailoverDecision::SwitchProvider | FailoverDecision::Abort
                        )
                    {
                        let snap = state.circuit.trigger_cooldown(
                            provider_id,
                            now_unix,
                            provider_cooldown_secs,
                        );
                        circuit_snapshot = snap;
                    }

                    let reason = format!("status={}", status.as_u16());
                    let outcome = format!(
                        "upstream_error: status={} category={} code={} decision={}",
                        status.as_u16(),
                        category.as_str(),
                        error_code,
                        decision.as_str()
                    );

                    attempts.push(FailoverAttempt {
                        provider_id,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: Some(status.as_u16()),
                        provider_index: Some(provider_index),
                        retry_index: Some(retry_index),
                        session_reuse,
                        error_category: Some(category.as_str()),
                        error_code: Some(error_code),
                        decision: Some(decision.as_str()),
                        reason: Some(reason),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before,
                        circuit_state_after,
                        circuit_failure_count,
                        circuit_failure_threshold,
                    });

                    let attempt_event = GatewayAttemptEvent {
                        trace_id: trace_id.clone(),
                        cli_key: cli_key.clone(),
                        method: method_hint.clone(),
                        path: forwarded_path.clone(),
                        query: query.clone(),
                        attempt_index,
                        provider_id,
                        session_reuse,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: Some(status.as_u16()),
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before,
                        circuit_state_after,
                        circuit_failure_count,
                        circuit_failure_threshold,
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    match decision {
                        FailoverDecision::RetrySameProvider => {
                            if let Some(delay) = retry_backoff_delay(status, retry_index) {
                                tokio::time::sleep(delay).await;
                            }
                            continue;
                        }
                        FailoverDecision::SwitchProvider => {
                            failed_provider_ids.insert(provider_id);
                            break;
                        }
                        FailoverDecision::Abort => {
                            strip_hop_headers(&mut response_headers);
                            let should_gunzip = has_gzip_content_encoding(&response_headers);
                            if should_gunzip {
                                // 上游可能无视 accept-encoding: identity 返回 gzip；对齐 claude-code-hub：解压并移除头。
                                response_headers.remove(header::CONTENT_ENCODING);
                                response_headers.remove(header::CONTENT_LENGTH);
                            }
                            let attempts_json = serde_json::to_string(&attempts)
                                .unwrap_or_else(|_| "[]".to_string());
                            let ctx = StreamFinalizeCtx {
                                app: state.app.clone(),
                                log_tx: state.log_tx.clone(),
                                circuit: state.circuit.clone(),
                                session: state.session.clone(),
                                session_id: session_id.clone(),
                                sort_mode_id: effective_sort_mode_id,
                                trace_id: trace_id.clone(),
                                cli_key: cli_key.clone(),
                                method: method_hint.clone(),
                                path: forwarded_path.clone(),
                                query: query.clone(),
                                excluded_from_stats: false,
                                special_settings: special_settings.clone(),
                                status: status.as_u16(),
                                error_category: Some(category.as_str()),
                                error_code: Some(error_code),
                                started,
                                attempts: attempts.clone(),
                                attempts_json,
                                requested_model: requested_model.clone(),
                                created_at_ms,
                                created_at,
                                provider_cooldown_secs,
                                provider_id,
                                provider_name: provider_name_base.clone(),
                                base_url: provider_base_url_base.clone(),
                            };

                            let body = if should_gunzip {
                                let upstream = GunzipStream::new(resp.bytes_stream());
                                let stream = TimingOnlyTeeStream::new(
                                    upstream,
                                    ctx,
                                    upstream_request_timeout_non_streaming,
                                );
                                Body::from_stream(stream)
                            } else {
                                let stream = TimingOnlyTeeStream::new(
                                    resp.bytes_stream(),
                                    ctx,
                                    upstream_request_timeout_non_streaming,
                                );
                                Body::from_stream(stream)
                            };
                            abort_guard.disarm();
                            return build_response(status, &response_headers, &trace_id, body);
                        }
                    }
                }
                SendResult::Timeout => {
                    let category = ErrorCategory::SystemError;
                    let error_code = "GW_UPSTREAM_TIMEOUT";
                    let decision = if retry_index < max_attempts_per_provider {
                        FailoverDecision::RetrySameProvider
                    } else {
                        FailoverDecision::SwitchProvider
                    };

                    let outcome = format!(
                        "request_timeout: category={} code={} decision={} timeout_secs={}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
                        upstream_first_byte_timeout_secs,
                    );

                    attempts.push(FailoverAttempt {
                        provider_id,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: None,
                        provider_index: Some(provider_index),
                        retry_index: Some(retry_index),
                        session_reuse,
                        error_category: Some(category.as_str()),
                        error_code: Some(error_code),
                        decision: Some(decision.as_str()),
                        reason: Some("request timeout".to_string()),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    });

                    let attempt_event = GatewayAttemptEvent {
                        trace_id: trace_id.clone(),
                        cli_key: cli_key.clone(),
                        method: method_hint.clone(),
                        path: forwarded_path.clone(),
                        query: query.clone(),
                        attempt_index,
                        provider_id,
                        session_reuse,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome,
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    if provider_cooldown_secs > 0
                        && matches!(
                            decision,
                            FailoverDecision::SwitchProvider | FailoverDecision::Abort
                        )
                    {
                        let now_unix = now_unix_seconds() as i64;
                        let snap = state.circuit.trigger_cooldown(
                            provider_id,
                            now_unix,
                            provider_cooldown_secs,
                        );
                        circuit_snapshot = snap;
                    }

                    match decision {
                        FailoverDecision::RetrySameProvider => continue,
                        FailoverDecision::SwitchProvider => {
                            failed_provider_ids.insert(provider_id);
                            break;
                        }
                        FailoverDecision::Abort => break,
                    }
                }
                SendResult::Err(err) => {
                    let (category, error_code) = classify_reqwest_error(&err);
                    let decision = if retry_index < max_attempts_per_provider {
                        FailoverDecision::RetrySameProvider
                    } else {
                        FailoverDecision::SwitchProvider
                    };

                    let outcome = format!(
                        "request_error: category={} code={} decision={} err={err}",
                        category.as_str(),
                        error_code,
                        decision.as_str(),
                    );

                    attempts.push(FailoverAttempt {
                        provider_id,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome: outcome.clone(),
                        status: None,
                        provider_index: Some(provider_index),
                        retry_index: Some(retry_index),
                        session_reuse,
                        error_category: Some(category.as_str()),
                        error_code: Some(error_code),
                        decision: Some(decision.as_str()),
                        reason: Some("reqwest error".to_string()),
                        attempt_started_ms: Some(attempt_started_ms),
                        attempt_duration_ms: Some(attempt_started.elapsed().as_millis()),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    });

                    let attempt_event = GatewayAttemptEvent {
                        trace_id: trace_id.clone(),
                        cli_key: cli_key.clone(),
                        method: method_hint.clone(),
                        path: forwarded_path.clone(),
                        query: query.clone(),
                        attempt_index,
                        provider_id,
                        session_reuse,
                        provider_name: provider_name_base.clone(),
                        base_url: provider_base_url_base.clone(),
                        outcome,
                        status: None,
                        attempt_started_ms,
                        attempt_duration_ms: attempt_started.elapsed().as_millis(),
                        circuit_state_before: Some(circuit_before.state.as_str()),
                        circuit_state_after: None,
                        circuit_failure_count: Some(circuit_before.failure_count),
                        circuit_failure_threshold: Some(circuit_before.failure_threshold),
                    };
                    emit_attempt_event(&state.app, attempt_event.clone());
                    enqueue_attempt_log_with_backpressure(
                        &state.app,
                        &state.attempt_log_tx,
                        &attempt_event,
                        created_at,
                    )
                    .await;

                    last_error_category = Some(category.as_str());
                    last_error_code = Some(error_code);

                    if provider_cooldown_secs > 0
                        && matches!(
                            decision,
                            FailoverDecision::SwitchProvider | FailoverDecision::Abort
                        )
                    {
                        let now_unix = now_unix_seconds() as i64;
                        state.circuit.trigger_cooldown(
                            provider_id,
                            now_unix,
                            provider_cooldown_secs,
                        );
                    }

                    match decision {
                        FailoverDecision::RetrySameProvider => continue,
                        FailoverDecision::SwitchProvider => {
                            failed_provider_ids.insert(provider_id);
                            break;
                        }
                        FailoverDecision::Abort => break,
                    }
                }
            }
        }
    }

    if attempts.is_empty() && !providers.is_empty() {
        let now_unix = now_unix_seconds() as i64;
        let retry_after_seconds = earliest_available_unix
            .and_then(|t| t.checked_sub(now_unix))
            .filter(|v| *v > 0)
            .map(|v| v as u64);

        let message = format!(
            "no provider available (skipped: open={skipped_open}, cooldown={skipped_cooldown}) for cli_key={cli_key}",
        );

        let resp = error_response_with_retry_after(
            StatusCode::SERVICE_UNAVAILABLE,
            trace_id.clone(),
            "GW_ALL_PROVIDERS_UNAVAILABLE",
            message.clone(),
            vec![],
            retry_after_seconds,
        );

        emit_request_event(
            &state.app,
            trace_id.clone(),
            cli_key.clone(),
            method_hint.clone(),
            forwarded_path.clone(),
            query.clone(),
            Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
            None,
            Some("GW_ALL_PROVIDERS_UNAVAILABLE"),
            started.elapsed().as_millis(),
            None,
            vec![],
            None,
        );

        enqueue_request_log_with_backpressure(
            &state.app,
            &state.log_tx,
            RequestLogEnqueueArgs {
                trace_id: trace_id.clone(),
                cli_key,
                session_id: session_id.clone(),
                method: method_hint,
                path: forwarded_path,
                query,
                excluded_from_stats: false,
                special_settings_json: response_fixer::special_settings_json(&special_settings),
                status: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                error_code: Some("GW_ALL_PROVIDERS_UNAVAILABLE"),
                duration_ms: started.elapsed().as_millis(),
                ttfb_ms: None,
                attempts_json: "[]".to_string(),
                requested_model: requested_model.clone(),
                created_at_ms,
                created_at,
                usage: None,
            },
        )
        .await;

        if let Some(retry_after_seconds) = retry_after_seconds.filter(|v| *v > 0) {
            if let Ok(mut cache) = state.recent_errors.lock() {
                cache.insert_error(
                    now_unix,
                    unavailable_fingerprint_key,
                    CachedGatewayError {
                        trace_id: trace_id.clone(),
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
                        message: message.clone(),
                        retry_after_seconds: Some(retry_after_seconds),
                        expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                        fingerprint_debug: unavailable_fingerprint_debug.clone(),
                    },
                );
                cache.insert_error(
                    now_unix,
                    fingerprint_key,
                    CachedGatewayError {
                        trace_id: trace_id.clone(),
                        status: StatusCode::SERVICE_UNAVAILABLE,
                        error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
                        message,
                        retry_after_seconds: Some(retry_after_seconds),
                        expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                        fingerprint_debug: fingerprint_debug.clone(),
                    },
                );
            }
        }

        abort_guard.disarm();
        return resp;
    }

    let final_error_code = last_error_code.unwrap_or("GW_UPSTREAM_ALL_FAILED");
    let final_error_category = last_error_category;

    let resp = error_response(
        StatusCode::BAD_GATEWAY,
        trace_id.clone(),
        final_error_code,
        format!("all providers failed for cli_key={cli_key}"),
        attempts.clone(),
    );

    emit_request_event(
        &state.app,
        trace_id.clone(),
        cli_key.clone(),
        method_hint.clone(),
        forwarded_path.clone(),
        query.clone(),
        Some(StatusCode::BAD_GATEWAY.as_u16()),
        final_error_category,
        Some(final_error_code),
        started.elapsed().as_millis(),
        None,
        attempts.clone(),
        None,
    );

    enqueue_request_log_with_backpressure(
        &state.app,
        &state.log_tx,
        RequestLogEnqueueArgs {
            trace_id,
            cli_key,
            session_id: session_id.clone(),
            method: method_hint,
            path: forwarded_path,
            query,
            excluded_from_stats: false,
            special_settings_json: response_fixer::special_settings_json(&special_settings),
            status: Some(StatusCode::BAD_GATEWAY.as_u16()),
            error_code: Some(final_error_code),
            duration_ms: started.elapsed().as_millis(),
            ttfb_ms: None,
            attempts_json: serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string()),
            requested_model,
            created_at_ms,
            created_at,
            usage: None,
        },
    )
    .await;

    abort_guard.disarm();
    resp
}

#[cfg(test)]
mod tests {
    use super::select_next_provider_id_from_order;
    use std::collections::HashSet;

    fn set(ids: &[i64]) -> HashSet<i64> {
        ids.iter().copied().collect()
    }

    #[test]
    fn select_next_provider_id_wraps_and_skips_missing() {
        let order = vec![1, 2, 3, 4];
        let current = set(&[2, 4]);

        assert_eq!(
            select_next_provider_id_from_order(4, &order, &current),
            Some(2)
        );
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &current),
            Some(4)
        );
    }

    #[test]
    fn select_next_provider_id_returns_none_when_no_candidate() {
        let order = vec![1, 2, 3];
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &set(&[])),
            None
        );
        assert_eq!(
            select_next_provider_id_from_order(2, &order, &set(&[99])),
            None
        );
    }

    #[test]
    fn select_next_provider_id_starts_from_head_when_bound_missing() {
        let order = vec![10, 20, 30];
        let current = set(&[30]);
        assert_eq!(
            select_next_provider_id_from_order(999, &order, &current),
            Some(30)
        );
    }
}
