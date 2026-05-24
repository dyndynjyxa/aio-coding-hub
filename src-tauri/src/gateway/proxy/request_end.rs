//! Usage: Shared helpers to emit request-end events and enqueue request logs consistently.

use super::logging::enqueue_request_log_with_backpressure;
use super::status_override;
use super::{spawn_enqueue_request_log_with_backpressure, RequestLogEnqueueArgs};
use crate::gateway::events::{emit_request_event, ClaudeModelMapping, FailoverAttempt};
use crate::{db, request_logs};
use serde_json::Value;

const REQUEST_END_LOG_MAX_ATTEMPTS: usize = 100;
const REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS: usize = 512;
const REQUEST_END_LOG_URL_MAX_CHARS: usize = 2048;
const REQUEST_END_LOG_REASON_MAX_CHARS: usize = 2048;

pub(super) struct RequestEndDeps<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) app: &'a tauri::AppHandle<R>,
    pub(super) db: &'a db::Db,
    pub(super) log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
}

impl<'a, R: tauri::Runtime> RequestEndDeps<'a, R> {
    pub(super) fn new(
        app: &'a tauri::AppHandle<R>,
        db: &'a db::Db,
        log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    ) -> Self {
        Self { app, db, log_tx }
    }
}

pub(super) struct RequestCompletion {
    pub(super) status: Option<u16>,
    pub(super) error_category: Option<&'static str>,
    pub(super) error_code: Option<&'static str>,
    pub(super) event_ttfb_ms: Option<u128>,
    pub(super) log_ttfb_ms: Option<u128>,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) log_usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
}

impl RequestCompletion {
    pub(super) fn success(
        status: u16,
        ttfb_ms: Option<u128>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
        log_usage_metrics: Option<crate::usage::UsageMetrics>,
        usage: Option<crate::usage::UsageExtract>,
    ) -> Self {
        Self {
            status: Some(status),
            error_category: None,
            error_code: None,
            event_ttfb_ms: ttfb_ms,
            log_ttfb_ms: ttfb_ms,
            usage_metrics,
            log_usage_metrics,
            usage,
        }
    }

    pub(super) fn failure(
        status: u16,
        error_category: Option<&'static str>,
        error_code: &'static str,
    ) -> Self {
        Self {
            status: Some(status),
            error_category,
            error_code: Some(error_code),
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        }
    }

    pub(super) fn failure_with_ttfb(
        status: u16,
        error_category: Option<&'static str>,
        error_code: &'static str,
        ttfb_ms: u128,
    ) -> Self {
        Self {
            event_ttfb_ms: Some(ttfb_ms),
            log_ttfb_ms: Some(ttfb_ms),
            ..Self::failure(status, error_category, error_code)
        }
    }

    pub(super) fn client_abort() -> Self {
        Self {
            status: None,
            error_category: Some(crate::gateway::proxy::ErrorCategory::ClientAbort.as_str()),
            error_code: Some(crate::gateway::proxy::GatewayErrorCode::RequestAborted.as_str()),
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        }
    }
}

pub(super) struct RequestEndContextArgs<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) deps: RequestEndDeps<'a, R>,
    pub(super) trace_id: &'a str,
    pub(super) cli_key: &'a str,
    pub(super) method: &'a str,
    pub(super) path: &'a str,
    pub(super) observe: bool,
    pub(super) query: Option<&'a str>,
    pub(super) excluded_from_stats: bool,
    pub(super) duration_ms: u128,
    pub(super) attempts: &'a [FailoverAttempt],
    pub(super) special_settings_json: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
}

pub(super) struct RequestEndArgs<'a, R: tauri::Runtime = tauri::Wry> {
    deps: RequestEndDeps<'a, R>,
    trace_id: &'a str,
    cli_key: &'a str,
    method: &'a str,
    path: &'a str,
    observe: bool,
    query: Option<&'a str>,
    excluded_from_stats: bool,
    status: Option<u16>,
    error_category: Option<&'static str>,
    error_code: Option<&'static str>,
    duration_ms: u128,
    event_ttfb_ms: Option<u128>,
    log_ttfb_ms: Option<u128>,
    attempts: &'a [FailoverAttempt],
    special_settings_json: Option<String>,
    session_id: Option<String>,
    requested_model: Option<String>,
    created_at_ms: i64,
    created_at: i64,
    usage_metrics: Option<crate::usage::UsageMetrics>,
    log_usage_metrics: Option<crate::usage::UsageMetrics>,
    usage: Option<crate::usage::UsageExtract>,
}

impl<'a, R: tauri::Runtime> RequestEndArgs<'a, R> {
    pub(super) fn from_context(context: RequestEndContextArgs<'a, R>) -> Self {
        Self {
            deps: context.deps,
            trace_id: context.trace_id,
            cli_key: context.cli_key,
            method: context.method,
            path: context.path,
            observe: context.observe,
            query: context.query,
            excluded_from_stats: context.excluded_from_stats,
            status: None,
            error_category: None,
            error_code: None,
            duration_ms: context.duration_ms,
            event_ttfb_ms: None,
            log_ttfb_ms: None,
            attempts: context.attempts,
            special_settings_json: context.special_settings_json,
            session_id: context.session_id,
            requested_model: context.requested_model,
            created_at_ms: context.created_at_ms,
            created_at: context.created_at,
            usage_metrics: None,
            log_usage_metrics: None,
            usage: None,
        }
    }

    pub(super) fn with_completion(mut self, completion: RequestCompletion) -> Self {
        self.status = completion.status;
        self.error_category = completion.error_category;
        self.error_code = completion.error_code;
        self.event_ttfb_ms = completion.event_ttfb_ms;
        self.log_ttfb_ms = completion.log_ttfb_ms;
        self.usage_metrics = completion.usage_metrics;
        self.log_usage_metrics = completion.log_usage_metrics;
        self.usage = completion.usage;
        self
    }
}

