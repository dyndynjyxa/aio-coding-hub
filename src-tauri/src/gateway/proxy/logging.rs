//! Usage: Best-effort enqueue to DB log tasks with backpressure and fallbacks.

use crate::{db, request_logs};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::super::events::emit_gateway_log;
use super::super::util::now_unix_seconds;
use super::GatewayErrorCode;

const LOG_ENQUEUE_MAX_WAIT: Duration = Duration::from_millis(100);

const REQUEST_LOG_ENQUEUE_TASK_MAX_CONCURRENT: usize = 256;
const REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC: u32 = 50;
const REQUEST_LOG_METHOD_MAX_CHARS: usize = 32;
const REQUEST_LOG_SHORT_TEXT_MAX_CHARS: usize = 512;
const REQUEST_LOG_PATH_MAX_CHARS: usize = 2048;
const REQUEST_LOG_QUERY_MAX_CHARS: usize = 4096;
const REQUEST_LOG_JSON_MAX_BYTES: usize = 256 * 1024;
static REQUEST_LOG_ENQUEUE_TASK_LIMITER: OnceLock<Arc<Semaphore>> = OnceLock::new();
static REQUEST_LOG_WRITE_THROUGH_WINDOW_UNIX: AtomicU64 = AtomicU64::new(0);
static REQUEST_LOG_WRITE_THROUGH_COUNT: AtomicU32 = AtomicU32::new(0);

fn request_log_enqueue_task_limiter() -> Arc<Semaphore> {
    REQUEST_LOG_ENQUEUE_TASK_LIMITER
        .get_or_init(|| Arc::new(Semaphore::new(REQUEST_LOG_ENQUEUE_TASK_MAX_CONCURRENT)))
        .clone()
}

fn try_acquire_request_log_enqueue_task_permit(
    limiter: Arc<Semaphore>,
) -> Option<OwnedSemaphorePermit> {
    limiter.try_acquire_owned().ok()
}

fn next_request_log_write_through_count(now_unix: u64) -> u32 {
    let prev = REQUEST_LOG_WRITE_THROUGH_WINDOW_UNIX.load(Ordering::Relaxed);
    if prev != now_unix
        && REQUEST_LOG_WRITE_THROUGH_WINDOW_UNIX
            .compare_exchange(prev, now_unix, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        REQUEST_LOG_WRITE_THROUGH_COUNT.store(0, Ordering::Relaxed);
    }
    REQUEST_LOG_WRITE_THROUGH_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}

fn truncate_chars(mut value: String, max_chars: usize) -> String {
    if let Some((index, _)) = value.char_indices().nth(max_chars) {
        value.truncate(index);
    }
    value
}

fn bound_optional_chars(value: Option<String>, max_chars: usize) -> Option<String> {
    value.map(|value| truncate_chars(value, max_chars))
}

fn byte_len_exceeds(value: &str, max_bytes: usize) -> bool {
    value.len() > max_bytes
}

fn request_log_json_truncated_object(field: &'static str) -> String {
    serde_json::json!({
        "type": "request_log_payload_truncated",
        "field": field,
        "maxBytes": REQUEST_LOG_JSON_MAX_BYTES,
    })
    .to_string()
}

fn request_log_json_truncated_array(field: &'static str) -> String {
    serde_json::json!([{
        "type": "request_log_payload_truncated",
        "field": field,
        "maxBytes": REQUEST_LOG_JSON_MAX_BYTES,
    }])
    .to_string()
}

fn bound_attempts_json(value: String) -> String {
    if byte_len_exceeds(&value, REQUEST_LOG_JSON_MAX_BYTES) {
        "[]".to_string()
    } else {
        value
    }
}

fn bound_optional_json_object(value: Option<String>, field: &'static str) -> Option<String> {
    value.map(|value| {
        if byte_len_exceeds(&value, REQUEST_LOG_JSON_MAX_BYTES) {
            request_log_json_truncated_object(field)
        } else {
            value
        }
    })
}

