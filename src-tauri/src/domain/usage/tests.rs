use super::*;

const EXPECTED_OPENAI_CACHE_CREATION_ALIASES: [&str; 8] = [
    "/cache_creation_input_tokens",
    "/cache_write_input_tokens",
    "/cache_creation_tokens",
    "/cache_write_tokens",
    "/input_tokens_details/cache_creation_tokens",
    "/input_tokens_details/cache_write_tokens",
    "/prompt_tokens_details/cache_creation_tokens",
    "/prompt_tokens_details/cache_write_tokens",
];

fn openai_usage_with_alias(pointer: &str, value: Value) -> Value {
    let mut usage = serde_json::json!({
        "input_tokens": 1000,
        "output_tokens": 50,
        "total_tokens": 1050,
        "input_tokens_details": {},
        "prompt_tokens_details": {},
    });
    let segments = pointer
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [field] => {
            usage
                .as_object_mut()
                .expect("usage object")
                .insert((*field).to_string(), value);
        }
        [parent, field] => {
            usage
                .get_mut(*parent)
                .and_then(Value::as_object_mut)
                .expect("usage detail object")
                .insert((*field).to_string(), value);
        }
        _ => panic!("unsupported test pointer: {pointer}"),
    }
    usage
}

#[test]
fn parse_openai_chatcompletions_usage() {
    let body =
        br#"{"id":"x","usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;
    let extract = parse_usage_from_json_bytes("codex", body).expect("should parse usage");
    assert_eq!(extract.metrics.input_tokens, Some(10));
    assert_eq!(extract.metrics.output_tokens, Some(5));
    assert_eq!(extract.metrics.total_tokens, Some(15));
    assert_eq!(extract.metrics.cache_read_input_tokens, None);
}

#[test]
fn parse_openai_responses_usage_with_cached_tokens() {
    let body = br#"{"usage":{"input_tokens":11,"output_tokens":7,"total_tokens":18,"input_tokens_details":{"cached_tokens":3}}}"#;
    let extract = parse_usage_from_json_bytes("codex", body).expect("should parse usage");
    assert_eq!(extract.metrics.input_tokens, Some(11));
    assert_eq!(extract.metrics.output_tokens, Some(7));
    assert_eq!(extract.metrics.total_tokens, Some(18));
    assert_eq!(extract.metrics.cache_read_input_tokens, Some(3));
    assert_eq!(extract.metrics.cache_creation_input_tokens, None);
}

#[test]
fn parse_openai_cache_creation_aliases_in_json_and_sse() {
    for pointer in EXPECTED_OPENAI_CACHE_CREATION_ALIASES {
        for expected in [200, 0] {
            let usage = openai_usage_with_alias(pointer, serde_json::json!(expected));
            let json_body = serde_json::to_vec(&serde_json::json!({ "usage": usage.clone() }))
                .expect("serialize JSON response");
            let json_extract = parse_usage_from_json_bytes("codex", &json_body)
                .unwrap_or_else(|| panic!("JSON alias should parse: {pointer}"));
            assert_eq!(
                json_extract.metrics.cache_creation_input_tokens,
                Some(expected),
                "JSON alias={pointer}"
            );
            assert_canonical_cache_creation_usage(&json_extract.usage_json, expected, pointer);

            let sse_payload = serde_json::json!({
                "type": "response.completed",
                "response": { "usage": usage },
            });
            let sse = format!("data: {sse_payload}\n\n");
            let sse_extract = parse_usage_from_json_or_sse_bytes("codex", sse.as_bytes())
                .unwrap_or_else(|| panic!("SSE alias should parse: {pointer}"));
            assert_eq!(
                sse_extract.metrics.cache_creation_input_tokens,
                Some(expected),
                "SSE alias={pointer}"
            );
            assert_canonical_cache_creation_usage(&sse_extract.usage_json, expected, pointer);
        }
    }
}

