use crate::{circuit_breaker, notice, settings, usage};
use serde::Serialize;
use tauri::Manager;

pub(crate) const GATEWAY_STATUS_EVENT_NAME: &str = "gateway:status";
pub(crate) const GATEWAY_REQUEST_START_EVENT_NAME: &str = "gateway:request_start";
pub(crate) const GATEWAY_ATTEMPT_EVENT_NAME: &str = "gateway:attempt";
pub(crate) const GATEWAY_REQUEST_EVENT_NAME: &str = "gateway:request";
pub(crate) const GATEWAY_REQUEST_SIGNAL_EVENT_NAME: &str = "gateway:request_signal";
pub(crate) const GATEWAY_LOG_EVENT_NAME: &str = "gateway:log";
pub(crate) const GATEWAY_CIRCUIT_EVENT_NAME: &str = "gateway:circuit";

use crate::app::heartbeat_watchdog::gated_emit;

const MAIN_WINDOW_LABEL: &str = "main";

pub(in crate::gateway) mod decision_chain {
    pub(in crate::gateway) const SELECTION_METHOD_SESSION_REUSE: &str = "session_reuse";
    pub(in crate::gateway) const SELECTION_METHOD_ORDERED: &str = "ordered";
    pub(in crate::gateway) const SELECTION_METHOD_FILTERED: &str = "filtered";

    pub(in crate::gateway) const REASON_REQUEST_SUCCESS: &str = "request_success";
    pub(in crate::gateway) const REASON_RETRY_SUCCESS: &str = "retry_success";
    pub(in crate::gateway) const REASON_RETRY_FAILED: &str = "retry_failed";
    pub(in crate::gateway) const REASON_SYSTEM_ERROR: &str = "system_error";
    pub(in crate::gateway) const REASON_RESOURCE_NOT_FOUND: &str = "resource_not_found";
    pub(in crate::gateway) const REASON_CLIENT_ERROR_NON_RETRYABLE: &str =
        "client_error_non_retryable";
    pub(in crate::gateway) const REASON_ABORTED: &str = "aborted";
    pub(in crate::gateway) const REASON_CIRCUIT_OPEN: &str = "circuit_open";
    pub(in crate::gateway) const REASON_CIRCUIT_COOLDOWN: &str = "circuit_cooldown";
    pub(in crate::gateway) const REASON_RATE_LIMITED: &str = "rate_limited";

