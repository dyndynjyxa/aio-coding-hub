//! Usage: Best-effort enqueue to DB log tasks with backpressure and fallbacks.

use crate::{db, request_attempt_logs, request_logs};
use std::time::Duration;

use super::super::events::{emit_gateway_log, GatewayAttemptEvent};

const LOG_ENQUEUE_MAX_WAIT: Duration = Duration::from_millis(100);

fn attempt_log_insert_from_event(
    attempt: &GatewayAttemptEvent,
    created_at: i64,
) -> Option<request_attempt_logs::RequestAttemptLogInsert> {
    if !crate::shared::cli_key::is_supported_cli_key(attempt.cli_key.as_str()) {
        return None;
    }

    Some(request_attempt_logs::RequestAttemptLogInsert {
        trace_id: attempt.trace_id.clone(),
        cli_key: attempt.cli_key.clone(),
        method: attempt.method.clone(),
        path: attempt.path.clone(),
        query: attempt.query.clone(),
        attempt_index: attempt.attempt_index as i64,
        provider_id: attempt.provider_id,
        provider_name: attempt.provider_name.clone(),
        base_url: attempt.base_url.clone(),
        outcome: attempt.outcome.clone(),
        status: attempt.status.map(|v| v as i64),
        attempt_started_ms: attempt.attempt_started_ms.min(i64::MAX as u128) as i64,
        attempt_duration_ms: attempt.attempt_duration_ms.min(i64::MAX as u128) as i64,
        created_at,
    })
}

pub(super) async fn enqueue_attempt_log_with_backpressure(
    app: &tauri::AppHandle,
    db: &db::Db,
    attempt_log_tx: &tokio::sync::mpsc::Sender<request_attempt_logs::RequestAttemptLogInsert>,
    attempt: &GatewayAttemptEvent,
    created_at: i64,
) {
    let Some(insert) = attempt_log_insert_from_event(attempt, created_at) else {
        return;
    };

    let reserve = tokio::time::timeout(LOG_ENQUEUE_MAX_WAIT, attempt_log_tx.reserve()).await;
    match reserve {
        Ok(Ok(permit)) => {
            permit.send(insert);
        }
        Ok(Err(_)) => {
            emit_gateway_log(
                app,
                "warn",
                "GW_ATTEMPT_LOG_CHANNEL_CLOSED",
                format!(
                    "attempt log channel closed; using write-through fallback trace_id={} cli={}",
                    attempt.trace_id, attempt.cli_key
                ),
            );
            request_attempt_logs::spawn_write_through(app.clone(), db.clone(), insert);
        }
        Err(_) => {
            if attempt_log_tx.try_send(insert).is_ok() {
                emit_gateway_log(
                    app,
                    "warn",
                    "GW_ATTEMPT_LOG_ENQUEUE_TIMEOUT",
                    format!(
                        "attempt log enqueue timed out ({}ms); used try_send fallback trace_id={} cli={}",
                        LOG_ENQUEUE_MAX_WAIT.as_millis(),
                        attempt.trace_id,
                        attempt.cli_key
                    ),
                );
                return;
            }

            emit_gateway_log(
                app,
                "error",
                "GW_ATTEMPT_LOG_DROPPED",
                format!(
                    "attempt log dropped (queue full after {}ms) trace_id={} cli={}",
                    LOG_ENQUEUE_MAX_WAIT.as_millis(),
                    attempt.trace_id,
                    attempt.cli_key
                ),
            );
        }
    }
}

fn request_log_insert_from_args(
    args: super::RequestLogEnqueueArgs,
) -> Option<request_logs::RequestLogInsert> {
    let super::RequestLogEnqueueArgs {
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
    } = args;

    if !crate::shared::cli_key::is_supported_cli_key(cli_key.as_str()) {
        return None;
    }

    let (metrics, usage_json) = match usage {
        Some(extract) => (extract.metrics, Some(extract.usage_json)),
        None => (usage_metrics.unwrap_or_default(), None),
    };

    let duration_ms = duration_ms.min(i64::MAX as u128) as i64;
    let ttfb_ms = ttfb_ms.and_then(|v| {
        if v >= duration_ms as u128 {
            return None;
        }
        Some(v.min(i64::MAX as u128) as i64)
    });

    Some(request_logs::RequestLogInsert {
        trace_id,
        cli_key,
        session_id,
        method,
        path,
        query,
        excluded_from_stats,
        special_settings_json,
        status: status.map(|v| v as i64),
        error_code: error_code.map(str::to_string),
        duration_ms,
        ttfb_ms,
        attempts_json,
        input_tokens: metrics.input_tokens,
        output_tokens: metrics.output_tokens,
        total_tokens: metrics.total_tokens,
        cache_read_input_tokens: metrics.cache_read_input_tokens,
        cache_creation_input_tokens: metrics.cache_creation_input_tokens,
        cache_creation_5m_input_tokens: metrics.cache_creation_5m_input_tokens,
        cache_creation_1h_input_tokens: metrics.cache_creation_1h_input_tokens,
        usage_json,
        requested_model,
        created_at_ms,
        created_at,
    })
}

