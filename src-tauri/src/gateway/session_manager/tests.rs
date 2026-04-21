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