    /// Determine how the provider was selected for this attempt.
    /// Only meaningful for the first attempt (provider_index=1, retry_index=1).
    pub(in crate::gateway) fn selection_method(
        provider_index: u32,
        retry_index: u32,
        session_reuse: Option<bool>,
    ) -> Option<&'static str> {
        if provider_index == 1 && retry_index == 1 {
            Some(if session_reuse == Some(true) {
                SELECTION_METHOD_SESSION_REUSE
            } else {
                SELECTION_METHOD_ORDERED
            })
        } else {
            None
        }
    }

    /// Determine reason code for a successful attempt.
    pub(in crate::gateway) fn success_reason_code(
        provider_index: u32,
        retry_index: u32,
    ) -> &'static str {
        if provider_index == 1 && retry_index == 1 {
            REASON_REQUEST_SUCCESS
        } else {
            REASON_RETRY_SUCCESS
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub(super) struct FailoverAttempt {
    pub(super) provider_id: i64,
    pub(super) provider_name: String,
    pub(super) base_url: String,
    pub(super) outcome: String,
    pub(super) status: Option<u16>,
    pub(super) provider_index: Option<u32>,
    pub(super) retry_index: Option<u32>,
    pub(super) session_reuse: Option<bool>,
    pub(super) error_category: Option<&'static str>,
    pub(super) error_code: Option<&'static str>,
    pub(super) decision: Option<&'static str>,
    pub(super) reason: Option<String>,
    pub(super) selection_method: Option<&'static str>,
    pub(super) reason_code: Option<&'static str>,
    pub(super) attempt_started_ms: Option<u128>,
    pub(super) attempt_duration_ms: Option<u128>,
    pub(super) circuit_state_before: Option<&'static str>,
    pub(super) circuit_state_after: Option<&'static str>,
    pub(super) circuit_failure_count: Option<u32>,
    pub(super) circuit_failure_threshold: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
struct GatewayRequestEvent {
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    method: String,
    path: String,
    query: Option<String>,
    requested_model: Option<String>,
    status: Option<u16>,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
    duration_ms: u128,
    ttfb_ms: Option<u128>,
    attempts: Vec<FailoverAttempt>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_creation_5m_input_tokens: Option<i64>,
    cache_creation_1h_input_tokens: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
struct GatewayRequestStartEvent {
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    method: String,
    path: String,
    query: Option<String>,
    requested_model: Option<String>,
    ts: i64,
}

#[derive(Debug, Serialize, Clone)]
struct GatewayRequestSignalEvent {
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    requested_model: Option<String>,
    phase: &'static str,
    ts: i64,
}

#[derive(Debug, Serialize, Clone)]
pub(super) struct GatewayAttemptEvent {
    pub(super) trace_id: String,
    pub(super) cli_key: String,
    pub(super) session_id: Option<String>,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) query: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) attempt_index: u32,
    pub(super) provider_id: i64,
    pub(super) session_reuse: Option<bool>,
    pub(super) provider_name: String,
    pub(super) base_url: String,
    pub(super) outcome: String,
    pub(super) status: Option<u16>,
    pub(super) attempt_started_ms: u128,
    pub(super) attempt_duration_ms: u128,
    pub(super) circuit_state_before: Option<&'static str>,
    pub(super) circuit_state_after: Option<&'static str>,
    pub(super) circuit_failure_count: Option<u32>,
    pub(super) circuit_failure_threshold: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub(super) struct GatewayCircuitEvent {
    pub(super) trace_id: String,
    pub(super) cli_key: String,
    pub(super) provider_id: i64,
    pub(super) provider_name: String,
    pub(super) base_url: String,
    pub(super) prev_state: &'static str,
    pub(super) next_state: &'static str,
    pub(super) failure_count: u32,
    pub(super) failure_threshold: u32,
    pub(super) open_until: Option<i64>,
    pub(super) cooldown_until: Option<i64>,
    pub(super) reason: &'static str,
    pub(super) ts: i64,
}

#[derive(Debug, Serialize, Clone)]
pub(super) struct GatewayLogEvent {
    pub(super) level: &'static str,
    pub(super) error_code: &'static str,
    pub(super) message: String,
    pub(super) requested_port: u16,
    pub(super) bound_port: u16,
    pub(super) base_url: String,
}

pub(crate) fn emit_gateway_log(
    app: &tauri::AppHandle,
    level: &'static str,
    error_code: &'static str,
    message: String,
) {
    let payload = GatewayLogEvent {
        level,
        error_code,
        message,
        requested_port: 0,
        bound_port: 0,
        base_url: String::new(),
    };
    gated_emit(app, GATEWAY_LOG_EVENT_NAME, payload);
}

fn should_emit_gateway_detail_event(app: &tauri::AppHandle) -> bool {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return true;
    };

    let visible = window.is_visible().unwrap_or(true);
    let minimized = window.is_minimized().unwrap_or(false);
    visible && !minimized
}

fn emit_request_signal(
    app: &tauri::AppHandle,
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    requested_model: Option<String>,
    phase: &'static str,
    ts: i64,
) {
    let payload = GatewayRequestSignalEvent {
        trace_id,
        cli_key,
        session_id,
        requested_model,
        phase,
        ts,
    };
    gated_emit(app, GATEWAY_REQUEST_SIGNAL_EVENT_NAME, payload);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_request_event(
    app: &tauri::AppHandle,
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    method: String,
    path: String,
    query: Option<String>,
    requested_model: Option<String>,
    status: Option<u16>,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
    duration_ms: u128,
    ttfb_ms: Option<u128>,
    attempts: Vec<FailoverAttempt>,
    usage: Option<usage::UsageMetrics>,
) {
    emit_request_signal(
        app,
        trace_id.clone(),
        cli_key.clone(),
        session_id.clone(),
        requested_model.clone(),
        "complete",
        crate::gateway::util::now_unix_seconds() as i64,
    );

    if !should_emit_gateway_detail_event(app) {
        return;
    }

    let usage = usage.unwrap_or_default();
    let payload = GatewayRequestEvent {
        trace_id,
        cli_key,
        session_id,
        method,
        path,
        query,
        requested_model,
        status,
        error_category,
        error_code,
        duration_ms,
        ttfb_ms,
        attempts,
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        cache_read_input_tokens: usage.cache_read_input_tokens,
        cache_creation_input_tokens: usage.cache_creation_input_tokens,
        cache_creation_5m_input_tokens: usage.cache_creation_5m_input_tokens,
        cache_creation_1h_input_tokens: usage.cache_creation_1h_input_tokens,
    };

    gated_emit(app, GATEWAY_REQUEST_EVENT_NAME, payload);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_request_start_event(
    app: &tauri::AppHandle,
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    method: String,
    path: String,
    query: Option<String>,
    requested_model: Option<String>,
    ts: i64,
) {
    emit_request_signal(
        app,
        trace_id.clone(),
        cli_key.clone(),
        session_id.clone(),
        requested_model.clone(),
        "start",
        ts,
    );

    if !should_emit_gateway_detail_event(app) {
        return;
    }

    let payload = GatewayRequestStartEvent {
        trace_id,
        cli_key,
        session_id,
        method,
        path,
        query,
        requested_model,
        ts,
    };
    gated_emit(app, GATEWAY_REQUEST_START_EVENT_NAME, payload);
}

pub(super) fn emit_attempt_event(app: &tauri::AppHandle, payload: GatewayAttemptEvent) {
    if !should_emit_gateway_detail_event(app) {
        return;
    }
    gated_emit(app, GATEWAY_ATTEMPT_EVENT_NAME, payload);
}

pub(super) fn emit_circuit_event(app: &tauri::AppHandle, payload: GatewayCircuitEvent) {
    gated_emit(app, GATEWAY_CIRCUIT_EVENT_NAME, payload);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_circuit_transition(
    app: &tauri::AppHandle,
    trace_id: &str,
    cli_key: &str,
    provider_id: i64,
    provider_name: &str,
    base_url: &str,
    transition: &circuit_breaker::CircuitTransition,
    now_unix: i64,
) {
    let payload = GatewayCircuitEvent {
        trace_id: trace_id.to_string(),
        cli_key: cli_key.to_string(),
        provider_id,
        provider_name: provider_name.to_string(),
        base_url: base_url.to_string(),
        prev_state: transition.prev_state.as_str(),
        next_state: transition.next_state.as_str(),
        failure_count: transition.snapshot.failure_count,
        failure_threshold: transition.snapshot.failure_threshold,
        open_until: transition.snapshot.open_until,
        cooldown_until: transition.snapshot.cooldown_until,
        reason: transition.reason,
        ts: now_unix,
    };

    emit_circuit_event(app, payload);

    let enable_notice = match settings::read(app) {
        Ok(cfg) => cfg.enable_circuit_breaker_notice,
        Err(err) => {
            tracing::warn!("skip circuit notice because settings read failed: {err}");
            return;
        }
    };
    if !enable_notice {
        return;
    }

    let prev_state_text = match transition.prev_state {
        circuit_breaker::CircuitState::Closed => "正常",
        circuit_breaker::CircuitState::Open => "熔断",
        circuit_breaker::CircuitState::HalfOpen => "半开",
    };
    let next_state_text = match transition.next_state {
        circuit_breaker::CircuitState::Closed => "正常",
        circuit_breaker::CircuitState::Open => "熔断",
        circuit_breaker::CircuitState::HalfOpen => "半开",
    };

    let (level, title) = match transition.next_state {
        circuit_breaker::CircuitState::Open => (
            notice::NoticeLevel::Warning,
            format!("熔断触发：{provider_name}"),
        ),
        circuit_breaker::CircuitState::HalfOpen => (
            notice::NoticeLevel::Info,
            format!("熔断试探：{provider_name}"),
        ),
        circuit_breaker::CircuitState::Closed => (
            notice::NoticeLevel::Success,
            format!("熔断恢复：{provider_name}"),
        ),
    };

    let reason_text = match transition.reason {
        "FAILURE_THRESHOLD_REACHED" => "失败次数达到阈值",
        "OPEN_EXPIRED" => "熔断到期，进入半开试探",
        "HALF_OPEN_SUCCESS" => "半开试探成功，恢复正常",
        "HALF_OPEN_FAILURE" => "半开试探失败，重新熔断",
        other => other,
    };

    let mut lines: Vec<String> = Vec::with_capacity(10);
    lines.push(format!("CLI：{cli_key}"));
    lines.push(format!("Provider：{provider_name} (id={provider_id})"));
    lines.push(format!("Base URL：{base_url}"));
    lines.push(format!("状态：{prev_state_text} → {next_state_text}"));
    lines.push(format!(
        "失败：{} / {}",
        transition.snapshot.failure_count, transition.snapshot.failure_threshold
    ));
    lines.push(format!("原因：{reason_text}（{}）", transition.reason));

    match transition.snapshot.open_until {
        Some(open_until) => {
            let remaining_secs = open_until.saturating_sub(now_unix);
            let remaining_minutes = remaining_secs.saturating_add(59) / 60;
            if remaining_secs > 0 {
                lines.push(format!(
                    "熔断至：{open_until}（约 {remaining_minutes} 分钟后）"
                ));
            } else {
                lines.push(format!("熔断至：{open_until}（已到期）"));
            }
        }
        None => lines.push("熔断至：—".to_string()),
    }

    lines.push(format!("Trace：{trace_id}"));

    if let Err(err) = notice::emit(app, notice::build(level, Some(title), lines.join("\n"))) {
        tracing::warn!("failed to emit circuit breaker notice: {}", err);
    }
}
