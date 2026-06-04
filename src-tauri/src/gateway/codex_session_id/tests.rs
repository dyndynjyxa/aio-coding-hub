use super::*;

#[test]
fn prefers_prompt_cache_key_over_metadata_session_id() {
    let mut cache = CodexSessionIdCache::default();
    let now_unix = 123;
    let now_unix_ms = 123_000;
    let mut headers = HeaderMap::new();
    let mut body = serde_json::json!({
        "prompt_cache_key": "01234567-89ab-cdef-0123-456789abcdef",
        "metadata": { "session_id": "11111111-2222-3333-4444-555555555555" }
    });

    let result = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers,
        Some(&mut body),
    );

    assert!(result.applied);
    assert_eq!(result.source, "body_prompt_cache_key");
    assert_eq!(result.action, "completed_missing_fields");
    assert_eq!(
        result.session_id,
        "01234567-89ab-cdef-0123-456789abcdef".to_string()
    );
    assert_eq!(
        headers.get("session_id").unwrap().to_str().unwrap(),
        result.session_id
    );
    assert_eq!(
        headers.get("x-session-id").unwrap().to_str().unwrap(),
        result.session_id
    );
    assert_eq!(
        body.get("prompt_cache_key").unwrap().as_str().unwrap(),
        result.session_id
    );
}

#[test]
fn uses_metadata_session_id_when_prompt_cache_key_missing() {
    let mut cache = CodexSessionIdCache::default();
    let now_unix = 123;
    let now_unix_ms = 123_000;
    let mut headers = HeaderMap::new();
    let mut body = serde_json::json!({
        "metadata": { "session_id": "01234567-89ab-cdef-0123-456789abcdef" }
    });

    let result = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers,
        Some(&mut body),
    );

    assert!(result.applied);
    assert_eq!(result.source, "body_metadata_session_id");
    assert_eq!(result.action, "completed_missing_fields");
    assert_eq!(
        headers.get("session_id").unwrap().to_str().unwrap(),
        result.session_id
    );
    assert_eq!(
        body.get("prompt_cache_key").unwrap().as_str().unwrap(),
        result.session_id
    );
}

#[test]
fn uses_previous_response_id_when_other_sources_missing() {
    let mut cache = CodexSessionIdCache::default();
    let now_unix = 123;
    let now_unix_ms = 123_000;
    let mut headers = HeaderMap::new();
    let mut body = serde_json::json!({
        "previous_response_id": "resp_01234567-89ab-cdef-0123-456789abcdef"
    });

    let result = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers,
        Some(&mut body),
    );

    assert!(result.applied);
    assert_eq!(result.source, "body_previous_response_id");
    assert_eq!(result.action, "completed_missing_fields");
    assert_eq!(
        result.session_id,
        "codex_prev_resp_01234567-89ab-cdef-0123-456789abcdef".to_string()
    );
    assert_eq!(
        headers.get("session_id").unwrap().to_str().unwrap(),
        result.session_id
    );
    assert_eq!(
        body.get("prompt_cache_key").unwrap().as_str().unwrap(),
        result.session_id
    );
}

#[test]
fn fingerprint_cache_scopes_by_credential() {
    let mut cache = CodexSessionIdCache::default();
    let now_unix = 123;
    let now_unix_ms = 123_000;

    let mut headers1 = HeaderMap::new();
    headers1.insert(
        "authorization",
        HeaderValue::from_static("Bearer test_key_1"),
    );
    headers1.insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
    headers1.insert("user-agent", HeaderValue::from_static("ua"));
    let mut body1 = serde_json::json!({
        "input": [
            { "type": "message", "content": "hello" }
        ]
    });

    let result1 = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers1,
        Some(&mut body1),
    );

    let mut headers2 = HeaderMap::new();
    headers2.insert(
        "authorization",
        HeaderValue::from_static("Bearer test_key_2"),
    );
    headers2.insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
    headers2.insert("user-agent", HeaderValue::from_static("ua"));
    let mut body2 = serde_json::json!({
        "input": [
            { "type": "message", "content": "hello" }
        ]
    });

    let result2 = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers2,
        Some(&mut body2),
    );

    assert_ne!(result1.session_id, result2.session_id);
}

#[test]
fn fingerprint_cache_reuses_with_same_credential() {
    let mut cache = CodexSessionIdCache::default();
    let now_unix = 123;
    let now_unix_ms = 123_000;

    let mut headers1 = HeaderMap::new();
    headers1.insert(
        "authorization",
        HeaderValue::from_static("Bearer test_key_1"),
    );
    headers1.insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
    headers1.insert("user-agent", HeaderValue::from_static("ua"));
    let mut body1 = serde_json::json!({
        "input": [
            { "type": "message", "content": "hello" }
        ]
    });

    let result1 = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers1,
        Some(&mut body1),
    );
    assert_eq!(result1.action, "generated_uuid_v7");

    let mut headers2 = HeaderMap::new();
    headers2.insert(
        "authorization",
        HeaderValue::from_static("Bearer test_key_1"),
    );
    headers2.insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
    headers2.insert("user-agent", HeaderValue::from_static("ua"));
    let mut body2 = serde_json::json!({
        "input": [
            { "type": "message", "content": "hello" }
        ]
    });

    let result2 = complete_codex_session_identifiers(
        &mut cache,
        now_unix,
        now_unix_ms,
        &mut headers2,
        Some(&mut body2),
    );

    assert_eq!(result2.action, "reused_fingerprint_cache");
    assert_eq!(result1.session_id, result2.session_id);
}

