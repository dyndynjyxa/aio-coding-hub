//! Usage: Finalize responses for failover loop terminal states.

use super::context::AttemptOutcome;
use super::{
    emit_request_event_and_enqueue_request_log, RequestEndArgs, RequestEndContextArgs,
    RequestEndDeps,
};
use crate::gateway::events::FailoverAttempt;
use crate::gateway::proxy::abort_guard::RequestAbortGuard;
use crate::gateway::proxy::caches::CachedGatewayError;
use crate::gateway::proxy::errors::{error_response, error_response_with_retry_after};
use crate::gateway::proxy::request_end::RequestCompletion;
use crate::gateway::proxy::GatewayErrorCode;
use crate::gateway::response_fixer;
use crate::gateway::runtime::GatewayAppState;
use crate::gateway::util::now_unix_seconds;
use crate::shared::mutex_ext::MutexExt;
use axum::http::StatusCode;
use axum::response::Response;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(super) struct AllUnavailableInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) state: &'a GatewayAppState<R>,
    pub(super) abort_guard: &'a mut RequestAbortGuard<R>,
    pub(super) observe: bool,
    pub(super) attempts: Vec<FailoverAttempt>,
    pub(super) cli_key: String,
    pub(super) method_hint: String,
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) verbose_provider_error: bool,
    pub(super) earliest_available_unix: Option<i64>,
    pub(super) skipped_open: usize,
    pub(super) skipped_cooldown: usize,
    pub(super) skipped_limits: usize,
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
}

pub(super) async fn all_providers_unavailable<R: tauri::Runtime>(
    input: AllUnavailableInput<'_, R>,
) -> Response {
    let AllUnavailableInput {
        state,
        abort_guard,
        observe,
        attempts,
        cli_key,
        method_hint,
        forwarded_path,
        query,
        trace_id,
        started,
        created_at_ms,
        created_at,
        session_id,
        requested_model,
        special_settings,
        verbose_provider_error,
        earliest_available_unix,
        skipped_open,
        skipped_cooldown,
        skipped_limits,
        fingerprint_key,
        fingerprint_debug,
        unavailable_fingerprint_key,
        unavailable_fingerprint_debug,
    } = input;

    let now_unix = now_unix_seconds() as i64;
    let retry_after_seconds = earliest_available_unix
        .and_then(|t| t.checked_sub(now_unix))
        .filter(|v| *v > 0)
        .map(|v| v as u64);

    let detailed_message = format!(
        "no provider available (skipped: open={skipped_open}, cooldown={skipped_cooldown}, limits={skipped_limits}) for cli_key={cli_key}",
    );
    let message = if verbose_provider_error {
        detailed_message
    } else {
        "No available providers".to_string()
    };

    // Disk log: all providers unavailable (circuit breaker / cooldown / limits).
    tracing::error!(
        trace_id = %trace_id,
        error_code = GatewayErrorCode::AllProvidersUnavailable.as_str(),
        cli_key = %cli_key,
        skipped_open = skipped_open,
        skipped_cooldown = skipped_cooldown,
        skipped_limits = skipped_limits,
        "all providers unavailable"
    );

    let resp = error_response_with_retry_after(
        StatusCode::SERVICE_UNAVAILABLE,
        trace_id.clone(),
        GatewayErrorCode::AllProvidersUnavailable.as_str(),
        message.clone(),
        if verbose_provider_error {
            attempts.clone()
        } else {
            vec![]
        },
        retry_after_seconds,
    );

    let duration_ms = started.elapsed().as_millis();
    emit_request_event_and_enqueue_request_log(
        RequestEndArgs::from_context(RequestEndContextArgs {
            deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method: method_hint.as_str(),
            path: forwarded_path.as_str(),
            observe,
            query: query.as_deref(),
            excluded_from_stats: false,
            duration_ms,
            attempts: attempts.as_slice(),
            special_settings_json: response_fixer::special_settings_json(&special_settings),
            session_id,
            requested_model,
            created_at_ms,
            created_at,
        })
        .with_completion(RequestCompletion::failure(
            StatusCode::SERVICE_UNAVAILABLE.as_u16(),
            None,
            GatewayErrorCode::AllProvidersUnavailable.as_str(),
        )),
    )
    .await;

    if let Some(retry_after_seconds) = retry_after_seconds.filter(|v| *v > 0) {
        let mut cache = state.recent_errors.lock_or_recover();
        cache.insert_error(
            now_unix,
            unavailable_fingerprint_key,
            CachedGatewayError {
                trace_id: trace_id.clone(),
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: GatewayErrorCode::AllProvidersUnavailable.as_str(),
                message: message.clone(),
                retry_after_seconds: Some(retry_after_seconds),
                expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                fingerprint_debug: unavailable_fingerprint_debug.clone(),
            },
        );
        cache.insert_error(
            now_unix,
            fingerprint_key,
            CachedGatewayError {
                trace_id: trace_id.clone(),
                status: StatusCode::SERVICE_UNAVAILABLE,
                error_code: GatewayErrorCode::AllProvidersUnavailable.as_str(),
                message,
                retry_after_seconds: Some(retry_after_seconds),
                expires_at_unix: now_unix.saturating_add(retry_after_seconds as i64),
                fingerprint_debug: fingerprint_debug.clone(),
            },
        );
    }

    abort_guard.disarm();
    resp
}

