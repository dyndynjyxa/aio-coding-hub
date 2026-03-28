//! Outbound adapter: IR <-> OpenAI Responses API.
//!
//! Converts IR requests into OpenAI Responses API JSON and parses
//! OpenAI Responses API responses (both non-streaming and SSE) back
//! into IR types.

use crate::gateway::proxy::cx2cc::settings::Cx2ccSettings;
use crate::gateway::proxy::protocol_bridge::ir::*;
use crate::gateway::proxy::protocol_bridge::traits::*;
use serde_json::{json, Value};

/// Outbound adapter for the OpenAI Responses API protocol.
pub(crate) struct OpenAIResponsesOutbound;

impl Outbound for OpenAIResponsesOutbound {
    fn protocol(&self) -> &'static str {
        "openai_responses"
    }

    fn target_path(&self) -> &str {
        "/v1/responses"
    }

    fn ir_to_request(
        &self,
        ir: &InternalRequest,
        ctx: &BridgeContext,
    ) -> Result<Value, BridgeError> {
        ir_to_request(ir, &ctx.cx2cc_settings)
    }

    fn response_to_ir(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<InternalResponse, BridgeError> {
        response_to_ir(body, &ctx.cx2cc_settings)
    }

    fn sse_event_to_ir(
        &self,
        event_type: &str,
        data: &Value,
        state: &mut StreamState,
    ) -> Result<Vec<IRStreamChunk>, BridgeError> {
        sse_event_to_ir(event_type, data, state)
    }
}

// ---------------------------------------------------------------------------
// ir_to_request
// ---------------------------------------------------------------------------

fn ir_to_request(ir: &InternalRequest, settings: &Cx2ccSettings) -> Result<Value, BridgeError> {
    let mut result = json!({});

    result["model"] = json!(ir.model);

    // system → instructions (always set, even if empty — ChatGPT backend requires this field)
    result["instructions"] = json!(ir.system.as_deref().unwrap_or(""));

    // messages -> input
    let mut input: Vec<Value> = Vec::new();
    for msg in &ir.messages {
        let mut message_content: Vec<Value> = Vec::new();
        let role_str = match msg.role {
            IRRole::User => "user",
            IRRole::Assistant => "assistant",
        };

        for block in &msg.content {
            match block {
                IRContentBlock::Text { text } => {
                    let content_type = match msg.role {
                        IRRole::User => "input_text",
                        IRRole::Assistant => "output_text",
                    };
                    message_content.push(json!({"type": content_type, "text": text}));
                }

                IRContentBlock::Image { media_type, data } => {
                    message_content.push(json!({
                        "type": "input_image",
                        "image_url": format!("data:{media_type};base64,{data}")
                    }));
                }

                IRContentBlock::ToolUse {
                    id,
                    name,
                    input: tool_input,
                } => {
                    // Flush accumulated message content first
                    if !message_content.is_empty() {
                        input_items_push(&mut input, role_str, &mut message_content);
                    }
                    input.push(json!({
                        "type": "function_call",
                        "call_id": id,
                        "name": name,
                        "arguments": serde_json::to_string(tool_input).unwrap_or_default()
                    }));
                }

                IRContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    ..
                } => {
                    // Flush accumulated message content first
                    if !message_content.is_empty() {
                        input_items_push(&mut input, role_str, &mut message_content);
                    }
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": tool_use_id,
                        "output": content
                    }));
                }

                IRContentBlock::Thinking { .. } => {
                    // Responses API has no thinking input type; skip.
                }
            }
        }

        if !message_content.is_empty() {
            input_items_push(&mut input, role_str, &mut message_content);
        }
    }
    if !input.is_empty() {
        result["input"] = json!(input);
    }

    if let Some(max_tokens) = ir.max_tokens {
        result["max_output_tokens"] = json!(max_tokens);
    }
    if let Some(temperature) = ir.temperature {
        result["temperature"] = json!(temperature);
    }
    if let Some(top_p) = ir.top_p {
        result["top_p"] = json!(top_p);
    }
    result["stream"] = json!(ir.stream);

    if !ir.stop_sequences.is_empty() {
        if settings.drop_stop_sequences {
            tracing::debug!(
                "openai_responses outbound: dropping stop_sequences (not supported by Responses API)"
            );
        } else {
            result["stop"] = json!(ir.stop_sequences);
        }
    }

    // tools
    if !ir.tools.is_empty() {
        let response_tools: Vec<Value> = ir
            .tools
            .iter()
            .map(|t| {
                let mut params = t.parameters.clone();
                if settings.clean_schema {
                    clean_schema(&mut params);
                }
                json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": params,
                    // Claude Code tools rely on optional/conditional fields.
                    // Responses API normalizes omitted `strict` schemas, which
                    // can unintentionally over-constrain plan/team tools.
                    "strict": false
                })
            })
            .collect();
        result["tools"] = json!(response_tools);
    }

    // tool_choice
    if let Some(ref tc) = ir.tool_choice {
        result["tool_choice"] = match tc {
            IRToolChoice::Auto => json!("auto"),
            IRToolChoice::Required => json!("required"),
            IRToolChoice::None => json!("none"),
            IRToolChoice::Specific { name } => json!({"type": "function", "name": name}),
        };
    }

    Ok(result)
}

/// Flush accumulated message content blocks into the input array wrapped with role.
///
/// The OpenAI Responses API expects text/image content wrapped as:
///   `{"role": "user", "content": [{type: "input_text", ...}, ...]}`
/// while tool_use/tool_result are top-level items without wrapping.
fn input_items_push(input: &mut Vec<Value>, role: &str, content: &mut Vec<Value>) {
    input.push(json!({"role": role, "content": std::mem::take(content)}));
}