struct PreparedRequestEnd<'a, R: tauri::Runtime = tauri::Wry> {
    deps: RequestEndDeps<'a, R>,
    error_category: Option<&'static str>,
    event_ttfb_ms: Option<u128>,
    attempts: Vec<FailoverAttempt>,
    usage_metrics: Option<crate::usage::UsageMetrics>,
    log_args: RequestLogEnqueueArgs,
}

struct RequestEndPayloadParts {
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    method: String,
    path: String,
    query: Option<String>,
    excluded_from_stats: bool,
    special_settings_json: Option<String>,
    status: Option<u16>,
    error_code: Option<&'static str>,
    duration_ms: u128,
    ttfb_ms: Option<u128>,
    attempts: Vec<FailoverAttempt>,
    attempts_json: Option<String>,
    requested_model: Option<String>,
    created_at_ms: i64,
    created_at: i64,
    usage_metrics: Option<crate::usage::UsageMetrics>,
    usage: Option<crate::usage::UsageExtract>,
    provider_chain_json: Option<String>,
    error_details_json: Option<String>,
}

fn truncate_chars(mut value: String, max_chars: usize) -> String {
    if let Some((index, _)) = value.char_indices().nth(max_chars) {
        value.truncate(index);
    }
    value
}

fn truncate_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value.map(|value| truncate_chars(value, max_chars))
}

fn truncate_text_ref(value: &str, max_chars: usize) -> String {
    truncate_chars(value.to_string(), max_chars)
}

fn bounded_log_attempt(mut attempt: FailoverAttempt) -> FailoverAttempt {
    attempt.provider_name =
        truncate_chars(attempt.provider_name, REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS);
    attempt.base_url = truncate_chars(attempt.base_url, REQUEST_END_LOG_URL_MAX_CHARS);
    attempt.outcome = truncate_chars(attempt.outcome, REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS);
    attempt.reason = truncate_optional_text(attempt.reason, REQUEST_END_LOG_REASON_MAX_CHARS);
    attempt
}

fn bounded_log_attempts(attempts: &[FailoverAttempt]) -> Vec<FailoverAttempt> {
    if attempts.is_empty() {
        return Vec::new();
    }

    let start = attempts.len().saturating_sub(REQUEST_END_LOG_MAX_ATTEMPTS);
    attempts[start..]
        .iter()
        .cloned()
        .map(bounded_log_attempt)
        .collect()
}

fn serialize_attempts(attempts: &[FailoverAttempt]) -> String {
    let attempts = bounded_log_attempts(attempts);
    if attempts.is_empty() {
        return "[]".to_string();
    }
    serde_json::to_string(&attempts).unwrap_or_else(|_| "[]".to_string())
}

fn build_provider_chain_json(attempts: &[FailoverAttempt]) -> Option<String> {
    if attempts.is_empty() {
        return None;
    }
    let attempts = bounded_log_attempts(attempts);
    let chain: Vec<serde_json::Value> = attempts
        .into_iter()
        .map(|a| {
            let mut obj = serde_json::Map::new();
            obj.insert("provider_id".into(), serde_json::json!(a.provider_id));
            obj.insert("provider_name".into(), serde_json::json!(a.provider_name));
            if let Some(status) = a.status {
                obj.insert("status".into(), serde_json::json!(status));
            }
            obj.insert("outcome".into(), serde_json::json!(a.outcome));
            if let Some(decision) = a.decision {
                obj.insert("decision".into(), serde_json::json!(decision));
            }
            if let Some(ref reason) = a.reason {
                obj.insert("reason".into(), serde_json::json!(reason));
            }
            if let Some(duration_ms) = a.attempt_duration_ms {
                obj.insert("duration_ms".into(), serde_json::json!(duration_ms));
            }
            serde_json::Value::Object(obj)
        })
        .collect();
    serde_json::to_string(&chain).ok()
}

fn non_empty_text(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn parse_claude_model_mapping_setting(value: &Value) -> Option<ClaudeModelMapping> {
    let obj = value.as_object()?;
    if obj.get("type").and_then(Value::as_str) != Some("claude_model_mapping") {
        return None;
    }

    let requested_model = obj
        .get("requestedModel")
        .and_then(Value::as_str)
        .and_then(non_empty_text)?;
    let effective_model = obj
        .get("effectiveModel")
        .and_then(Value::as_str)
        .and_then(non_empty_text)?;
    let mapping_kind = obj
        .get("mappingKind")
        .and_then(Value::as_str)
        .and_then(non_empty_text)?;
    let provider_name = obj
        .get("providerName")
        .and_then(Value::as_str)
        .and_then(non_empty_text)?;
    let provider_id = obj.get("providerId").and_then(Value::as_i64)?;
    let applied = obj.get("applied").and_then(Value::as_bool).unwrap_or(false);

    Some(ClaudeModelMapping {
        requested_model: requested_model.to_string(),
        effective_model: effective_model.to_string(),
        mapping_kind: mapping_kind.to_string(),
        provider_id,
        provider_name: provider_name.to_string(),
        applied,
    })
}

fn select_claude_model_mapping(
    special_settings_json: Option<&str>,
    attempts: &[FailoverAttempt],
) -> Option<ClaudeModelMapping> {
    let raw = special_settings_json?.trim();
    if raw.is_empty() {
        return None;
    }

    let parsed: Value = serde_json::from_str(raw).ok()?;
    let mut mappings: Vec<ClaudeModelMapping> = match &parsed {
        Value::Array(items) => items
            .iter()
            .filter_map(parse_claude_model_mapping_setting)
            .collect(),
        Value::Object(_) => parse_claude_model_mapping_setting(&parsed)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    };

    mappings
        .retain(|mapping| mapping.applied && mapping.requested_model != mapping.effective_model);
    if mappings.is_empty() {
        return None;
    }

    if let Some(success_provider_id) = attempts
        .iter()
        .rev()
        .find(|attempt| attempt.outcome == "success")
        .map(|attempt| attempt.provider_id)
    {
        if let Some(mapping) = mappings
            .iter()
            .rev()
            .find(|mapping| mapping.provider_id == success_provider_id)
        {
            return Some(mapping.clone());
        }
    }

    mappings.pop()
}

fn select_error_observation_attempt(attempts: &[FailoverAttempt]) -> Option<&FailoverAttempt> {
    attempts
        .iter()
        .rev()
        .find(|attempt| {
            attempt.error_code.is_some()
                || attempt.error_category.is_some()
                || attempt.reason.as_deref().and_then(non_empty_text).is_some()
                || attempt.decision.is_some()
                || attempt.reason_code.is_some()
                || attempt.status.is_some()
        })
        .or_else(|| attempts.last())
}

fn split_attempt_reason(reason: &str) -> (Option<&str>, Option<&str>, Option<&str>) {
    let Some(reason) = non_empty_text(reason) else {
        return (None, None, None);
    };

    let marker = "upstream_body=";
    let (base_reason, upstream_body_preview) = match reason.find(marker) {
        Some(index) => {
            let base = reason[..index].trim().trim_end_matches(',').trim();
            let preview = reason[index + marker.len()..].trim();
            (base, non_empty_text(preview))
        }
        None => (reason, None),
    };

    let matched_rule = base_reason
        .split(',')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("rule="))
        .and_then(non_empty_text);

    (
        non_empty_text(base_reason),
        upstream_body_preview,
        matched_rule,
    )
}

