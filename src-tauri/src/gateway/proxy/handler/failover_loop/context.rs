//! Usage: Shared context types for `failover_loop` internal submodules.

use super::super::super::abort_guard::RequestAbortGuard;
use super::super::super::gemini_oauth;
use crate::circuit_breaker;
use crate::gateway::events::FailoverAttempt;
use crate::gateway::manager::GatewayAppState;
use crate::gateway::proxy::cx2cc::settings::Cx2ccSettings;
use crate::gateway::response_fixer;
use crate::gateway::streams::StreamFinalizeCtx;
use axum::response::Response;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) const MAX_NON_SSE_BODY_BYTES: usize = 20 * 1024 * 1024;

pub(super) struct CommonCtxArgs<'a> {
    pub(super) state: &'a GatewayAppState,
    pub(super) cli_key: &'a String,
    pub(super) forwarded_path: &'a String,
    pub(super) method_hint: &'a String,
    pub(super) query: &'a Option<String>,
    pub(super) trace_id: &'a String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: &'a Option<String>,
    pub(super) requested_model: &'a Option<String>,
    pub(super) cx2cc_settings: &'a Cx2ccSettings,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) special_settings: &'a Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_first_byte_timeout: Option<Duration>,
    pub(super) upstream_stream_idle_timeout: Option<Duration>,
    pub(super) upstream_request_timeout_non_streaming: Option<Duration>,
    pub(super) verbose_provider_error: bool,
    pub(super) max_attempts_per_provider: u32,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) introspection_body: &'a [u8],
}

#[derive(Clone, Copy)]
pub(super) struct CommonCtx<'a> {
    pub(super) state: &'a GatewayAppState,
    pub(super) cli_key: &'a String,
    pub(super) forwarded_path: &'a String,
    pub(super) method_hint: &'a String,
    pub(super) query: &'a Option<String>,
    pub(super) trace_id: &'a String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: &'a Option<String>,
    pub(super) requested_model: &'a Option<String>,
    pub(super) cx2cc_settings: &'a Cx2ccSettings,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) special_settings: &'a Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_first_byte_timeout: Option<Duration>,
    pub(super) upstream_stream_idle_timeout: Option<Duration>,
    pub(super) upstream_request_timeout_non_streaming: Option<Duration>,
    pub(super) verbose_provider_error: bool,
    pub(super) max_attempts_per_provider: u32,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) introspection_body: &'a [u8],
}

impl<'a> CommonCtx<'a> {
    pub(super) fn new(args: CommonCtxArgs<'a>) -> Self {
        Self {
            state: args.state,
            cli_key: args.cli_key,
            forwarded_path: args.forwarded_path,
            method_hint: args.method_hint,
            query: args.query,
            trace_id: args.trace_id,
            started: args.started,
            created_at_ms: args.created_at_ms,
            created_at: args.created_at,
            session_id: args.session_id,
            requested_model: args.requested_model,
            cx2cc_settings: args.cx2cc_settings,
            effective_sort_mode_id: args.effective_sort_mode_id,
            special_settings: args.special_settings,
            provider_cooldown_secs: args.provider_cooldown_secs,
            upstream_first_byte_timeout_secs: args.upstream_first_byte_timeout_secs,
            upstream_first_byte_timeout: args.upstream_first_byte_timeout,
            upstream_stream_idle_timeout: args.upstream_stream_idle_timeout,
            upstream_request_timeout_non_streaming: args.upstream_request_timeout_non_streaming,
            verbose_provider_error: args.verbose_provider_error,
            max_attempts_per_provider: args.max_attempts_per_provider,
            enable_response_fixer: args.enable_response_fixer,
            response_fixer_stream_config: args.response_fixer_stream_config,
            response_fixer_non_stream_config: args.response_fixer_non_stream_config,
            introspection_body: args.introspection_body,
        }
    }
}

impl<'a> From<CommonCtxArgs<'a>> for CommonCtx<'a> {
    fn from(args: CommonCtxArgs<'a>) -> Self {
        Self::new(args)
    }
}

pub(super) struct CommonCtxOwned<'a> {
    pub(super) state: &'a GatewayAppState,
    pub(super) cli_key: String,
    pub(super) forwarded_path: String,
    pub(super) method_hint: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) cx2cc_settings: Cx2ccSettings,
    pub(super) effective_sort_mode_id: Option<i64>,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) provider_cooldown_secs: i64,
    pub(super) upstream_first_byte_timeout_secs: u32,
    pub(super) upstream_first_byte_timeout: Option<Duration>,
    pub(super) upstream_stream_idle_timeout: Option<Duration>,
    pub(super) upstream_request_timeout_non_streaming: Option<Duration>,
    pub(super) max_attempts_per_provider: u32,
    pub(super) enable_response_fixer: bool,
    pub(super) response_fixer_stream_config: response_fixer::ResponseFixerConfig,
    pub(super) response_fixer_non_stream_config: response_fixer::ResponseFixerConfig,
}