fn assert_canonical_cache_creation_usage(usage_json: &str, expected: i64, pointer: &str) {
    let normalized: Value = serde_json::from_str(usage_json).expect("normalized usage JSON");
    assert_eq!(
        normalized.get("cache_creation_input_tokens"),
        Some(&serde_json::json!(expected)),
        "canonical value alias={pointer}"
    );
    for alias in [
        "cache_write_input_tokens",
        "cache_creation_tokens",
        "cache_write_tokens",
        "input_tokens_details",
        "prompt_tokens_details",
    ] {
        assert!(
            normalized.get(alias).is_none(),
            "alias leaked into canonical JSON: alias={pointer} key={alias}"
        );
    }
}

#[test]
fn openai_cache_creation_alias_selection_preserves_explicit_zero() {
    let mut usage = openai_usage_with_alias("/cache_creation_input_tokens", serde_json::json!(0));
    usage["input_tokens_details"]["cache_write_tokens"] = serde_json::json!(200);
    assert_eq!(
        extract_openai_cache_creation_input_tokens(&usage),
        Some(200),
        "a leading zero must not mask a later positive value"
    );

    usage["cache_creation_input_tokens"] = serde_json::json!(100);
    assert_eq!(
        extract_openai_cache_creation_input_tokens(&usage),
        Some(100),
        "the first positive alias wins"
    );

    let mut zero_usage = openai_usage_with_alias(
        "/prompt_tokens_details/cache_write_tokens",
        serde_json::json!(0),
    );
    zero_usage["cache_creation_input_tokens"] = serde_json::json!(0);
    let body = serde_json::to_vec(&serde_json::json!({ "usage": zero_usage.clone() }))
        .expect("serialize zero usage");
    let extract = parse_usage_from_json_bytes("codex", &body).expect("parse explicit zero");
    assert_eq!(extract.metrics.cache_creation_input_tokens, Some(0));
    let normalized: Value = serde_json::from_str(&extract.usage_json).expect("normalized usage");
    assert_eq!(
        normalized.get("cache_creation_input_tokens"),
        Some(&serde_json::json!(0))
    );
    for alias in [
        "cache_write_input_tokens",
        "cache_creation_tokens",
        "cache_write_tokens",
        "input_tokens_details",
        "prompt_tokens_details",
    ] {
        assert!(
            normalized.get(alias).is_none(),
            "alias leaked into canonical JSON: {alias}"
        );
    }

    let sse_payload = serde_json::json!({
        "type": "response.completed",
        "response": { "usage": zero_usage },
    });
    let sse = format!("data: {sse_payload}\n\n");
    let sse_extract =
        parse_usage_from_json_or_sse_bytes("codex", sse.as_bytes()).expect("parse SSE zero");
    assert_eq!(sse_extract.metrics.cache_creation_input_tokens, Some(0));
    assert_eq!(
        serde_json::from_str::<Value>(&sse_extract.usage_json)
            .expect("normalized SSE usage")
            .get("cache_creation_input_tokens"),
        Some(&serde_json::json!(0))
    );
}

#[test]
fn openai_cache_creation_aliases_ignore_invalid_values() {
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!("2"),
        serde_json::json!(2.5),
        serde_json::json!(true),
        serde_json::json!({ "tokens": 2 }),
        serde_json::json!([2]),
        serde_json::json!(u64::MAX),
    ] {
        let usage = openai_usage_with_alias("/cache_write_tokens", invalid.clone());
        assert_eq!(
            extract_openai_cache_creation_input_tokens(&usage),
            None,
            "invalid alias value should be ignored: {invalid}"
        );
    }

    let mut usage = openai_usage_with_alias("/cache_creation_input_tokens", serde_json::json!(-1));
    usage["prompt_tokens_details"]["cache_write_tokens"] = serde_json::json!(200);
    assert_eq!(
        extract_openai_cache_creation_input_tokens(&usage),
        Some(200)
    );
}