fn insert_text_if_present(
    obj: &mut serde_json::Map<String, Value>,
    key: &'static str,
    value: &str,
    max_chars: usize,
) {
    if let Some(value) = non_empty_text(value) {
        obj.insert(
            key.into(),
            serde_json::json!(truncate_text_ref(value, max_chars)),
        );
    }
}

fn build_error_details_json(
    error_code: Option<&str>,
    attempts: &[FailoverAttempt],
) -> Option<String> {
    let mut obj = serde_json::Map::new();

    if let Some(gateway_error_code) = error_code {
        obj.insert(
            "gateway_error_code".into(),
            serde_json::json!(gateway_error_code),
        );
    }

    if let Some(last_attempt) = select_error_observation_attempt(attempts) {
        if let Some(display_error_code) = last_attempt.error_code.or(error_code) {
            obj.insert("error_code".into(), serde_json::json!(display_error_code));
        }
        if let Some(error_category) = last_attempt.error_category {
            obj.insert("error_category".into(), serde_json::json!(error_category));
        }
        if let Some(status) = last_attempt.status {
            obj.insert("upstream_status".into(), serde_json::json!(status));
        }
        if let Some(outcome) = non_empty_text(last_attempt.outcome.as_str()) {
            obj.insert("outcome".into(), serde_json::json!(outcome));
        }
        if let Some(decision) = last_attempt.decision {
            obj.insert("decision".into(), serde_json::json!(decision));
        }
        if let Some(reason_code) = last_attempt.reason_code {
            obj.insert("reason_code".into(), serde_json::json!(reason_code));
        }
        if let Some(selection_method) = last_attempt.selection_method {
            obj.insert(
                "selection_method".into(),
                serde_json::json!(selection_method),
            );
        }
        if let Some(provider_index) = last_attempt.provider_index {
            obj.insert("provider_index".into(), serde_json::json!(provider_index));
        }
        if let Some(retry_index) = last_attempt.retry_index {
            obj.insert("retry_index".into(), serde_json::json!(retry_index));
        }
        if last_attempt.provider_id > 0 {
            obj.insert(
                "provider_id".into(),
                serde_json::json!(last_attempt.provider_id),
            );
        }
        insert_text_if_present(
            &mut obj,
            "provider_name",
            last_attempt.provider_name.as_str(),
            REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS,
        );
        if let Some(attempt_duration_ms) = last_attempt.attempt_duration_ms {
            obj.insert(
                "attempt_duration_ms".into(),
                serde_json::json!(attempt_duration_ms),
            );
        }
        if let Some(circuit_state_before) = last_attempt.circuit_state_before {
            obj.insert(
                "circuit_state_before".into(),
                serde_json::json!(circuit_state_before),
            );
        }
        if let Some(circuit_state_after) = last_attempt.circuit_state_after {
            obj.insert(
                "circuit_state_after".into(),
                serde_json::json!(circuit_state_after),
            );
        }
        if let Some(circuit_failure_count) = last_attempt.circuit_failure_count {
            obj.insert(
                "circuit_failure_count".into(),
                serde_json::json!(circuit_failure_count),
            );
        }
        if let Some(circuit_failure_threshold) = last_attempt.circuit_failure_threshold {
            obj.insert(
                "circuit_failure_threshold".into(),
                serde_json::json!(circuit_failure_threshold),
            );
        }
        if let Some(ref reason) = last_attempt.reason {
            let (reason_summary, upstream_body_preview, matched_rule) =
                split_attempt_reason(reason.as_str());
            if let Some(reason_summary) = reason_summary {
                insert_text_if_present(
                    &mut obj,
                    "reason",
                    reason_summary,
                    REQUEST_END_LOG_REASON_MAX_CHARS,
                );
            }
            if let Some(upstream_body_preview) = upstream_body_preview {
                insert_text_if_present(
                    &mut obj,
                    "upstream_body_preview",
                    upstream_body_preview,
                    REQUEST_END_LOG_REASON_MAX_CHARS,
                );
            }
            if let Some(matched_rule) = matched_rule {
                insert_text_if_present(
                    &mut obj,
                    "matched_rule",
                    matched_rule,
                    REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS,
                );
            }
        }
    } else if let Some(gateway_error_code) = error_code {
        obj.insert("error_code".into(), serde_json::json!(gateway_error_code));
    }

    if obj.is_empty() {
        return None;
    }
    serde_json::to_string(&serde_json::Value::Object(obj)).ok()
}

