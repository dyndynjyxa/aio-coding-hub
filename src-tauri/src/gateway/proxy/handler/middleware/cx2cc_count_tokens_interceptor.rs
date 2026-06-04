//! Middleware: answers Claude count-tokens requests locally for CX2CC bridges.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::events::emit_gateway_debug_log_lazy;
use crate::providers;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

pub(in crate::gateway::proxy::handler) struct Cx2ccCountTokensInterceptorMiddleware;

impl Cx2ccCountTokensInterceptorMiddleware {
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        if !should_intercept_cx2cc_count_tokens(ctx.is_claude_count_tokens, &ctx.providers) {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let response_body = build_cx2cc_count_tokens_response_body(ctx.introspection_json.as_ref());
        let input_tokens = response_body
            .get("input_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(1);
        emit_gateway_debug_log_lazy(&ctx.state.app, || {
            format!(
                "[CX2CC] count_tokens intercepted locally trace_id={} input_tokens={input_tokens}",
                ctx.trace_id
            )
        });

        MiddlewareAction::ShortCircuit(build_cx2cc_count_tokens_response(
            response_body,
            &ctx.trace_id,
        ))
    }
}

pub(in crate::gateway::proxy::handler) fn should_intercept_cx2cc_count_tokens(
    is_claude_count_tokens: bool,
    providers: &[providers::ProviderForGateway],
) -> bool {
    is_claude_count_tokens
        && providers
            .first()
            .is_some_and(providers::ProviderForGateway::is_cx2cc_bridge)
}

pub(in crate::gateway::proxy::handler) fn build_cx2cc_count_tokens_response_body(
    root: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "input_tokens": estimate_cx2cc_count_tokens(root),
    })
}

fn estimate_cx2cc_count_tokens(root: Option<&serde_json::Value>) -> u64 {
    let Some(root) = root else {
        return 1;
    };

    let bytes = serde_json::to_vec(root)
        .map(|encoded| encoded.len())
        .unwrap_or(0);
    // Best-effort compatibility estimate. Claude Code Auto mode needs the
    // Anthropic response shape; exact tokenizer parity is out of scope here.
    let estimated = (bytes as u64).saturating_add(3) / 4;
    estimated.max(1)
}

fn build_cx2cc_count_tokens_response(body: serde_json::Value, trace_id: &str) -> Response {
    let mut resp = (StatusCode::OK, Json(body)).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    resp.headers_mut().insert(
        "x-aio-intercepted",
        HeaderValue::from_static("cx2cc-count-tokens"),
    );
    resp.headers_mut().insert(
        "x-aio-intercepted-by",
        HeaderValue::from_static("aio-coding-hub"),
    );
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        resp.headers_mut().insert("x-trace-id", value);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: i64) -> providers::ProviderForGateway {
        providers::ProviderForGateway {
            id,
            name: format!("p{id}"),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: String::new(),
            claude_models: providers::ClaudeModels::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
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

    fn cx2cc_provider(id: i64) -> providers::ProviderForGateway {
        providers::ProviderForGateway {
            source_provider_id: Some(99),
            bridge_type: Some("cx2cc".to_string()),
            ..provider(id)
        }
    }

    #[test]
    fn intercepts_count_tokens_only_when_first_provider_is_cx2cc() {
        assert!(should_intercept_cx2cc_count_tokens(
            true,
            &[cx2cc_provider(1)]
        ));
        assert!(!should_intercept_cx2cc_count_tokens(
            false,
            &[cx2cc_provider(1)]
        ));
        assert!(!should_intercept_cx2cc_count_tokens(true, &[provider(1)]));
        assert!(!should_intercept_cx2cc_count_tokens(
            true,
            &[provider(1), cx2cc_provider(2)]
        ));
    }

    #[test]
    fn cx2cc_count_tokens_response_body_is_positive() {
        let body = serde_json::json!({
            "model": "claude-sonnet-4",
            "messages": [
                {"role": "user", "content": "hello"}
            ],
            "tools": [
                {"name": "Read", "description": "Read a file", "input_schema": {"type": "object"}}
            ]
        });

        let response = build_cx2cc_count_tokens_response_body(Some(&body));

        assert!(response
            .get("input_tokens")
            .and_then(|value| value.as_u64())
            .is_some_and(|tokens| tokens > 0));
    }

    #[test]
    fn cx2cc_count_tokens_response_body_falls_back_to_one_without_json() {
        let response = build_cx2cc_count_tokens_response_body(None);

        assert_eq!(
            response.get("input_tokens").and_then(|v| v.as_u64()),
            Some(1)
        );
    }

    #[test]
    fn cx2cc_count_tokens_response_sets_intercept_headers() {
        let response =
            build_cx2cc_count_tokens_response(serde_json::json!({"input_tokens": 1}), "trace-test");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("x-aio-intercepted")
                .and_then(|value| value.to_str().ok()),
            Some("cx2cc-count-tokens")
        );
        assert_eq!(
            response
                .headers()
                .get("x-trace-id")
                .and_then(|value| value.to_str().ok()),
            Some("trace-test")
        );
    }
}
