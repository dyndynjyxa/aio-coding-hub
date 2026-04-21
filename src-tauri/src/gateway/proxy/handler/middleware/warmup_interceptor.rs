//! Middleware: intercepts Anthropic warmup requests and responds locally.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::events::{decision_chain as dc, emit_request_start_event, FailoverAttempt};
use crate::gateway::proxy::request_end::{
    emit_request_event_and_spawn_request_log, RequestEndArgs, RequestEndDeps,
};
use crate::gateway::warmup;
use crate::usage;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

pub(in crate::gateway::proxy::handler) struct WarmupInterceptorMiddleware;

impl WarmupInterceptorMiddleware {
    /// Intercepts Anthropic warmup requests and responds locally.
    ///
    /// Requires `ctx.runtime_settings` to be populated (by `RuntimeSettingsMiddleware`).
    pub(in crate::gateway::proxy::handler) fn run(ctx: ProxyContext) -> MiddlewareAction {
        let intercept_warmup = ctx
            .runtime_settings
            .as_ref()
            .map(|rs| rs.intercept_warmup)
            .unwrap_or(false);

        let is_warmup = should_intercept_warmup_request(
            &ctx.cli_key,
            intercept_warmup,
            &ctx.forwarded_path,
            ctx.introspection_json.as_ref(),
        );

        if !is_warmup {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let duration_ms = ctx.started.elapsed().as_millis();
        let resp = respond_warmup_intercept(&ctx, duration_ms);
        MiddlewareAction::ShortCircuit(resp)
    }
}

pub(in crate::gateway::proxy::handler) fn should_intercept_warmup_request(
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

fn respond_warmup_intercept(ctx: &ProxyContext, duration_ms: u128) -> axum::response::Response {
    let response_body =
        warmup::build_warmup_response_body(ctx.requested_model.as_deref(), &ctx.trace_id);
    let special_settings_json = warmup_intercept_special_settings_json();

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

    let warmup_attempts = [FailoverAttempt {
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
        trace_id: &ctx.trace_id,
        cli_key: &ctx.cli_key,
        method: &ctx.method_hint,
        path: &ctx.forwarded_path,
        observe: ctx.observe_request,
        query: ctx.query.as_deref(),
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
        requested_model: ctx.requested_model.clone(),
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
    if let Ok(v) = HeaderValue::from_str(&ctx.trace_id) {
        resp.headers_mut().insert("x-trace-id", v);
    }
    resp.headers_mut().insert(
        "x-aio-upstream-meta-url",
        HeaderValue::from_static("/__aio__/warmup"),
    );
    resp
}

pub(in crate::gateway::proxy::handler) fn warmup_intercept_special_settings_json() -> String {
    serde_json::json!([{
        "type": "warmup_intercept",
        "scope": "request",
        "hit": true,
        "reason": "anthropic_warmup_intercepted",
        "note": "已由 aio-coding-hub 抢答，未转发上游；写入日志但排除统计",
    }])
    .to_string()
}

pub(in crate::gateway::proxy::handler) fn warmup_log_usage_metrics() -> usage::UsageMetrics {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_intercept_special_settings_json_has_expected_shape() {
        let encoded = warmup_intercept_special_settings_json();
        let value: serde_json::Value =
            serde_json::from_str(&encoded).expect("should be valid json");
        let row = value
            .as_array()
            .and_then(|rows| rows.first())
            .expect("should contain one object");

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
        let m = warmup_log_usage_metrics();
        assert_eq!(m.input_tokens, Some(0));
        assert_eq!(m.output_tokens, Some(0));
        assert_eq!(m.total_tokens, Some(0));
        assert_eq!(m.cache_read_input_tokens, Some(0));
        assert_eq!(m.cache_creation_input_tokens, Some(0));
        assert_eq!(m.cache_creation_5m_input_tokens, Some(0));
        assert_eq!(m.cache_creation_1h_input_tokens, Some(0));
    }

    #[test]
    fn should_intercept_warmup_rejects_non_claude() {
        let body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": "warmup",
                    "cache_control": {"type": "ephemeral"}}]
            }]
        });
        assert!(!should_intercept_warmup_request(
            "codex",
            true,
            "/v1/messages",
            Some(&body)
        ));
    }

    #[test]
    fn should_intercept_warmup_detects_valid_claude_warmup() {
        let body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": "warmup",
                    "cache_control": {"type": "ephemeral"}}]
            }]
        });
        assert!(should_intercept_warmup_request(
            "claude",
            true,
            "/v1/messages",
            Some(&body)
        ));
    }
}