fn build_request_end_payload(
    parts: RequestEndPayloadParts,
) -> (RequestLogEnqueueArgs, Vec<FailoverAttempt>) {
    let RequestEndPayloadParts {
        trace_id,
        cli_key,
        session_id,
        method,
        path,
        query,
        excluded_from_stats,
        special_settings_json,
        status,
        error_code,
        duration_ms,
        ttfb_ms,
        attempts,
        attempts_json,
        requested_model,
        created_at_ms,
        created_at,
        usage_metrics,
        usage,
        provider_chain_json,
        error_details_json,
    } = parts;

    let provider_chain_json = provider_chain_json.or_else(|| build_provider_chain_json(&attempts));
    let error_details_json =
        error_details_json.or_else(|| build_error_details_json(error_code, &attempts));
    let attempts_json = attempts_json.unwrap_or_else(|| serialize_attempts(&attempts));
    let log_args = RequestLogEnqueueArgs {
        trace_id,
        cli_key,
        session_id,
        method,
        path,
        query,
        excluded_from_stats,
        special_settings_json,
        status,
        error_code,
        duration_ms,
        ttfb_ms,
        attempts_json,
        requested_model,
        created_at_ms,
        created_at,
        usage_metrics,
        usage,
        provider_chain_json,
        error_details_json,
    };

    (log_args, attempts)
}

impl RequestLogEnqueueArgs {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gateway) fn from_proxy_request_end_parts(
        trace_id: &str,
        cli_key: &str,
        session_id: Option<String>,
        method: &str,
        path: &str,
        query: Option<&str>,
        excluded_from_stats: bool,
        special_settings_json: Option<String>,
        status: Option<u16>,
        error_code: Option<&'static str>,
        duration_ms: u128,
        ttfb_ms: Option<u128>,
        attempts: &[FailoverAttempt],
        requested_model: Option<String>,
        created_at_ms: i64,
        created_at: i64,
        usage_metrics: Option<crate::usage::UsageMetrics>,
        usage: Option<crate::usage::UsageExtract>,
    ) -> (Self, Vec<FailoverAttempt>) {
        let status = status_override::effective_status(status, error_code);
        let excluded_from_stats = excluded_from_stats
            || super::is_claude_count_tokens_request(cli_key, path)
            || status_override::is_client_abort(error_code);

        build_request_end_payload(RequestEndPayloadParts {
            trace_id: trace_id.to_string(),
            cli_key: cli_key.to_string(),
            session_id,
            method: method.to_string(),
            path: path.to_string(),
            query: query.map(str::to_string),
            excluded_from_stats,
            special_settings_json,
            status,
            error_code,
            duration_ms,
            ttfb_ms,
            attempts: attempts.to_vec(),
            attempts_json: None,
            requested_model,
            created_at_ms,
            created_at,
            usage_metrics,
            usage,
            provider_chain_json: None,
            error_details_json: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::gateway) fn from_stream_request_end_parts(
        trace_id: String,
        cli_key: String,
        session_id: Option<String>,
        method: String,
        path: String,
        query: Option<String>,
        excluded_from_stats: bool,
        special_settings_json: Option<String>,
        status: u16,
        error_code: Option<&'static str>,
        duration_ms: u128,
        ttfb_ms: Option<u128>,
        attempts: Vec<FailoverAttempt>,
        attempts_json: String,
        requested_model: Option<String>,
        created_at_ms: i64,
        created_at: i64,
        usage: Option<crate::usage::UsageExtract>,
    ) -> (Self, Vec<FailoverAttempt>) {
        build_request_end_payload(RequestEndPayloadParts {
            trace_id,
            cli_key,
            session_id,
            method,
            path,
            query,
            excluded_from_stats: excluded_from_stats
                || status_override::is_client_abort(error_code),
            special_settings_json,
            status: status_override::effective_status(Some(status), error_code),
            error_code,
            duration_ms,
            ttfb_ms,
            attempts,
            attempts_json: Some(attempts_json),
            requested_model,
            created_at_ms,
            created_at,
            usage_metrics: None,
            usage,
            provider_chain_json: None,
            error_details_json: None,
        })
    }

    pub(in crate::gateway) fn emit_gateway_request_event<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        error_category: Option<&'static str>,
        event_ttfb_ms: Option<u128>,
        attempts: Vec<FailoverAttempt>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
    ) {
        let claude_model_mapping =
            select_claude_model_mapping(self.special_settings_json.as_deref(), &attempts);
        emit_request_event(
            app,
            self.trace_id.clone(),
            self.cli_key.clone(),
            self.session_id.clone(),
            self.method.clone(),
            self.path.clone(),
            self.query.clone(),
            self.requested_model.clone(),
            self.status,
            error_category,
            self.error_code,
            self.duration_ms,
            event_ttfb_ms,
            attempts,
            claude_model_mapping,
            usage_metrics,
        );
    }
}

fn prepare_request_end<R: tauri::Runtime>(
    args: RequestEndArgs<'_, R>,
) -> PreparedRequestEnd<'_, R> {
    let (log_args, attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
        args.trace_id,
        args.cli_key,
        args.session_id,
        args.method,
        args.path,
        args.query,
        args.excluded_from_stats,
        args.special_settings_json,
        args.status,
        args.error_code,
        args.duration_ms,
        args.log_ttfb_ms,
        args.attempts,
        args.requested_model,
        args.created_at_ms,
        args.created_at,
        args.log_usage_metrics,
        args.usage,
    );

    PreparedRequestEnd {
        deps: args.deps,
        error_category: args.error_category,
        event_ttfb_ms: args.event_ttfb_ms,
        attempts,
        usage_metrics: args.usage_metrics,
        log_args,
    }
}

