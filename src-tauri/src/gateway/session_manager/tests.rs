use super::*;
use axum::http::{header, HeaderMap, HeaderValue};

// ---------------------------------------------------------------------------
// Sliding TTL tests
// ---------------------------------------------------------------------------

#[test]
fn sliding_ttl_refreshes_on_get_bound_provider() {
    let manager = SessionManager::new(); // TTL = 300s
    let t0 = 1000;

    // Create a binding at t0
    manager.bind_success("claude", "s1", 42, None, t0);

    // Access at t0 + 200 (within TTL) — should succeed and refresh
    let t1 = t0 + 200;
    let provider = manager.get_bound_provider("claude", "s1", t1);
    assert_eq!(provider, Some(42));

    // After refresh, binding should survive until t1 + 300 = 1500
    // Access at t0 + 400 (> original t0+300 but < refreshed t1+300)
    let t2 = t0 + 400;
    let provider = manager.get_bound_provider("claude", "s1", t2);
    assert_eq!(
        provider,
        Some(42),
        "binding should still be valid after sliding TTL refresh"
    );
}

#[test]
fn sliding_ttl_expired_without_access() {
    let manager = SessionManager::new(); // TTL = 300s
    let t0 = 1000;

    manager.bind_success("claude", "s1", 42, None, t0);

    // No access in between — check after TTL expires
    let t_expired = t0 + 301;
    let provider = manager.get_bound_provider("claude", "s1", t_expired);
    assert_eq!(
        provider, None,
        "binding should expire without sliding refresh"
    );
}

#[test]
fn sliding_ttl_chain_of_accesses_extends_lifetime() {
    let manager = SessionManager::new(); // TTL = 300s
    let t0 = 1000;

    manager.bind_success("claude", "s1", 42, None, t0);

    // Chain of accesses, each within TTL of the previous
    for i in 1..=5 {
        let t = t0 + i * 200; // 1200, 1400, 1600, 1800, 2000
        let provider = manager.get_bound_provider("claude", "s1", t);
        assert_eq!(provider, Some(42), "access {i} at t={t} should succeed");
    }

    // Last access at 2000 refreshed to 2300. Access at 2299 should work.
    let provider = manager.get_bound_provider("claude", "s1", 2299);
    assert_eq!(provider, Some(42));

    // But 2600 (after last refresh) should fail
    let provider = manager.get_bound_provider("claude", "s1", 2601);
    assert_eq!(provider, None);
}

#[test]
fn sliding_ttl_refreshes_on_get_bound_sort_mode_id() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_sort_mode("claude", "s1", Some(7), None, t0);

    // Access at t0 + 200 refreshes TTL
    let t1 = t0 + 200;
    let mode = manager.get_bound_sort_mode_id("claude", "s1", t1);
    assert_eq!(mode, Some(Some(7)));

    // Should survive past original expiry (t0 + 300) because of refresh
    let t2 = t0 + 400;
    let mode = manager.get_bound_sort_mode_id("claude", "s1", t2);
    assert_eq!(
        mode,
        Some(Some(7)),
        "sort_mode binding should survive after sliding refresh"
    );
}

#[test]
fn sliding_ttl_refreshes_on_get_bound_provider_order() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_sort_mode("claude", "s1", Some(1), Some(vec![10, 20]), t0);

    // Access at t0 + 200 refreshes
    let t1 = t0 + 200;
    let order = manager.get_bound_provider_order("claude", "s1", t1);
    assert_eq!(order, Some(vec![10, 20]));

    // Should survive past original expiry
    let t2 = t0 + 400;
    let order = manager.get_bound_provider_order("claude", "s1", t2);
    assert_eq!(order, Some(vec![10, 20]));
}

