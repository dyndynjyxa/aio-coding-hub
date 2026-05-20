//! Usage: Shared stream finalize helpers (cooldown/circuit/session).

use super::super::proxy::{provider_router, ErrorCategory, GatewayErrorCode};
use super::super::util::now_unix_seconds;
use super::StreamFinalizeCtx;

pub(super) fn finalize_circuit_and_session<R: tauri::Runtime>(
    ctx: &StreamFinalizeCtx<R>,
    error_code: Option<&'static str>,
) -> Option<&'static str> {
    let effective_error_category = if error_code == Some(GatewayErrorCode::StreamAborted.as_str()) {
        Some(ErrorCategory::ClientAbort.as_str())
    } else if error_code == Some(GatewayErrorCode::Fake200.as_str()) {
        Some(ErrorCategory::ProviderError.as_str())
    } else {
        ctx.error_category
    };

    let now_unix = now_unix_seconds() as i64;

    if error_code.is_some()
        && effective_error_category != Some(ErrorCategory::ClientAbort.as_str())
        && ctx.provider_cooldown_secs > 0
    {
        provider_router::trigger_cooldown(
            ctx.circuit.as_ref(),
            ctx.provider_id,
            now_unix,
            ctx.provider_cooldown_secs,
        );
    }

    if error_code.is_none() && (200..300).contains(&ctx.status) && !ctx.fake_200_detected {
        let _ = provider_router::record_success_and_emit_transition(
            provider_router::RecordCircuitArgs::from_stream_ctx(ctx, now_unix),
        );
        if let Some(session_id) = ctx.session_id.as_deref() {
            ctx.session.bind_success(
                &ctx.cli_key,
                session_id,
                ctx.provider_id,
                ctx.sort_mode_id,
                now_unix,
            );
        }
    } else if ctx.fake_200_detected && (200..300).contains(&ctx.status) {
        // Fake 200: upstream returned HTTP 200 but body contained an error payload.
        // Record as failure for circuit breaker; do not bind session.
        let _ = provider_router::record_failure_and_emit_transition(
            provider_router::RecordCircuitArgs::from_stream_ctx(ctx, now_unix),
        );
    } else if effective_error_category == Some(ErrorCategory::ProviderError.as_str()) {
        let _ = provider_router::record_failure_and_emit_transition(
            provider_router::RecordCircuitArgs::from_stream_ctx(ctx, now_unix),
        );
    }

    effective_error_category
}