pub(super) async fn emit_request_event_and_enqueue_request_log<R: tauri::Runtime>(
    args: RequestEndArgs<'_, R>,
) {
    // Disk log: request ended with error (failure path only).
    if let Some(error_code) = args.error_code {
        tracing::warn!(
            trace_id = %args.trace_id,
            error_code = error_code,
            cli_key = %args.cli_key,
            status = ?args.status,
            duration_ms = %args.duration_ms,
            "gateway request completed with error"
        );
    }

    if !args.observe {
        return;
    }

    let PreparedRequestEnd {
        deps,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
        log_args,
    } = prepare_request_end(args);

    log_args.emit_gateway_request_event(
        deps.app,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
    );

    enqueue_request_log_with_backpressure(deps.app, deps.db, deps.log_tx, log_args).await;
}

pub(super) fn emit_request_event_and_spawn_request_log<R: tauri::Runtime>(
    args: RequestEndArgs<'_, R>,
) {
    // Disk log: request ended with error (failure path only).
    if let Some(error_code) = args.error_code {
        tracing::warn!(
            trace_id = %args.trace_id,
            error_code = error_code,
            cli_key = %args.cli_key,
            status = ?args.status,
            duration_ms = %args.duration_ms,
            "gateway request completed with error"
        );
    }

    if !args.observe {
        return;
    }

    let PreparedRequestEnd {
        deps,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
        log_args,
    } = prepare_request_end(args);

    log_args.emit_gateway_request_event(
        deps.app,
        error_category,
        event_ttfb_ms,
        attempts,
        usage_metrics,
    );

    spawn_enqueue_request_log_with_backpressure(
        deps.app.clone(),
        deps.db.clone(),
        deps.log_tx.clone(),
        log_args,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};
    use serde_json::json;

    fn sample_attempt() -> FailoverAttempt {
        FailoverAttempt {
            provider_id: 7,
            provider_name: "provider".to_string(),
            base_url: "https://example.com".to_string(),
            outcome: "success".to_string(),
            status: Some(200),
            provider_index: Some(1),
            retry_index: Some(1),
            session_reuse: Some(false),
            error_category: None,
            error_code: None,
            decision: None,
            reason: None,
            selection_method: None,
            reason_code: None,
            attempt_started_ms: Some(1),
            attempt_duration_ms: Some(2),
            circuit_state_before: None,
            circuit_state_after: None,
            circuit_failure_count: None,
            circuit_failure_threshold: None,
        }
    }

    fn timeout_attempt(
        provider_id: i64,
        provider_index: u32,
        session_reuse: Option<bool>,
    ) -> FailoverAttempt {
        FailoverAttempt {
            provider_id,
            provider_name: format!("provider-{provider_id}"),
            base_url: "http://127.0.0.1:1".to_string(),
            outcome: "request_timeout: category=SYSTEM_ERROR code=GW_UPSTREAM_TIMEOUT decision=switch timeout_secs=1".to_string(),
            status: None,
            provider_index: Some(provider_index),
            retry_index: Some(1),
            session_reuse,
            error_category: Some(ErrorCategory::SystemError.as_str()),
            error_code: Some(GatewayErrorCode::UpstreamTimeout.as_str()),
            decision: Some("switch"),
            reason: Some("request timeout".to_string()),
            selection_method: Some("session_reuse"),
            reason_code: Some(ErrorCategory::SystemError.reason_code()),
            attempt_started_ms: Some(1),
            attempt_duration_ms: Some(1_000),
            circuit_state_before: Some("CLOSED"),
            circuit_state_after: Some("OPEN"),
            circuit_failure_count: Some(5),
            circuit_failure_threshold: Some(5),
        }
    }

    fn json_field_char_count(value: &serde_json::Value, key: &str) -> Option<usize> {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(|text| text.chars().count())
    }

    #[test]
    fn proxy_request_end_parts_apply_count_tokens_exclusion_and_serialize_attempts() {
        let attempts = vec![sample_attempt()];
        let expected_attempts_json = serde_json::to_string(&attempts).unwrap();

        let (log_args, cloned_attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
            "trace-1",
            "claude",
            Some("session-1".to_string()),
            "POST",
            "/v1/messages/count_tokens",
            Some("a=1"),
            false,
            Some("{\"type\":\"x\"}".to_string()),
            Some(200),
            None,
            345,
            Some(12),
            &attempts,
            Some("claude-3-7".to_string()),
            100,
            200,
            Some(crate::usage::UsageMetrics::default()),
            None,
        );

        assert!(log_args.excluded_from_stats);
        assert_eq!(log_args.status, Some(200));
        assert_eq!(log_args.query.as_deref(), Some("a=1"));
        assert_eq!(log_args.attempts_json, expected_attempts_json);
        assert_eq!(cloned_attempts.len(), 1);
        assert_eq!(cloned_attempts[0].provider_id, 7);
    }

    #[test]
    fn proxy_request_end_parts_preserve_timeout_storm_attempts_in_log_payload() {
        let attempts = vec![
            timeout_attempt(10, 1, Some(true)),
            timeout_attempt(20, 2, None),
        ];

        let (log_args, cloned_attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
            "trace-timeout-storm",
            "claude",
            Some("session-timeout".to_string()),
            "POST",
            "/v1/messages",
            None,
            false,
            None,
            Some(502),
            Some(GatewayErrorCode::UpstreamTimeout.as_str()),
            2_000,
            None,
            &attempts,
            Some("claude-sonnet-4-5".to_string()),
            100,
            200,
            None,
            None,
        );

        assert!(!log_args.excluded_from_stats);
        assert_eq!(log_args.status, Some(524));
        assert_eq!(
            log_args.error_code,
            Some(GatewayErrorCode::UpstreamTimeout.as_str())
        );
        assert_eq!(cloned_attempts.len(), 2);
        assert_eq!(cloned_attempts[0].session_reuse, Some(true));
        assert!(cloned_attempts.iter().all(|attempt| {
            attempt.outcome.starts_with("request_timeout:")
                && attempt.error_code == Some(GatewayErrorCode::UpstreamTimeout.as_str())
        }));

        let encoded_attempts: Vec<serde_json::Value> =
            serde_json::from_str(&log_args.attempts_json).expect("attempts json");
        assert_eq!(encoded_attempts.len(), 2);
        assert_eq!(
            encoded_attempts[0]
                .get("outcome")
                .and_then(serde_json::Value::as_str),
            Some(attempts[0].outcome.as_str())
        );

        let provider_chain: Vec<serde_json::Value> = serde_json::from_str(
            log_args
                .provider_chain_json
                .as_deref()
                .expect("provider chain json"),
        )
        .expect("provider chain json parses");
        assert_eq!(provider_chain.len(), 2);
        assert_eq!(provider_chain[0].get("provider_id"), Some(&json!(10)));
        assert_eq!(
            provider_chain[0].get("reason"),
            Some(&json!("request timeout"))
        );

        let error_details: serde_json::Value = serde_json::from_str(
            log_args
                .error_details_json
                .as_deref()
                .expect("error details json"),
        )
        .expect("error details json parses");
        assert_eq!(
            error_details.get("gateway_error_code"),
            Some(&json!(GatewayErrorCode::UpstreamTimeout.as_str()))
        );
        assert_eq!(
            error_details.get("error_code"),
            Some(&json!(GatewayErrorCode::UpstreamTimeout.as_str()))
        );
        assert_eq!(
            error_details.get("reason_code"),
            Some(&json!(ErrorCategory::SystemError.reason_code()))
        );
    }

    #[test]
    fn proxy_request_end_parts_bounds_attempt_log_tail_and_text_fields() {
        let attempts: Vec<FailoverAttempt> = (0..(REQUEST_END_LOG_MAX_ATTEMPTS + 5))
            .map(|index| {
                let mut attempt = timeout_attempt(index as i64, index as u32, None);
                attempt.provider_name = format!(
                    "provider-{index}-{}",
                    "p".repeat(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS)
                );
                attempt.base_url = format!(
                    "https://provider-{index}.example/{}",
                    "b".repeat(REQUEST_END_LOG_URL_MAX_CHARS)
                );
                attempt.outcome = format!(
                    "request_timeout-{index}:{}",
                    "o".repeat(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS)
                );
                attempt.reason = Some(format!(
                    "request timeout {index}:{}",
                    "r".repeat(REQUEST_END_LOG_REASON_MAX_CHARS)
                ));
                attempt
            })
            .collect();
        let original_last_reason = attempts
            .last()
            .and_then(|attempt| attempt.reason.as_ref())
            .expect("last reason")
            .clone();

        let (log_args, cloned_attempts) = RequestLogEnqueueArgs::from_proxy_request_end_parts(
            "trace-bounded-attempts",
            "claude",
            Some("session-bounded".to_string()),
            "POST",
            "/v1/messages",
            None,
            false,
            None,
            Some(502),
            Some(GatewayErrorCode::UpstreamTimeout.as_str()),
            2_000,
            None,
            &attempts,
            Some("claude-sonnet-4-5".to_string()),
            100,
            200,
            None,
            None,
        );

        assert_eq!(cloned_attempts.len(), REQUEST_END_LOG_MAX_ATTEMPTS + 5);
        assert_eq!(
            cloned_attempts
                .last()
                .and_then(|attempt| attempt.reason.as_ref()),
            Some(&original_last_reason)
        );

        let encoded_attempts: Vec<serde_json::Value> =
            serde_json::from_str(&log_args.attempts_json).expect("attempts json");
        assert_eq!(encoded_attempts.len(), REQUEST_END_LOG_MAX_ATTEMPTS);
        assert_eq!(encoded_attempts[0].get("provider_id"), Some(&json!(5)));
        assert_eq!(
            encoded_attempts
                .last()
                .and_then(|attempt| attempt.get("provider_id")),
            Some(&json!((REQUEST_END_LOG_MAX_ATTEMPTS + 4) as i64))
        );

        let last_attempt = encoded_attempts.last().expect("last bounded attempt");
        assert_eq!(
            json_field_char_count(last_attempt, "provider_name"),
            Some(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS)
        );
        assert_eq!(
            json_field_char_count(last_attempt, "base_url"),
            Some(REQUEST_END_LOG_URL_MAX_CHARS)
        );
        assert_eq!(
            json_field_char_count(last_attempt, "outcome"),
            Some(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS)
        );
        assert_eq!(
            json_field_char_count(last_attempt, "reason"),
            Some(REQUEST_END_LOG_REASON_MAX_CHARS)
        );
    }

    #[test]
    fn build_provider_chain_json_bounds_latest_tail_and_reason_text() {
        let attempts: Vec<FailoverAttempt> = (0..(REQUEST_END_LOG_MAX_ATTEMPTS + 2))
            .map(|index| {
                let mut attempt = timeout_attempt(index as i64, index as u32, None);
                attempt.reason = Some(format!(
                    "timeout reason {index}:{}",
                    "由".repeat(REQUEST_END_LOG_REASON_MAX_CHARS)
                ));
                attempt
            })
            .collect();

        let encoded = build_provider_chain_json(&attempts).expect("provider chain json");
        let chain: Vec<serde_json::Value> =
            serde_json::from_str(encoded.as_str()).expect("provider chain json parses");

        assert_eq!(chain.len(), REQUEST_END_LOG_MAX_ATTEMPTS);
        assert_eq!(chain[0].get("provider_id"), Some(&json!(2)));
        assert_eq!(
            chain.last().and_then(|attempt| attempt.get("provider_id")),
            Some(&json!((REQUEST_END_LOG_MAX_ATTEMPTS + 1) as i64))
        );
        let last_attempt = chain.last().expect("last provider-chain attempt");
        assert_eq!(
            json_field_char_count(last_attempt, "reason"),
            Some(REQUEST_END_LOG_REASON_MAX_CHARS)
        );
    }

    #[test]
    fn request_completion_builds_success_with_usage_and_ttfb() {
        let usage_metrics = crate::usage::UsageMetrics::default();
        let completion = RequestCompletion::success(
            200,
            Some(42),
            Some(usage_metrics.clone()),
            Some(usage_metrics),
            None,
        );

        assert_eq!(completion.status, Some(200));
        assert!(completion.error_category.is_none());
        assert!(completion.error_code.is_none());
        assert_eq!(completion.event_ttfb_ms, Some(42));
        assert_eq!(completion.log_ttfb_ms, Some(42));
        assert!(completion.usage_metrics.is_some());
        assert!(completion.log_usage_metrics.is_some());
    }

    #[test]
    fn request_completion_builds_client_abort_without_status() {
        let completion = RequestCompletion::client_abort();

        assert!(completion.status.is_none());
        assert_eq!(
            completion.error_category,
            Some(ErrorCategory::ClientAbort.as_str())
        );
        assert_eq!(
            completion.error_code,
            Some(GatewayErrorCode::RequestAborted.as_str())
        );
        assert!(completion.event_ttfb_ms.is_none());
        assert!(completion.usage_metrics.is_none());
    }

    #[test]
    fn request_completion_builds_terminal_failure_with_ttfb() {
        let completion = RequestCompletion::failure_with_ttfb(
            502,
            Some(ErrorCategory::ProviderError.as_str()),
            GatewayErrorCode::Upstream5xx.as_str(),
            77,
        );

        assert_eq!(completion.status, Some(502));
        assert_eq!(
            completion.error_category,
            Some(ErrorCategory::ProviderError.as_str())
        );
        assert_eq!(
            completion.error_code,
            Some(GatewayErrorCode::Upstream5xx.as_str())
        );
        assert_eq!(completion.event_ttfb_ms, Some(77));
        assert_eq!(completion.log_ttfb_ms, Some(77));
        assert!(completion.usage_metrics.is_none());
    }

    #[test]
    fn stream_request_end_parts_keep_attempts_json_and_apply_abort_override() {
        let attempts = vec![sample_attempt()];

        let (log_args, cloned_attempts) = RequestLogEnqueueArgs::from_stream_request_end_parts(
            "trace-2".to_string(),
            "codex".to_string(),
            None,
            "POST".to_string(),
            "/v1/responses".to_string(),
            None,
            false,
            Some("{\"type\":\"client_abort\"}".to_string()),
            200,
            Some(GatewayErrorCode::StreamAborted.as_str()),
            678,
            Some(34),
            attempts,
            "[{\"cached\":true}]".to_string(),
            Some("gpt-5".to_string()),
            300,
            400,
            None,
        );

        assert!(log_args.excluded_from_stats);
        assert_eq!(log_args.status, Some(499));
        assert_eq!(log_args.attempts_json, "[{\"cached\":true}]");
        assert_eq!(
            log_args.special_settings_json.as_deref(),
            Some("{\"type\":\"client_abort\"}")
        );
        assert!(log_args.usage_metrics.is_none());
        assert_eq!(cloned_attempts.len(), 1);
        assert_eq!(cloned_attempts[0].provider_id, 7);
    }

    #[test]
    fn select_claude_model_mapping_prefers_success_provider() {
        let mut failed_attempt = sample_attempt();
        failed_attempt.provider_id = 1;
        failed_attempt.outcome = "failed".to_string();
        failed_attempt.status = Some(500);

        let mut success_attempt = sample_attempt();
        success_attempt.provider_id = 2;
        success_attempt.provider_name = "Provider B".to_string();

        let special_settings_json = json!([
            {
                "type": "claude_model_mapping",
                "scope": "attempt",
                "applied": true,
                "providerId": 1,
                "providerName": "Provider A",
                "requestedModel": "claude-sonnet",
                "effectiveModel": "gpt-4.1",
                "mappingKind": "sonnet"
            },
            {
                "type": "claude_model_mapping",
                "scope": "attempt",
                "applied": true,
                "providerId": 2,
                "providerName": "Provider B",
                "requestedModel": "claude-sonnet",
                "effectiveModel": "gpt-5.4",
                "mappingKind": "sonnet"
            }
        ])
        .to_string();

        let mapping = select_claude_model_mapping(
            Some(special_settings_json.as_str()),
            &[failed_attempt, success_attempt],
        )
        .expect("selected mapping");

        assert_eq!(mapping.provider_id, 2);
        assert_eq!(mapping.effective_model, "gpt-5.4");
    }

    #[test]
    fn select_claude_model_mapping_ignores_unapplied_or_identity_mapping() {
        let special_settings_json = json!([
            {
                "type": "claude_model_mapping",
                "scope": "attempt",
                "applied": false,
                "providerId": 1,
                "providerName": "Provider A",
                "requestedModel": "claude-sonnet",
                "effectiveModel": "gpt-5.4",
                "mappingKind": "sonnet"
            },
            {
                "type": "claude_model_mapping",
                "scope": "attempt",
                "applied": true,
                "providerId": 2,
                "providerName": "Provider B",
                "requestedModel": "claude-sonnet",
                "effectiveModel": "claude-sonnet",
                "mappingKind": "sonnet"
            }
        ])
        .to_string();

        assert!(select_claude_model_mapping(Some(special_settings_json.as_str()), &[]).is_none());
    }

    #[test]
    fn should_not_observe_non_messages_claude_request_end() {
        assert!(!super::super::should_observe_request(
            "claude",
            "/v1/messages/count_tokens"
        ));
        assert!(!super::super::should_observe_request("claude", "/v1/other"));
        assert!(super::super::should_observe_request(
            "claude",
            "/v1/messages"
        ));
        assert!(super::super::should_observe_request(
            "codex",
            "/v1/messages/count_tokens"
        ));
    }

    #[test]
    fn build_error_details_json_includes_rich_attempt_context() {
        let mut attempt = sample_attempt();
        attempt.provider_name = "Alpha".to_string();
        attempt.outcome = "upstream_error: status=502 category=PROVIDER_ERROR".to_string();
        attempt.status = Some(502);
        attempt.provider_index = Some(2);
        attempt.retry_index = Some(3);
        attempt.error_category = Some(ErrorCategory::ProviderError.as_str());
        attempt.error_code = Some(GatewayErrorCode::Upstream5xx.as_str());
        attempt.decision = Some("switch");
        attempt.reason =
            Some("status=502, rule=bad_gateway, upstream_body={\"error\":\"boom\"}".to_string());
        attempt.selection_method = Some("ordered");
        attempt.reason_code = Some(ErrorCategory::ProviderError.reason_code());
        attempt.attempt_duration_ms = Some(88);
        attempt.circuit_state_before = Some("closed");
        attempt.circuit_state_after = Some("open");
        attempt.circuit_failure_count = Some(3);
        attempt.circuit_failure_threshold = Some(3);

        let encoded = build_error_details_json(
            Some(GatewayErrorCode::UpstreamAllFailed.as_str()),
            &[attempt],
        )
        .expect("error details json");
        let value: serde_json::Value =
            serde_json::from_str(encoded.as_str()).expect("valid error details json");

        assert_eq!(
            value.get("gateway_error_code"),
            Some(&json!(GatewayErrorCode::UpstreamAllFailed.as_str()))
        );
        assert_eq!(
            value.get("error_code"),
            Some(&json!(GatewayErrorCode::Upstream5xx.as_str()))
        );
        assert_eq!(value.get("provider_name"), Some(&json!("Alpha")));
        assert_eq!(value.get("provider_index"), Some(&json!(2)));
        assert_eq!(value.get("retry_index"), Some(&json!(3)));
        assert_eq!(value.get("decision"), Some(&json!("switch")));
        assert_eq!(
            value.get("reason_code"),
            Some(&json!(ErrorCategory::ProviderError.reason_code()))
        );
        assert_eq!(
            value.get("reason"),
            Some(&json!("status=502, rule=bad_gateway"))
        );
        assert_eq!(value.get("matched_rule"), Some(&json!("bad_gateway")));
        assert_eq!(
            value.get("upstream_body_preview"),
            Some(&json!("{\"error\":\"boom\"}"))
        );
        assert_eq!(value.get("circuit_state_before"), Some(&json!("closed")));
        assert_eq!(value.get("circuit_state_after"), Some(&json!("open")));
        assert_eq!(value.get("circuit_failure_count"), Some(&json!(3)));
        assert_eq!(value.get("circuit_failure_threshold"), Some(&json!(3)));
    }

    #[test]
    fn build_error_details_json_does_not_require_top_level_error_code() {
        let mut attempt = sample_attempt();
        attempt.outcome = "system_error".to_string();
        attempt.status = None;
        attempt.error_category = Some(ErrorCategory::SystemError.as_str());
        attempt.error_code = None;
        attempt.decision = Some("abort");
        attempt.reason = Some("network timeout".to_string());
        attempt.selection_method = Some("ordered");
        attempt.reason_code = Some(ErrorCategory::SystemError.reason_code());

        let encoded = build_error_details_json(None, &[attempt])
            .expect("error details without top-level code");
        let value: serde_json::Value =
            serde_json::from_str(encoded.as_str()).expect("valid error details json");

        assert!(value.get("gateway_error_code").is_none());
        assert!(value.get("error_code").is_none());
        assert_eq!(
            value.get("error_category"),
            Some(&json!(ErrorCategory::SystemError.as_str()))
        );
        assert_eq!(value.get("reason"), Some(&json!("network timeout")));
        assert_eq!(
            value.get("reason_code"),
            Some(&json!(ErrorCategory::SystemError.reason_code()))
        );
        assert_eq!(value.get("decision"), Some(&json!("abort")));
    }

    #[test]
    fn build_error_details_json_truncates_large_upstream_body_preview() {
        let mut attempt = sample_attempt();
        attempt.provider_name = "Provider".repeat(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS);
        attempt.outcome = "upstream_error".to_string();
        attempt.status = Some(502);
        attempt.error_category = Some(ErrorCategory::ProviderError.as_str());
        attempt.error_code = Some(GatewayErrorCode::Upstream5xx.as_str());
        attempt.reason = Some(format!(
            "status=502, rule=bad_gateway, upstream_body={}",
            "错".repeat(REQUEST_END_LOG_REASON_MAX_CHARS + 64)
        ));

        let encoded = build_error_details_json(
            Some(GatewayErrorCode::UpstreamAllFailed.as_str()),
            &[attempt],
        )
        .expect("error details json");
        let value: serde_json::Value =
            serde_json::from_str(encoded.as_str()).expect("valid error details json");

        assert_eq!(
            json_field_char_count(&value, "provider_name"),
            Some(REQUEST_END_LOG_SHORT_TEXT_MAX_CHARS)
        );
        assert_eq!(
            json_field_char_count(&value, "upstream_body_preview"),
            Some(REQUEST_END_LOG_REASON_MAX_CHARS)
        );
        assert_eq!(
            value.get("reason"),
            Some(&json!("status=502, rule=bad_gateway"))
        );
        assert_eq!(value.get("matched_rule"), Some(&json!("bad_gateway")));
    }
}
