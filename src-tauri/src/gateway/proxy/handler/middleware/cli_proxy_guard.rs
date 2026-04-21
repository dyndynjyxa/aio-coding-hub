//! Middleware: cached enable/disable check per CLI key.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::events::emit_gateway_log;
use crate::gateway::proxy::cli_proxy_guard::cli_proxy_enabled_cached;
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, respond_early_error_with_enqueue,
    EarlyErrorKind,
};
use crate::gateway::proxy::{compute_observe_request, GatewayErrorCode};

pub(in crate::gateway::proxy::handler) struct CliProxyGuardMiddleware;

impl CliProxyGuardMiddleware {
    pub(in crate::gateway::proxy::handler) async fn run(ctx: ProxyContext) -> MiddlewareAction {
        let bypass = ctx.forced_provider_id.is_some();
        if !crate::shared::cli_key::is_supported_cli_key(&ctx.cli_key) || bypass {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        let enabled_snapshot = cli_proxy_enabled_cached(&ctx.state.app, &ctx.cli_key);
        if enabled_snapshot.enabled {
            return MiddlewareAction::Continue(Box::new(ctx));
        }

        if !enabled_snapshot.cache_hit {
            if let Some(err) = enabled_snapshot.error.as_deref() {
                emit_gateway_log(
                    &ctx.state.app,
                    "warn",
                    GatewayErrorCode::CliProxyGuardError.as_str(),
                    format!(
                        "CLI proxy guard read failed (treating as disabled) \
                         cli={} trace_id={} err={err}",
                        ctx.cli_key, ctx.trace_id
                    ),
                );
            }
        }

        let contract = early_error_contract(EarlyErrorKind::CliProxyDisabled);
        let message = cli_proxy_disabled_message(&ctx.cli_key, enabled_snapshot.error.as_deref());
        let special_settings_json = cli_proxy_guard_special_settings_json(
            enabled_snapshot.cache_hit,
            enabled_snapshot.cache_ttl_ms,
            enabled_snapshot.error.as_deref(),
        );
        // observe_request not yet computed; derive it for the error log.
        let mut ctx = ctx;
        ctx.observe_request =
            compute_observe_request(&ctx.cli_key, &ctx.forwarded_path, &ctx.headers, None);
        let log_ctx = build_early_error_log_ctx(&ctx);

        let resp = respond_early_error_with_enqueue(
            &log_ctx,
            contract,
            message,
            Some(special_settings_json),
            None,
            None,
        )
        .await;

        MiddlewareAction::ShortCircuit(resp)
    }
}

pub(in crate::gateway::proxy::handler) fn cli_proxy_disabled_message(
    cli_key: &str,
    error: Option<&str>,
) -> String {
    match error {
        Some(err) => format!(
            "CLI 代理状态读取失败（按未开启处理）：{err}；请在首页开启 {cli_key} 的 CLI 代理开关后重试"
        ),
        None => format!(
            "CLI 代理未开启：请在首页开启 {cli_key} 的 CLI 代理开关后重试"
        ),
    }
}

pub(in crate::gateway::proxy::handler) fn cli_proxy_guard_special_settings_json(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_message_without_error_is_actionable() {
        let message = cli_proxy_disabled_message("claude", None);
        assert!(message.contains("CLI"));
        assert!(message.contains("claude"));
    }

    #[test]
    fn disabled_message_with_error_preserves_context() {
        let message = cli_proxy_disabled_message("codex", Some("manifest read failed"));
        assert!(message.contains("manifest read failed"));
        assert!(message.contains("codex"));
    }

    #[test]
    fn special_settings_json_has_expected_shape() {
        let encoded = cli_proxy_guard_special_settings_json(false, 5000, Some("boom"));
        let value: serde_json::Value =
            serde_json::from_str(&encoded).expect("should be valid json");
        let row = value
            .as_array()
            .and_then(|rows| rows.first())
            .expect("should contain one object");

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
}