impl<'a> From<CommonCtx<'a>> for CommonCtxOwned<'a> {
    fn from(ctx: CommonCtx<'a>) -> Self {
        Self {
            state: ctx.state,
            cli_key: ctx.cli_key.clone(),
            forwarded_path: ctx.forwarded_path.clone(),
            method_hint: ctx.method_hint.clone(),
            query: ctx.query.clone(),
            trace_id: ctx.trace_id.clone(),
            started: ctx.started,
            created_at_ms: ctx.created_at_ms,
            created_at: ctx.created_at,
            session_id: ctx.session_id.clone(),
            requested_model: ctx.requested_model.clone(),
            cx2cc_settings: ctx.cx2cc_settings.clone(),
            effective_sort_mode_id: ctx.effective_sort_mode_id,
            special_settings: Arc::clone(ctx.special_settings),
            provider_cooldown_secs: ctx.provider_cooldown_secs,
            upstream_first_byte_timeout_secs: ctx.upstream_first_byte_timeout_secs,
            upstream_first_byte_timeout: ctx.upstream_first_byte_timeout,
            upstream_stream_idle_timeout: ctx.upstream_stream_idle_timeout,
            upstream_request_timeout_non_streaming: ctx.upstream_request_timeout_non_streaming,
            max_attempts_per_provider: ctx.max_attempts_per_provider,
            enable_response_fixer: ctx.enable_response_fixer,
            response_fixer_stream_config: ctx.response_fixer_stream_config,
            response_fixer_non_stream_config: ctx.response_fixer_non_stream_config,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ProviderCtx<'a> {
    pub(super) provider_id: i64,
    pub(super) provider_name_base: &'a String,
    pub(super) provider_base_url_base: &'a String,
    pub(super) provider_index: u32,
    pub(super) session_reuse: Option<bool>,
}

pub(super) struct ProviderCtxOwned {
    pub(super) provider_id: i64,
    pub(super) provider_name_base: String,
    pub(super) provider_base_url_base: String,
    pub(super) provider_index: u32,
    pub(super) session_reuse: Option<bool>,
}

impl<'a> From<ProviderCtx<'a>> for ProviderCtxOwned {
    fn from(ctx: ProviderCtx<'a>) -> Self {
        Self {
            provider_id: ctx.provider_id,
            provider_name_base: ctx.provider_name_base.clone(),
            provider_base_url_base: ctx.provider_base_url_base.clone(),
            provider_index: ctx.provider_index,
            session_reuse: ctx.session_reuse,
        }
    }
}

pub(super) fn build_stream_finalize_ctx(
    ctx: &CommonCtxOwned<'_>,
    provider_ctx: &ProviderCtxOwned,
    attempts: &[FailoverAttempt],
    status: u16,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
) -> StreamFinalizeCtx {
    let attempts_json = serde_json::to_string(attempts).unwrap_or_else(|_| "[]".to_string());

    StreamFinalizeCtx {
        app: ctx.state.app.clone(),
        db: ctx.state.db.clone(),
        log_tx: ctx.state.log_tx.clone(),
        circuit: ctx.state.circuit.clone(),
        session: ctx.state.session.clone(),
        session_id: ctx.session_id.clone(),
        sort_mode_id: ctx.effective_sort_mode_id,
        trace_id: ctx.trace_id.clone(),
        cli_key: ctx.cli_key.clone(),
        method: ctx.method_hint.clone(),
        path: ctx.forwarded_path.clone(),
        query: ctx.query.clone(),
        excluded_from_stats: false,
        special_settings: Arc::clone(&ctx.special_settings),
        status,
        error_category,
        error_code,
        started: ctx.started,
        attempts: attempts.to_vec(),
        attempts_json,
        requested_model: ctx.requested_model.clone(),
        created_at_ms: ctx.created_at_ms,
        created_at: ctx.created_at,
        provider_cooldown_secs: ctx.provider_cooldown_secs,
        provider_id: provider_ctx.provider_id,
        provider_name: provider_ctx.provider_name_base.clone(),
        base_url: provider_ctx.provider_base_url_base.clone(),
    }
}

#[derive(Clone, Copy)]
pub(super) struct AttemptCtx<'a> {
    pub(super) attempt_index: u32,
    pub(super) retry_index: u32,
    pub(super) attempt_started_ms: u128,
    pub(super) attempt_started: Instant,
    pub(super) circuit_before: &'a circuit_breaker::CircuitSnapshot,
    pub(super) gemini_oauth_response_mode: Option<gemini_oauth::GeminiOAuthResponseMode>,
    pub(super) cx2cc_active: bool,
    pub(super) anthropic_stream_requested: bool,
}

pub(super) struct LoopState<'a> {
    pub(super) attempts: &'a mut Vec<FailoverAttempt>,
    pub(super) failed_provider_ids: &'a mut HashSet<i64>,
    pub(super) last_error_category: &'a mut Option<&'static str>,
    pub(super) last_error_code: &'a mut Option<&'static str>,
    pub(super) circuit_snapshot: &'a mut circuit_breaker::CircuitSnapshot,
    pub(super) abort_guard: &'a mut RequestAbortGuard,
}

impl<'a> LoopState<'a> {
    pub(super) fn new(
        attempts: &'a mut Vec<FailoverAttempt>,
        failed_provider_ids: &'a mut HashSet<i64>,
        last_error_category: &'a mut Option<&'static str>,
        last_error_code: &'a mut Option<&'static str>,
        circuit_snapshot: &'a mut circuit_breaker::CircuitSnapshot,
        abort_guard: &'a mut RequestAbortGuard,
    ) -> Self {
        Self {
            attempts,
            failed_provider_ids,
            last_error_category,
            last_error_code,
            circuit_snapshot,
            abort_guard,
        }
    }
}

pub(super) enum LoopControl {
    ContinueRetry,
    BreakRetry,
    Return(Response),
}
