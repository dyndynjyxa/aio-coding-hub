use super::context::CommonCtx;
use crate::gateway::codex_session_id::{self, CodexSessionCompletionResult, CodexSessionIdCache};
use crate::gateway::response_fixer;
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue};
use serde_json::Value;

pub(super) struct ApplyCodexSessionIdCompletionInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) enabled: bool,
    pub(super) source_cli_key: &'a str,
    pub(super) session_id: Option<&'a str>,
    pub(super) base_headers: &'a HeaderMap,
    pub(super) upstream_body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
}

struct BridgeCodexSessionCompletion {
    result: CodexSessionCompletionResult,
    body_bytes: Option<Vec<u8>>,
}

pub(super) fn apply_if_needed<R: tauri::Runtime>(
    input: ApplyCodexSessionIdCompletionInput<'_, R>,
) -> Option<String> {
    let ApplyCodexSessionIdCompletionInput {
        ctx,
        enabled,
        source_cli_key,
        session_id,
        base_headers,
        upstream_body_bytes,
        strip_request_content_encoding,
    } = input;

    if !enabled || source_cli_key != "codex" {
        return None;
    }

    let completion = {
        let mut cache = ctx.state.codex_session_cache.lock_or_recover();
        complete_translated_codex_request(
            &mut cache,
            ctx.created_at,
            ctx.created_at_ms,
            base_headers,
            session_id,
            upstream_body_bytes.as_ref(),
        )
    };

    match completion {
        Some(completion) => {
            if let Some(body_bytes) = completion.body_bytes {
                *upstream_body_bytes = Bytes::from(body_bytes);
                *strip_request_content_encoding = true;
            }

            response_fixer::push_special_setting(
                ctx.special_settings,
                serde_json::json!({
                    "type": "codex_session_id_completion",
                    "scope": "request",
                    "hit": completion.result.applied,
                    "action": completion.result.action,
                    "source": completion.result.source,
                    "sessionId": completion.result.session_id,
                    "changedHeader": completion.result.changed_headers,
                    "changedBody": completion.result.changed_body,
                    "bridgeType": "cx2cc",
                    "upstreamCliKey": source_cli_key,
                }),
            );

            Some(completion.result.session_id)
        }
        None => {
            response_fixer::push_special_setting(
                ctx.special_settings,
                serde_json::json!({
                    "type": "codex_session_id_completion",
                    "scope": "request",
                    "hit": false,
                    "action": "skipped",
                    "source": "cx2cc_bridge",
                    "sessionId": session_id,
                    "changedHeader": false,
                    "changedBody": false,
                    "bridgeType": "cx2cc",
                    "upstreamCliKey": source_cli_key,
                    "reason": "invalid_translated_body",
                }),
            );

            session_id.map(str::to_string)
        }
    }
}

pub(super) fn inject_session_headers_if_needed(headers: &mut HeaderMap, session_id: Option<&str>) {
    let Some(session_id) = session_id.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };

    if headers.get("session_id").is_none() {
        if let Ok(value) = HeaderValue::from_str(session_id) {
            headers.insert("session_id", value);
        }
    }

    if headers.get("x-session-id").is_none() {
        if let Ok(value) = HeaderValue::from_str(session_id) {
            headers.insert("x-session-id", value);
        }
    }
}

fn complete_translated_codex_request(
    cache: &mut CodexSessionIdCache,
    now_unix: i64,
    now_unix_ms: i64,
    base_headers: &HeaderMap,
    session_id: Option<&str>,
    upstream_body_bytes: &[u8],
) -> Option<BridgeCodexSessionCompletion> {
    let mut headers = base_headers.clone();
    inject_session_headers_if_needed(&mut headers, session_id);

    let mut request_body = serde_json::from_slice::<Value>(upstream_body_bytes).ok()?;
    let result = codex_session_id::complete_codex_session_identifiers(
        cache,
        now_unix,
        now_unix_ms,
        &mut headers,
        Some(&mut request_body),
    );
    let body_bytes = if result.changed_body {
        serde_json::to_vec(&request_body).ok()
    } else {
        None
    };

    Some(BridgeCodexSessionCompletion { result, body_bytes })
}

#[cfg(test)]
mod tests {
    use super::{complete_translated_codex_request, inject_session_headers_if_needed};
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use axum::http::HeaderMap;
    use serde_json::json;

    const SESSION_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

    #[test]
    fn translated_codex_request_uses_existing_session_id_for_prompt_cache_key() {
        let mut cache = CodexSessionIdCache::default();
        let body = json!({
            "model": "gpt-4.1",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "hello"}]}],
            "stream": true
        });
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let completion = complete_translated_codex_request(
            &mut cache,
            1_710_000_000,
            1_710_000_000_123,
            &HeaderMap::new(),
            Some(SESSION_ID),
            &encoded,
        )
        .expect("completion");

        assert_eq!(completion.result.session_id, SESSION_ID);
        assert_eq!(completion.result.source, "header_session_id");
        assert!(completion.result.changed_body);

        let next: serde_json::Value =
            serde_json::from_slice(&completion.body_bytes.expect("body bytes")).expect("json");
        assert_eq!(next["prompt_cache_key"], SESSION_ID);
    }

    #[test]
    fn translated_codex_request_reuses_fingerprint_cache_without_explicit_session() {
        let mut cache = CodexSessionIdCache::default();
        let body = json!({
            "model": "gpt-4.1",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "same"}]}],
            "stream": true
        });
        let encoded = serde_json::to_vec(&body).expect("serialize");

        let first = complete_translated_codex_request(
            &mut cache,
            1_710_000_000,
            1_710_000_000_123,
            &HeaderMap::new(),
            None,
            &encoded,
        )
        .expect("first completion");
        let second = complete_translated_codex_request(
            &mut cache,
            1_710_000_100,
            1_710_000_100_456,
            &HeaderMap::new(),
            None,
            &encoded,
        )
        .expect("second completion");

        assert_eq!(first.result.session_id, second.result.session_id);
        assert_eq!(first.result.action, "generated_uuid_v7");
        assert_eq!(second.result.action, "reused_fingerprint_cache");
    }

    #[test]
    fn inject_session_headers_adds_both_codex_header_names() {
        let mut headers = HeaderMap::new();
        inject_session_headers_if_needed(&mut headers, Some(SESSION_ID));

        assert_eq!(
            headers
                .get("session_id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            SESSION_ID
        );
        assert_eq!(
            headers
                .get("x-session-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            SESSION_ID
        );
    }
}