pub(super) struct AllFailedInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) state: &'a GatewayAppState<R>,
    pub(super) abort_guard: &'a mut RequestAbortGuard<R>,
    pub(super) observe: bool,
    pub(super) attempts: Vec<FailoverAttempt>,
    pub(super) last_outcome: Option<AttemptOutcome>,
    pub(super) cli_key: String,
    pub(super) method_hint: String,
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) started: Instant,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(super) verbose_provider_error: bool,
}

pub(super) async fn all_providers_failed<R: tauri::Runtime>(
    input: AllFailedInput<'_, R>,
) -> Response {
    let AllFailedInput {
        state,
        abort_guard,
        observe,
        attempts,
        last_outcome,
        cli_key,
        method_hint,
        forwarded_path,
        query,
        trace_id,
        started,
        created_at_ms,
        created_at,
        session_id,
        requested_model,
        special_settings,
        verbose_provider_error,
    } = input;

    let final_error_code = last_outcome
        .map(|outcome| outcome.error_code)
        .unwrap_or(GatewayErrorCode::UpstreamAllFailed.as_str());
    let final_error_category = last_outcome.map(|outcome| outcome.error_category);

    // Disk log: all providers tried and failed.
    tracing::error!(
        trace_id = %trace_id,
        error_code = final_error_code,
        cli_key = %cli_key,
        attempt_count = attempts.len(),
        duration_ms = %started.elapsed().as_millis(),
        "all providers failed"
    );

    let resp = error_response(
        StatusCode::BAD_GATEWAY,
        trace_id.clone(),
        final_error_code,
        format!("all providers failed for cli_key={cli_key}"),
        if verbose_provider_error {
            attempts.clone()
        } else {
            vec![]
        },
    );

    let duration_ms = started.elapsed().as_millis();
    emit_request_event_and_enqueue_request_log(
        RequestEndArgs::from_context(RequestEndContextArgs {
            deps: RequestEndDeps::new(&state.app, &state.db, &state.log_tx),
            trace_id: trace_id.as_str(),
            cli_key: cli_key.as_str(),
            method: method_hint.as_str(),
            path: forwarded_path.as_str(),
            observe,
            query: query.as_deref(),
            excluded_from_stats: false,
            duration_ms,
            attempts: attempts.as_slice(),
            special_settings_json: response_fixer::special_settings_json(&special_settings),
            session_id,
            requested_model,
            created_at_ms,
            created_at,
        })
        .with_completion(RequestCompletion::failure(
            StatusCode::BAD_GATEWAY.as_u16(),
            final_error_category,
            final_error_code,
        )),
    )
    .await;

    abort_guard.disarm();
    resp
}