fn bound_optional_json_array(value: Option<String>, field: &'static str) -> Option<String> {
    value.map(|value| {
        if byte_len_exceeds(&value, REQUEST_LOG_JSON_MAX_BYTES) {
            request_log_json_truncated_array(field)
        } else {
            value
        }
    })
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
        provider_chain_json,
        error_details_json,
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
        if v > duration_ms as u128 {
            return None;
        }
        Some(v.min(i64::MAX as u128) as i64)
    });

    Some(request_logs::RequestLogInsert {
        trace_id,
        cli_key,
        session_id: bound_optional_chars(session_id, REQUEST_LOG_SHORT_TEXT_MAX_CHARS),
        method: truncate_chars(method, REQUEST_LOG_METHOD_MAX_CHARS),
        path: truncate_chars(path, REQUEST_LOG_PATH_MAX_CHARS),
        query: bound_optional_chars(query, REQUEST_LOG_QUERY_MAX_CHARS),
        excluded_from_stats,
        special_settings_json: bound_optional_json_array(
            special_settings_json,
            "special_settings_json",
        ),
        status: status.map(|v| v as i64),
        error_code: error_code.map(str::to_string),
        duration_ms,
        ttfb_ms,
        attempts_json: bound_attempts_json(attempts_json),
        input_tokens: metrics.input_tokens,
        output_tokens: metrics.output_tokens,
        total_tokens: metrics.total_tokens,
        cache_read_input_tokens: metrics.cache_read_input_tokens,
        cache_creation_input_tokens: metrics.cache_creation_input_tokens,
        cache_creation_5m_input_tokens: metrics.cache_creation_5m_input_tokens,
        cache_creation_1h_input_tokens: metrics.cache_creation_1h_input_tokens,
        usage_json: bound_optional_json_object(usage_json, "usage_json"),
        requested_model: bound_optional_chars(requested_model, REQUEST_LOG_SHORT_TEXT_MAX_CHARS),
        created_at_ms,
        created_at,
        provider_chain_json: bound_optional_json_array(provider_chain_json, "provider_chain_json"),
        error_details_json: bound_optional_json_object(error_details_json, "error_details_json"),
    })
}

pub(super) async fn enqueue_request_log_with_backpressure<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    log_tx: &tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    args: super::RequestLogEnqueueArgs,
) {
    let trace_id = args.trace_id.clone();
    let cli_key = args.cli_key.clone();
    let Some(insert) = request_log_insert_from_args(args) else {
        return;
    };

    let status = insert.status.unwrap_or(0);
    let is_important = insert.error_code.is_some() || status >= 400;

    let reserve = tokio::time::timeout(LOG_ENQUEUE_MAX_WAIT, log_tx.reserve()).await;
    match reserve {
        Ok(Ok(permit)) => {
            permit.send(insert);
        }
        Ok(Err(_)) => {
            emit_gateway_log(
                app,
                "warn",
                GatewayErrorCode::RequestLogChannelClosed.as_str(),
                format!(
                    "request log channel closed; using write-through fallback trace_id={} cli={}",
                    trace_id, cli_key
                ),
            );
            request_logs::spawn_write_through(app.clone(), db.clone(), insert);
        }
        Err(_) => {
            match log_tx.try_send(insert) {
                Ok(()) => {
                    emit_gateway_log(
                        app,
                        "warn",
                        GatewayErrorCode::RequestLogEnqueueTimeout.as_str(),
                        format!(
                            "request log enqueue timed out ({}ms); used try_send fallback trace_id={} cli={}",
                            LOG_ENQUEUE_MAX_WAIT.as_millis(),
                            trace_id,
                            cli_key
                        ),
                    );
                    return;
                }
                Err(err) => {
                    let insert = err.into_inner();
                    if is_important {
                        let count = next_request_log_write_through_count(now_unix_seconds());
                        if count <= REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC {
                            emit_gateway_log(
                                app,
                                "warn",
                                GatewayErrorCode::RequestLogWriteThroughOnBackpressure.as_str(),
                                format!(
                                    "request log enqueue timed out ({}ms) and channel full; using write-through fallback trace_id={} cli={} status={}",
                                    LOG_ENQUEUE_MAX_WAIT.as_millis(),
                                    trace_id,
                                    cli_key,
                                    status
                                ),
                            );
                            request_logs::spawn_write_through(app.clone(), db.clone(), insert);
                        } else if count == REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC + 1 {
                            emit_gateway_log(
                                app,
                                "error",
                                GatewayErrorCode::RequestLogWriteThroughRateLimited.as_str(),
                                format!(
                                    "request log write-through rate limited: max_per_sec={} (dropping important logs) trace_id={} cli={} status={}",
                                    REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC,
                                    trace_id,
                                    cli_key,
                                    status
                                ),
                            );
                        }
                        return;
                    }
                }
            }

            emit_gateway_log(
                app,
                "error",
                GatewayErrorCode::RequestLogDropped.as_str(),
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

pub(super) async fn enqueue_request_log_placeholder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
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
                GatewayErrorCode::RequestLogChannelClosed.as_str(),
                format!(
                    "request log placeholder dropped; channel closed trace_id={} cli={}",
                    trace_id, cli_key
                ),
            );
        }
        Err(_) => match log_tx.try_send(insert) {
            Ok(()) => {
                emit_gateway_log(
                    app,
                    "warn",
                    GatewayErrorCode::RequestLogEnqueueTimeout.as_str(),
                    format!(
                        "request log placeholder used try_send fallback after {}ms trace_id={} cli={}",
                        LOG_ENQUEUE_MAX_WAIT.as_millis(),
                        trace_id,
                        cli_key
                    ),
                );
            }
            Err(_) => {
                emit_gateway_log(
                    app,
                    "warn",
                    GatewayErrorCode::RequestLogDropped.as_str(),
                    format!(
                        "request log placeholder dropped (queue full after {}ms) trace_id={} cli={}",
                        LOG_ENQUEUE_MAX_WAIT.as_millis(),
                        trace_id,
                        cli_key
                    ),
                );
            }
        },
    }
}

