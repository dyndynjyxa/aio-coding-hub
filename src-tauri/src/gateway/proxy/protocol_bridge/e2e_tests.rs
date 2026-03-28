//! End-to-end integration tests for the protocol bridge framework.
//!
//! These tests verify the full translation round-trip through the IR layer:
//!   Anthropic JSON → Inbound → IR → Outbound → OpenAI JSON
//!   OpenAI JSON → Outbound → IR → Inbound → Anthropic JSON

#[cfg(test)]
mod tests {
    use crate::gateway::proxy::protocol_bridge::{get_bridge, registry, BridgeContext};
    use serde_json::json;

    fn cx2cc_ctx() -> BridgeContext {
        BridgeContext {
            claude_models: crate::domain::providers::ClaudeModels::default(),
            cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
            requested_model: Some("claude-sonnet-4-20250514".into()),
            mapped_model: None,
            stream_requested: false,
            is_chatgpt_backend: false,
        }
    }

    // ── Registry ────────────────────────────────────────────────────────

    #[test]
    fn registry_returns_cx2cc_bridge() {
        let bridge = get_bridge("cx2cc");
        assert!(bridge.is_some());
        assert_eq!(bridge.unwrap().bridge_type, "cx2cc");
    }

    #[test]
    fn registry_returns_none_for_unknown_type() {
        assert!(get_bridge("nonexistent").is_none());
    }

    #[test]
    fn available_bridge_types_includes_cx2cc() {
        let types = registry::available_bridge_types();
        assert!(types.contains(&"cx2cc"));
    }

    // ── Request round-trip ──────────────────────────────────────────────

    #[test]
    fn e2e_simple_text_request() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let translated = bridge.translate_request(anthropic_req, &ctx).unwrap();