#[test]
fn sliding_ttl_bind_success_refreshes_existing_binding() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_success("claude", "s1", 42, None, t0);

    // bind_success again at t0 + 200 with same session
    let t1 = t0 + 200;
    manager.bind_success("claude", "s1", 42, None, t1);

    // Should survive until t1 + 300 = 1500
    let t2 = t0 + 400;
    let provider = manager.get_bound_provider("claude", "s1", t2);
    assert_eq!(provider, Some(42));
}

#[test]
fn sliding_ttl_lru_eviction_works_with_refreshed_bindings() {
    let manager = SessionManager::new();
    let t0 = 1000;

    // Create two bindings
    manager.bind_success("claude", "old_session", 1, None, t0);
    manager.bind_success("claude", "new_session", 2, None, t0);

    // Refresh only new_session at t0 + 100
    let t1 = t0 + 100;
    manager.get_bound_provider("claude", "new_session", t1);

    // Both active — list should show new_session with higher expires_at
    let active = manager.list_active(t1, 10);
    assert_eq!(active.len(), 2);
    // First (sorted by expires_at desc) should be new_session (refreshed)
    assert_eq!(active[0].session_id, "new_session");
    assert_eq!(active[1].session_id, "old_session");
    assert!(active[0].expires_at > active[1].expires_at);
}

#[test]
fn clear_cli_bindings_removes_only_target_cli() {
    let manager = SessionManager::new();
    let now_unix = 100;

    manager.bind_sort_mode(
        "claude",
        "session_a",
        Some(1),
        Some(vec![101, 102]),
        now_unix,
    );
    manager.bind_sort_mode("claude", "session_b", None, None, now_unix);
    manager.bind_sort_mode("codex", "session_c", Some(2), Some(vec![201]), now_unix);

    assert_eq!(manager.clear_cli_bindings(""), 0);

    let removed = manager.clear_cli_bindings("claude");
    assert_eq!(removed, 2);

    assert_eq!(
        manager.get_bound_sort_mode_id("claude", "session_a", now_unix),
        None
    );
    assert_eq!(
        manager.get_bound_sort_mode_id("claude", "session_b", now_unix),
        None
    );
    assert_eq!(
        manager.get_bound_sort_mode_id("codex", "session_c", now_unix),
        Some(Some(2))
    );
}

#[test]
fn extract_session_id_fallback_uses_message_fingerprint_and_ignores_user_agent() {
    let body = serde_json::json!({
        "messages": [
            { "role": "user", "content": "hello" },
            { "role": "assistant", "content": "world" }
        ]
    });

    let mut h1 = HeaderMap::new();
    h1.insert(header::USER_AGENT, HeaderValue::from_static("ua-1"));
    let mut h2 = HeaderMap::new();
    h2.insert(header::USER_AGENT, HeaderValue::from_static("ua-2"));

    let id1 = SessionManager::extract_session_id_from_json(&h1, Some(&body)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&h2, Some(&body)).expect("sid 2");
    assert_eq!(id1, id2);
}

#[test]
fn extract_session_id_fallback_changes_when_message_fingerprint_changes() {
    let mut headers = HeaderMap::new();
    headers.insert(header::USER_AGENT, HeaderValue::from_static("ua"));

    let body1 = serde_json::json!({
        "messages": [{ "role": "user", "content": "hello" }]
    });
    let body2 = serde_json::json!({
        "messages": [{ "role": "user", "content": "goodbye" }]
    });

    let id1 = SessionManager::extract_session_id_from_json(&headers, Some(&body1)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&headers, Some(&body2)).expect("sid 2");
    assert_ne!(id1, id2);
}

#[test]
fn extract_session_id_fallback_uses_only_first_three_segments() {
    let headers = HeaderMap::new();

    let body_with_four = serde_json::json!({
        "messages": [
            { "role": "user", "content": "a" },
            { "role": "assistant", "content": "b" },
            { "role": "user", "content": "c" },
            { "role": "assistant", "content": "d" }
        ]
    });
    let body_with_three = serde_json::json!({
        "messages": [
            { "role": "user", "content": "a" },
            { "role": "assistant", "content": "b" },
            { "role": "user", "content": "c" }
        ]
    });

    let id1 =
        SessionManager::extract_session_id_from_json(&headers, Some(&body_with_four)).expect("sid");
    let id2 = SessionManager::extract_session_id_from_json(&headers, Some(&body_with_three))
        .expect("sid");
    assert_eq!(id1, id2);
}