#[test]
fn new_openai_cache_creation_aliases_are_protocol_scoped() {
    let usage = openai_usage_with_alias("/cache_write_tokens", serde_json::json!(200));
    let body = serde_json::to_vec(&serde_json::json!({ "usage": usage }))
        .expect("serialize protocol fixture");

    assert_eq!(
        parse_usage_from_json_bytes("codex", &body)
            .expect("codex usage")
            .metrics
            .cache_creation_input_tokens,
        Some(200)
    );
    for cli_key in ["gemini", "claude", "unknown"] {
        assert_eq!(
            parse_usage_from_json_bytes(cli_key, &body)
                .expect("base usage should still parse")
                .metrics
                .cache_creation_input_tokens,
            None,
            "cli_key={cli_key}"
        );
    }

    let claude_body = br#"{"usage":{"input_tokens":1,"cache_creation_input_tokens":0}}"#;
    assert_eq!(
        parse_usage_from_json_bytes("claude", claude_body)
            .expect("claude canonical usage")
            .metrics
            .cache_creation_input_tokens,
        Some(0)
    );
}

#[test]
fn parse_gemini_usage_metadata() {
    let body = br#"{"usageMetadata":{"promptTokenCount":8,"candidatesTokenCount":9,"thoughtsTokenCount":2,"totalTokenCount":19,"cachedContentTokenCount":4}}"#;
    let extract = parse_usage_from_json_bytes("gemini", body).expect("should parse usage");
    assert_eq!(extract.metrics.input_tokens, Some(8));
    assert_eq!(extract.metrics.output_tokens, Some(11));
    assert_eq!(extract.metrics.total_tokens, Some(19));
    assert_eq!(extract.metrics.cache_read_input_tokens, Some(4));
}

#[test]
fn parse_claude_sse_merge_message_start_and_delta() {
    let sse = b"event: message_start\n\
            data: {\"message\":{\"model\":\"claude-haiku-4-5-20251001\",\"usage\":{\"cache_creation\":{\"ephemeral_5m_input_tokens\":20,\"ephemeral_1h_input_tokens\":5},\"cache_read_input_tokens\":4}}}\n\
            \n\
            event: message_delta\n\
            data: {\"delta\":{\"usage\":{\"input_tokens\":30,\"output_tokens\":10,\"total_tokens\":40}}}\n\
            \n";

    let mut tracker = SseUsageTracker::new("claude");
    tracker.ingest_chunk(&sse[..20]);
    tracker.ingest_chunk(&sse[20..]);
    let extract = tracker.finalize().expect("should parse usage");

    assert_eq!(
        tracker.best_effort_model().as_deref(),
        Some("claude-haiku-4-5-20251001")
    );
    assert_eq!(extract.metrics.input_tokens, Some(30));
    assert_eq!(extract.metrics.output_tokens, Some(10));
    assert_eq!(extract.metrics.total_tokens, Some(40));
    assert_eq!(extract.metrics.cache_read_input_tokens, Some(4));
    assert_eq!(extract.metrics.cache_creation_5m_input_tokens, Some(20));
    assert_eq!(extract.metrics.cache_creation_1h_input_tokens, Some(5));
    assert_eq!(extract.metrics.cache_creation_input_tokens, Some(25));
}

#[test]
fn parse_model_top_level() {
    let body = br#"{"model":"claude-opus-4-5-20251101"}"#;
    assert_eq!(
        parse_model_from_json_bytes(body).as_deref(),
        Some("claude-opus-4-5-20251101")
    );
}

#[test]
fn parse_model_nested_message() {
    let body = br#"{"message":{"model":"claude-haiku-4-5-20251001"}}"#;
    assert_eq!(
        parse_model_from_json_bytes(body).as_deref(),
        Some("claude-haiku-4-5-20251001")
    );
}

#[test]
fn parse_generic_sse_usage_without_event_name() {
    let sse =
        b"data: {\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n";
    let mut tracker = SseUsageTracker::new("codex");
    tracker.ingest_chunk(sse);
    let extract = tracker.finalize().expect("should parse usage");
    assert_eq!(extract.metrics.input_tokens, Some(1));
    assert_eq!(extract.metrics.output_tokens, Some(2));
    assert_eq!(extract.metrics.total_tokens, Some(3));
}

#[test]
fn parse_sse_done_marker_marks_completion_seen() {
    let sse = b"data: [DONE]\n\n";
    let mut tracker = SseUsageTracker::new("codex");
    tracker.ingest_chunk(sse);
    assert!(tracker.completion_seen());
    assert!(tracker.finalize().is_none());
}