        // Model should be mapped from claude-sonnet to the runtime fallback.
        assert_eq!(
            translated.body.get("model").unwrap().as_str().unwrap(),
            "gpt-5.4"
        );
        // Path should be /v1/responses
        assert_eq!(translated.target_path, "/v1/responses");
        // System becomes instructions
        assert_eq!(
            translated
                .body
                .get("instructions")
                .unwrap()
                .as_str()
                .unwrap(),
            "You are helpful."
        );
        // max_tokens becomes max_output_tokens
        assert_eq!(
            translated
                .body
                .get("max_output_tokens")
                .unwrap()
                .as_u64()
                .unwrap(),
            1024
        );
        // Input should have the user message wrapped with role
        let input = translated.body.get("input").unwrap().as_array().unwrap();
        assert!(!input.is_empty());
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"][0]["type"], "input_text");
        assert_eq!(input[0]["content"][0]["text"], "Hello");
    }

    #[test]
    fn e2e_request_with_tools() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "What's the weather?"}
            ],
            "tools": [
                {
                    "name": "get_weather",
                    "description": "Get weather for a city",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "city": {"type": "string"}
                        },
                        "required": ["city"]
                    }
                }
            ],
            "tool_choice": {"type": "any"}
        });

        let translated = bridge.translate_request(anthropic_req, &ctx).unwrap();

        let tools = translated.body.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["name"], "get_weather");
        assert!(tools[0]["parameters"].is_object());

        // "any" → "required"
        assert_eq!(translated.body["tool_choice"], "required");
    }

    #[test]
    fn e2e_request_with_tool_use_and_tool_result() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "What's the weather?"},
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "call_123",
                            "name": "get_weather",
                            "input": {"city": "Tokyo"}
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "call_123",
                            "content": "Sunny, 25°C"
                        }
                    ]
                }
            ]
        });

        let translated = bridge.translate_request(anthropic_req, &ctx).unwrap();
        let input = translated.body.get("input").unwrap().as_array().unwrap();

        // Should have: role-wrapped text, function_call, function_call_output
        let types: Vec<&str> = input
            .iter()
            .filter_map(|item| {
                // Top-level items have "type", role-wrapped items don't
                item.get("type").and_then(|t| t.as_str()).or_else(|| {
                    // Check content inside role wrapper
                    item.get("content")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|b| b.get("type"))
                        .and_then(|t| t.as_str())
                })
            })
            .collect();
        assert!(types.contains(&"input_text"));
        assert!(types.contains(&"function_call"));
        assert!(types.contains(&"function_call_output"));
    }

    #[test]
    fn e2e_request_preserves_image_content() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "What's in this image?"},
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "iVBORw0KGgo="
                            }
                        }
                    ]
                }
            ]
        });

        let translated = bridge.translate_request(anthropic_req, &ctx).unwrap();
        let input = translated.body.get("input").unwrap().as_array().unwrap();

        let has_image = input.iter().any(|item| {
            // Check inside role-wrapped content
            item.get("content")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("input_image"))
                })
                .unwrap_or(false)
        });
        assert!(
            has_image,
            "image content should be preserved in translated request"
        );
    }

    #[test]
    fn e2e_request_drops_thinking_blocks() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {"type": "thinking", "thinking": "Let me think..."},
                        {"type": "text", "text": "Here's my answer"}
                    ]
                }
            ]
        });

        let translated = bridge.translate_request(anthropic_req, &ctx).unwrap();
        let input = translated.body.get("input").unwrap().as_array().unwrap();

        // Thinking should be dropped, only text preserved
        let types: Vec<&str> = input
            .iter()
            .flat_map(|item| {
                // Check top-level type or inside role wrapper
                let top = item.get("type").and_then(|t| t.as_str());
                let nested: Vec<&str> = item
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|b| b.get("type").and_then(|t| t.as_str()))
                            .collect()
                    })
                    .unwrap_or_default();
                top.into_iter().chain(nested)
            })
            .collect();
        assert!(!types.contains(&"thinking"));
        assert!(types.contains(&"output_text"));
    }

    // ── Response round-trip ─────────────────────────────────────────────

    #[test]
    fn e2e_simple_text_response() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_abc",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "Hello! How can I help?"}
                    ]
                }
            ],
            "usage": {
                "input_tokens": 15,
                "output_tokens": 8
            }
        });

        let anthropic = bridge.translate_response(openai_resp, &ctx).unwrap();

        assert_eq!(anthropic["type"], "message");
        assert_eq!(anthropic["role"], "assistant");
        // Model should be overridden to requested model
        assert_eq!(anthropic["model"], "claude-sonnet-4-20250514");
        assert_eq!(anthropic["stop_reason"], "end_turn");

        let content = anthropic["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello! How can I help?");

        assert_eq!(anthropic["usage"]["input_tokens"], 15);
        assert_eq!(anthropic["usage"]["output_tokens"], 8);
    }

    #[test]
    fn e2e_tool_use_response() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_tool",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_456",
                    "name": "get_weather",
                    "arguments": "{\"city\":\"Tokyo\"}"
                }
            ],
            "usage": {"input_tokens": 20, "output_tokens": 10}
        });

        let anthropic = bridge.translate_response(openai_resp, &ctx).unwrap();

        assert_eq!(anthropic["stop_reason"], "tool_use");
        let content = anthropic["content"].as_array().unwrap();
        assert!(content.iter().any(|c| c["type"] == "tool_use"));

        let tool_use = content.iter().find(|c| c["type"] == "tool_use").unwrap();
        assert_eq!(tool_use["name"], "get_weather");
        assert_eq!(tool_use["id"], "call_456");
        assert_eq!(tool_use["input"]["city"], "Tokyo");
    }

    #[test]
    fn e2e_reasoning_response_becomes_thinking() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_reason",
            "model": "o3",
            "status": "completed",
            "output": [
                {
                    "type": "reasoning",
                    "summary": [
                        {"type": "summary_text", "text": "I need to think about this..."}
                    ]
                },
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "The answer is 42."}
                    ]
                }
            ],
            "usage": {"input_tokens": 30, "output_tokens": 50}
        });

        let anthropic = bridge.translate_response(openai_resp, &ctx).unwrap();
        let content = anthropic["content"].as_array().unwrap();

        let has_thinking = content.iter().any(|c| c["type"] == "thinking");
        let has_text = content.iter().any(|c| c["type"] == "text");
        assert!(has_thinking, "reasoning should become thinking block");
        assert!(has_text, "message text should be present");
    }

    #[test]
    fn e2e_incomplete_response_maps_to_max_tokens() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_inc",
            "model": "gpt-4.1",
            "status": "incomplete",
            "incomplete_details": {"reason": "max_output_tokens"},
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "Partial..."}]
                }
            ],
            "usage": {"input_tokens": 10, "output_tokens": 4096}
        });

        let anthropic = bridge.translate_response(openai_resp, &ctx).unwrap();
        assert_eq!(anthropic["stop_reason"], "max_tokens");
    }

    // ── SSE synthesis (non-stream JSON → Anthropic SSE) ─────────────────

    #[test]
    fn e2e_response_to_sse_preserves_usage_and_model() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_sse",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "SSE test response"}
                    ]
                }
            ],
            "usage": {"input_tokens": 25, "output_tokens": 12}
        });

        let sse_bytes = bridge.translate_response_to_sse(openai_resp, &ctx).unwrap();
        let sse_text = String::from_utf8(sse_bytes.to_vec()).unwrap();

        // Should contain Anthropic SSE events
        assert!(sse_text.contains("event: message_start"));
        assert!(sse_text.contains("event: content_block_start"));
        assert!(sse_text.contains("event: content_block_delta"));
        assert!(sse_text.contains("event: content_block_stop"));
        assert!(sse_text.contains("event: message_delta"));
        assert!(sse_text.contains("event: message_stop"));

        // Model should be overridden
        assert!(sse_text.contains("claude-sonnet-4-20250514"));
        assert!(!sse_text.contains("gpt-4.1"));

        // Usage should be preserved (parseable by downstream)
        let usage = crate::usage::parse_usage_from_json_or_sse_bytes("claude", &sse_bytes);
        assert!(usage.is_some(), "usage should be extractable from SSE");
        let usage = usage.unwrap();
        assert_eq!(usage.metrics.input_tokens, Some(25));
        assert_eq!(usage.metrics.output_tokens, Some(12));
    }

    #[test]
    fn e2e_response_to_sse_with_tool_use() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_tool_sse",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_789",
                    "name": "search",
                    "arguments": "{\"query\":\"rust\"}"
                }
            ],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let sse_bytes = bridge.translate_response_to_sse(openai_resp, &ctx).unwrap();
        let sse_text = String::from_utf8(sse_bytes.to_vec()).unwrap();

        assert!(sse_text.contains("tool_use"));
        assert!(sse_text.contains("call_789"));
        assert!(sse_text.contains("search"));
        // stop_reason should be tool_use
        assert!(sse_text.contains("\"stop_reason\":\"tool_use\""));
    }

    // ── Full round-trip (Anthropic → OpenAI → Anthropic) ────────────────

    #[test]
    fn e2e_full_round_trip_text() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        // Step 1: Translate Anthropic request → OpenAI request
        let anthropic_req = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 512,
            "system": "Be concise.",
            "messages": [
                {"role": "user", "content": "Say hello"}
            ]
        });
        let translated_req = bridge.translate_request(anthropic_req, &ctx).unwrap();
        assert_eq!(translated_req.target_path, "/v1/responses");

        // Step 2: Simulate OpenAI response
        let openai_resp = json!({
            "id": "resp_round",
            "model": translated_req.body["model"],
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "Hello!"}
                    ]
                }
            ],
            "usage": {"input_tokens": 8, "output_tokens": 2}
        });

        // Step 3: Translate OpenAI response → Anthropic response
        let anthropic_resp = bridge.translate_response(openai_resp, &ctx).unwrap();

        // Verify it's a valid Anthropic response
        assert_eq!(anthropic_resp["type"], "message");
        assert_eq!(anthropic_resp["role"], "assistant");
        assert_eq!(anthropic_resp["model"], "claude-sonnet-4-20250514");
        assert_eq!(anthropic_resp["stop_reason"], "end_turn");
        assert_eq!(anthropic_resp["content"][0]["type"], "text");
        assert_eq!(anthropic_resp["content"][0]["text"], "Hello!");
        assert_eq!(anthropic_resp["usage"]["input_tokens"], 8);
        assert_eq!(anthropic_resp["usage"]["output_tokens"], 2);
    }

    // ── Model mapping ───────────────────────────────────────────────────

    #[test]
    fn e2e_model_mapping_opus() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = BridgeContext {
            requested_model: Some("claude-opus-4-6-20250515".into()),
            ..cx2cc_ctx()
        };

        let req = json!({
            "model": "claude-opus-4-6-20250515",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}]
        });

        let translated = bridge.translate_request(req, &ctx).unwrap();
        assert_eq!(translated.body["model"], "gpt-5.4");
    }

    #[test]
    fn e2e_model_mapping_haiku() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = BridgeContext {
            requested_model: Some("claude-haiku-4-5".into()),
            ..cx2cc_ctx()
        };

        let req = json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}]
        });

        let translated = bridge.translate_request(req, &ctx).unwrap();
        assert_eq!(translated.body["model"], "gpt-5.4");
    }

    // ── BridgeStream integration ────────────────────────────────────────

    #[test]
    fn bridge_stream_passthrough_when_inactive() {
        use crate::gateway::proxy::protocol_bridge::stream::BridgeStream;
        use axum::body::Bytes;
        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct MockStream(Vec<Bytes>);
        impl Stream for MockStream {
            type Item = Result<Bytes, reqwest::Error>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                if self.0.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(self.0.remove(0))))
                }
            }
        }
        impl Unpin for MockStream {}

        let data = Bytes::from("event: response.created\ndata: {}\n\n");
        let stream = BridgeStream::for_cx2cc(
            MockStream(vec![data.clone()]),
            false,
            None,
            crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
        );

        // When inactive, should pass through unchanged — verify via direct poll
        let waker = std::task::Waker::from(std::sync::Arc::new(NoopWaker));
        let mut cx = Context::from_waker(&waker);
        let mut stream = stream;
        match Pin::new(&mut stream).poll_next(&mut cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                assert_eq!(chunk, data, "passthrough should return data unchanged");
            }
            other => panic!("expected Ready(Some(Ok)), got {other:?}"),
        }
    }

    struct NoopWaker;
    impl std::task::Wake for NoopWaker {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    // ── Cache token preservation ────────────────────────────────────────

    #[test]
    fn e2e_response_preserves_cache_tokens() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_cache",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "cached response"}]
                }
            ],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 10,
                "input_tokens_details": {
                    "cached_tokens": 80
                }
            }
        });

        let anthropic = bridge.translate_response(openai_resp, &ctx).unwrap();
        assert_eq!(anthropic["usage"]["input_tokens"], 100);
        assert_eq!(anthropic["usage"]["output_tokens"], 10);
        assert_eq!(anthropic["usage"]["cache_read_input_tokens"], 80);
    }

    #[test]
    fn e2e_response_to_sse_preserves_cache_tokens_for_usage_tracker() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_cache_sse",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "cached response"}
                    ]
                }
            ],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 10,
                "input_tokens_details": {
                    "cached_tokens": 80
                }
            }
        });

        let sse_bytes = bridge.translate_response_to_sse(openai_resp, &ctx).unwrap();
        let usage = crate::usage::parse_usage_from_json_or_sse_bytes("claude", &sse_bytes)
            .expect("usage should be extractable from SSE");

        assert_eq!(usage.metrics.input_tokens, Some(100));
        assert_eq!(usage.metrics.output_tokens, Some(10));
        assert_eq!(usage.metrics.cache_read_input_tokens, Some(80));
    }

    #[test]
    fn e2e_response_to_sse_preserves_cache_creation_tokens_for_usage_tracker() {
        let bridge = get_bridge("cx2cc").unwrap();
        let ctx = cx2cc_ctx();

        let openai_resp = json!({
            "id": "resp_cache_creation_sse",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "output_text", "text": "cached response"}
                    ]
                }
            ],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 10,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 20,
                    "ephemeral_1h_input_tokens": 5
                }
            }
        });

        let sse_bytes = bridge.translate_response_to_sse(openai_resp, &ctx).unwrap();
        let usage = crate::usage::parse_usage_from_json_or_sse_bytes("claude", &sse_bytes)
            .expect("usage should be extractable from SSE");

        assert_eq!(usage.metrics.input_tokens, Some(100));
        assert_eq!(usage.metrics.output_tokens, Some(10));
        assert_eq!(usage.metrics.cache_creation_5m_input_tokens, Some(20));
        assert_eq!(usage.metrics.cache_creation_1h_input_tokens, Some(5));
        assert_eq!(usage.metrics.cache_creation_input_tokens, Some(25));
    }
}
