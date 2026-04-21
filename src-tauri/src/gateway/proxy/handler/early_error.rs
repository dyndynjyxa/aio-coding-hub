//! Shared early-error infrastructure for handler middlewares.
//!
//! Provides types and helpers for returning structured error responses before
//! the request reaches the failover/forwarder stage.

use super::SpecialSettings;
use crate::gateway::proxy::errors::error_response;
use crate::gateway::proxy::request_end::{
    emit_request_event_and_enqueue_request_log, emit_request_event_and_spawn_request_log,
    RequestEndArgs, RequestEndDeps,
};
use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};
use crate::gateway::runtime::GatewayAppState;
use crate::shared::mutex_ext::MutexExt;
use axum::http::StatusCode;
use axum::response::Response;

// ---------------------------------------------------------------------------
// Early-error contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) enum EarlyErrorKind {
    CliProxyDisabled,
    BodyTooLarge,
    LargeBodyMissingModel,
    InvalidCliKey,
    NoEnabledProvider,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EarlyErrorContract {
    pub(super) status: StatusCode,
    pub(super) error_code: &'static str,
    pub(super) error_category: Option<&'static str>,
    pub(super) excluded_from_stats: bool,
}

pub(super) fn early_error_contract(kind: EarlyErrorKind) -> EarlyErrorContract {
    match kind {
        EarlyErrorKind::CliProxyDisabled => EarlyErrorContract {
            status: StatusCode::FORBIDDEN,
            error_code: GatewayErrorCode::CliProxyDisabled.as_str(),
            error_category: Some(ErrorCategory::NonRetryableClientError.as_str()),
            excluded_from_stats: true,
        },
        EarlyErrorKind::BodyTooLarge => EarlyErrorContract {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            error_code: GatewayErrorCode::BodyTooLarge.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
        EarlyErrorKind::LargeBodyMissingModel => EarlyErrorContract {
            status: StatusCode::BAD_REQUEST,
            error_code: GatewayErrorCode::LargeBodyMissingModel.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
        EarlyErrorKind::InvalidCliKey => EarlyErrorContract {
            status: StatusCode::BAD_REQUEST,
            error_code: GatewayErrorCode::InvalidCliKey.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
        EarlyErrorKind::NoEnabledProvider => EarlyErrorContract {
            status: StatusCode::SERVICE_UNAVAILABLE,
            error_code: GatewayErrorCode::NoEnabledProvider.as_str(),
            error_category: None,
            excluded_from_stats: false,
        },
    }
}

// ---------------------------------------------------------------------------
// Forced provider
// ---------------------------------------------------------------------------

pub(super) fn extract_forced_provider_id(headers: &axum::http::HeaderMap) -> Option<i64> {
    let raw = headers.get("x-aio-provider-id")?.to_str().ok()?.trim();
    let provider_id = raw.parse::<i64>().ok()?;
    (provider_id > 0).then_some(provider_id)
}

pub(super) fn force_provider_if_requested(
    providers: &mut Vec<crate::providers::ProviderForGateway>,
    provider_id: Option<i64>,
    special_settings: &SpecialSettings,
) {
    let Some(provider_id) = provider_id else {
        return;
    };

    if let Some(index) = providers.iter().position(|p| p.id == provider_id) {
        if index > 0 {
            providers.rotate_left(index);
        }
        providers.truncate(1);
        push_special_setting(
            special_settings,
            serde_json::json!({
                "type": "provider_lock",
                "scope": "request",
                "hit": true,
                "providerId": provider_id,
            }),
        );
    } else {
        providers.clear();
    }
}

// ---------------------------------------------------------------------------
// Special settings helpers
// ---------------------------------------------------------------------------

pub(super) fn push_special_setting(special_settings: &SpecialSettings, setting: serde_json::Value) {
    let mut settings = special_settings.lock_or_recover();
    settings.push(setting);
}

// ---------------------------------------------------------------------------
// Early-error logging context
// ---------------------------------------------------------------------------

pub(super) struct EarlyErrorLogCtx<'a> {
    pub(super) state: &'a GatewayAppState,
    pub(super) trace_id: &'a str,
    pub(super) cli_key: &'a str,
    pub(super) method_hint: &'a str,
    pub(super) forwarded_path: &'a str,
    pub(super) observe: bool,
    pub(super) query: Option<&'a str>,
    pub(super) duration_ms: u128,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
}

pub(super) fn build_early_error_log_ctx<'a>(
    ctx: &'a super::middleware::ProxyContext,
) -> EarlyErrorLogCtx<'a> {
    EarlyErrorLogCtx {
        state: &ctx.state,
        trace_id: &ctx.trace_id,
        cli_key: &ctx.cli_key,
        method_hint: &ctx.method_hint,
        forwarded_path: &ctx.forwarded_path,
        observe: ctx.observe_request,
        query: ctx.query.as_deref(),
        duration_ms: ctx.started.elapsed().as_millis(),
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
    }
}

// ---------------------------------------------------------------------------
// Early-error response builders
// ---------------------------------------------------------------------------

fn build_early_error_response(
    trace_id: &str,
    contract: EarlyErrorContract,
    message: String,
) -> Response {
    error_response(
        contract.status,
        trace_id.to_string(),
        contract.error_code,
        message,
        vec![],
    )
}

fn early_error_request_end_args<'a>(
    ctx: &'a EarlyErrorLogCtx<'a>,
    contract: EarlyErrorContract,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> RequestEndArgs<'a> {
    RequestEndArgs {
        deps: RequestEndDeps::new(&ctx.state.app, &ctx.state.db, &ctx.state.log_tx),
        trace_id: ctx.trace_id,
        cli_key: ctx.cli_key,
        method: ctx.method_hint,
        path: ctx.forwarded_path,
        observe: ctx.observe,
        query: ctx.query,
        excluded_from_stats: contract.excluded_from_stats,
        status: Some(contract.status.as_u16()),
        error_category: contract.error_category,
        error_code: Some(contract.error_code),
        duration_ms: ctx.duration_ms,
        event_ttfb_ms: None,
        log_ttfb_ms: None,
        attempts: &[],
        special_settings_json,
        session_id,
        requested_model,
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
        usage_metrics: None,
        log_usage_metrics: None,
        usage: None,
    }
}

pub(super) async fn respond_early_error_with_enqueue(
    ctx: &EarlyErrorLogCtx<'_>,
    contract: EarlyErrorContract,
    message: String,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> Response {
    let resp = build_early_error_response(ctx.trace_id, contract, message);
    emit_request_event_and_enqueue_request_log(early_error_request_end_args(
        ctx,
        contract,
        special_settings_json,
        session_id,
        requested_model,
    ))
    .await;
    resp
}

pub(super) fn respond_early_error_with_spawn(
    ctx: &EarlyErrorLogCtx<'_>,
    contract: EarlyErrorContract,
    message: String,
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
) -> Response {
    let resp = build_early_error_response(ctx.trace_id, contract, message);
    emit_request_event_and_spawn_request_log(early_error_request_end_args(
        ctx,
        contract,
        special_settings_json,
        session_id,
        requested_model,
    ));
    resp
}

pub(super) fn respond_invalid_cli_key_with_spawn(
    ctx: &EarlyErrorLogCtx<'_>,
    session_id: Option<String>,
    requested_model: Option<String>,
    err: String,
) -> Response {
    let contract = early_error_contract(EarlyErrorKind::InvalidCliKey);
    respond_early_error_with_spawn(ctx, contract, err, None, session_id, requested_model)
}
