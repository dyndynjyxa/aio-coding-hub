//! Inbound adapter for the Anthropic Messages API protocol.
//!
//! Converts between:
//! - Client Anthropic JSON request <-> IR (`request_to_ir`)
//! - IR <-> Client Anthropic JSON response (`ir_to_response`)
//! - IR stream chunk -> Anthropic SSE bytes (`ir_chunk_to_sse`)

use super::super::ir::*;
use super::super::traits::*;
use crate::gateway::proxy::cx2cc::settings::Cx2ccSettings;
use axum::body::Bytes;
use serde_json::{json, Value};

/// Inbound adapter for the Anthropic Messages API.
pub(crate) struct AnthropicMessagesInbound;

impl Inbound for AnthropicMessagesInbound {
    fn protocol(&self) -> &'static str {
        "anthropic_messages"
    }

    fn request_to_ir(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<InternalRequest, BridgeError> {
        parse_request(body, &ctx.cx2cc_settings)
    }

    fn ir_to_response(
        &self,
        ir: &InternalResponse,
        ctx: &BridgeContext,
    ) -> Result<Value, BridgeError> {
        build_response(ir, ctx)
    }

    fn ir_chunk_to_sse(
        &self,
        chunk: &IRStreamChunk,
        ctx: &BridgeContext,
    ) -> Result<Vec<Bytes>, BridgeError> {
        render_chunk(chunk, ctx)
    }
}

// ---------------------------------------------------------------------------
// request_to_ir
// ---------------------------------------------------------------------------

fn parse_request(body: Value, settings: &Cx2ccSettings) -> Result<InternalRequest, BridgeError> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();

    let system = parse_system(&body);
    let messages = parse_messages(&body)?;
    let tools = parse_tools(&body, settings.filter_batch_tool);
    let tool_choice = parse_tool_choice(&body);

    let max_tokens = body.get("max_tokens").and_then(|v| v.as_u64());
    let temperature = body.get("temperature").and_then(|v| v.as_f64());
    let top_p = body.get("top_p").and_then(|v| v.as_f64());
    let stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let stop_sequences = body
        .get("stop_sequences")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    Ok(InternalRequest {
        model,
        messages,
        system,
        tools,
        tool_choice,
        max_tokens,
        temperature,
        top_p,
        stop_sequences,
        stream,
        metadata: IRMetadata::default(),
    })
}

fn parse_system(body: &Value) -> Option<String> {
    match body.get("system") {
        Some(Value::String(text)) => {
            if text.is_empty() {
                None
            } else {
                Some(text.clone())
            }
        }
        Some(Value::Array(arr)) => {
            let joined: String = arr
                .iter()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    }
}

fn parse_messages(body: &Value) -> Result<Vec<IRMessage>, BridgeError> {
    let msgs = body
        .get("messages")
        .and_then(|m| m.as_array())
        .ok_or_else(|| BridgeError::TransformFailed("missing or invalid messages".into()))?;

    let mut result = Vec::with_capacity(msgs.len());
    for msg in msgs {
        let role = match msg.get("role").and_then(|r| r.as_str()).unwrap_or("user") {
            "assistant" => IRRole::Assistant,
            _ => IRRole::User,
        };

        let content = parse_content_blocks(msg.get("content"))?;
        result.push(IRMessage { role, content });
    }
    Ok(result)
}

fn parse_content_blocks(content: Option<&Value>) -> Result<Vec<IRContentBlock>, BridgeError> {
    match content {
        Some(Value::String(text)) => Ok(vec![IRContentBlock::Text { text: text.clone() }]),
        Some(Value::Array(blocks)) => {
            let mut result = Vec::with_capacity(blocks.len());
            for block in blocks {
                if let Some(b) = parse_single_block(block)? {
                    result.push(b);
                }
            }
            Ok(result)
        }
        _ => Ok(Vec::new()),
    }
}

fn parse_single_block(block: &Value) -> Result<Option<IRContentBlock>, BridgeError> {
    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match block_type {
        "text" => {
            let text = block
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            Ok(Some(IRContentBlock::Text { text }))
        }

        "image" => {
            let source = block
                .get("source")
                .ok_or_else(|| BridgeError::TransformFailed("image block missing source".into()))?;
            let media_type = source
                .get("media_type")
                .and_then(|m| m.as_str())
                .unwrap_or("image/png")
                .to_string();
            let data = source
                .get("data")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            Ok(Some(IRContentBlock::Image { media_type, data }))
        }

        "tool_use" => {
            let id = block
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let name = block
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let input = block.get("input").cloned().unwrap_or(json!({}));
            Ok(Some(IRContentBlock::ToolUse { id, name, input }))
        }

        "tool_result" => {
            let tool_use_id = block
                .get("tool_use_id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let content = match block.get("content") {
                Some(Value::String(s)) => s.clone(),
                Some(v) => serde_json::to_string(v).unwrap_or_default(),
                None => String::new(),
            };
            let is_error = block
                .get("is_error")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(Some(IRContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            }))
        }

        "thinking" => {
            let thinking = block
                .get("thinking")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            Ok(Some(IRContentBlock::Thinking { thinking }))
        }

        // Unknown block types (including cache_control-only blocks) are silently skipped.
        _ => Ok(None),
    }
}

