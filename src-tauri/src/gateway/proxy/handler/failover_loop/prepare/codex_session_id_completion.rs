//! Usage: CX2CC session / prompt-cache stickiness for Responses-style sources.
//!
//! - **codex**: fill `session_id` / `x-session-id` + body `prompt_cache_key`
//!   (gated by `enable_codex_session_id_completion`).
//! - **grok**: same body `prompt_cache_key`, plus `x-grok-conv-id` /
//!   `x-grok-session-id` for xAI sticky routing (always on for CX2CC→Grok).

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

/// True when the upstream model is a Grok/xAI model id (e.g. `grok-4.5`).
///
/// Users often register OpenAI-compatible Grok mid-proxies as `cli_key=codex`
/// (provider type Codex). Wire stickiness must still apply when the *mapped*
/// model is Grok — otherwise Build-equivalent headers never fire.
pub(super) fn is_grok_upstream_model(model: Option<&str>) -> bool {
    model
        .map(|m| {
            let m = m.trim().to_ascii_lowercase();
            m.starts_with("grok") || m.contains("grok-") || m.contains("grok.")
        })
        .unwrap_or(false)
}

/// Whether CX2CC should complete sticky session fields for this source CLI / model.
fn should_complete_session_identifiers(
    source_cli_key: &str,
    codex_setting_enabled: bool,
    model: Option<&str>,
) -> bool {
    // Grok wire path: always on (cli_key=grok OR mapped model is grok-*).
    if source_cli_key == "grok" || is_grok_upstream_model(model) {
        return true;
    }
    match source_cli_key {
        "codex" => codex_setting_enabled,
        _ => false,
    }
}

/// Whether to inject Build-equivalent `x-grok-*` headers on CX2CC outbound.
pub(super) fn needs_grok_build_wire(source_cli_key: Option<&str>, model: Option<&str>) -> bool {
    source_cli_key == Some("grok") || is_grok_upstream_model(model)
}

