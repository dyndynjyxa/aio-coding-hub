//! Usage: Provider gating helpers (circuit allow/skip + event emission).

use super::context::CommonCtx;
use crate::circuit_breaker;
use crate::gateway::proxy::provider_router;
use crate::gateway::util::now_unix_seconds;

pub(super) struct ProviderGateInput<'a> {
    pub(super) ctx: CommonCtx<'a>,
    pub(super) provider_id: i64,
    pub(super) provider_name_base: &'a String,
    pub(super) provider_base_url_display: &'a String,
    pub(super) earliest_available_unix: &'a mut Option<i64>,
    pub(super) skipped_open: &'a mut usize,
    pub(super) skipped_cooldown: &'a mut usize,
}

pub(super) struct ProviderGateAllow {
    pub(super) circuit_after: circuit_breaker::CircuitSnapshot,
}

pub(super) fn gate_provider(input: ProviderGateInput<'_>) -> Option<ProviderGateAllow> {
    let ProviderGateInput {
        ctx,
        provider_id,
        provider_name_base,
        provider_base_url_display,
        earliest_available_unix,
        skipped_open,
        skipped_cooldown,
    } = input;

    let now_unix = now_unix_seconds() as i64;
    provider_router::gate_provider(provider_router::GateProviderArgs {
        app: Some(&ctx.state.app),
        circuit: ctx.state.circuit.as_ref(),
        trace_id: ctx.trace_id.as_str(),
        cli_key: ctx.cli_key.as_str(),
        provider_id,
        provider_name: provider_name_base.as_str(),
        provider_base_url_display: provider_base_url_display.as_str(),
        now_unix,
        earliest_available_unix,
        skipped_open,
        skipped_cooldown,
    })
    .map(|circuit_after| ProviderGateAllow { circuit_after })
}
