//! Usage: Error classification + standardized gateway error responses.

use axum::{
    body::{to_bytes, Bytes},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use super::failover::FailoverDecision;
use super::{ErrorCategory, GatewayErrorCode};
use crate::gateway::events::FailoverAttempt;
use crate::gateway::plugins::context::{GatewayPluginHookName, GatewayResponseHookInput};
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct GatewayErrorResponse {
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

pub(super) fn classify_reqwest_error(err: &reqwest::Error) -> (ErrorCategory, &'static str) {
    if err.is_timeout() {
        return (
            ErrorCategory::SystemError,
            GatewayErrorCode::UpstreamTimeout.as_str(),
        );
    }
    if err.is_connect() {
        return (
            ErrorCategory::SystemError,
            GatewayErrorCode::UpstreamConnectFailed.as_str(),
        );
    }
    (
        ErrorCategory::SystemError,
        GatewayErrorCode::InternalError.as_str(),
    )
}

pub(super) fn classify_upstream_status(
    status: reqwest::StatusCode,
) -> (ErrorCategory, &'static str, FailoverDecision) {
    if status.is_server_error() {
        return (
            ErrorCategory::ProviderError,
            GatewayErrorCode::Upstream5xx.as_str(),
            FailoverDecision::SwitchProvider,
        );
    }

    match status.as_u16() {
        401 | 403 => (
            ErrorCategory::ProviderError,
            GatewayErrorCode::Upstream4xx.as_str(),
            FailoverDecision::SwitchProvider,
        ),
        402 => (
            // Payment Required / insufficient balance / subscription required.
            // Align with : treat as provider-side limitation and allow failover.
            ErrorCategory::ProviderError,
            GatewayErrorCode::Upstream4xx.as_str(),
            FailoverDecision::SwitchProvider,
        ),
        404 => (
            // Resource not found is often provider-specific (path/model support mismatch).
            ErrorCategory::ResourceNotFound,
            GatewayErrorCode::Upstream4xx.as_str(),
            FailoverDecision::SwitchProvider,
        ),
        408 | 429 => (
            ErrorCategory::ProviderError,
            GatewayErrorCode::Upstream4xx.as_str(),
            FailoverDecision::RetrySameProvider,
        ),
        _ if status.is_client_error() => (
            // Default: allow retry + failover for upstream 4xx.
            // Non-retryable client input errors are detected separately by scanning upstream error bodies.
            ErrorCategory::ProviderError,
            GatewayErrorCode::Upstream4xx.as_str(),
            FailoverDecision::RetrySameProvider,
        ),
        _ => (
            ErrorCategory::ProviderError,
            GatewayErrorCode::InternalError.as_str(),
            FailoverDecision::Abort,
        ),
    }
}

pub(super) fn error_response(
    status: StatusCode,
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
) -> Response {
    error_response_with_retry_after(status, trace_id, error_code, message, attempts, None)
}

pub(super) fn error_response_with_retry_after(
    status: StatusCode,
    trace_id: String,
    error_code: &'static str,
    message: String,
    attempts: Vec<FailoverAttempt>,
    retry_after_seconds: Option<u64>,
) -> Response {
    let payload = GatewayErrorResponse {
        trace_id: trace_id.clone(),
        error_code,
        message,
        attempts,
        retry_after_seconds,
    };

    let mut resp = (status, Json(payload)).into_response();

    if let Ok(v) = HeaderValue::from_str(&trace_id) {
        resp.headers_mut().insert("x-trace-id", v);
    }

    if let Some(seconds) = retry_after_seconds.filter(|v| *v > 0) {
        let value = seconds.to_string();
        if let Ok(v) = HeaderValue::from_str(&value) {
            resp.headers_mut().insert(header::RETRY_AFTER, v);
        }
    }

    resp
}

pub(super) async fn apply_gateway_error_hook(
    db: &crate::db::Db,
    pipeline: Arc<GatewayPluginPipeline>,
    trace_id: String,
    response: Response,
) -> Response {
    let status = response.status();
    let mut headers = response.headers().clone();
    let body = match to_bytes(response.into_body(), usize::MAX).await {
        Ok(body) => body,
        Err(err) => {
            tracing::warn!(
                trace_id = %trace_id,
                error = %err,
                "failed to read gateway error response body for plugin hook"
            );
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                trace_id,
                GatewayErrorCode::ResponseBuildError.as_str(),
                "failed to read gateway error response body".to_string(),
                vec![],
            );
        }
    };

    let input = GatewayResponseHookInput {
        hook_name: GatewayPluginHookName::Error,
        trace_id: trace_id.clone(),
        status: status.as_u16(),
        headers: headers.clone(),
        body: body.clone(),
    };

    let output = match pipeline.run_response_hook(input).await {
        Ok(output) => {
            crate::gateway::plugins::audit::persist_gateway_plugin_audit_events(
                db,
                &trace_id,
                output.audit_events.clone(),
            );
            output
        }
        Err(mut err) => {
            crate::gateway::plugins::audit::persist_gateway_plugin_error_audit_events(
                db, &trace_id, &mut err,
            );
            tracing::warn!(
                trace_id = %trace_id,
                error = %err,
                "plugin gateway.error hook failed; keeping original error response"
            );
            let mut builder = Response::builder().status(status);
            for (name, value) in headers.iter() {
                builder = builder.header(name, value);
            }
            return builder
                .body(axum::body::Body::from(body))
                .unwrap_or_else(|_| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        trace_id,
                        GatewayErrorCode::ResponseBuildError.as_str(),
                        "failed to rebuild gateway error response".to_string(),
                        vec![],
                    )
                });
        }
    };

    if let Some(blocked) = output.blocked {
        tracing::warn!(
            trace_id = %trace_id,
            status = blocked.status,
            reason = %blocked.reason,
            "plugin blocked gateway error response"
        );
        return error_response(
            StatusCode::from_u16(blocked.status).unwrap_or(StatusCode::BAD_GATEWAY),
            trace_id,
            GatewayErrorCode::InternalError.as_str(),
            blocked.reason,
            vec![],
        );
    }

    headers = output.headers;
    headers.remove(header::CONTENT_LENGTH);
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    builder
        .body(axum::body::Body::from(Bytes::copy_from_slice(
            output.body.as_ref(),
        )))
        .unwrap_or_else(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                trace_id,
                GatewayErrorCode::ResponseBuildError.as_str(),
                "failed to rebuild gateway error response".to_string(),
                vec![],
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{classify_upstream_status, FailoverDecision};
    use crate::gateway::proxy::{ErrorCategory, GatewayErrorCode};

    #[test]
    fn upstream_402_switches_provider() {
        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::PAYMENT_REQUIRED);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn upstream_404_switches_provider() {
        let (category, code, decision) = classify_upstream_status(reqwest::StatusCode::NOT_FOUND);
        assert!(matches!(category, ErrorCategory::ResourceNotFound));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn upstream_other_4xx_retries_then_failover() {
        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }

    #[test]
    fn upstream_5xx_switches_provider() {
        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream5xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));

        let (category, code, decision) = classify_upstream_status(reqwest::StatusCode::BAD_GATEWAY);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream5xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));

        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream5xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn upstream_401_403_switches_provider() {
        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::UNAUTHORIZED);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));

        let (category, code, decision) = classify_upstream_status(reqwest::StatusCode::FORBIDDEN);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::SwitchProvider));
    }

    #[test]
    fn upstream_408_429_retries_same_provider() {
        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::REQUEST_TIMEOUT);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));

        let (category, code, decision) =
            classify_upstream_status(reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }

    #[test]
    fn upstream_400_retries_same_provider() {
        let (category, code, decision) = classify_upstream_status(reqwest::StatusCode::BAD_REQUEST);
        assert!(matches!(category, ErrorCategory::ProviderError));
        assert_eq!(code, GatewayErrorCode::Upstream4xx.as_str());
        assert!(matches!(decision, FailoverDecision::RetrySameProvider));
    }
}