// ---------------------------------------------------------------------------
// response_to_ir
// ---------------------------------------------------------------------------

fn response_to_ir(body: Value, settings: &Cx2ccSettings) -> Result<InternalResponse, BridgeError> {
    let output = body
        .get("output")
        .and_then(|o| o.as_array())
        .ok_or_else(|| BridgeError::TransformFailed("No output in response".into()))?;

    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut content: Vec<IRContentBlock> = Vec::new();
    let mut has_tool_use = false;

    for item in output {
        match item.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "message" => {
                if let Some(blocks) = item.get("content").and_then(|c| c.as_array()) {
                    for block in blocks {
                        match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                            "output_text" => {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    if !text.is_empty() {
                                        content.push(IRContentBlock::Text {
                                            text: text.to_string(),
                                        });
                                    }
                                }
                            }
                            "refusal" => {
                                if let Some(text) = block.get("refusal").and_then(|t| t.as_str()) {
                                    if !text.is_empty() {
                                        content.push(IRContentBlock::Text {
                                            text: format!("[Refusal] {text}"),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            "function_call" => {
                let call_id = item
                    .get("call_id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = item
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");
                let input: Value =
                    serde_json::from_str(args_str).unwrap_or_else(|_| json!(args_str));
                content.push(IRContentBlock::ToolUse {
                    id: call_id,
                    name,
                    input,
                });
                has_tool_use = true;
            }

            "reasoning" => {
                if settings.enable_reasoning_to_thinking {
                    if let Some(text) = reasoning_text_from_item(item) {
                        content.push(IRContentBlock::Thinking { thinking: text });
                    }
                }
            }

            _ => {}
        }
    }

    // stop_reason
    let stop_reason = match body.get("status").and_then(|s| s.as_str()) {
        Some("completed") => {
            if has_tool_use {
                IRStopReason::ToolUse
            } else {
                IRStopReason::EndTurn
            }
        }
        Some("incomplete") => {
            let reason = body
                .pointer("/incomplete_details/reason")
                .and_then(|r| r.as_str());
            if matches!(reason, Some("max_output_tokens") | Some("max_tokens")) || reason.is_none()
            {
                IRStopReason::MaxTokens
            } else {
                IRStopReason::EndTurn
            }
        }
        Some(other) => IRStopReason::Unknown(other.to_string()),
        None => IRStopReason::Unknown("missing_status".to_string()),
    };

    // usage
    let usage = parse_usage(body.get("usage"));

    Ok(InternalResponse {
        id,
        model,
        content,
        stop_reason,
        usage,
    })
}

fn parse_usage(usage: Option<&Value>) -> IRUsage {
    let u = match usage {
        Some(v) if !v.is_null() => v,
        _ => return IRUsage::default(),
    };

    let input_tokens = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let output_tokens = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

    let cache_read_input_tokens = u
        .pointer("/input_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            u.pointer("/prompt_tokens_details/cached_tokens")
                .and_then(|v| v.as_u64())
        })
        .or_else(|| u.get("cache_read_input_tokens").and_then(|v| v.as_u64()));

    let cache_creation_5m_input_tokens = u
        .get("cache_creation_5m_input_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            u.pointer("/cache_creation/ephemeral_5m_input_tokens")
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            u.get("claude_cache_creation_5_m_tokens")
                .and_then(|v| v.as_u64())
        });

    let cache_creation_1h_input_tokens = u
        .get("cache_creation_1h_input_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            u.pointer("/cache_creation/ephemeral_1h_input_tokens")
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            u.get("claude_cache_creation_1_h_tokens")
                .and_then(|v| v.as_u64())
        });

    let cache_creation_input_tokens = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            match (
                cache_creation_5m_input_tokens,
                cache_creation_1h_input_tokens,
            ) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }
        });

    IRUsage {
        input_tokens,
        output_tokens,
        cache_creation_input_tokens,
        cache_creation_5m_input_tokens,
        cache_creation_1h_input_tokens,
        cache_read_input_tokens,
    }
}

fn reasoning_text_from_item(item: &Value) -> Option<String> {
    let summary = item.get("summary").and_then(Value::as_array)?;
    let text = summary
        .iter()
        .filter_map(|entry| {
            if entry.get("type").and_then(Value::as_str) == Some("summary_text") {
                entry.get("text").and_then(Value::as_str)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn reasoning_emitted(state: &StreamState) -> bool {
    state
        .extra
        .get("reasoning_emitted")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn mark_reasoning_emitted(state: &mut StreamState) {
    state
        .extra
        .insert("reasoning_emitted".to_string(), Value::Bool(true));
}

// ---------------------------------------------------------------------------
// sse_event_to_ir
// ---------------------------------------------------------------------------

fn sse_event_to_ir(
    event_type: &str,
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    match event_type {
        "response.created" => handle_response_created(data, state),
        "response.output_item.added" => handle_output_item_added(data, state),
        "response.output_text.delta" | "response.content_part.delta" => {
            handle_text_delta(data, state)
        }
        "response.function_call_arguments.delta" => handle_function_args_delta(data, state),
        "response.output_item.done" => handle_output_item_done(data, state),
        "response.completed" => handle_response_completed(data, state),
        _ => Ok(Vec::new()),
    }
}

fn handle_response_created(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let response = data.get("response").unwrap_or(data);

    let id = response
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let model = response
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let idx = state.block_index;
    state.block_index += 1;
    state.block_open = true;

    Ok(vec![
        IRStreamChunk::MessageStart {
            id,
            model,
            initial_usage: None,
        },
        IRStreamChunk::ContentBlockStart {
            index: idx,
            block_type: IRBlockType::Text,
        },
    ])
}

fn handle_output_item_added(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let item = match data.get("item") {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");

    match item_type {
        "function_call" => {
            let mut chunks = Vec::new();

            // Close the currently open block if needed
            if state.block_open {
                chunks.push(IRStreamChunk::ContentBlockStop {
                    index: state.block_index.saturating_sub(1),
                });
                state.block_open = false;
            }

            let call_id = item
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let idx = state.block_index;
            state.block_index += 1;
            state.block_open = true;
            state.saw_tool_use = true;
            state.active_tool = Some(ActiveToolState {
                id: call_id.clone(),
                name: name.clone(),
            });

            chunks.push(IRStreamChunk::ContentBlockStart {
                index: idx,
                block_type: IRBlockType::ToolUse { id: call_id, name },
            });
            Ok(chunks)
        }
        _ => Ok(Vec::new()),
    }
}

fn handle_text_delta(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let text = data
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if text.is_empty() {
        return Ok(Vec::new());
    }

    state.text_emitted = true;
    state.saw_visible_text = true;

    let mut chunks = open_text_block_if_needed(state);
    chunks.push(IRStreamChunk::ContentBlockDelta {
        index: state.block_index.saturating_sub(1),
        delta: IRDelta::TextDelta { text },
    });

    Ok(chunks)
}

fn handle_function_args_delta(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let partial_json = data
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let idx = state.block_index.saturating_sub(1);

    Ok(vec![IRStreamChunk::ContentBlockDelta {
        index: idx,
        delta: IRDelta::InputJsonDelta { partial_json },
    }])
}

fn emit_reasoning_chunks(
    state: &mut StreamState,
    thinking: &str,
    reopen_text_block: bool,
) -> Vec<IRStreamChunk> {
    if !state.enable_reasoning_to_thinking || thinking.trim().is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();

    if state.block_open {
        chunks.push(IRStreamChunk::ContentBlockStop {
            index: state.block_index.saturating_sub(1),
        });
        state.block_open = false;
        state.text_emitted = false;
    }

    let reasoning_index = state.block_index;
    state.block_index += 1;
    chunks.push(IRStreamChunk::ContentBlockStart {
        index: reasoning_index,
        block_type: IRBlockType::Thinking,
    });
    chunks.push(IRStreamChunk::ContentBlockDelta {
        index: reasoning_index,
        delta: IRDelta::ThinkingDelta {
            thinking: thinking.to_string(),
        },
    });
    chunks.push(IRStreamChunk::ContentBlockStop {
        index: reasoning_index,
    });
    mark_reasoning_emitted(state);

    if reopen_text_block {
        let text_index = state.block_index;
        state.block_index += 1;
        state.block_open = true;
        chunks.push(IRStreamChunk::ContentBlockStart {
            index: text_index,
            block_type: IRBlockType::Text,
        });
    }

    chunks
}

fn open_text_block_if_needed(state: &mut StreamState) -> Vec<IRStreamChunk> {
    if state.block_open {
        return Vec::new();
    }

    let index = state.block_index;
    state.block_index += 1;
    state.block_open = true;
    state.text_emitted = false;

    vec![IRStreamChunk::ContentBlockStart {
        index,
        block_type: IRBlockType::Text,
    }]
}

fn handle_output_item_done(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let item = match data.get("item") {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
    let mut chunks = Vec::new();

    // If active tool matches this item, close the tool block
    if item_type == "function_call" {
        let item_call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
        let matches = state
            .active_tool
            .as_ref()
            .map(|t| t.id == item_call_id)
            .unwrap_or(false);
        if matches {
            state.active_tool = None;
            state.block_open = false;
            chunks.push(IRStreamChunk::ContentBlockStop {
                index: state.block_index.saturating_sub(1),
            });
            return Ok(chunks);
        }
    }

    if item_type == "reasoning" {
        if let Some(thinking) = reasoning_text_from_item(item) {
            chunks.extend(emit_reasoning_chunks(state, &thinking, true));
        }
        return Ok(chunks);
    }

    // For message type: emit fallback text if not already emitted
    if item_type == "message" && !state.text_emitted {
        if let Some(blocks) = item.get("content").and_then(|c| c.as_array()) {
            for block in blocks {
                match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                    "output_text" => {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            let text = text.trim();
                            if !text.is_empty() {
                                chunks.extend(open_text_block_if_needed(state));
                                state.text_emitted = true;
                                state.saw_visible_text = true;
                                chunks.push(IRStreamChunk::ContentBlockDelta {
                                    index: state.block_index.saturating_sub(1),
                                    delta: IRDelta::TextDelta {
                                        text: text.to_string(),
                                    },
                                });
                            }
                        }
                    }
                    "refusal" => {
                        if let Some(text) = block.get("refusal").and_then(Value::as_str) {
                            let text = text.trim();
                            if !text.is_empty() {
                                chunks.extend(open_text_block_if_needed(state));
                                state.text_emitted = true;
                                state.saw_visible_text = true;
                                chunks.push(IRStreamChunk::ContentBlockDelta {
                                    index: state.block_index.saturating_sub(1),
                                    delta: IRDelta::TextDelta {
                                        text: text.to_string(),
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(chunks)
}

fn handle_response_completed(
    data: &Value,
    state: &mut StreamState,
) -> Result<Vec<IRStreamChunk>, BridgeError> {
    let response = data.get("response").unwrap_or(data);

    let mut chunks = Vec::new();

    if state.enable_reasoning_to_thinking && !reasoning_emitted(state) {
        if let Some(items) = response.get("output").and_then(Value::as_array) {
            for item in items {
                if item.get("type").and_then(Value::as_str) != Some("reasoning") {
                    continue;
                }
                if let Some(thinking) = reasoning_text_from_item(item) {
                    chunks.extend(emit_reasoning_chunks(state, &thinking, false));
                }
            }
        }
    }

    // Three-layer dedup: emit unemitted text from the completed response
    if !state.saw_visible_text {
        if let Some(items) = response.get("output").and_then(Value::as_array) {
            for item in items {
                if item.get("type").and_then(Value::as_str) != Some("message") {
                    continue;
                }
                if let Some(blocks) = item.get("content").and_then(|c| c.as_array()) {
                    for block in blocks {
                        let text = match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                            "output_text" => block.get("text").and_then(Value::as_str),
                            "refusal" => block.get("refusal").and_then(Value::as_str),
                            _ => None,
                        };
                        if let Some(text) = text {
                            let text = text.trim();
                            if !text.is_empty() {
                                chunks.extend(open_text_block_if_needed(state));
                                state.saw_visible_text = true;
                                state.text_emitted = true;
                                chunks.push(IRStreamChunk::ContentBlockDelta {
                                    index: state.block_index.saturating_sub(1),
                                    delta: IRDelta::TextDelta {
                                        text: text.to_string(),
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Close any open block
    if state.block_open {
        chunks.push(IRStreamChunk::ContentBlockStop {
            index: state.block_index.saturating_sub(1),
        });
        state.block_open = false;
        state.active_tool = None;
    }

    // Determine stop reason
    let status = response.get("status").and_then(Value::as_str).unwrap_or("");
    let stop_reason = match status {
        "completed" if state.saw_tool_use => IRStopReason::ToolUse,
        "incomplete" => IRStopReason::MaxTokens,
        _ => IRStopReason::EndTurn,
    };

    // Extract usage
    let usage = parse_usage(response.get("usage"));

    chunks.push(IRStreamChunk::MessageDelta { stop_reason, usage });
    chunks.push(IRStreamChunk::MessageStop);

    Ok(chunks)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Remove unsupported `format: "uri"` from JSON schemas, recursively.
fn clean_schema(schema: &mut Value) {
    match schema {
        Value::Object(obj) => {
            obj.remove("$schema");
            obj.remove("default");
            obj.remove("propertyNames");

            if obj.get("format").and_then(|v| v.as_str()) == Some("uri") {
                obj.remove("format");
            }

            if let Some(Value::Array(variants)) = obj.get_mut("anyOf") {
                for variant in variants.iter_mut() {
                    clean_schema(variant);
                }
            }
            if let Some(Value::Array(variants)) = obj.get_mut("oneOf") {
                for variant in variants.iter_mut() {
                    clean_schema(variant);
                }
            }
            if let Some(Value::Array(variants)) = obj.get_mut("allOf") {
                for variant in variants.iter_mut() {
                    clean_schema(variant);
                }
            }

            simplify_enum_union(obj);
            normalize_additional_properties(obj);

            if let Some(props) = obj.get_mut("properties").and_then(|v| v.as_object_mut()) {
                for val in props.values_mut() {
                    clean_schema(val);
                }
            }
            if let Some(items) = obj.get_mut("items") {
                clean_schema(items);
            }
        }
        Value::Array(items) => {
            for item in items {
                clean_schema(item);
            }
        }
        _ => {}
    }
}

fn normalize_additional_properties(obj: &mut serde_json::Map<String, Value>) {
    let Some(additional_properties) = obj.get_mut("additionalProperties") else {
        return;
    };

    match additional_properties {
        Value::Object(map) if map.is_empty() => {
            *additional_properties = Value::Bool(true);
        }
        Value::Object(_) | Value::Array(_) => clean_schema(additional_properties),
        _ => {}
    }
}

fn simplify_enum_union(obj: &mut serde_json::Map<String, Value>) {
    let variants = match obj.get("anyOf").and_then(Value::as_array) {
        Some(variants) if !variants.is_empty() => variants,
        _ => {
            normalize_const_enum(obj);
            return;
        }
    };

    let mut enum_values = Vec::new();
    let mut common_type: Option<String> = None;

    for variant in variants {
        let Some(variant_obj) = variant.as_object() else {
            return;
        };

        let variant_type = variant_obj
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                variant_obj
                    .get("const")
                    .map(|const_value| match const_value {
                        Value::String(_) => "string".to_string(),
                        Value::Bool(_) => "boolean".to_string(),
                        Value::Number(_) => "number".to_string(),
                        _ => "string".to_string(),
                    })
            });

        match (&common_type, &variant_type) {
            (Some(expected), Some(actual)) if expected != actual => return,
            (None, Some(actual)) => common_type = Some(actual.clone()),
            _ => {}
        }

        if let Some(values) = variant_obj.get("enum").and_then(Value::as_array) {
            enum_values.extend(values.iter().cloned());
            continue;
        }
        if let Some(value) = variant_obj.get("const") {
            enum_values.push(value.clone());
            continue;
        }
        return;
    }

    if enum_values.is_empty() {
        return;
    }

    dedupe_json_values(&mut enum_values);
    obj.remove("anyOf");
    if let Some(common_type) = common_type {
        obj.insert("type".to_string(), Value::String(common_type));
    }
    obj.insert("enum".to_string(), Value::Array(enum_values));
    obj.remove("const");
}

fn normalize_const_enum(obj: &mut serde_json::Map<String, Value>) {
    let Some(const_value) = obj.remove("const") else {
        return;
    };

    if obj.get("enum").is_none() {
        obj.insert("enum".to_string(), Value::Array(vec![const_value.clone()]));
    }

    if obj.get("type").is_none() {
        let type_name = match const_value {
            Value::String(_) => Some("string"),
            Value::Bool(_) => Some("boolean"),
            Value::Number(_) => Some("number"),
            Value::Array(_) => Some("array"),
            Value::Object(_) => Some("object"),
            Value::Null => None,
        };
        if let Some(type_name) = type_name {
            obj.insert("type".to_string(), Value::String(type_name.to_string()));
        }
    }
}

fn dedupe_json_values(values: &mut Vec<Value>) {
    let mut deduped = Vec::with_capacity(values.len());
    for value in values.drain(..) {
        if !deduped.iter().any(|existing| existing == &value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_settings() -> Cx2ccSettings {
        Cx2ccSettings::default()
    }

    fn make_ctx() -> BridgeContext {
        BridgeContext {
            claude_models: Default::default(),
            cx2cc_settings: default_settings(),
            requested_model: None,
            mapped_model: None,
            stream_requested: false,
            is_chatgpt_backend: false,
        }
    }

    // ── ir_to_request ─────────────────────────────────────────────────

    #[test]
    fn ir_to_request_simple_text() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::Text {
                    text: "Hello".into(),
                }],
            }],
            system: Some("Be helpful".into()),
            tools: vec![],
            tool_choice: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let adapter = OpenAIResponsesOutbound;
        let result = adapter.ir_to_request(&ir, &make_ctx()).unwrap();

        assert_eq!(result["model"], "gpt-4o");
        assert_eq!(result["instructions"], "Be helpful");
        assert_eq!(result["max_output_tokens"], 1024);
        assert_eq!(result["temperature"], 0.7);
        assert_eq!(result["stream"], false);
        assert_eq!(result["input"][0]["role"], "user");
        assert_eq!(result["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(result["input"][0]["content"][0]["text"], "Hello");
    }

    #[test]
    fn ir_to_request_assistant_text_becomes_output_text() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::Assistant,
                content: vec![IRContentBlock::Text {
                    text: "I can help".into(),
                }],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        assert_eq!(result["input"][0]["content"][0]["type"], "output_text");
    }

    #[test]
    fn ir_to_request_image_block() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::Image {
                    media_type: "image/png".into(),
                    data: "abc123".into(),
                }],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        assert_eq!(result["input"][0]["content"][0]["type"], "input_image");
        assert_eq!(
            result["input"][0]["content"][0]["image_url"],
            "data:image/png;base64,abc123"
        );
    }

    #[test]
    fn ir_to_request_tool_use_becomes_function_call() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::Assistant,
                content: vec![
                    IRContentBlock::Text {
                        text: "Let me check".into(),
                    },
                    IRContentBlock::ToolUse {
                        id: "call_123".into(),
                        name: "get_weather".into(),
                        input: json!({"location": "Tokyo"}),
                    },
                ],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        let input = result["input"].as_array().unwrap();
        assert_eq!(input.len(), 2);
        assert_eq!(input[0]["content"][0]["type"], "output_text");
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[1]["call_id"], "call_123");
        assert_eq!(input[1]["name"], "get_weather");
    }

    #[test]
    fn ir_to_request_tool_result_becomes_function_call_output() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::ToolResult {
                    tool_use_id: "call_123".into(),
                    content: "Sunny, 25C".into(),
                    is_error: false,
                }],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        let input = result["input"].as_array().unwrap();
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["type"], "function_call_output");
        assert_eq!(input[0]["call_id"], "call_123");
        assert_eq!(input[0]["output"], "Sunny, 25C");
    }

    #[test]
    fn ir_to_request_thinking_is_skipped() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::Assistant,
                content: vec![
                    IRContentBlock::Thinking {
                        thinking: "Internal reasoning".into(),
                    },
                    IRContentBlock::Text {
                        text: "Answer".into(),
                    },
                ],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        let input = result["input"].as_array().unwrap();
        // Only the text block should remain (thinking is skipped)
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["content"][0]["type"], "output_text");
    }

    #[test]
    fn ir_to_request_preserves_stop_sequences_when_enabled() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::Text { text: "Hi".into() }],
            }],
            system: None,
            tools: vec![],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec!["STOP".into(), "END".into()],
            stream: false,
            metadata: IRMetadata::default(),
        };
        let mut settings = default_settings();
        settings.drop_stop_sequences = false;

        let result = ir_to_request(&ir, &settings).unwrap();
        assert_eq!(result["stop"], json!(["STOP", "END"]));
    }

    #[test]
    fn ir_to_request_tools_with_clean_schema() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::Text { text: "Hi".into() }],
            }],
            system: None,
            tools: vec![IRToolDefinition {
                name: "fetch_url".into(),
                description: Some("Fetch a URL".into()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "format": "uri"}
                    }
                }),
            }],
            tool_choice: Some(IRToolChoice::Auto),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };

        let result = ir_to_request(&ir, &default_settings()).unwrap();
        assert_eq!(result["tools"][0]["type"], "function");
        assert_eq!(result["tools"][0]["name"], "fetch_url");
        assert_eq!(result["tools"][0]["strict"], false);
        // format: "uri" should be stripped
        assert!(result["tools"][0]["parameters"]["properties"]["url"]
            .get("format")
            .is_none());
        assert_eq!(result["tool_choice"], "auto");
    }

    #[test]
    fn ir_to_request_preserves_schema_when_cleaning_disabled() {
        let ir = InternalRequest {
            model: "gpt-4o".into(),
            messages: vec![IRMessage {
                role: IRRole::User,
                content: vec![IRContentBlock::Text { text: "Hi".into() }],
            }],
            system: None,
            tools: vec![IRToolDefinition {
                name: "fetch_url".into(),
                description: Some("Fetch a URL".into()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "format": "uri"}
                    }
                }),
            }],
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_sequences: vec![],
            stream: false,
            metadata: IRMetadata::default(),
        };
        let mut settings = default_settings();
        settings.clean_schema = false;

        let result = ir_to_request(&ir, &settings).unwrap();
        assert_eq!(
            result["tools"][0]["parameters"]["properties"]["url"]["format"],
            "uri"
        );
    }

    #[test]
    fn ir_to_request_tool_choice_variants() {
        let check = |tc: IRToolChoice, expected: Value| {
            let ir = InternalRequest {
                model: "m".into(),
                messages: vec![],
                system: None,
                tools: vec![],
                tool_choice: Some(tc),
                max_tokens: None,
                temperature: None,
                top_p: None,
                stop_sequences: vec![],
                stream: false,
                metadata: IRMetadata::default(),
            };
            let result = ir_to_request(&ir, &default_settings()).unwrap();
            assert_eq!(result["tool_choice"], expected);
        };

        check(IRToolChoice::Auto, json!("auto"));
        check(IRToolChoice::Required, json!("required"));
        check(IRToolChoice::None, json!("none"));
        check(
            IRToolChoice::Specific {
                name: "my_fn".into(),
            },
            json!({"type": "function", "name": "my_fn"}),
        );
    }

    // ── response_to_ir ────────────────────────────────────────────────

    #[test]
    fn response_to_ir_simple_text() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hello!"}]}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.id, "resp_1");
        assert_eq!(ir.model, "gpt-4o");
        assert_eq!(ir.stop_reason, IRStopReason::EndTurn);
        assert_eq!(ir.usage.input_tokens, 10);
        assert_eq!(ir.usage.output_tokens, 5);
        assert!(matches!(&ir.content[0], IRContentBlock::Text { text } if text == "Hello!"));
    }

    #[test]
    fn response_to_ir_function_call() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{
                "type": "function_call",
                "call_id": "call_123",
                "name": "get_weather",
                "arguments": "{\"location\": \"Tokyo\"}"
            }],
            "usage": {"input_tokens": 10, "output_tokens": 15}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.stop_reason, IRStopReason::ToolUse);
        assert!(matches!(
            &ir.content[0],
            IRContentBlock::ToolUse { id, name, input }
                if id == "call_123" && name == "get_weather" && input["location"] == "Tokyo"
        ));
    }

    #[test]
    fn response_to_ir_refusal() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "refusal", "refusal": "I cannot do that."}]}],
            "usage": {"input_tokens": 5, "output_tokens": 2}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.content[0], IRContentBlock::Text { text } if text == "[Refusal] I cannot do that.")
        );
    }

    #[test]
    fn response_to_ir_reasoning() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [
                {"type": "reasoning", "summary": [{"type": "summary_text", "text": "Thinking..."}]},
                {"type": "message", "content": [{"type": "output_text", "text": "42"}]}
            ],
            "usage": {"input_tokens": 10, "output_tokens": 20}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.content[0], IRContentBlock::Thinking { thinking } if thinking == "Thinking...")
        );
        assert!(matches!(&ir.content[1], IRContentBlock::Text { text } if text == "42"));
    }

    #[test]
    fn response_to_ir_skips_reasoning_when_disabled() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [
                {"type": "reasoning", "summary": [{"type": "summary_text", "text": "Thinking..."}]},
                {"type": "message", "content": [{"type": "output_text", "text": "42"}]}
            ],
            "usage": {"input_tokens": 10, "output_tokens": 20}
        });
        let mut settings = default_settings();
        settings.enable_reasoning_to_thinking = false;

        let ir = response_to_ir(body, &settings).unwrap();
        assert_eq!(ir.content.len(), 1);
        assert!(matches!(&ir.content[0], IRContentBlock::Text { text } if text == "42"));
    }

    #[test]
    fn response_to_ir_incomplete_max_tokens() {
        let body = json!({
            "id": "resp_1",
            "status": "incomplete",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Partial"}]}],
            "usage": {"input_tokens": 10, "output_tokens": 4096}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.stop_reason, IRStopReason::MaxTokens);
    }

    #[test]
    fn response_to_ir_incomplete_non_token_reason() {
        let body = json!({
            "id": "resp_1",
            "status": "incomplete",
            "incomplete_details": {"reason": "content_filter"},
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "..."}]}],
            "usage": {"input_tokens": 10, "output_tokens": 1}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.stop_reason, IRStopReason::EndTurn);
    }

    #[test]
    fn response_to_ir_cache_tokens() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hi"}]}],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "input_tokens_details": {"cached_tokens": 80}
            }
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.usage.cache_read_input_tokens, Some(80));
    }

    #[test]
    fn response_to_ir_cache_creation_tokens() {
        let body = json!({
            "id": "resp_2",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hi"}]}],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 20,
                    "ephemeral_1h_input_tokens": 5
                }
            }
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert_eq!(ir.usage.cache_creation_input_tokens, Some(25));
    }

    #[test]
    fn response_to_ir_no_output_returns_error() {
        let body = json!({"id": "resp_1", "status": "completed"});
        assert!(response_to_ir(body, &default_settings()).is_err());
    }

    #[test]
    fn response_to_ir_invalid_json_arguments_fallback() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{
                "type": "function_call",
                "call_id": "c1",
                "name": "test",
                "arguments": "not valid json"
            }],
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });

        let ir = response_to_ir(body, &default_settings()).unwrap();
        assert!(
            matches!(&ir.content[0], IRContentBlock::ToolUse { input, .. } if input == &json!("not valid json"))
        );
    }

    // ── sse_event_to_ir ───────────────────────────────────────────────

    #[test]
    fn sse_response_created_emits_message_start_and_block_start() {
        let mut state = StreamState::default();
        let data = json!({
            "response": {
                "id": "resp_123",
                "model": "gpt-5",
                "status": "in_progress",
                "output": [],
                "usage": {"input_tokens": 11, "output_tokens": 0}
            }
        });

        let chunks = sse_event_to_ir("response.created", &data, &mut state).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(
            matches!(&chunks[0], IRStreamChunk::MessageStart { id, model, .. } if id == "resp_123" && model == "gpt-5")
        );
        assert!(matches!(
            &chunks[1],
            IRStreamChunk::ContentBlockStart {
                index: 0,
                block_type: IRBlockType::Text
            }
        ));
        assert!(state.block_open);
        assert_eq!(state.block_index, 1);
    }

    #[test]
    fn sse_text_delta_emits_content_block_delta() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            ..StreamState::default()
        };

        let data = json!({"delta": "Hello"});
        let chunks = sse_event_to_ir("response.output_text.delta", &data, &mut state).unwrap();

        assert_eq!(chunks.len(), 1);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::ContentBlockDelta {
                index: 0,
                delta: IRDelta::TextDelta { text }
            } if text == "Hello"
        ));
        assert!(state.text_emitted);
        assert!(state.saw_visible_text);
    }

    #[test]
    fn sse_empty_text_delta_is_ignored() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            ..StreamState::default()
        };

        let data = json!({"delta": ""});
        let chunks = sse_event_to_ir("response.output_text.delta", &data, &mut state).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn sse_function_call_flow() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            ..StreamState::default()
        };
        // Simulate response.created already happened

        // output_item.added for function_call
        let added_data = json!({
            "item": {
                "id": "fc_1",
                "type": "function_call",
                "call_id": "call_1",
                "name": "shell",
                "arguments": ""
            }
        });
        let chunks =
            sse_event_to_ir("response.output_item.added", &added_data, &mut state).unwrap();
        // Should close text block + open tool block
        assert_eq!(chunks.len(), 2);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::ContentBlockStop { index: 0 }
        ));
        assert!(matches!(
            &chunks[1],
            IRStreamChunk::ContentBlockStart { index: 1, block_type: IRBlockType::ToolUse { id, name } }
                if id == "call_1" && name == "shell"
        ));
        assert!(state.saw_tool_use);

        // function_call_arguments.delta
        let args_data = json!({"delta": "{\"cmd\":\"pwd\"}"});
        let chunks = sse_event_to_ir(
            "response.function_call_arguments.delta",
            &args_data,
            &mut state,
        )
        .unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::ContentBlockDelta {
                index: 1,
                delta: IRDelta::InputJsonDelta { partial_json }
            } if partial_json == "{\"cmd\":\"pwd\"}"
        ));

        // output_item.done for function_call
        let done_data = json!({
            "item": {
                "id": "fc_1",
                "type": "function_call",
                "call_id": "call_1",
                "name": "shell",
                "arguments": "{\"cmd\":\"pwd\"}"
            }
        });
        let chunks = sse_event_to_ir("response.output_item.done", &done_data, &mut state).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::ContentBlockStop { index: 1 }
        ));
        assert!(state.active_tool.is_none());
    }

    #[test]
    fn sse_response_completed_with_tool_use() {
        let mut state = StreamState {
            block_index: 2,
            block_open: false,
            saw_tool_use: true,
            saw_visible_text: true,
            ..StreamState::default()
        };

        let data = json!({
            "response": {
                "id": "resp_123",
                "model": "gpt-5",
                "status": "completed",
                "usage": {"input_tokens": 11, "output_tokens": 7}
            }
        });

        let chunks = sse_event_to_ir("response.completed", &data, &mut state).unwrap();
        // No block to close (already closed), so: MessageDelta + MessageStop
        assert_eq!(chunks.len(), 2);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::MessageDelta { stop_reason, usage }
                if *stop_reason == IRStopReason::ToolUse && usage.output_tokens == 7
        ));
        assert!(matches!(&chunks[1], IRStreamChunk::MessageStop));
    }

    #[test]
    fn sse_response_completed_closes_open_block() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            saw_visible_text: true,
            ..StreamState::default()
        };

        let data = json!({
            "response": {
                "id": "resp_123",
                "model": "gpt-5",
                "status": "completed",
                "usage": {"input_tokens": 5, "output_tokens": 3}
            }
        });

        let chunks = sse_event_to_ir("response.completed", &data, &mut state).unwrap();
        // ContentBlockStop + MessageDelta + MessageStop
        assert_eq!(chunks.len(), 3);
        assert!(matches!(
            &chunks[0],
            IRStreamChunk::ContentBlockStop { index: 0 }
        ));
        assert!(matches!(
            &chunks[1],
            IRStreamChunk::MessageDelta { stop_reason, .. } if *stop_reason == IRStopReason::EndTurn
        ));
        assert!(matches!(&chunks[2], IRStreamChunk::MessageStop));
    }

    #[test]
    fn sse_response_completed_fallback_text_extraction() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            saw_visible_text: false,
            text_emitted: false,
            ..StreamState::default()
        };
        // No text was emitted via deltas

        let data = json!({
            "response": {
                "id": "resp_123",
                "model": "gpt-5",
                "status": "completed",
                "output": [
                    {"type": "message", "content": [{"type": "output_text", "text": "Fallback text"}]}
                ],
                "usage": {"input_tokens": 5, "output_tokens": 3}
            }
        });

        let chunks = sse_event_to_ir("response.completed", &data, &mut state).unwrap();
        // TextDelta (fallback) + ContentBlockStop + MessageDelta + MessageStop
        assert!(chunks.len() >= 3);
        assert!(chunks.iter().any(|c| matches!(
            c,
            IRStreamChunk::ContentBlockDelta {
                delta: IRDelta::TextDelta { text },
                ..
            } if text == "Fallback text"
        )));
    }

    #[test]
    fn sse_output_item_done_message_fallback_text() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            text_emitted: false,
            ..StreamState::default()
        };

        let data = json!({
            "item": {
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello from done"}]
            }
        });

        let chunks = sse_event_to_ir("response.output_item.done", &data, &mut state).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| matches!(
            c,
            IRStreamChunk::ContentBlockDelta {
                delta: IRDelta::TextDelta { text },
                ..
            } if text == "Hello from done"
        )));
    }

    #[test]
    fn sse_output_item_done_reasoning_emits_thinking_when_enabled() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            ..StreamState::default()
        };

        let data = json!({
            "item": {
                "id": "reason_1",
                "type": "reasoning",
                "summary": [{"type": "summary_text", "text": "Think first"}]
            }
        });

        let chunks = sse_event_to_ir("response.output_item.done", &data, &mut state).unwrap();
        assert!(chunks.iter().any(|c| matches!(
            c,
            IRStreamChunk::ContentBlockDelta {
                delta: IRDelta::ThinkingDelta { thinking },
                ..
            } if thinking == "Think first"
        )));
        assert!(state.block_open, "text block should reopen after thinking");
    }

    #[test]
    fn sse_output_item_done_reasoning_is_skipped_when_disabled() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            enable_reasoning_to_thinking: false,
            ..StreamState::default()
        };

        let data = json!({
            "item": {
                "id": "reason_1",
                "type": "reasoning",
                "summary": [{"type": "summary_text", "text": "Think first"}]
            }
        });

        let chunks = sse_event_to_ir("response.output_item.done", &data, &mut state).unwrap();
        assert!(chunks.is_empty());
        assert!(state.block_open);
    }

    #[test]
    fn sse_unknown_event_returns_empty() {
        let mut state = StreamState::default();
        let data = json!({});
        let chunks = sse_event_to_ir("response.some_unknown_event", &data, &mut state).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn sse_incomplete_status_maps_to_max_tokens() {
        let mut state = StreamState {
            block_index: 1,
            block_open: true,
            saw_visible_text: true,
            ..StreamState::default()
        };

        let data = json!({
            "response": {
                "id": "resp_123",
                "model": "gpt-5",
                "status": "incomplete",
                "usage": {"input_tokens": 5, "output_tokens": 4096}
            }
        });

        let chunks = sse_event_to_ir("response.completed", &data, &mut state).unwrap();
        assert!(chunks.iter().any(|c| matches!(
            c,
            IRStreamChunk::MessageDelta { stop_reason, .. } if *stop_reason == IRStopReason::MaxTokens
        )));
    }

    // ── clean_schema ──────────────────────────────────────────────────

    #[test]
    fn clean_schema_removes_format_uri() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "format": "uri"},
                "name": {"type": "string"}
            }
        });
        clean_schema(&mut schema);
        assert!(schema["properties"]["url"].get("format").is_none());
        assert_eq!(schema["properties"]["name"]["type"], "string");
    }

    #[test]
    fn clean_schema_recursive_in_items() {
        let mut schema = json!({
            "type": "array",
            "items": {"type": "string", "format": "uri"}
        });
        clean_schema(&mut schema);
        assert!(schema["items"].get("format").is_none());
    }

    #[test]
    fn clean_schema_preserves_other_formats() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "ts": {"type": "string", "format": "date-time"}
            }
        });
        clean_schema(&mut schema);
        assert_eq!(schema["properties"]["ts"]["format"], "date-time");
    }

    #[test]
    fn clean_schema_simplifies_plan_mode_keywords() {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "status": {
                    "anyOf": [
                        {"type": "string", "enum": ["pending", "in_progress", "completed"]},
                        {"type": "string", "const": "deleted"}
                    ]
                },
                "metadata": {
                    "type": "object",
                    "propertyNames": {"type": "string"},
                    "additionalProperties": {}
                },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": {}
                    }
                }
            },
            "additionalProperties": {}
        });

        clean_schema(&mut schema);

        assert!(schema.get("$schema").is_none());
        assert_eq!(schema["additionalProperties"], true);
        assert_eq!(
            schema["properties"]["status"]["enum"],
            json!(["pending", "in_progress", "completed", "deleted"])
        );
        assert!(schema["properties"]["status"].get("anyOf").is_none());
        assert!(schema["properties"]["metadata"]
            .get("propertyNames")
            .is_none());
        assert_eq!(
            schema["properties"]["metadata"]["additionalProperties"],
            true
        );
        assert_eq!(
            schema["properties"]["items"]["items"]["additionalProperties"],
            true
        );
    }

    // ── Outbound trait contract ───────────────────────────────────────

    #[test]
    fn protocol_returns_expected_value() {
        let adapter = OpenAIResponsesOutbound;
        assert_eq!(adapter.protocol(), "openai_responses");
    }

    #[test]
    fn target_path_returns_expected_value() {
        let adapter = OpenAIResponsesOutbound;
        assert_eq!(adapter.target_path(), "/v1/responses");
    }
}