#[test]
fn fingerprint_hash_samples_large_message_text_with_tail() {
    let headers = HeaderMap::new();
    let prefix = "a".repeat(FINGERPRINT_TEXT_SAMPLE_BYTES + 1024);
    let body1 = serde_json::json!({
        "input": [{ "type": "message", "content": format!("{prefix}tail-a") }]
    });
    let body2 = serde_json::json!({
        "input": [{ "type": "message", "content": format!("{prefix}tail-b") }]
    });

    assert_ne!(
        calculate_fingerprint_hash(&headers, Some(&body1)),
        calculate_fingerprint_hash(&headers, Some(&body2))
    );
}

#[test]
fn fingerprint_hash_bounds_content_part_scanning() {
    let headers = HeaderMap::new();
    let mut parts = Vec::new();
    for index in 0..FINGERPRINT_CONTENT_PARTS_MAX_ITEMS {
        parts.push(serde_json::json!({ "text": format!("part-{index};") }));
    }

    let mut parts_with_extra_a = parts.clone();
    parts_with_extra_a.push(serde_json::json!({ "text": "ignored-a" }));
    let mut parts_with_extra_b = parts;
    parts_with_extra_b.push(serde_json::json!({ "text": "ignored-b" }));

    let body1 = serde_json::json!({
        "input": [{ "type": "message", "content": parts_with_extra_a }]
    });
    let body2 = serde_json::json!({
        "input": [{ "type": "message", "content": parts_with_extra_b }]
    });

    assert_eq!(
        calculate_fingerprint_hash(&headers, Some(&body1)),
        calculate_fingerprint_hash(&headers, Some(&body2))
    );
}

fn cache_entry(index: usize, expires_at_unix: i64) -> CacheEntry {
    CacheEntry {
        session_id: format!("cached-session-{index:04}"),
        expires_at_unix,
    }
}

#[test]
fn prune_cache_removes_expired_entries_and_keeps_fresh_entries() {
    let mut cache = CodexSessionIdCache::default();
    cache.entries.insert("expired".into(), cache_entry(0, 100));
    cache.entries.insert("fresh".into(), cache_entry(1, 101));

    prune_cache(&mut cache, 100);

    assert!(!cache.entries.contains_key("expired"));
    assert!(cache.entries.contains_key("fresh"));
}

#[test]
fn prune_cache_trims_oldest_entries_without_clearing_cache() {
    let mut cache = CodexSessionIdCache::default();
    for index in 0..(MAX_CACHE_ENTRIES + 2) {
        cache.entries.insert(
            format!("fingerprint-{index}"),
            cache_entry(index, 1_000 + index as i64),
        );
    }

    prune_cache(&mut cache, 1);

    assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
    assert!(!cache.entries.contains_key("fingerprint-0"));
    assert!(!cache.entries.contains_key("fingerprint-1"));
    assert!(cache.entries.contains_key("fingerprint-2"));
    assert!(cache
        .entries
        .contains_key(&format!("fingerprint-{}", MAX_CACHE_ENTRIES + 1)));
}

#[test]
fn fingerprint_cache_evicts_oldest_entry_when_inserting_at_capacity() {
    let mut cache = CodexSessionIdCache::default();
    for index in 0..MAX_CACHE_ENTRIES {
        cache.entries.insert(
            format!("fingerprint-{index}"),
            cache_entry(index, 1_000 + index as i64),
        );
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer capacity_test_key"),
    );
    headers.insert("x-real-ip", HeaderValue::from_static("1.2.3.4"));
    headers.insert("user-agent", HeaderValue::from_static("ua"));
    let body = serde_json::json!({
        "input": [
            { "type": "message", "content": "capacity test" }
        ]
    });
    let fingerprint_hash = calculate_fingerprint_hash(&headers, Some(&body));
    assert!(!cache.entries.contains_key(&fingerprint_hash));

    let (session_id, _, _) =
        get_or_create_from_fingerprint(&mut cache, 10, 10_000, &headers, Some(&body));

    assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
    assert!(!cache.entries.contains_key("fingerprint-0"));
    assert!(cache.entries.contains_key("fingerprint-1"));
    assert_eq!(
        cache
            .entries
            .get(&fingerprint_hash)
            .map(|entry| entry.session_id.as_str()),
        Some(session_id.as_str())
    );
}
