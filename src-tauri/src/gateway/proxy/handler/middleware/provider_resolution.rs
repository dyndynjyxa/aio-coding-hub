//! Middleware: resolves session routing and selects providers with session binding.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, force_provider_if_requested,
    push_special_setting, respond_early_error_with_enqueue, respond_invalid_cli_key_with_spawn,
    EarlyErrorKind,
};
use crate::gateway::proxy::handler::provider_selection::{
    resolve_session_bound_provider_id, resolve_session_routing_decision,
    select_providers_with_session_binding,
};
use crate::gateway::response_fixer;

pub(in crate::gateway::proxy::handler) struct ProviderResolutionMiddleware;

const SESSION_ID_DIAGNOSTIC_SUFFIX_LEN: usize = 8;

impl ProviderResolutionMiddleware {
    pub(in crate::gateway::proxy::handler) async fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
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

        let initial_provider_ids = provider_ids(&selection.providers);
        ctx.effective_sort_mode_id = selection.effective_sort_mode_id;
        ctx.providers = selection.providers;

        // --- forced provider ---
        let forced_provider_missing = force_provider_if_requested(
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
            let final_provider_ids = provider_ids(&ctx.providers);
            push_special_setting(
                &ctx.special_settings,
                no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
                    cli_key: &ctx.cli_key,
                    active_sort_mode_id: selection.active_sort_mode_id,
                    effective_sort_mode_id: ctx.effective_sort_mode_id,
                    session_bound_sort_mode_id: selection.session_bound_sort_mode_id,
                    session_id: ctx.session_id.as_deref(),
                    session_bound_provider_id: ctx.session_bound_provider_id,
                    forced_provider_id: ctx.forced_provider_id,
                    initial_provider_ids: &initial_provider_ids,
                    final_provider_ids: &final_provider_ids,
                    forced_provider_missing,
                }),
            );
            let contract = early_error_contract(EarlyErrorKind::NoEnabledProvider);
            let message = no_enabled_provider_message(&ctx.cli_key);
            let session_id = ctx.session_id.take();
            let requested_model = ctx.requested_model.take();
            let special_settings_json =
                response_fixer::special_settings_json(&ctx.special_settings);
            let log_ctx = build_early_error_log_ctx(&ctx);

            let resp = respond_early_error_with_enqueue(
                &log_ctx,
                contract,
                message,
                special_settings_json,
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

struct NoEnabledProviderDiagnosticArgs<'a> {
    cli_key: &'a str,
    active_sort_mode_id: Option<i64>,
    effective_sort_mode_id: Option<i64>,
    session_bound_sort_mode_id: Option<Option<i64>>,
    session_id: Option<&'a str>,
    session_bound_provider_id: Option<i64>,
    forced_provider_id: Option<i64>,
    initial_provider_ids: &'a [i64],
    final_provider_ids: &'a [i64],
    forced_provider_missing: bool,
}

fn no_enabled_provider_diagnostic(args: &NoEnabledProviderDiagnosticArgs<'_>) -> serde_json::Value {
    let sort_mode = match args.effective_sort_mode_id {
        Some(id) => serde_json::json!({"kind": "custom", "modeId": id}),
        None => serde_json::json!({"kind": "default", "modeId": serde_json::Value::Null}),
    };
    let cleared_reason = if args.forced_provider_missing {
        "forced_provider_not_in_candidates"
    } else if args.effective_sort_mode_id.is_some() {
        "empty_sort_mode_candidates"
    } else {
        "empty_default_candidates"
    };

    serde_json::json!({
        "type": "provider_selection_diagnostic",
        "scope": "request",
        "hit": true,
        "reason": "no_enabled_provider",
        "clearedReason": cleared_reason,
        "cliKey": args.cli_key,
        "sortMode": sort_mode,
        "activeSortModeId": args.active_sort_mode_id,
        "effectiveSortModeId": args.effective_sort_mode_id,
        "sessionBoundSortModeId": args.session_bound_sort_mode_id,
        "sortModeSource": if args.session_bound_sort_mode_id.is_some() {
            "session_bound"
        } else {
            "active"
        },
        "sessionIdPresent": args.session_id.is_some(),
        "sessionIdSuffix": args.session_id.map(diagnostic_session_suffix),
        "sessionBoundProviderId": args.session_bound_provider_id,
        "forcedProviderId": args.forced_provider_id,
        "forcedProviderMissing": args.forced_provider_missing,
        "candidateProviderIdsBeforeForce": args.initial_provider_ids,
        "candidateProviderCountBeforeForce": args.initial_provider_ids.len(),
        "candidateProviderIdsAfterForce": args.final_provider_ids,
        "candidateProviderCountAfterForce": args.final_provider_ids.len(),
    })
}

