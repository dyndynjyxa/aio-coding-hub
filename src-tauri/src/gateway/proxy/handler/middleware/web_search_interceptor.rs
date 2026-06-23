//! Middleware: intercepts Claude Code WebSearchTool sub-calls and answers
//! them locally when the user has opted in to `force_replace` mode.
//!
//! Detection logic and SSE synthesis live in `crate::gateway::web_search`.
//! This module only handles middleware plumbing: deciding whether to
//! short-circuit, calling the configured backend, and emitting a
//! request-log entry identical in shape to the warmup interceptor's.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::events::{decision_chain as dc, emit_request_start_event, FailoverAttempt};
use crate::gateway::proxy::handler::runtime_settings::WebSearchRuntimeSettings;
use crate::gateway::proxy::request_end::{
    emit_request_event_and_spawn_request_log, RequestCompletion, RequestEndArgs,
    RequestEndContextArgs, RequestEndDeps,
};
use crate::gateway::web_search::backend::{SearchBackendImpl, SearchError, SearchOptions};
use crate::gateway::web_search::detection::detect_web_search_request;
use crate::gateway::web_search::factory::build_backend;
use crate::gateway::web_search::sse::{build_search_error_response, build_search_success_response};
use crate::usage::UsageMetrics;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::time::Instant;

pub(in crate::gateway::proxy::handler) struct WebSearchInterceptorMiddleware;

impl WebSearchInterceptorMiddleware {
    /// Runs after `RuntimeSettingsMiddleware` so that
    /// `ctx.runtime_settings.web_search_settings` is populated.
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        let ws = match ctx.runtime_settings.as_ref() {
            Some(rs) => &rs.web_search_settings,
            None => return MiddlewareAction::Continue(Box::new(ctx)),
        };
        if !ws.intercept {
            return MiddlewareAction::Continue(Box::new(ctx));
        }
        if ctx.cli_key != "claude" {
            return MiddlewareAction::Continue(Box::new(ctx));
        }
        if ctx.forwarded_path != "/v1/messages" {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let body: serde_json::Value = match serde_json::from_slice(&ctx.body_bytes) {
            Ok(v) => v,
            Err(_) => return MiddlewareAction::Continue(Box::new(ctx)),
        };

        let detection = match detect_web_search_request(&body) {
            Some(d) => d,
            None => return MiddlewareAction::Continue(Box::new(ctx)),
        };

        let started = Instant::now();
        let duration_ms = started.elapsed().as_millis();
        let resp = build_response_blocking(&ctx, &detection, ws, duration_ms);
        MiddlewareAction::ShortCircuit(resp)
    }
}