pub(in crate::gateway) fn spawn_enqueue_request_log_with_backpressure<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    args: super::RequestLogEnqueueArgs,
) {
    let Some(permit) =
        try_acquire_request_log_enqueue_task_permit(request_log_enqueue_task_limiter())
    else {
        enqueue_request_log_when_spawn_saturated(&app, &db, &log_tx, args);
        return;
    };

    tauri::async_runtime::spawn(async move {
        let _permit = permit;
        enqueue_request_log_with_backpressure(&app, &db, &log_tx, args).await;
    });
}

fn enqueue_request_log_when_spawn_saturated<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    log_tx: &tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    args: super::RequestLogEnqueueArgs,
) {
    let trace_id = args.trace_id.clone();
    let cli_key = args.cli_key.clone();
    let Some(insert) = request_log_insert_from_args(args) else {
        return;
    };

    let status = insert.status.unwrap_or(0);
    let is_important = insert.error_code.is_some() || status >= 400;

    match log_tx.try_send(insert) {
        Ok(()) => {}
        Err(tokio::sync::mpsc::error::TrySendError::Closed(insert)) => {
            emit_gateway_log(
                app,
                "warn",
                GatewayErrorCode::RequestLogChannelClosed.as_str(),
                format!(
                    "request log enqueue task saturated and channel closed; using write-through fallback trace_id={} cli={}",
                    trace_id, cli_key
                ),
            );
            request_logs::spawn_write_through(app.clone(), db.clone(), insert);
        }
        Err(tokio::sync::mpsc::error::TrySendError::Full(insert)) if is_important => {
            let count = next_request_log_write_through_count(now_unix_seconds());
            if count <= REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC {
                emit_gateway_log(
                    app,
                    "warn",
                    GatewayErrorCode::RequestLogWriteThroughOnBackpressure.as_str(),
                    format!(
                        "request log enqueue task saturated and channel full; using write-through fallback trace_id={} cli={} status={}",
                        trace_id, cli_key, status
                    ),
                );
                request_logs::spawn_write_through(app.clone(), db.clone(), insert);
            } else if count == REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC + 1 {
                emit_gateway_log(
                    app,
                    "error",
                    GatewayErrorCode::RequestLogWriteThroughRateLimited.as_str(),
                    format!(
                        "request log write-through rate limited while enqueue tasks saturated: max_per_sec={} (dropping important logs) trace_id={} cli={} status={}",
                        REQUEST_LOG_WRITE_THROUGH_MAX_PER_SEC, trace_id, cli_key, status
                    ),
                );
            }
        }
        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
            emit_gateway_log(
                app,
                "error",
                GatewayErrorCode::RequestLogDropped.as_str(),
                format!(
                    "request log dropped (enqueue task saturated and queue full) trace_id={} cli={}",
                    trace_id, cli_key
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::{UsageExtract, UsageMetrics};

    #[test]
    fn request_log_enqueue_task_permit_returns_none_when_full() {
        let limiter = Arc::new(Semaphore::new(1));
        let _held =
            try_acquire_request_log_enqueue_task_permit(limiter.clone()).expect("first permit");

        assert!(try_acquire_request_log_enqueue_task_permit(limiter).is_none());
    }

    #[test]
    fn request_log_enqueue_task_permit_releases_on_drop() {
        let limiter = Arc::new(Semaphore::new(1));
        let held =
            try_acquire_request_log_enqueue_task_permit(limiter.clone()).expect("first permit");

        drop(held);

        assert!(try_acquire_request_log_enqueue_task_permit(limiter).is_some());
    }

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
            provider_chain_json: None,
            error_details_json: None,
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
    fn request_log_start_placeholder_is_in_progress_row() {
        let mut args = base_args();
        args.status = None;
        args.error_code = None;
        args.duration_ms = 0;
        args.session_id = Some("session-1".to_string());
        args.query = Some("foo=1".to_string());
        args.special_settings_json = Some(r#"[{"type":"provider_lock"}]"#.to_string());
        args.requested_model = Some("claude-sonnet".to_string());
        args.created_at_ms = 1234;
        args.created_at = 1;

        assert_eq!(args.status, None);
        assert_eq!(args.error_code, None);
        assert_eq!(args.duration_ms, 0);
        assert_eq!(args.attempts_json, "[]");
        assert_eq!(args.path, "/v1/messages");
        assert_eq!(args.requested_model.as_deref(), Some("claude-sonnet"));
        assert_eq!(args.created_at_ms, 1234);
        assert_eq!(args.created_at, 1);
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

    #[test]
    fn request_log_insert_keeps_ttfb_when_equal_to_duration_and_filters_only_greater() {
        let mut equal_args = base_args();
        equal_args.duration_ms = 123;
        equal_args.ttfb_ms = Some(123);

        let equal_insert = request_log_insert_from_args(equal_args).expect("insert");
        assert_eq!(equal_insert.ttfb_ms, Some(123));

        let mut greater_args = base_args();
        greater_args.duration_ms = 123;
        greater_args.ttfb_ms = Some(124);

        let greater_insert = request_log_insert_from_args(greater_args).expect("insert");
        assert_eq!(greater_insert.ttfb_ms, None);
    }

    #[test]
    fn request_log_insert_preserves_in_progress_placeholder_shape() {
        let mut args = base_args();
        args.status = None;
        args.error_code = None;
        args.duration_ms = 0;
        args.requested_model = Some("claude-sonnet".to_string());

        let insert = request_log_insert_from_args(args).expect("insert");
        assert_eq!(insert.status, None);
        assert_eq!(insert.error_code, None);
        assert_eq!(insert.duration_ms, 0);
        assert_eq!(insert.requested_model.as_deref(), Some("claude-sonnet"));
    }

    #[test]
    fn request_log_insert_bounds_text_fields_on_utf8_boundaries() {
        let mut args = base_args();
        args.session_id = Some("会".repeat(REQUEST_LOG_SHORT_TEXT_MAX_CHARS + 1));
        args.method = "M".repeat(REQUEST_LOG_METHOD_MAX_CHARS + 1);
        args.path = "路".repeat(REQUEST_LOG_PATH_MAX_CHARS + 1);
        args.query = Some("查".repeat(REQUEST_LOG_QUERY_MAX_CHARS + 1));
        args.requested_model = Some("模".repeat(REQUEST_LOG_SHORT_TEXT_MAX_CHARS + 1));

        let insert = request_log_insert_from_args(args).expect("insert");

        assert_eq!(
            insert
                .session_id
                .as_deref()
                .map(|value| value.chars().count()),
            Some(REQUEST_LOG_SHORT_TEXT_MAX_CHARS)
        );
        assert_eq!(insert.method.chars().count(), REQUEST_LOG_METHOD_MAX_CHARS);
        assert_eq!(insert.path.chars().count(), REQUEST_LOG_PATH_MAX_CHARS);
        assert_eq!(
            insert.query.as_deref().map(|value| value.chars().count()),
            Some(REQUEST_LOG_QUERY_MAX_CHARS)
        );
        assert_eq!(
            insert
                .requested_model
                .as_deref()
                .map(|value| value.chars().count()),
            Some(REQUEST_LOG_SHORT_TEXT_MAX_CHARS)
        );
    }

    #[test]
    fn request_log_insert_replaces_oversized_json_fields_with_valid_markers() {
        let oversized = "x".repeat(REQUEST_LOG_JSON_MAX_BYTES + 1);
        let mut args = base_args();
        args.attempts_json = oversized.clone();
        args.special_settings_json = Some(oversized.clone());
        args.usage = Some(UsageExtract {
            metrics: UsageMetrics::default(),
            usage_json: oversized.clone(),
        });
        args.provider_chain_json = Some(oversized.clone());
        args.error_details_json = Some(oversized);

        let insert = request_log_insert_from_args(args).expect("insert");

        assert_eq!(insert.attempts_json, "[]");

        let special_settings: serde_json::Value = serde_json::from_str(
            insert
                .special_settings_json
                .as_deref()
                .expect("settings json"),
        )
        .expect("valid settings marker");
        assert_eq!(
            special_settings
                .get(0)
                .and_then(|item| item.get("field"))
                .and_then(serde_json::Value::as_str),
            Some("special_settings_json")
        );

        let provider_chain: serde_json::Value =
            serde_json::from_str(insert.provider_chain_json.as_deref().expect("chain json"))
                .expect("valid chain marker");
        assert!(provider_chain.is_array());
        assert_eq!(
            provider_chain
                .get(0)
                .and_then(|item| item.get("field"))
                .and_then(serde_json::Value::as_str),
            Some("provider_chain_json")
        );

        let usage_json: serde_json::Value =
            serde_json::from_str(insert.usage_json.as_deref().expect("usage json"))
                .expect("valid usage marker");
        assert_eq!(
            usage_json.get("field").and_then(serde_json::Value::as_str),
            Some("usage_json")
        );

        let error_details: serde_json::Value =
            serde_json::from_str(insert.error_details_json.as_deref().expect("error json"))
                .expect("valid error marker");
        assert_eq!(
            error_details
                .get("field")
                .and_then(serde_json::Value::as_str),
            Some("error_details_json")
        );
    }

    #[test]
    fn request_log_insert_bounds_json_fields_by_utf8_bytes() {
        let oversized_multibyte = format!(
            "[\"{}\"]",
            "界".repeat((REQUEST_LOG_JSON_MAX_BYTES / "界".len()) + 1)
        );
        assert!(oversized_multibyte.len() > REQUEST_LOG_JSON_MAX_BYTES);
        assert!(oversized_multibyte.chars().count() < REQUEST_LOG_JSON_MAX_BYTES);

        let mut args = base_args();
        args.attempts_json = oversized_multibyte.clone();
        args.special_settings_json = Some(oversized_multibyte);

        let insert = request_log_insert_from_args(args).expect("insert");

        assert_eq!(insert.attempts_json, "[]");
        let special_settings: serde_json::Value = serde_json::from_str(
            insert
                .special_settings_json
                .as_deref()
                .expect("settings json"),
        )
        .expect("valid settings marker");
        assert_eq!(
            special_settings
                .get(0)
                .and_then(|item| item.get("maxBytes"))
                .and_then(serde_json::Value::as_u64),
            Some(REQUEST_LOG_JSON_MAX_BYTES as u64)
        );
    }
}