fn provider_ids(providers: &[crate::providers::ProviderForGateway]) -> Vec<i64> {
    providers.iter().map(|provider| provider.id).collect()
}

fn diagnostic_session_suffix(session_id: &str) -> String {
    let suffix: Vec<char> = session_id
        .chars()
        .rev()
        .take(SESSION_ID_DIAGNOSTIC_SUFFIX_LEN)
        .collect();
    suffix.into_iter().rev().collect()
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

    #[test]
    fn no_enabled_provider_diagnostic_marks_empty_active_candidates() {
        let value = no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
            cli_key: "claude",
            active_sort_mode_id: Some(6),
            effective_sort_mode_id: Some(6),
            session_bound_sort_mode_id: None,
            session_id: Some("01234567-89ab-cdef-0123-456789abcdef"),
            session_bound_provider_id: None,
            forced_provider_id: None,
            initial_provider_ids: &[],
            final_provider_ids: &[],
            forced_provider_missing: false,
        });

        assert_eq!(
            value.get("type").and_then(|v| v.as_str()),
            Some("provider_selection_diagnostic")
        );
        assert_eq!(
            value.get("clearedReason").and_then(|v| v.as_str()),
            Some("empty_sort_mode_candidates")
        );
        assert_eq!(
            value.pointer("/sortMode/kind").and_then(|v| v.as_str()),
            Some("custom")
        );
        assert_eq!(
            value.get("activeSortModeId").and_then(|v| v.as_i64()),
            Some(6)
        );
        assert_eq!(
            value.get("effectiveSortModeId").and_then(|v| v.as_i64()),
            Some(6)
        );
        assert_eq!(
            value.get("sortModeSource").and_then(|v| v.as_str()),
            Some("active")
        );
        assert_eq!(
            value.get("sessionIdPresent").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            value.get("sessionIdSuffix").and_then(|v| v.as_str()),
            Some("89abcdef")
        );
        assert!(!value.to_string().contains("01234567-89ab-cdef"));
        assert_eq!(
            value
                .get("candidateProviderCountBeforeForce")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[test]
    fn no_enabled_provider_diagnostic_marks_forced_provider_missing() {
        let value = no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
            cli_key: "claude",
            active_sort_mode_id: Some(7),
            effective_sort_mode_id: None,
            session_bound_sort_mode_id: Some(None),
            session_id: None,
            session_bound_provider_id: Some(11),
            forced_provider_id: Some(99),
            initial_provider_ids: &[11, 22],
            final_provider_ids: &[],
            forced_provider_missing: true,
        });

        assert_eq!(
            value.get("clearedReason").and_then(|v| v.as_str()),
            Some("forced_provider_not_in_candidates")
        );
        assert_eq!(
            value.pointer("/sortMode/kind").and_then(|v| v.as_str()),
            Some("default")
        );
        assert_eq!(
            value.get("activeSortModeId").and_then(|v| v.as_i64()),
            Some(7)
        );
        assert_eq!(
            value.get("sortModeSource").and_then(|v| v.as_str()),
            Some("session_bound")
        );
        assert_eq!(
            value
                .get("candidateProviderIdsBeforeForce")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>()),
            Some(vec![11, 22])
        );
        assert_eq!(
            value.get("forcedProviderId").and_then(|v| v.as_i64()),
            Some(99)
        );
        assert_eq!(
            value.get("forcedProviderMissing").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