#[test]
fn parse_claude_message_stop_type_marks_completion_seen() {
    let sse = b"data: {\"type\":\"message_stop\"}\n\n";
    let mut tracker = SseUsageTracker::new("claude");
    tracker.ingest_chunk(sse);
    tracker.finalize();
    assert!(tracker.completion_seen());
}

#[test]
fn parse_codex_response_completed_marks_completion_seen() {
    let sse = b"data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n";
    let mut tracker = SseUsageTracker::new("codex");
    tracker.ingest_chunk(sse);
    let extract = tracker.finalize().expect("should parse usage");
    assert!(tracker.completion_seen());
    assert_eq!(extract.metrics.input_tokens, Some(1));
    assert_eq!(extract.metrics.output_tokens, Some(2));
    assert_eq!(extract.metrics.total_tokens, Some(3));
}

#[test]
fn parse_openai_chat_finish_reason_marks_completion_seen() {
    let sse = b"data: {\"choices\":[{\"finish_reason\":\"stop\"}]}\n\n";
    let mut tracker = SseUsageTracker::new("codex");
    tracker.ingest_chunk(sse);
    tracker.finalize();
    assert!(tracker.completion_seen());
}

#[test]
fn parse_gemini_finish_reason_marks_completion_seen() {
    let sse = b"data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n";
    let mut tracker = SseUsageTracker::new("gemini");
    tracker.ingest_chunk(sse);
    tracker.finalize();
    assert!(tracker.completion_seen());
}

#[test]
fn parse_sse_error_event_marks_terminal_error_seen() {
    let sse = b"event: error\ndata: {\"error\":{\"message\":\"upstream failed\"}}\n\n";
    let mut tracker = SseUsageTracker::new("claude");
    tracker.ingest_chunk(sse);
    assert!(tracker.terminal_error_seen());
}

#[test]
fn parse_response_error_type_marks_terminal_error_seen() {
    let sse = b"data: {\"type\":\"response.error\",\"error\":{\"message\":\"broken\"}}\n\n";
    let mut tracker = SseUsageTracker::new("codex");
    tracker.ingest_chunk(sse);
    assert!(tracker.terminal_error_seen());
}

#[test]
fn sse_usage_tracker_drops_oversized_pending_line() {
    let mut tracker = SseUsageTracker::new("codex");
    let oversized = vec![b'a'; MAX_SSE_USAGE_TRACKER_PENDING_BYTES + 1];

    tracker.ingest_chunk(&oversized);

    assert!(tracker.buffer.is_empty());
    assert!(tracker.current_event.is_empty());
    assert!(tracker.current_data.is_empty());
    assert!(tracker.finalize().is_none());
}

#[test]
fn sse_usage_tracker_recovers_after_oversized_pending_line() {
    let mut tracker = SseUsageTracker::new("codex");
    let oversized = vec![b'a'; MAX_SSE_USAGE_TRACKER_PENDING_BYTES + 1];

    tracker.ingest_chunk(&oversized);
    tracker.ingest_chunk(b"\n\n");
    tracker.ingest_chunk(
        b"data: {\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
    );
    let extract = tracker.finalize().expect("should parse later valid usage");

    assert_eq!(extract.metrics.input_tokens, Some(1));
    assert_eq!(extract.metrics.output_tokens, Some(2));
    assert_eq!(extract.metrics.total_tokens, Some(3));
}

#[test]
fn sse_usage_tracker_drops_oversized_event_data() {
    let mut tracker = SseUsageTracker::new("codex");
    let half = vec![b'a'; (MAX_SSE_USAGE_TRACKER_PENDING_BYTES / 2) + 1];

    tracker.ingest_chunk(b"data: ");
    tracker.ingest_chunk(&half);
    tracker.ingest_chunk(b"\n");
    tracker.ingest_chunk(b"data: ");
    tracker.ingest_chunk(&half);
    tracker.ingest_chunk(b"\n\n");

    assert!(tracker.buffer.is_empty());
    assert!(tracker.current_event.is_empty());
    assert!(tracker.current_data.is_empty());
    assert!(tracker.finalize().is_none());
}
