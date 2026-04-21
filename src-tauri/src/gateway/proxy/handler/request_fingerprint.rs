use crate::gateway::proxy::caches::RecentErrorCache;
use crate::gateway::proxy::errors::error_response_with_retry_after;
use crate::gateway::util::{
    body_for_introspection, compute_all_providers_unavailable_fingerprint,
    compute_request_fingerprint, extract_idempotency_key_hash, now_unix_seconds,
};
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;
use axum::response::Response;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(super) struct RequestFingerprints {
    pub(super) fingerprint_key: u64,
    pub(super) fingerprint_debug: String,
    pub(super) unavailable_fingerprint_key: u64,
    pub(super) unavailable_fingerprint_debug: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_request_fingerprints(
    cli_key: &str,
    effective_sort_mode_id: Option<i64>,
    method_hint: &str,
    forwarded_path: &str,
    query: Option<&str>,
    session_id: Option<&str>,
    requested_model: Option<&str>,
    headers: &axum::http::HeaderMap,
    body_bytes: &Bytes,
) -> RequestFingerprints {
    let (unavailable_fingerprint_key, unavailable_fingerprint_debug) =
        compute_all_providers_unavailable_fingerprint(
            cli_key,
            effective_sort_mode_id,
            method_hint,
            forwarded_path,
        );

    let idempotency_key_hash = extract_idempotency_key_hash(headers);
    let introspection_body = body_for_introspection(headers, body_bytes);
    let (fingerprint_key, fingerprint_debug) = compute_request_fingerprint(
        cli_key,
        method_hint,
        forwarded_path,
        query,
        session_id,
        requested_model,
        idempotency_key_hash,
        introspection_body.as_ref(),
    );

    RequestFingerprints {
        fingerprint_key,
        fingerprint_debug,
        unavailable_fingerprint_key,
        unavailable_fingerprint_debug,
    }
}

pub(super) fn apply_recent_error_cache_gate(
    recent_errors: &Arc<Mutex<RecentErrorCache>>,
    fingerprints: &RequestFingerprints,
    trace_id: String,
) -> Result<String, Box<Response>> {
    let mut cache = recent_errors.lock_or_recover();
    let now_unix = now_unix_seconds() as i64;
    let cached_error = cache
        .get_error(
            now_unix,
            fingerprints.fingerprint_key,
            &fingerprints.fingerprint_debug,
        )
        .or_else(|| {
            cache.get_error(
                now_unix,
                fingerprints.unavailable_fingerprint_key,
                &fingerprints.unavailable_fingerprint_debug,
            )
        });

    if let Some(entry) = cached_error {
        return Err(Box::new(error_response_with_retry_after(
            entry.status,
            entry.trace_id,
            entry.error_code,
            entry.message,
            vec![],
            entry.retry_after_seconds,
        )));
    }

    // NOTE: trace_id 必须做到“每次请求唯一”。
    // 过去这里会在短 TTL 内按 fingerprint 复用 trace_id；但 request_logs 写入使用
    // `ON CONFLICT(trace_id) DO UPDATE`，会导致前一条已落库的请求日志（如 499 取消）
    // 被后续新请求覆盖，从而在 UI 上表现为“自动合并/计时不归零”。
    //
    // 现在仅在命中 recent error cache（直接返回缓存错误，不会写入新的 request_log）时复用 trace_id。
    Ok(trace_id)
}

#[cfg(test)]
mod tests {
    use super::{apply_recent_error_cache_gate, RecentErrorCache, RequestFingerprints};
    use crate::gateway::proxy::caches::CachedGatewayError;
    use crate::gateway::proxy::GatewayErrorCode;
    use crate::gateway::util::now_unix_seconds;
    use axum::http::StatusCode;
    use std::sync::{Arc, Mutex};

    fn fingerprints() -> RequestFingerprints {
        RequestFingerprints {
            fingerprint_key: 101,
            fingerprint_debug: "fp-101".to_string(),
            unavailable_fingerprint_key: 202,
            unavailable_fingerprint_debug: "fp-unavailable-202".to_string(),
        }
    }

    #[test]
    fn does_not_reuse_trace_id_without_cached_error() {
        let recent_errors = Arc::new(Mutex::new(RecentErrorCache::default()));
        let fps = fingerprints();

        // 第一次请求：正常放行
        let first = apply_recent_error_cache_gate(&recent_errors, &fps, "trace-a".to_string())
            .expect("first request should pass");
        assert_eq!(first, "trace-a");

        // 同 fingerprint 的下一次请求：仍必须保持“每次请求唯一”的 trace_id（不复用 trace-a）
        let second = apply_recent_error_cache_gate(&recent_errors, &fps, "trace-b".to_string())
            .expect("second request should pass");
        assert_eq!(second, "trace-b");
    }

    #[test]
    fn uses_cached_error_trace_id_in_response_header() {
        let recent_errors = Arc::new(Mutex::new(RecentErrorCache::default()));
        let fps = fingerprints();

        let now_unix = now_unix_seconds() as i64;
        {
            let mut cache = recent_errors.lock().expect("lock recent_errors");
            cache.insert_error(
                now_unix,
                fps.fingerprint_key,
                CachedGatewayError {
                    trace_id: "trace-cached".to_string(),
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    error_code: GatewayErrorCode::AllProvidersUnavailable.as_str(),
                    message: "cached unavailable".to_string(),
                    retry_after_seconds: Some(30),
                    expires_at_unix: now_unix.saturating_add(30),
                    fingerprint_debug: fps.fingerprint_debug.clone(),
                },
            );
        }

        let resp = apply_recent_error_cache_gate(&recent_errors, &fps, "trace-new".to_string())
            .expect_err("should be gated by cached error");

        let trace_id = resp
            .headers()
            .get("x-trace-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(trace_id, "trace-cached");
    }
}