fn build_response_blocking<R: tauri::Runtime>(
    ctx: &ProxyContext<R>,
    detection: &crate::gateway::web_search::detection::WebSearchDetection,
    ws: &WebSearchRuntimeSettings,
    duration_ms: u128,
) -> Response {
    // We need to run the async search synchronously here because the
    // middleware's `run` is sync (matches the codebase pattern). The
    // tokio runtime is available via `tokio::handle::current()`.
    let backend = build_backend(&ws.backend_settings, &ctx.providers);
    let requested_model = ctx
        .requested_model
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let (body, attempts, special_settings_json, used_tag) = match backend {
        None => {
            let body = build_search_error_response(
                &requested_model,
                &ctx.trace_id,
                &detection.query,
                "invalid_request_error",
            );
            let settings = web_search_special_settings_json(
                "config_missing",
                ws.backend_settings.kind,
                None,
                "backend not configured",
            );
            (body, vec![], settings, "config_missing".to_string())
        }
        Some(backend) => {
            let opts = SearchOptions {
                max_results: ws.backend_settings.max_results as usize,
                allowed_domains: detection.allowed_domains.clone(),
                blocked_domains: detection.blocked_domains.clone(),
                ..Default::default()
            };
            let query = detection.query.clone();
            let trace_id = ctx.trace_id.clone();

            // The middleware `run` is sync, so we block on the async search
            // using the current tokio runtime. This is the same pattern used
            // by `body_reader` for inline JSON parsing and is acceptable for
            // a sub-15s operation.
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(run_search(&backend, &query, &opts))
            });

            match result {
                Ok(hits) => {
                    let body =
                        build_search_success_response(&requested_model, &trace_id, &query, &hits);
                    let settings = web_search_special_settings_json(
                        "success",
                        ws.backend_settings.kind,
                        Some(backend.tag()),
                        &format!("hits={}", hits.len()),
                    );
                    let attempts = vec![FailoverAttempt {
                        provider_id: 0,
                        provider_name: format!("WebSearch/{}", backend.tag()),
                        base_url: format!("/__aio__/web_search/{}", backend.tag()),
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
                    (body, attempts, settings, backend.tag().to_string())
                }
                Err(err) => {
                    let error_code = match &err {
                        SearchError::Upstream { status, .. } if *status == 429 => {
                            "too_many_requests"
                        }
                        SearchError::Upstream { status, .. } if *status >= 500 => "internal_error",
                        SearchError::InvalidConfig { .. } => "invalid_request_error",
                        SearchError::Transport { .. } => "network_error",
                        _ => "internal_error",
                    };
                    let body = build_search_error_response(
                        &requested_model,
                        &trace_id,
                        &query,
                        error_code,
                    );
                    let settings = web_search_special_settings_json(
                        "error",
                        ws.backend_settings.kind,
                        Some(backend.tag()),
                        &format!("error={}", truncate(&err.to_string(), 200)),
                    );
                    let attempts = vec![FailoverAttempt {
                        provider_id: 0,
                        provider_name: format!("WebSearch/{}", backend.tag()),
                        base_url: format!("/__aio__/web_search/{}", backend.tag()),
                        outcome: "error".to_string(),
                        status: Some(StatusCode::BAD_GATEWAY.as_u16()),
                        provider_index: None,
                        retry_index: None,
                        session_reuse: Some(false),
                        error_category: None,
                        error_code: Some(error_code),
                        decision: Some("error"),
                        reason: Some(truncate(&err.to_string(), 200).to_string()),
                        selection_method: None,
                        reason_code: Some(dc::REASON_SYSTEM_ERROR),
                        attempt_started_ms: None,
                        attempt_duration_ms: None,
                        circuit_state_before: None,
                        circuit_state_after: None,
                        circuit_failure_count: None,
                        circuit_failure_threshold: None,
                    }];
                    (body, attempts, settings, backend.tag().to_string())
                }
            }
        }
    };

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

    let usage_metrics = UsageMetrics {
        input_tokens: Some(0),
        output_tokens: Some(0),
        total_tokens: Some(0),
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cache_creation_5m_input_tokens: Some(0),
        cache_creation_1h_input_tokens: Some(0),
    };

    emit_request_event_and_spawn_request_log(
        RequestEndArgs::from_context(RequestEndContextArgs {
            deps: RequestEndDeps::new(&ctx.state.app, &ctx.state.db, &ctx.state.log_tx),
            trace_id: &ctx.trace_id,
            cli_key: &ctx.cli_key,
            method: &ctx.method_hint,
            path: &ctx.forwarded_path,
            observe: ctx.observe_request,
            query: ctx.query.as_deref(),
            excluded_from_stats: true,
            duration_ms,
            attempts: &attempts,
            special_settings_json: Some(special_settings_json),
            session_id: None,
            requested_model: ctx.requested_model.clone(),
            created_at_ms: ctx.created_at_ms,
            created_at: ctx.created_at,
        })
        .with_completion(RequestCompletion::success(
            StatusCode::OK.as_u16(),
            Some(duration_ms),
            Some(usage_metrics.clone()),
            Some(usage_metrics),
            None,
        )),
    );

    let mut resp = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")],
        body,
    )
        .into_response();
    let headers = resp.headers_mut();
    headers.insert("x-aio-intercepted", HeaderValue::from_static("web_search"));
    headers.insert(
        "x-aio-intercepted-by",
        HeaderValue::from_static("aio-coding-hub"),
    );
    headers.insert(
        "x-aio-intercepted-backend",
        HeaderValue::from_str(&used_tag).unwrap_or(HeaderValue::from_static("unknown")),
    );
    if let Ok(v) = HeaderValue::from_str(&ctx.trace_id) {
        headers.insert("x-trace-id", v);
    }
    headers.insert(
        "x-aio-upstream-meta-url",
        HeaderValue::from_static("/__aio__/web_search"),
    );
    resp
}