#[test]
fn extract_session_id_fallback_treats_content_parts_equivalent_to_string_content() {
    let headers = HeaderMap::new();

    let body_parts = serde_json::json!({
        "messages": [
            { "role": "user", "content": [{ "text": "he" }, { "text": "llo" }] }
        ]
    });
    let body_string = serde_json::json!({
        "messages": [
            { "role": "user", "content": "hello" }
        ]
    });

    let id1 =
        SessionManager::extract_session_id_from_json(&headers, Some(&body_parts)).expect("sid");
    let id2 =
        SessionManager::extract_session_id_from_json(&headers, Some(&body_string)).expect("sid");
    assert_eq!(id1, id2);
}

#[test]
fn extract_session_id_fallback_supports_input_string_shape() {
    let body = serde_json::json!({ "input": "hello" });

    let mut h1 = HeaderMap::new();
    h1.insert(header::USER_AGENT, HeaderValue::from_static("ua-1"));
    let mut h2 = HeaderMap::new();
    h2.insert(header::USER_AGENT, HeaderValue::from_static("ua-2"));

    let id1 = SessionManager::extract_session_id_from_json(&h1, Some(&body)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&h2, Some(&body)).expect("sid 2");
    assert_eq!(id1, id2);
}

#[test]
fn extract_session_id_fallback_samples_large_input_text_with_tail() {
    let headers = HeaderMap::new();
    let prefix = "a".repeat(SESSION_FINGERPRINT_TEXT_SAMPLE_BYTES + 1024);
    let body1 = serde_json::json!({ "input": format!("{prefix}tail-a") });
    let body2 = serde_json::json!({ "input": format!("{prefix}tail-b") });

    let id1 = SessionManager::extract_session_id_from_json(&headers, Some(&body1)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&headers, Some(&body2)).expect("sid 2");

    assert_ne!(id1, id2);
}

#[test]
fn extract_session_id_fallback_bounds_content_part_scanning() {
    let headers = HeaderMap::new();
    let mut parts = Vec::new();
    for index in 0..SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS {
        parts.push(serde_json::json!({ "text": format!("part-{index};") }));
    }

    let mut parts_with_extra_a = parts.clone();
    parts_with_extra_a.push(serde_json::json!({ "text": "ignored-a" }));
    let mut parts_with_extra_b = parts;
    parts_with_extra_b.push(serde_json::json!({ "text": "ignored-b" }));

    let body1 = serde_json::json!({
        "messages": [{ "role": "user", "content": parts_with_extra_a }]
    });
    let body2 = serde_json::json!({
        "messages": [{ "role": "user", "content": parts_with_extra_b }]
    });

    let id1 = SessionManager::extract_session_id_from_json(&headers, Some(&body1)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&headers, Some(&body2)).expect("sid 2");

    assert_eq!(id1, id2);
}

#[test]
fn extract_session_id_fallback_distinguishes_different_api_keys() {
    let body = serde_json::json!({ "messages": [{ "role": "user", "content": "hello" }] });

    let mut h1 = HeaderMap::new();
    h1.insert("x-api-key", HeaderValue::from_static("key-a-123456789"));
    let mut h2 = HeaderMap::new();
    h2.insert("x-api-key", HeaderValue::from_static("key-b-123456789"));

    let id1 = SessionManager::extract_session_id_from_json(&h1, Some(&body)).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&h2, Some(&body)).expect("sid 2");
    assert_ne!(id1, id2);
}