fn parse_tools(body: &Value, filter_batch_tool: bool) -> Vec<IRToolDefinition> {
    let Some(tools) = body.get("tools").and_then(|t| t.as_array()) else {
        return Vec::new();
    };

    tools
        .iter()
        .filter(|t| {
            !filter_batch_tool || t.get("type").and_then(|v| v.as_str()) != Some("BatchTool")
        })
        .map(|t| IRToolDefinition {
            name: t
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
            description: t
                .get("description")
                .and_then(|d| d.as_str())
                .map(str::to_string),
            parameters: t.get("input_schema").cloned().unwrap_or(json!({})),
        })
        .collect()
}

fn parse_tool_choice(body: &Value) -> Option<IRToolChoice> {
    let v = body.get("tool_choice")?;

    match v {
        Value::String(s) => match s.as_str() {
            "auto" => Some(IRToolChoice::Auto),
            "any" => Some(IRToolChoice::Required),
            "none" => Some(IRToolChoice::None),
            _ => None,
        },
        Value::Object(obj) => match obj.get("type").and_then(|t| t.as_str()) {
            Some("auto") => Some(IRToolChoice::Auto),
            Some("any") => Some(IRToolChoice::Required),
            Some("none") => Some(IRToolChoice::None),
            Some("tool") => {
                let name = obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(IRToolChoice::Specific { name })
            }
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ir_to_response
// ---------------------------------------------------------------------------

fn build_response(ir: &InternalResponse, ctx: &BridgeContext) -> Result<Value, BridgeError> {
    let model = ctx
        .requested_model
        .as_deref()
        .filter(|m| !m.is_empty())
        .unwrap_or(&ir.model);

    let content: Vec<Value> = ir
        .content
        .iter()
        .filter_map(|block| match block {
            IRContentBlock::Text { text } => Some(json!({"type": "text", "text": text})),
            IRContentBlock::ToolUse { id, name, input } => {
                Some(json!({"type": "tool_use", "id": id, "name": name, "input": input}))
            }
            IRContentBlock::Thinking { thinking } => {
                Some(json!({"type": "thinking", "thinking": thinking}))
            }
            _ => None,
        })
        .collect();

    let stop_reason = match &ir.stop_reason {
        IRStopReason::EndTurn => "end_turn",
        IRStopReason::ToolUse => "tool_use",
        IRStopReason::MaxTokens => "max_tokens",
        IRStopReason::StopSequence => "stop_sequence",
        IRStopReason::Unknown(s) => s.as_str(),
    };

    let mut usage = json!({
        "input_tokens": ir.usage.input_tokens,
        "output_tokens": ir.usage.output_tokens,
    });

    if let Some(v) = ir.usage.cache_creation_input_tokens {
        usage["cache_creation_input_tokens"] = json!(v);
    }
    if let Some(v) = ir.usage.cache_creation_5m_input_tokens {
        usage["cache_creation_5m_input_tokens"] = json!(v);
    }
    if let Some(v) = ir.usage.cache_creation_1h_input_tokens {
        usage["cache_creation_1h_input_tokens"] = json!(v);
    }
    if let Some(v) = ir.usage.cache_read_input_tokens {
        usage["cache_read_input_tokens"] = json!(v);
    }

    Ok(json!({
        "id": ir.id,
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": model,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": usage,
    }))
}

// ---------------------------------------------------------------------------
// ir_chunk_to_sse
// ---------------------------------------------------------------------------

fn sse_frame(event_type: &str, payload: Value) -> Bytes {
    let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    Bytes::from(format!("event: {event_type}\ndata: {data}\n\n"))
}

fn render_chunk(chunk: &IRStreamChunk, ctx: &BridgeContext) -> Result<Vec<Bytes>, BridgeError> {
    let frames = match chunk {
        IRStreamChunk::MessageStart {
            id,
            model,
            initial_usage,
        } => {
            let effective_model = ctx
                .requested_model
                .as_deref()
                .filter(|m| !m.is_empty())
                .unwrap_or(model);

            let input_tokens = initial_usage.as_ref().map_or(0, |u| u.input_tokens);
            let cache_creation = initial_usage
                .as_ref()
                .and_then(|u| u.cache_creation_input_tokens);
            let cache_creation_5m = initial_usage
                .as_ref()
                .and_then(|u| u.cache_creation_5m_input_tokens);
            let cache_creation_1h = initial_usage
                .as_ref()
                .and_then(|u| u.cache_creation_1h_input_tokens);
            let cache_read = initial_usage
                .as_ref()
                .and_then(|u| u.cache_read_input_tokens);

            let mut usage_json = json!({
                "input_tokens": input_tokens,
                "output_tokens": 0
            });
            if let Some(v) = cache_creation {
                usage_json["cache_creation_input_tokens"] = json!(v);
            }
            if let Some(v) = cache_creation_5m {
                usage_json["cache_creation_5m_input_tokens"] = json!(v);
            }
            if let Some(v) = cache_creation_1h {
                usage_json["cache_creation_1h_input_tokens"] = json!(v);
            }
            if let Some(v) = cache_read {
                usage_json["cache_read_input_tokens"] = json!(v);
            }

            let msg_start = sse_frame(
                "message_start",
                json!({
                    "type": "message_start",
                    "message": {
                        "id": id,
                        "type": "message",
                        "role": "assistant",
                        "content": [],
                        "model": effective_model,
                        "stop_reason": null,
                        "stop_sequence": null,
                        "usage": usage_json
                    }
                }),
            );

            let ping = sse_frame("ping", json!({"type": "ping"}));
            vec![msg_start, ping]
        }

        IRStreamChunk::ContentBlockStart { index, block_type } => {
            let content_block = match block_type {
                IRBlockType::Text => json!({"type": "text", "text": ""}),
                IRBlockType::ToolUse { id, name } => {
                    json!({"type": "tool_use", "id": id, "name": name, "input": {}})
                }
                IRBlockType::Thinking => json!({"type": "thinking", "thinking": ""}),
            };

            vec![sse_frame(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": content_block,
                }),
            )]
        }

        IRStreamChunk::ContentBlockDelta { index, delta } => {
            let delta_payload = match delta {
                IRDelta::TextDelta { text } => {
                    json!({"type": "text_delta", "text": text})
                }
                IRDelta::InputJsonDelta { partial_json } => {
                    json!({"type": "input_json_delta", "partial_json": partial_json})
                }
                IRDelta::ThinkingDelta { thinking } => {
                    json!({"type": "thinking_delta", "thinking": thinking})
                }
            };

            vec![sse_frame(
                "content_block_delta",
                json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": delta_payload,
                }),
            )]
        }

        IRStreamChunk::ContentBlockStop { index } => {
            vec![sse_frame(
                "content_block_stop",
                json!({"type": "content_block_stop", "index": index}),
            )]
        }

        IRStreamChunk::MessageDelta { stop_reason, usage } => {
            let sr = match stop_reason {
                IRStopReason::EndTurn => "end_turn",
                IRStopReason::ToolUse => "tool_use",
                IRStopReason::MaxTokens => "max_tokens",
                IRStopReason::StopSequence => "stop_sequence",
                IRStopReason::Unknown(s) => s.as_str(),
            };

            let mut usage_json = json!({
                "output_tokens": usage.output_tokens,
            });
            if usage.input_tokens > 0 {
                usage_json["input_tokens"] = json!(usage.input_tokens);
            }
            if let Some(v) = usage.cache_creation_input_tokens {
                usage_json["cache_creation_input_tokens"] = json!(v);
            }
            if let Some(v) = usage.cache_creation_5m_input_tokens {
                usage_json["cache_creation_5m_input_tokens"] = json!(v);
            }
            if let Some(v) = usage.cache_creation_1h_input_tokens {
                usage_json["cache_creation_1h_input_tokens"] = json!(v);
            }
            if let Some(v) = usage.cache_read_input_tokens {
                usage_json["cache_read_input_tokens"] = json!(v);
            }

            vec![sse_frame(
                "message_delta",
                json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": sr,
                        "stop_sequence": null,
                    },
                    "usage": usage_json
                }),
            )]
        }

        IRStreamChunk::MessageStop => {
            vec![sse_frame("message_stop", json!({"type": "message_stop"}))]
        }

        IRStreamChunk::Ping => {
            vec![sse_frame("ping", json!({"type": "ping"}))]
        }
    };

    Ok(frames)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ──────────────────────────────────────────────────────────

    fn default_settings() -> Cx2ccSettings {
        Cx2ccSettings::default()
    }

    fn default_ctx() -> BridgeContext {
        BridgeContext {
            claude_models: Default::default(),
            cx2cc_settings: default_settings(),
            requested_model: None,
            mapped_model: None,
            stream_requested: false,
            is_chatgpt_backend: false,
        }
    }

    fn ctx_with_model(model: &str) -> BridgeContext {
        BridgeContext {
            requested_model: Some(model.to_string()),
            ..default_ctx()
        }
    }

    fn parse_sse_frames(frames: &[Bytes]) -> Vec<(String, Value)> {
        frames
            .iter()
            .filter_map(|b| {
                let text = std::str::from_utf8(b).ok()?;
                let mut event = None;
                let mut data = None;
                for line in text.lines() {
                    if let Some(rest) = line.strip_prefix("event: ") {
                        event = Some(rest.to_string());
                    } else if let Some(rest) = line.strip_prefix("data: ") {
                        data = serde_json::from_str(rest).ok();
                    }
                }
                Some((event?, data?))
            })
            .collect()
    }

    // ── request_to_ir ───────────────────────────────────────────────────

    #[test]
    fn request_simple_text_message() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.model, "claude-sonnet-4-20250514");
        assert_eq!(ir.max_tokens, Some(1024));
        assert_eq!(ir.messages.len(), 1);
        assert_eq!(ir.messages[0].role, IRRole::User);
        assert!(
            matches!(&ir.messages[0].content[0], IRContentBlock::Text { text } if text == "Hello")
        );
    }

    #[test]
    fn request_system_string() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "Be helpful.",
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.system.as_deref(), Some("Be helpful."));
    }

    #[test]
    fn request_system_array_joined() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "Part 1"},
                {"type": "text", "text": "Part 2"}
            ],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.system.as_deref(), Some("Part 1\n\nPart 2"));
    }

    #[test]
    fn request_content_blocks_array() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Look at this"},
                    {"type": "image", "source": {"media_type": "image/png", "data": "abc123"}}
                ]
            }]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.messages[0].content.len(), 2);
        assert!(
            matches!(&ir.messages[0].content[0], IRContentBlock::Text { text } if text == "Look at this")
        );
        assert!(
            matches!(&ir.messages[0].content[1], IRContentBlock::Image { media_type, data }
            if media_type == "image/png" && data == "abc123")
        );
    }

    #[test]
    fn request_tool_use_and_tool_result() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{"type": "tool_use", "id": "call_1", "name": "get_weather", "input": {"city": "Tokyo"}}]
                },
                {
                    "role": "user",
                    "content": [{"type": "tool_result", "tool_use_id": "call_1", "content": "Sunny"}]
                }
            ]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.messages[0].content[0], IRContentBlock::ToolUse { id, name, input }
            if id == "call_1" && name == "get_weather" && input["city"] == "Tokyo")
        );
        assert!(
            matches!(&ir.messages[1].content[0], IRContentBlock::ToolResult { tool_use_id, content, .. }
            if tool_use_id == "call_1" && content == "Sunny")
        );
    }

    #[test]
    fn request_thinking_block() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "Let me think..."},
                    {"type": "text", "text": "Answer"}
                ]
            }]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.messages[0].content[0], IRContentBlock::Thinking { thinking } if thinking == "Let me think...")
        );
        assert!(
            matches!(&ir.messages[0].content[1], IRContentBlock::Text { text } if text == "Answer")
        );
    }

    #[test]
    fn request_tools_parsed_and_batch_tool_filtered() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [
                {"type": "BatchTool", "name": "batch", "input_schema": {}},
                {
                    "name": "get_weather",
                    "description": "Get weather",
                    "input_schema": {"type": "object", "properties": {"city": {"type": "string"}}}
                }
            ]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.tools.len(), 1);
        assert_eq!(ir.tools[0].name, "get_weather");
        assert_eq!(ir.tools[0].description.as_deref(), Some("Get weather"));
    }

    #[test]
    fn request_tools_preserve_batch_tool_when_filter_disabled() {
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [
                {"type": "BatchTool", "name": "batch", "input_schema": {}},
                {
                    "name": "get_weather",
                    "description": "Get weather",
                    "input_schema": {"type": "object", "properties": {"city": {"type": "string"}}}
                }
            ]
        });
        let mut settings = default_settings();
        settings.filter_batch_tool = false;

        let ir = parse_request(body, &settings).unwrap();
        assert_eq!(ir.tools.len(), 2);
        assert_eq!(ir.tools[0].name, "batch");
        assert_eq!(ir.tools[1].name, "get_weather");
    }

    #[test]
    fn request_tool_choice_auto_string() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": "auto"
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(matches!(ir.tool_choice, Some(IRToolChoice::Auto)));
    }

    #[test]
    fn request_tool_choice_any_object() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": {"type": "any"}
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(matches!(ir.tool_choice, Some(IRToolChoice::Required)));
    }

    #[test]
    fn request_tool_choice_specific() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": {"type": "tool", "name": "get_weather"}
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.tool_choice, Some(IRToolChoice::Specific { name }) if name == "get_weather")
        );
    }

    #[test]
    fn request_tool_choice_none_string() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": "none"
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(matches!(ir.tool_choice, Some(IRToolChoice::None)));
    }

    #[test]
    fn request_stop_sequences() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}],
            "stop_sequences": ["STOP", "END"]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert_eq!(ir.stop_sequences, vec!["STOP", "END"]);
    }

    #[test]
    fn request_stream_flag() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "stream": true,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(ir.stream);
    }

    #[test]
    fn request_missing_messages_is_error() {
        let body = json!({"model": "m", "max_tokens": 1});
        assert!(parse_request(body, &default_settings()).is_err());
    }

    #[test]
    fn request_cache_control_stripped_silently() {
        let body = json!({
            "model": "m",
            "max_tokens": 1,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Hello", "cache_control": {"type": "ephemeral"}}
                ]
            }]
        });
        // Should parse without error; cache_control is not part of IRContentBlock.
        let ir = parse_request(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.messages[0].content[0], IRContentBlock::Text { text } if text == "Hello")
        );
    }

    // ── ir_to_response ──────────────────────────────────────────────────

    #[test]
    fn response_simple_text() {
        let ir = InternalResponse {
            id: "msg_123".into(),
            model: "claude-sonnet-4-20250514".into(),
            content: vec![IRContentBlock::Text {
                text: "Hello!".into(),
            }],
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage {
                input_tokens: 10,
                output_tokens: 5,
                ..Default::default()
            },
        };
        let r = build_response(&ir, &default_ctx()).unwrap();
        assert_eq!(r["id"], "msg_123");
        assert_eq!(r["type"], "message");
        assert_eq!(r["role"], "assistant");
        assert_eq!(r["model"], "claude-sonnet-4-20250514");
        assert_eq!(r["content"][0]["type"], "text");
        assert_eq!(r["content"][0]["text"], "Hello!");
        assert_eq!(r["stop_reason"], "end_turn");
        assert_eq!(r["usage"]["input_tokens"], 10);
        assert_eq!(r["usage"]["output_tokens"], 5);
    }

    #[test]
    fn response_model_override() {
        let ir = InternalResponse {
            id: "msg_1".into(),
            model: "gpt-4o".into(),
            content: vec![IRContentBlock::Text { text: "Hi".into() }],
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage::default(),
        };
        let r = build_response(&ir, &ctx_with_model("claude-sonnet-4-20250514")).unwrap();
        assert_eq!(r["model"], "claude-sonnet-4-20250514");
    }

    #[test]
    fn response_tool_use_content() {
        let ir = InternalResponse {
            id: "msg_1".into(),
            model: "claude-sonnet-4-20250514".into(),
            content: vec![IRContentBlock::ToolUse {
                id: "call_1".into(),
                name: "get_weather".into(),
                input: json!({"city": "Tokyo"}),
            }],
            stop_reason: IRStopReason::ToolUse,
            usage: IRUsage::default(),
        };
        let r = build_response(&ir, &default_ctx()).unwrap();
        assert_eq!(r["content"][0]["type"], "tool_use");
        assert_eq!(r["content"][0]["id"], "call_1");
        assert_eq!(r["content"][0]["name"], "get_weather");
        assert_eq!(r["content"][0]["input"]["city"], "Tokyo");
        assert_eq!(r["stop_reason"], "tool_use");
    }

    #[test]
    fn response_thinking_content() {
        let ir = InternalResponse {
            id: "msg_1".into(),
            model: "claude-sonnet-4-20250514".into(),
            content: vec![
                IRContentBlock::Thinking {
                    thinking: "Let me think...".into(),
                },
                IRContentBlock::Text { text: "42".into() },
            ],
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage::default(),
        };
        let r = build_response(&ir, &default_ctx()).unwrap();
        assert_eq!(r["content"][0]["type"], "thinking");
        assert_eq!(r["content"][0]["thinking"], "Let me think...");
        assert_eq!(r["content"][1]["type"], "text");
        assert_eq!(r["content"][1]["text"], "42");
    }

    #[test]
    fn response_stop_reasons() {
        for (reason, expected) in [
            (IRStopReason::EndTurn, "end_turn"),
            (IRStopReason::ToolUse, "tool_use"),
            (IRStopReason::MaxTokens, "max_tokens"),
            (IRStopReason::StopSequence, "stop_sequence"),
            (IRStopReason::Unknown("custom".into()), "custom"),
        ] {
            let ir = InternalResponse {
                id: "m".into(),
                model: "m".into(),
                content: vec![],
                stop_reason: reason,
                usage: IRUsage::default(),
            };
            let r = build_response(&ir, &default_ctx()).unwrap();
            assert_eq!(r["stop_reason"], expected);
        }
    }

    #[test]
    fn response_cache_usage_fields() {
        let ir = InternalResponse {
            id: "m".into(),
            model: "m".into(),
            content: vec![],
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: Some(20),
                cache_creation_5m_input_tokens: Some(15),
                cache_creation_1h_input_tokens: Some(5),
                cache_read_input_tokens: Some(60),
            },
        };
        let r = build_response(&ir, &default_ctx()).unwrap();
        assert_eq!(r["usage"]["cache_creation_input_tokens"], 20);
        assert_eq!(r["usage"]["cache_creation_5m_input_tokens"], 15);
        assert_eq!(r["usage"]["cache_creation_1h_input_tokens"], 5);
        assert_eq!(r["usage"]["cache_read_input_tokens"], 60);
    }

    // ── ir_chunk_to_sse ─────────────────────────────────────────────────

    #[test]
    fn sse_message_start() {
        let chunk = IRStreamChunk::MessageStart {
            id: "msg_1".into(),
            model: "claude-sonnet-4-20250514".into(),
            initial_usage: None,
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "message_start");
        assert_eq!(parsed[0].1["message"]["id"], "msg_1");
        assert_eq!(parsed[0].1["message"]["model"], "claude-sonnet-4-20250514");
        assert_eq!(parsed[0].1["message"]["role"], "assistant");
        assert_eq!(parsed[1].0, "ping");
    }

    #[test]
    fn sse_message_start_model_override() {
        let chunk = IRStreamChunk::MessageStart {
            id: "msg_1".into(),
            model: "gpt-4o".into(),
            initial_usage: None,
        };
        let frames = render_chunk(&chunk, &ctx_with_model("claude-sonnet-4-20250514")).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].1["message"]["model"], "claude-sonnet-4-20250514");
    }

    #[test]
    fn sse_content_block_start_text() {
        let chunk = IRStreamChunk::ContentBlockStart {
            index: 0,
            block_type: IRBlockType::Text,
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "content_block_start");
        assert_eq!(parsed[0].1["content_block"]["type"], "text");
        assert_eq!(parsed[0].1["content_block"]["text"], "");
        assert_eq!(parsed[0].1["index"], 0);
    }

    #[test]
    fn sse_content_block_start_tool_use() {
        let chunk = IRStreamChunk::ContentBlockStart {
            index: 1,
            block_type: IRBlockType::ToolUse {
                id: "call_1".into(),
                name: "get_weather".into(),
            },
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].1["content_block"]["type"], "tool_use");
        assert_eq!(parsed[0].1["content_block"]["id"], "call_1");
        assert_eq!(parsed[0].1["content_block"]["name"], "get_weather");
        assert_eq!(parsed[0].1["content_block"]["input"], json!({}));
    }

    #[test]
    fn sse_content_block_start_thinking() {
        let chunk = IRStreamChunk::ContentBlockStart {
            index: 0,
            block_type: IRBlockType::Thinking,
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].1["content_block"]["type"], "thinking");
        assert_eq!(parsed[0].1["content_block"]["thinking"], "");
    }

    #[test]
    fn sse_content_block_delta_text() {
        let chunk = IRStreamChunk::ContentBlockDelta {
            index: 0,
            delta: IRDelta::TextDelta {
                text: "Hello".into(),
            },
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "content_block_delta");
        assert_eq!(parsed[0].1["delta"]["type"], "text_delta");
        assert_eq!(parsed[0].1["delta"]["text"], "Hello");
        assert_eq!(parsed[0].1["index"], 0);
    }

    #[test]
    fn sse_content_block_delta_input_json() {
        let chunk = IRStreamChunk::ContentBlockDelta {
            index: 1,
            delta: IRDelta::InputJsonDelta {
                partial_json: r#"{"ci"#.into(),
            },
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].1["delta"]["type"], "input_json_delta");
        assert_eq!(parsed[0].1["delta"]["partial_json"], r#"{"ci"#);
    }

    #[test]
    fn sse_content_block_delta_thinking() {
        let chunk = IRStreamChunk::ContentBlockDelta {
            index: 0,
            delta: IRDelta::ThinkingDelta {
                thinking: "Hmm...".into(),
            },
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].1["delta"]["type"], "thinking_delta");
        assert_eq!(parsed[0].1["delta"]["thinking"], "Hmm...");
    }

    #[test]
    fn sse_content_block_stop() {
        let chunk = IRStreamChunk::ContentBlockStop { index: 2 };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "content_block_stop");
        assert_eq!(parsed[0].1["index"], 2);
    }

    #[test]
    fn sse_message_delta() {
        let chunk = IRStreamChunk::MessageDelta {
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage {
                output_tokens: 42,
                ..Default::default()
            },
        };
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "message_delta");
        assert_eq!(parsed[0].1["delta"]["stop_reason"], "end_turn");
        assert_eq!(parsed[0].1["usage"]["output_tokens"], 42);
    }

    #[test]
    fn sse_message_stop() {
        let chunk = IRStreamChunk::MessageStop;
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "message_stop");
    }

    #[test]
    fn sse_ping() {
        let chunk = IRStreamChunk::Ping;
        let frames = render_chunk(&chunk, &default_ctx()).unwrap();
        let parsed = parse_sse_frames(&frames);
        assert_eq!(parsed[0].0, "ping");
    }

    // ── Trait impl wiring ───────────────────────────────────────────────

    #[test]
    fn trait_protocol_name() {
        let adapter = AnthropicMessagesInbound;
        assert_eq!(adapter.protocol(), "anthropic_messages");
    }

    #[test]
    fn trait_request_to_ir_delegates() {
        let adapter = AnthropicMessagesInbound;
        let body = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let ir = adapter.request_to_ir(body, &default_ctx()).unwrap();
        assert_eq!(ir.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn trait_ir_to_response_delegates() {
        let adapter = AnthropicMessagesInbound;
        let ir = InternalResponse {
            id: "msg_1".into(),
            model: "m".into(),
            content: vec![IRContentBlock::Text { text: "Hi".into() }],
            stop_reason: IRStopReason::EndTurn,
            usage: IRUsage::default(),
        };
        let r = adapter.ir_to_response(&ir, &default_ctx()).unwrap();
        assert_eq!(r["type"], "message");
    }

    #[test]
    fn trait_ir_chunk_to_sse_delegates() {
        let adapter = AnthropicMessagesInbound;
        let chunk = IRStreamChunk::Ping;
        let frames = adapter.ir_chunk_to_sse(&chunk, &default_ctx()).unwrap();
        assert!(!frames.is_empty());
    }
}
