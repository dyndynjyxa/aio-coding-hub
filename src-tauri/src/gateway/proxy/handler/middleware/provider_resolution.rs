//! Middleware: resolves session routing and selects providers with session binding.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, force_provider_if_requested,
    respond_early_error_with_enqueue, respond_invalid_cli_key_with_spawn, EarlyErrorKind,
};
use crate::gateway::proxy::handler::provider_selection::{
    resolve_session_bound_provider_id, resolve_session_routing_decision,
    select_providers_with_session_binding,
};

pub(in crate::gateway::proxy::handler) struct ProviderResolutionMiddleware;

impl ProviderResolutionMiddleware {
    pub(in crate::gateway::proxy::handler) async fn run(mut ctx: ProxyContext) -> MiddlewareAction {
        // --- session routing decision ---
        let decision = resolve_session_routing_decision(
            &ctx.headers,
            ctx.introspection_json.as_ref(),
            ctx.is_claude_count_tokens,
        );
        ctx.session_id = decision.session_id;
        ctx.allow_session_reuse = decision.allow_session_reuse;

        // --- provider selection ---
        let selection = match select_providers_with_session_binding(
            &ctx.state,
            &ctx.cli_key,
            ctx.session_id.as_deref(),
            ctx.created_at,
        ) {
            Ok(s) => s,
            Err(err) => {
                let log_ctx = build_early_error_log_ctx(&ctx);
                let resp = respond_invalid_cli_key_with_spawn(
                    &log_ctx,
                    ctx.session_id.clone(),
                    ctx.requested_model.clone(),
                    err.to_string(),
                );
                return MiddlewareAction::ShortCircuit(resp);
            }
        };

        ctx.effective_sort_mode_id = selection.effective_sort_mode_id;
        ctx.providers = selection.providers;

        // --- forced provider ---
        force_provider_if_requested(
            &mut ctx.providers,
            ctx.forced_provider_id,
            &ctx.special_settings,
        );

        // --- session bound provider ---
        ctx.session_bound_provider_id = resolve_session_bound_provider_id(
            ctx.state.session.as_ref(),
            ctx.state.circuit.as_ref(),
            &ctx.cli_key,
            ctx.session_id.as_deref(),
            ctx.created_at,
            ctx.allow_session_reuse,
            ctx.forced_provider_id,
            &mut ctx.providers,
            selection.bound_provider_order.as_deref(),
        );

        // --- no enabled provider guard ---
        if ctx.providers.is_empty() {
            let contract = early_error_contract(EarlyErrorKind::NoEnabledProvider);
            let message = no_enabled_provider_message(&ctx.cli_key);
            let session_id = ctx.session_id.take();
            let requested_model = ctx.requested_model.take();
            let log_ctx = build_early_error_log_ctx(&ctx);

            let resp = respond_early_error_with_enqueue(
                &log_ctx,
                contract,
                message,
                None,
                session_id,
                requested_model,
            )
            .await;
            return MiddlewareAction::ShortCircuit(resp);
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

pub(in crate::gateway::proxy::handler) fn no_enabled_provider_message(cli_key: &str) -> String {
    format!("no enabled provider for cli_key={cli_key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_enabled_provider_message_preserves_cli_key() {
        assert_eq!(
            no_enabled_provider_message("codex"),
            "no enabled provider for cli_key=codex"
        );
    }
}