#[test]
fn sanitize_session_id_truncates_without_splitting_utf8() {
    let raw = format!("{}{}", "a".repeat(MAX_SESSION_ID_LEN - 1), "é");

    let sanitized = sanitize_session_id(&raw).expect("sanitized");

    assert_eq!(sanitized.len(), MAX_SESSION_ID_LEN - 1);
    assert!(sanitized.ends_with('a'));
}

#[test]
fn sanitize_session_id_removes_log_injection_controls_before_truncating() {
    let raw = format!("{}\n{}", "a".repeat(MAX_SESSION_ID_LEN), "tail");

    let sanitized = sanitize_session_id(&raw).expect("sanitized");

    assert_eq!(sanitized.len(), MAX_SESSION_ID_LEN);
    assert!(!sanitized.contains('\n'));
}

#[test]
fn sanitize_deterministic_part_truncates_without_splitting_utf8() {
    let raw = format!("{}{}", "a".repeat(MAX_SESSION_ID_LEN - 1), "é");

    let sanitized = sanitize_deterministic_part(&raw).expect("sanitized");

    assert_eq!(sanitized.len(), MAX_SESSION_ID_LEN - 1);
    assert!(sanitized.ends_with('a'));
}

#[test]
fn sanitize_deterministic_part_removes_log_injection_controls_before_truncating() {
    let raw = format!("{}\n{}", "a".repeat(MAX_SESSION_ID_LEN), "tail");

    let sanitized = sanitize_deterministic_part(&raw).expect("sanitized");

    assert_eq!(sanitized.len(), MAX_SESSION_ID_LEN);
    assert!(!sanitized.contains('\n'));
}

// ---------------------------------------------------------------------------
// Claude Code session identification (Phase 0)
// ---------------------------------------------------------------------------

#[test]
fn extract_session_id_uses_claude_code_session_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-claude-code-session-id",
        HeaderValue::from_static("a679f21c-4ac5-46bf-8901-902c40e91cec"),
    );

    let id = SessionManager::extract_session_id_from_json(&headers, None).expect("sid");
    assert_eq!(id, "a679f21c-4ac5-46bf-8901-902c40e91cec");
}

#[test]
fn extract_session_id_distinguishes_claude_code_processes() {
    let mut h1 = HeaderMap::new();
    h1.insert(
        "x-claude-code-session-id",
        HeaderValue::from_static("a679f21c-4ac5-46bf-8901-902c40e91cec"),
    );
    let mut h2 = HeaderMap::new();
    h2.insert(
        "x-claude-code-session-id",
        HeaderValue::from_static("b2851212-db11-4d49-a615-3ac436e7f2eb"),
    );

    let id1 = SessionManager::extract_session_id_from_json(&h1, None).expect("sid 1");
    let id2 = SessionManager::extract_session_id_from_json(&h2, None).expect("sid 2");
    assert_ne!(id1, id2);
}

#[test]
fn extract_session_id_parses_claude_code_user_id_json() {
    let headers = HeaderMap::new();
    let body = serde_json::json!({
        "metadata": {
            "user_id": "{\"device_id\":\"dev-1\",\"account_uuid\":\"\",\"session_id\":\"a679f21c-4ac5-46bf-8901-902c40e91cec\"}"
        }
    });

    let id = SessionManager::extract_session_id_from_json(&headers, Some(&body)).expect("sid");
    assert_eq!(id, "a679f21c-4ac5-46bf-8901-902c40e91cec");
}

#[test]
fn extract_session_id_user_id_marker_still_supported() {
    let headers = HeaderMap::new();
    let body = serde_json::json!({
        "metadata": { "user_id": "acct-x_session_legacy-sid-123" }
    });

    let id = SessionManager::extract_session_id_from_json(&headers, Some(&body)).expect("sid");
    assert_eq!(id, "legacy-sid-123");
}