fn model_from_body_bytes(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(body).ok().and_then(|v| {
        v.get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
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

    let model = model_from_body_bytes(upstream_body_bytes.as_ref());
    if !should_complete_session_identifiers(source_cli_key, enabled, model.as_deref()) {
        return None;
    }

    let use_grok_wire = needs_grok_build_wire(Some(source_cli_key), model.as_deref());

    let completion = {
        let mut cache = ctx.state.codex_session_cache.lock_or_recover();
        complete_translated_codex_request(
            &mut cache,
            ctx.created_at,
            ctx.created_at_ms,
            base_headers,
            session_id,
            source_cli_key,
            use_grok_wire,
            model.as_deref(),
            upstream_body_bytes.as_ref(),
        )
    };

    let setting_type = if use_grok_wire {
        "cx2cc_grok_cache_stickiness"
    } else {
        "codex_session_id_completion"
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
                    "type": setting_type,
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
                    "type": setting_type,
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

/// Wire identity for CX2CC→Grok, mirroring `xai-org/grok-build`
/// `GrokRequestHeaders::apply` (xai-grok-sampler client).
#[derive(Debug, Clone, Copy)]
pub(super) struct GrokBuildWireHeaders<'a> {
    pub(super) session_id: &'a str,
    /// Mapped upstream model (e.g. `grok-4.5`) for `x-grok-model-override`.
    pub(super) model_id: Option<&'a str>,
    /// Optional turn index (Build uses prompt index); omit when unknown.
    pub(super) turn_idx: Option<&'a str>,
}

/// Inject sticky session headers for CX2CC upstream.
///
/// Always fills generic `session_id` / `x-session-id`. For Grok, injects the
/// Build-equivalent `x-grok-*` suite (see [`inject_grok_build_wire_headers`]).
pub(super) fn inject_session_headers_if_needed(
    headers: &mut HeaderMap,
    session_id: Option<&str>,
    source_cli_key: Option<&str>,
) {
    inject_session_headers_with_model(headers, session_id, source_cli_key, None);
}

/// Like [`inject_session_headers_if_needed`], but can attach `x-grok-model-override`.
///
/// Grok Build wire headers apply when `source_cli_key == "grok"` **or** the
/// mapped model is a Grok id (common mis-label: mid-proxy registered as Codex).
pub(super) fn inject_session_headers_with_model(
    headers: &mut HeaderMap,
    session_id: Option<&str>,
    source_cli_key: Option<&str>,
    model_id: Option<&str>,
) {
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

    if needs_grok_build_wire(source_cli_key, model_id) {
        inject_grok_build_wire_headers(
            headers,
            GrokBuildWireHeaders {
                session_id,
                model_id,
                turn_idx: None,
            },
        );
    }
}

/// Align CX2CC→Grok outbound headers with official grok-build wire format.
///
/// Build primarily stickies via **headers** (not body `prompt_cache_key`).
/// Missing `x-grok-model-override` / client identity is a known gap vs Build
/// traffic that hits the same API-key mid-proxy with cache hits.
pub(super) fn inject_grok_build_wire_headers(
    headers: &mut HeaderMap,
    wire: GrokBuildWireHeaders<'_>,
) {
    let session_id = wire.session_id.trim();
    if session_id.is_empty() {
        return;
    }

    // Stable affinity (session-scoped). Always overwrite so mid-pipeline cannot drop them.
    if let Ok(value) = HeaderValue::from_str(session_id) {
        headers.insert("x-grok-conv-id", value.clone());
        headers.insert("x-grok-session-id", value);
    }

    // Per-request id (Build: new every sample). Prefer fresh value each inject call.
    let req_id = new_grok_req_id();
    if let Ok(value) = HeaderValue::from_str(&req_id) {
        headers.insert("x-grok-req-id", value);
    }

    // Model override — Build always sets this for IC routing / proxy gating.
    if let Some(model) = wire.model_id.map(str::trim).filter(|v| !v.is_empty()) {
        if let Ok(value) = HeaderValue::from_str(model) {
            headers.insert("x-grok-model-override", value);
        }
    }

    // Client identity — matches OAuth adapter / Build default header set.
    let client_version = crate::gateway::oauth::adapters::grok::grok_client_version();
    if let Ok(value) = HeaderValue::from_str(&client_version) {
        headers.insert("x-grok-client-version", value);
    }

    // Stable agent surface for CX2CC bridge (Build uses process agent_id).
    headers.insert(
        "x-grok-agent-id",
        HeaderValue::from_static("aio-coding-hub-cx2cc"),
    );

    if let Some(turn) = wire.turn_idx.map(str::trim).filter(|v| !v.is_empty()) {
        if let Ok(value) = HeaderValue::from_str(turn) {
            headers.insert("x-grok-turn-idx", value);
        }
    }
}

fn new_grok_req_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("aio-req-{millis:x}-{n:x}")
}

/// Ensure Responses body has a stable `prompt_cache_key` for CX2CC→Grok.
///
/// Returns true when the body bytes were rewritten.
pub(super) fn ensure_prompt_cache_key_on_body(body: &mut Bytes, session_id: Option<&str>) -> bool {
    let Some(session_id) = session_id.map(str::trim).filter(|v| !v.is_empty()) else {
        return false;
    };
    let Ok(mut root) = serde_json::from_slice::<Value>(body.as_ref()) else {
        return false;
    };
    let Some(obj) = root.as_object_mut() else {
        return false;
    };
    let existing = obj
        .get("prompt_cache_key")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if existing == Some(session_id) {
        return false;
    }
    obj.insert(
        "prompt_cache_key".to_string(),
        Value::String(session_id.to_string()),
    );
    match serde_json::to_vec(&root) {
        Ok(encoded) => {
            *body = Bytes::from(encoded);
            true
        }
        Err(_) => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn complete_translated_codex_request(
    cache: &mut CodexSessionIdCache,
    now_unix: i64,
    now_unix_ms: i64,
    base_headers: &HeaderMap,
    session_id: Option<&str>,
    source_cli_key: &str,
    use_grok_wire: bool,
    model_id: Option<&str>,
    upstream_body_bytes: &[u8],
) -> Option<BridgeCodexSessionCompletion> {
    let mut headers = base_headers.clone();
    // Prefer model-aware inject so codex-labeled Grok mid-proxies still get x-grok-*.
    inject_session_headers_with_model(
        &mut headers,
        session_id,
        Some(source_cli_key),
        if use_grok_wire { model_id } else { None },
    );
    // When source_cli_key is codex but model is grok, force grok wire with model.
    if use_grok_wire {
        if let Some(sid) = session_id.map(str::trim).filter(|v| !v.is_empty()) {
            inject_grok_build_wire_headers(
                &mut headers,
                GrokBuildWireHeaders {
                    session_id: sid,
                    model_id,
                    turn_idx: None,
                },
            );
        }
    }

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
    use super::{
        complete_translated_codex_request, inject_session_headers_if_needed,
        inject_session_headers_with_model, needs_grok_build_wire,
        should_complete_session_identifiers,
    };
    use crate::gateway::codex_session_id::CodexSessionIdCache;
    use axum::http::HeaderMap;
    use serde_json::json;

    const SESSION_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

    #[test]
    fn completion_gate_allows_grok_always_and_codex_when_enabled() {
        assert!(should_complete_session_identifiers("grok", false, None));
        assert!(should_complete_session_identifiers("grok", true, None));
        assert!(should_complete_session_identifiers("codex", true, None));
        assert!(!should_complete_session_identifiers("codex", false, None));
        assert!(!should_complete_session_identifiers("claude", true, None));
        // Mid-proxy registered as Codex but mapped model is Grok → still enable.
        assert!(should_complete_session_identifiers(
            "codex",
            false,
            Some("grok-4.5")
        ));
        assert!(needs_grok_build_wire(Some("codex"), Some("grok-4.5")));
        assert!(!needs_grok_build_wire(Some("codex"), Some("gpt-4.1")));
    }

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
            "codex",
            false,
            Some("gpt-4.1"),
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
    fn translated_grok_request_sets_prompt_cache_key_for_xai_sticky() {
        let mut cache = CodexSessionIdCache::default();
        let body = json!({
            "model": "grok-4.5",
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
            "grok",
            true,
            Some("grok-4.5"),
            &encoded,
        )
        .expect("completion");

        assert_eq!(completion.result.session_id, SESSION_ID);
        assert!(completion.result.changed_body);

        let next: serde_json::Value =
            serde_json::from_slice(&completion.body_bytes.expect("body bytes")).expect("json");
        assert_eq!(next["prompt_cache_key"], SESSION_ID);
        assert_eq!(next["model"], "grok-4.5");
    }

    #[test]
    fn codex_labeled_midproxy_with_grok_model_gets_build_wire_headers() {
        let mut headers = HeaderMap::new();
        inject_session_headers_with_model(
            &mut headers,
            Some(SESSION_ID),
            Some("codex"),
            Some("grok-4.5"),
        );
        assert_eq!(
            headers.get("x-grok-conv-id").and_then(|v| v.to_str().ok()),
            Some(SESSION_ID)
        );
        assert_eq!(
            headers
                .get("x-grok-model-override")
                .and_then(|v| v.to_str().ok()),
            Some("grok-4.5")
        );
        assert!(headers.get("x-grok-client-version").is_some());
        assert!(headers.get("x-grok-req-id").is_some());
    }

    #[test]
    fn translated_grok_request_reuses_same_prompt_cache_key_across_turns() {
        let mut cache = CodexSessionIdCache::default();
        let body_turn1 = json!({
            "model": "grok-4.5",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "first"}]}],
            "stream": true
        });
        let body_turn2 = json!({
            "model": "grok-4.5",
            "input": [
                {"role": "user", "content": [{"type": "input_text", "text": "first"}]},
                {"role": "assistant", "content": [{"type": "output_text", "text": "ok"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "second"}]}
            ],
            "stream": true
        });
        let encoded1 = serde_json::to_vec(&body_turn1).expect("serialize");
        let encoded2 = serde_json::to_vec(&body_turn2).expect("serialize");

        let first = complete_translated_codex_request(
            &mut cache,
            1_710_000_000,
            1_710_000_000_123,
            &HeaderMap::new(),
            Some(SESSION_ID),
            "grok",
            true,
            Some("grok-4.5"),
            &encoded1,
        )
        .expect("first");
        let second = complete_translated_codex_request(
            &mut cache,
            1_710_000_100,
            1_710_000_100_456,
            &HeaderMap::new(),
            Some(SESSION_ID),
            "grok",
            true,
            Some("grok-4.5"),
            &encoded2,
        )
        .expect("second");

        assert_eq!(first.result.session_id, second.result.session_id);
        assert_eq!(first.result.session_id, SESSION_ID);

        let next1: serde_json::Value =
            serde_json::from_slice(&first.body_bytes.expect("body1")).expect("json");
        let next2: serde_json::Value =
            serde_json::from_slice(&second.body_bytes.expect("body2")).expect("json");
        assert_eq!(next1["prompt_cache_key"], SESSION_ID);
        assert_eq!(next2["prompt_cache_key"], SESSION_ID);
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
            "codex",
            false,
            Some("gpt-4.1"),
            &encoded,
        )
        .expect("first completion");
        let second = complete_translated_codex_request(
            &mut cache,
            1_710_000_100,
            1_710_000_100_456,
            &HeaderMap::new(),
            None,
            "codex",
            false,
            Some("gpt-4.1"),
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
        inject_session_headers_if_needed(&mut headers, Some(SESSION_ID), Some("codex"));

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
        assert!(headers.get("x-grok-conv-id").is_none());
        assert!(headers.get("x-grok-session-id").is_none());
    }

    #[test]
    fn inject_session_headers_for_grok_adds_xai_sticky_headers() {
        let mut headers = HeaderMap::new();
        inject_session_headers_if_needed(&mut headers, Some(SESSION_ID), Some("grok"));

        assert_eq!(
            headers
                .get("x-grok-conv-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            SESSION_ID
        );
        assert_eq!(
            headers
                .get("x-grok-session-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            SESSION_ID
        );
        assert_eq!(
            headers
                .get("session_id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            SESSION_ID
        );
        // Build wire suite (even without model_id yet).
        assert!(headers.get("x-grok-req-id").is_some());
        assert!(headers.get("x-grok-client-version").is_some());
        assert_eq!(
            headers.get("x-grok-agent-id").and_then(|v| v.to_str().ok()),
            Some("aio-coding-hub-cx2cc")
        );
    }

    #[test]
    fn inject_grok_build_wire_headers_sets_model_override_and_fresh_req_id() {
        use super::{inject_grok_build_wire_headers, GrokBuildWireHeaders};

        let mut headers = HeaderMap::new();
        inject_grok_build_wire_headers(
            &mut headers,
            GrokBuildWireHeaders {
                session_id: SESSION_ID,
                model_id: Some("grok-4.5"),
                turn_idx: Some("3"),
            },
        );
        assert_eq!(
            headers
                .get("x-grok-model-override")
                .and_then(|v| v.to_str().ok()),
            Some("grok-4.5")
        );
        assert_eq!(
            headers.get("x-grok-turn-idx").and_then(|v| v.to_str().ok()),
            Some("3")
        );
        let req1 = headers
            .get("x-grok-req-id")
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        inject_grok_build_wire_headers(
            &mut headers,
            GrokBuildWireHeaders {
                session_id: SESSION_ID,
                model_id: Some("grok-4.5"),
                turn_idx: None,
            },
        );
        let req2 = headers
            .get("x-grok-req-id")
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        assert_ne!(req1, req2, "req-id must be per-request like grok-build");
    }

    #[test]
    fn ensure_prompt_cache_key_rewrites_missing_or_mismatched_key() {
        use super::ensure_prompt_cache_key_on_body;
        use axum::body::Bytes;

        let mut body = Bytes::from(
            serde_json::to_vec(&json!({
                "model": "grok-4.5",
                "input": [],
                "stream": true
            }))
            .unwrap(),
        );
        assert!(ensure_prompt_cache_key_on_body(&mut body, Some(SESSION_ID)));
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["prompt_cache_key"], SESSION_ID);

        // Same key → no rewrite
        assert!(!ensure_prompt_cache_key_on_body(
            &mut body,
            Some(SESSION_ID)
        ));

        // Different key → rewrite
        assert!(ensure_prompt_cache_key_on_body(
            &mut body,
            Some("abcdef01-2345-6789-abcd-ef0123456789")
        ));
        let parsed2: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            parsed2["prompt_cache_key"],
            "abcdef01-2345-6789-abcd-ef0123456789"
        );
    }
}