async fn run_search(
    backend: &SearchBackendImpl,
    query: &str,
    opts: &SearchOptions,
) -> Result<Vec<crate::gateway::web_search::backend::SearchHit>, SearchError> {
    match backend {
        SearchBackendImpl::Brave(b) => b.search(query, opts).await,
        SearchBackendImpl::Tavily(t) => t.search(query, opts).await,
        SearchBackendImpl::Metaso(m) => m.search(query, opts).await,
        SearchBackendImpl::LlmBacked(l) => l.search(query, opts).await,
    }
}

fn web_search_special_settings_json(
    outcome: &str,
    kind: crate::gateway::web_search::backend::SearchBackendKind,
    backend_tag: Option<&str>,
    note: &str,
) -> String {
    serde_json::json!([{
        "type": "web_search_intercept",
        "scope": "request",
        "hit": true,
        "outcome": outcome,
        "kind": kind.to_string(),
        "backend": backend_tag.unwrap_or("-"),
        "note": note,
    }])
    .to_string()
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::web_search::backend::SearchBackendKind;
    use crate::gateway::web_search::factory::BackendSettings;

    fn empty_ws_settings() -> WebSearchRuntimeSettings {
        WebSearchRuntimeSettings {
            intercept: true,
            backend_settings: BackendSettings {
                kind: SearchBackendKind::Brave,
                brave_api_key: String::new(), // intentionally empty -> config error
                tavily_api_key: String::new(),
                metaso_api_key: String::new(),
                metaso_include_summary: false,
                metaso_concise_snippet: false,
                max_results: 5,
                llm_provider_id: None,
                proxy_url: String::new(),
            },
        }
    }

    #[test]
    fn web_search_special_settings_json_contains_backend_metadata() {
        let json = web_search_special_settings_json(
            "success",
            SearchBackendKind::Brave,
            Some("brave"),
            "hits=3",
        );
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let row = value.as_array().unwrap().first().unwrap();
        assert_eq!(
            row.get("type").and_then(|v| v.as_str()),
            Some("web_search_intercept")
        );
        assert_eq!(row.get("scope").and_then(|v| v.as_str()), Some("request"));
        assert_eq!(row.get("hit").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(row.get("outcome").and_then(|v| v.as_str()), Some("success"));
        assert_eq!(row.get("kind").and_then(|v| v.as_str()), Some("brave"));
        assert_eq!(row.get("backend").and_then(|v| v.as_str()), Some("brave"));
        assert_eq!(row.get("note").and_then(|v| v.as_str()), Some("hits=3"));
    }

    #[test]
    fn config_missing_branch_emits_invalid_request_error_block() {
        // Build a sample search-options payload and feed it through
        // build_search_error_response to confirm the wire format.
        let body = build_search_error_response(
            "claude-sonnet-4-5",
            "tid",
            "rust async",
            "invalid_request_error",
        );
        assert!(body.contains("event: message_start"));
        assert!(body.contains("event: content_block_start"));
        assert!(body.contains("event: content_block_stop"));
        assert!(body.contains("event: message_stop"));
        assert!(body.contains("\"type\":\"web_search_tool_result_error\""));
        assert!(body.contains("\"error_code\":\"invalid_request_error\""));
    }

    #[test]
    fn empty_settings_struct_reports_unchecked_fields() {
        let ws = empty_ws_settings();
        assert!(ws.intercept);
        assert_eq!(ws.backend_settings.kind, SearchBackendKind::Brave);
        assert_eq!(ws.backend_settings.brave_api_key, "");
        assert_eq!(ws.backend_settings.max_results, 5);
    }
}