#[test]
fn extract_session_id_header_precedes_fingerprint_fallback() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-claude-code-session-id",
        HeaderValue::from_static("a679f21c-4ac5-46bf-8901-902c40e91cec"),
    );
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static("claude-cli/2.1.191"),
    );
    // A body that would otherwise trigger the message-fingerprint fallback.
    let body = serde_json::json!({ "messages": [{ "role": "user", "content": "hi" }] });

    let id = SessionManager::extract_session_id_from_json(&headers, Some(&body)).expect("sid");
    assert_eq!(id, "a679f21c-4ac5-46bf-8901-902c40e91cec");
    assert!(!id.starts_with("sess_"));
}

// ---------------------------------------------------------------------------
// Manual sort_mode pin (Path A — template level)
// ---------------------------------------------------------------------------

#[test]
fn bind_and_get_pinned_sort_mode_roundtrips() {
    let manager = SessionManager::new();
    let t0 = 1000;

    assert!(manager.bind_pinned_sort_mode("claude", "s1", Some(7), t0));
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 10),
        Some(Some(7))
    );
}

#[test]
fn bind_pinned_sort_mode_default_is_some_none() {
    let manager = SessionManager::new();
    let t0 = 1000;

    // Pinning Default mode is represented as Some(None) (pinned, to Default).
    assert!(manager.bind_pinned_sort_mode("claude", "s1", None, t0));
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 10),
        Some(None)
    );
}

#[test]
fn unpinned_session_returns_none() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_success("claude", "s1", 3, Some(5), t0);
    // Auto-bound sort_mode exists, but no manual pin.
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 5),
        None
    );
}

#[test]
fn bind_pinned_sort_mode_rejects_invalid_input() {
    let manager = SessionManager::new();
    let t0 = 1000;

    assert!(!manager.bind_pinned_sort_mode("", "s1", Some(7), t0));
    assert!(!manager.bind_pinned_sort_mode("claude", "", Some(7), t0));
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0),
        None
    );
}

#[test]
fn pinned_sort_mode_expires_with_ttl() {
    let manager = SessionManager::new(); // TTL = 300s
    let t0 = 1000;

    manager.bind_pinned_sort_mode("claude", "s1", Some(7), t0);
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 301),
        None
    );
}

#[test]
fn pin_sort_mode_preserves_existing_auto_binding() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_success("claude", "s1", 3, Some(5), t0);
    manager.bind_pinned_sort_mode("claude", "s1", Some(9), t0 + 5);

    // Auto-bound provider/sort_mode preserved; manual pin recorded separately.
    assert_eq!(manager.get_bound_provider("claude", "s1", t0 + 10), Some(3));
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 10),
        Some(Some(9))
    );
}

#[test]
fn pinned_sort_mode_surfaces_in_active_snapshot() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_success("claude", "s1", 3, Some(5), t0);
    manager.bind_pinned_sort_mode("claude", "s1", Some(9), t0 + 1);

    let rows = manager.list_active(t0 + 10, 50);
    let row = rows
        .iter()
        .find(|r| r.session_id == "s1")
        .expect("session row");
    assert_eq!(row.pinned_sort_mode_id, Some(Some(9)));
}

#[test]
fn clear_pinned_sort_mode_reverts_to_auto() {
    let manager = SessionManager::new();
    let t0 = 1000;

    manager.bind_pinned_sort_mode("claude", "s1", Some(9), t0);
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 5),
        Some(Some(9))
    );

    assert!(manager.clear_pinned_sort_mode("claude", "s1", t0 + 6));
    // After clearing, the session is unpinned again.
    assert_eq!(
        manager.get_bound_pinned_sort_mode_id("claude", "s1", t0 + 7),
        None
    );
}

#[test]
fn clear_pinned_sort_mode_no_binding_is_noop() {
    let manager = SessionManager::new();
    let t0 = 1000;

    assert!(!manager.clear_pinned_sort_mode("claude", "missing", t0));
    assert!(!manager.clear_pinned_sort_mode("", "s1", t0));
}
