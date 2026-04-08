//! Usage: Shared helpers to emit request-end events and enqueue request logs consistently.

use super::logging::enqueue_request_log_with_backpressure;
use super::status_override;
use super::{spawn_enqueue_request_log_with_backpressure, RequestLogEnqueueArgs};
use crate::gateway::events::{emit_request_event, FailoverAttempt};
use crate::{db, request_logs};

pub(super) struct RequestEndDeps<'a> {
    pub(super) app: &'a tauri::AppHandle,
    pub(super) db: &'a db::Db,
    pub(super) log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
}

impl<'a> RequestEndDeps<'a> {
    pub(super) fn new(
        app: &'a tauri::AppHandle,
        db: &'a db::Db,
        log_tx: &'a tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    ) -> Self {
        Self { app, db, log_tx }
    }
}

pub(super) struct RequestEndArgs<'a> {
    pub(super) deps: RequestEndDeps<'a>,
    pub(super) trace_id: &'a str,
    pub(super) cli_key: &'a str,
    pub(super) method: &'a str,
    pub(super) path: &'a str,
    pub(super) observe: bool,
    pub(super) query: Option<&'a str>,
    pub(super) excluded_from_stats: bool,
    pub(super) status: Option<u16>,
    pub(super) error_category: Option<&'static str>,
    pub(super) error_code: Option<&'static str>,
    pub(super) duration_ms: u128,
    pub(super) event_ttfb_ms: Option<u128>,
    pub(super) log_ttfb_ms: Option<u128>,
    pub(super) attempts: &'a [FailoverAttempt],
    pub(super) special_settings_json: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) log_usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
}

struct PreparedRequestEnd<'a> {
    deps: RequestEndDeps<'a>,
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

fn serialize_attempts(attempts: &[FailoverAttempt]) -> String {
    if attempts.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(attempts).unwrap_or_else(|_| "[]".to_string())
    }
}

fn build_provider_chain_json(attempts: &[FailoverAttempt]) -> Option<String> {
    if attempts.is_empty() {
        return None;
    }
    let chain: Vec<serde_json::Value> = attempts
        .iter()
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
        if let Some(provider_name) = non_empty_text(last_attempt.provider_name.as_str()) {
            obj.insert("provider_name".into(), serde_json::json!(provider_name));
        }
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
                obj.insert("reason".into(), serde_json::json!(reason_summary));
            }
            if let Some(upstream_body_preview) = upstream_body_preview {
                obj.insert(
                    "upstream_body_preview".into(),
                    serde_json::json!(upstream_body_preview),
                );
            }
            if let Some(matched_rule) = matched_rule {
                obj.insert("matched_rule".into(), serde_json::json!(matched_rule));
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

    pub(in crate::gateway) fn emit_gateway_request_event(
        &self,
        app: &tauri::AppHandle,
        error_category: Option<&'static str>,
        event_ttfb_ms: Option<u128>,
        attempts: Vec<FailoverAttempt>,
        usage_metrics: Option<crate::usage::UsageMetrics>,
    ) {
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
            usage_metrics,
        );
    }
}

fn prepare_request_end(args: RequestEndArgs<'_>) -> PreparedRequestEnd<'_> {
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

pub(super) async fn emit_request_event_and_enqueue_request_log(args: RequestEndArgs<'_>) {
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

pub(super) fn emit_request_event_and_spawn_request_log(args: RequestEndArgs<'_>) {
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
}