pub(super) async fn enqueue_request_log_with_backpressure(
    app: &tauri::AppHandle,
    db: &db::Db,
    log_tx: &tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    args: super::RequestLogEnqueueArgs,
) {
    let trace_id = args.trace_id.clone();
    let cli_key = args.cli_key.clone();
    let Some(insert) = request_log_insert_from_args(args) else {
        return;
    };

    let reserve = tokio::time::timeout(LOG_ENQUEUE_MAX_WAIT, log_tx.reserve()).await;
    match reserve {
        Ok(Ok(permit)) => {
            permit.send(insert);
        }
        Ok(Err(_)) => {
            emit_gateway_log(
                app,
                "warn",
                "GW_REQUEST_LOG_CHANNEL_CLOSED",
                format!(
                    "request log channel closed; using write-through fallback trace_id={} cli={}",
                    trace_id, cli_key
                ),
            );
            request_logs::spawn_write_through(app.clone(), db.clone(), insert);
        }
        Err(_) => {
            if log_tx.try_send(insert).is_ok() {
                emit_gateway_log(
                    app,
                    "warn",
                    "GW_REQUEST_LOG_ENQUEUE_TIMEOUT",
                    format!(
                        "request log enqueue timed out ({}ms); used try_send fallback trace_id={} cli={}",
                        LOG_ENQUEUE_MAX_WAIT.as_millis(),
                        trace_id,
                        cli_key
                    ),
                );
                return;
            }

            emit_gateway_log(
                app,
                "error",
                "GW_REQUEST_LOG_DROPPED",
                format!(
                    "request log dropped (queue full after {}ms) trace_id={} cli={}",
                    LOG_ENQUEUE_MAX_WAIT.as_millis(),
                    trace_id,
                    cli_key
                ),
            );
        }
    }
}

pub(in crate::gateway) fn spawn_enqueue_request_log_with_backpressure(
    app: tauri::AppHandle,
    db: db::Db,
    log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    args: super::RequestLogEnqueueArgs,
) {
    tauri::async_runtime::spawn(async move {
        enqueue_request_log_with_backpressure(&app, &db, &log_tx, args).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::{UsageExtract, UsageMetrics};

    fn base_args() -> super::super::RequestLogEnqueueArgs {
        super::super::RequestLogEnqueueArgs {
            trace_id: "t".to_string(),
            cli_key: "claude".to_string(),
            session_id: None,
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            query: None,
            excluded_from_stats: false,
            special_settings_json: None,
            status: Some(200),
            error_code: None,
            duration_ms: 10,
            ttfb_ms: None,
            attempts_json: "[]".to_string(),
            requested_model: None,
            created_at_ms: 0,
            created_at: 0,
            usage_metrics: None,
            usage: None,
        }
    }

    #[test]
    fn request_log_insert_uses_usage_metrics_when_usage_missing() {
        let mut args = base_args();
        args.usage_metrics = Some(UsageMetrics {
            input_tokens: Some(1),
            output_tokens: Some(2),
            total_tokens: Some(3),
            cache_read_input_tokens: Some(4),
            cache_creation_input_tokens: Some(5),
            cache_creation_5m_input_tokens: Some(6),
            cache_creation_1h_input_tokens: Some(7),
        });

        let insert = request_log_insert_from_args(args).expect("insert");
        assert_eq!(insert.input_tokens, Some(1));
        assert_eq!(insert.output_tokens, Some(2));
        assert_eq!(insert.total_tokens, Some(3));
        assert_eq!(insert.cache_read_input_tokens, Some(4));
        assert_eq!(insert.cache_creation_input_tokens, Some(5));
        assert_eq!(insert.cache_creation_5m_input_tokens, Some(6));
        assert_eq!(insert.cache_creation_1h_input_tokens, Some(7));
        assert_eq!(insert.usage_json, None);
    }

    #[test]
    fn request_log_insert_prefers_usage_extract_over_usage_metrics() {
        let mut args = base_args();
        args.usage_metrics = Some(UsageMetrics {
            input_tokens: Some(99),
            output_tokens: Some(99),
            total_tokens: Some(99),
            cache_read_input_tokens: Some(99),
            cache_creation_input_tokens: Some(99),
            cache_creation_5m_input_tokens: Some(99),
            cache_creation_1h_input_tokens: Some(99),
        });
        args.usage = Some(UsageExtract {
            metrics: UsageMetrics {
                input_tokens: Some(1),
                output_tokens: Some(2),
                total_tokens: Some(3),
                cache_read_input_tokens: Some(4),
                cache_creation_input_tokens: Some(5),
                cache_creation_5m_input_tokens: Some(6),
                cache_creation_1h_input_tokens: Some(7),
            },
            usage_json: "{\"input_tokens\":1}".to_string(),
        });

        let insert = request_log_insert_from_args(args).expect("insert");
        assert_eq!(insert.input_tokens, Some(1));
        assert_eq!(insert.output_tokens, Some(2));
        assert_eq!(insert.total_tokens, Some(3));
        assert_eq!(insert.cache_read_input_tokens, Some(4));
        assert_eq!(insert.cache_creation_input_tokens, Some(5));
        assert_eq!(insert.cache_creation_5m_input_tokens, Some(6));
        assert_eq!(insert.cache_creation_1h_input_tokens, Some(7));
        assert_eq!(insert.usage_json, Some("{\"input_tokens\":1}".to_string()));
    }
}
