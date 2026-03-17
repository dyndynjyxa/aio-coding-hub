//! Bidirectional non-streaming translation: Anthropic Messages API ↔ OpenAI Responses API.

use serde_json::{json, Value};

/// Convert Anthropic Messages request to OpenAI Responses API request.
pub fn anthropic_to_responses(body: Value) -> Result<Value, String> {
    let mut result = json!({});

    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        result["model"] = json!(model);
    }

    // system → instructions (ChatGPT backend-api requires this field)
    let instructions = match body.get("system") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|msg| msg.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    };
    result["instructions"] = json!(instructions);

    // messages → input (tool_use/tool_result lifted to top-level items)
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        let input = convert_messages_to_input(msgs)?;
        result["input"] = json!(input);
    }

    // Claude Messages → canonical OpenAI Responses.
    // ChatGPT-backed Codex upstreams may apply a narrower compat whitelist later.
    // Keep the generic Responses field here.
    // max_tokens → max_output_tokens
    if let Some(v) = body.get("max_tokens") {
        result["max_output_tokens"] = v.clone();
    }

    if let Some(v) = body.get("temperature") {
        result["temperature"] = v.clone();
    }
    if let Some(v) = body.get("top_p") {
        result["top_p"] = v.clone();
    }
    if let Some(v) = body.get("stream") {
        result["stream"] = v.clone();
    }

    // stop_sequences dropped — Responses API does not support it
    if body
        .get("stop_sequences")
        .and_then(|v| v.as_array())
        .map_or(false, |a| !a.is_empty())
    {
        tracing::debug!("cx2cc: dropping stop_sequences (not supported by Responses API)");
    }

    // tools: wrap with type:function, input_schema → parameters, skip BatchTool
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let response_tools: Vec<Value> = tools
            .iter()
            .filter(|t| t.get("type").and_then(|v| v.as_str()) != Some("BatchTool"))
            .map(|t| {
                let mut params = t.get("input_schema").cloned().unwrap_or(json!({}));
                clean_schema(&mut params);
                json!({
                    "type": "function",
                    "name": t.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                    "description": t.get("description"),
                    "parameters": params
                })
            })
            .collect();

        if !response_tools.is_empty() {
            result["tools"] = json!(response_tools);
        }
    }

    if let Some(v) = body.get("tool_choice") {
        result["tool_choice"] = map_tool_choice(v);
    }

    Ok(result)
}

/// Convert OpenAI Responses API response to Anthropic Messages response.
pub fn responses_to_anthropic(body: Value) -> Result<Value, String> {
    responses_to_anthropic_with_model_override(body, None)
}

pub fn responses_to_anthropic_with_model_override(
    body: Value,
    model_override: Option<&str>,
) -> Result<Value, String> {
    let output = body
        .get("output")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No output in response".to_string())?;

    let mut content: Vec<Value> = Vec::new();
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
                                        content.push(json!({"type": "text", "text": text}));
                                    }
                                }
                            }
                            "refusal" => {
                                if let Some(text) = block.get("refusal").and_then(|t| t.as_str()) {
                                    if !text.is_empty() {
                                        content.push(json!({"type": "text", "text": text}));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            "function_call" => {
                let call_id = item.get("call_id").and_then(|i| i.as_str()).unwrap_or("");
                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args_str = item
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");
                let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                content.push(json!({
                    "type": "tool_use",
                    "id": call_id,
                    "name": name,
                    "input": input
                }));
                has_tool_use = true;
            }

            "reasoning" => {
                let mut extracted = false;
                if let Some(summary) = item.get("summary").and_then(|s| s.as_array()) {
                    let text: String = summary
                        .iter()
                        .filter_map(|s| {
                            if s.get("type").and_then(|t| t.as_str()) == Some("summary_text") {
                                s.get("text").and_then(|t| t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    if !text.is_empty() {
                        content.push(json!({"type": "thinking", "thinking": text}));
                        extracted = true;
                    }
                }
                if !extracted {
                    tracing::debug!("cx2cc: reasoning output item had no extractable summary_text");
                }
            }

            _ => {}
        }
    }

    let stop_reason = map_stop_reason(
        body.get("status").and_then(|s| s.as_str()),
        has_tool_use,
        body.pointer("/incomplete_details/reason")
            .and_then(|r| r.as_str()),
    );

    let usage = build_usage(body.get("usage"));
    let model = model_override
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| body.get("model").and_then(|m| m.as_str()).unwrap_or(""));

    Ok(json!({
        "id": body.get("id").and_then(|i| i.as_str()).unwrap_or(""),
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": model,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": usage
    }))
}

/// Remove unsupported `format: "uri"` from JSON schemas, recursively.
pub fn clean_schema(schema: &mut Value) {
    if let Some(obj) = schema.as_object_mut() {
        if obj.get("format").and_then(|v| v.as_str()) == Some("uri") {
            obj.remove("format");
        }
        if let Some(props) = obj.get_mut("properties").and_then(|v| v.as_object_mut()) {
            for val in props.values_mut() {
                clean_schema(val);
            }
        }
        if let Some(items) = obj.get_mut("items") {
            clean_schema(items);
        }
    }
}

// --- private helpers ---

fn convert_messages_to_input(messages: &[Value]) -> Result<Vec<Value>, String> {
    let mut input: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");

        match msg.get("content") {
            Some(Value::String(text)) => {
                let content_type = if role == "assistant" {
                    "output_text"
                } else {
                    "input_text"
                };
                input.push(json!({
                    "role": role,
                    "content": [{"type": content_type, "text": text}]
                }));
            }

            Some(Value::Array(blocks)) => {
                let mut message_content: Vec<Value> = Vec::new();

                for block in blocks {
                    match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                let content_type = if role == "assistant" {
                                    "output_text"
                                } else {
                                    "input_text"
                                };
                                // cache_control is stripped — not accepted by Responses API
                                message_content.push(json!({"type": content_type, "text": text}));
                            }
                        }

                        "image" => {
                            if let Some(source) = block.get("source") {
                                let media_type = source
                                    .get("media_type")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("image/png");
                                let data =
                                    source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                                message_content.push(json!({
                                    "type": "input_image",
                                    "image_url": format!("data:{media_type};base64,{data}")
                                }));
                            }
                        }

                        "tool_use" => {
                            // flush accumulated message content first
                            if !message_content.is_empty() {
                                input.push(
                                    json!({"role": role, "content": message_content.clone()}),
                                );
                                message_content.clear();
                            }
                            let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                            let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            let arguments = block.get("input").cloned().unwrap_or(json!({}));
                            input.push(json!({
                                "type": "function_call",
                                "call_id": id,
                                "name": name,
                                "arguments": serde_json::to_string(&arguments).unwrap_or_default()
                            }));
                        }

                        "tool_result" => {
                            // flush accumulated message content first
                            if !message_content.is_empty() {
                                input.push(
                                    json!({"role": role, "content": message_content.clone()}),
                                );
                                message_content.clear();
                            }
                            let call_id = block
                                .get("tool_use_id")
                                .and_then(|i| i.as_str())
                                .unwrap_or("");
                            let output = match block.get("content") {
                                Some(Value::String(s)) => s.clone(),
                                Some(v) => serde_json::to_string(v).unwrap_or_default(),
                                None => String::new(),
                            };
                            input.push(json!({
                                "type": "function_call_output",
                                "call_id": call_id,
                                "output": output
                            }));
                        }

                        "thinking" => {
                            // discard — Responses API has no equivalent input type
                        }

                        _ => {}
                    }
                }

                if !message_content.is_empty() {
                    input.push(json!({"role": role, "content": message_content}));
                }
            }

            _ => {
                input.push(json!({"role": role}));
            }
        }
    }

    Ok(input)
}

fn map_tool_choice(tool_choice: &Value) -> Value {
    match tool_choice {
        Value::String(_) => tool_choice.clone(),
        Value::Object(obj) => match obj.get("type").and_then(|t| t.as_str()) {
            Some("any") => json!("required"),
            Some("auto") => json!("auto"),
            Some("none") => json!("none"),
            Some("tool") => {
                let name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("");
                json!({"type": "function", "name": name})
            }
            _ => tool_choice.clone(),
        },
        _ => tool_choice.clone(),
    }
}

fn map_stop_reason(
    status: Option<&str>,
    has_tool_use: bool,
    incomplete_reason: Option<&str>,
) -> Option<&'static str> {
    status.map(|s| match s {
        "completed" => {
            if has_tool_use {
                "tool_use"
            } else {
                "end_turn"
            }
        }
        "incomplete" => {
            if matches!(
                incomplete_reason,
                Some("max_output_tokens") | Some("max_tokens")
            ) || incomplete_reason.is_none()
            {
                "max_tokens"
            } else {
                "end_turn"
            }
        }
        _ => "end_turn",
    })
}

fn build_usage(usage: Option<&Value>) -> Value {
    let u = match usage {
        Some(v) if !v.is_null() => v,
        _ => return json!({"input_tokens": 0, "output_tokens": 0}),
    };

    let input = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let output = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut result = json!({"input_tokens": input, "output_tokens": output});

    // OpenAI Responses nested details (lower priority)
    if let Some(cached) = u
        .pointer("/input_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
    {
        result["cache_read_input_tokens"] = json!(cached);
    }
    if let Some(cached) = u
        .pointer("/prompt_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
    {
        if result.get("cache_read_input_tokens").is_none() {
            result["cache_read_input_tokens"] = json!(cached);
        }
    }

    // Direct Anthropic-style fields override
    if let Some(v) = u.get("cache_read_input_tokens") {
        result["cache_read_input_tokens"] = v.clone();
    }
    if let Some(v) = u.get("cache_creation_input_tokens") {
        result["cache_creation_input_tokens"] = v.clone();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_user_message() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["model"], "gpt-4o");
        assert_eq!(r["max_output_tokens"], 1024);
        assert_eq!(r["input"][0]["role"], "user");
        assert_eq!(r["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(r["input"][0]["content"][0]["text"], "Hello");
        assert!(r.get("stop_sequences").is_none());
    }

    #[test]
    fn system_string_becomes_instructions() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["instructions"], "You are helpful.");
        assert_eq!(r["input"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn system_array_joined_with_double_newline() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "Part 1"},
                {"type": "text", "text": "Part 2"}
            ],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["instructions"], "Part 1\n\nPart 2");
    }

    #[test]
    fn tools_converted_with_function_wrapper() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Weather?"}],
            "tools": [{
                "name": "get_weather",
                "description": "Get weather",
                "input_schema": {"type": "object", "properties": {"location": {"type": "string"}}}
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["tools"][0]["type"], "function");
        assert_eq!(r["tools"][0]["name"], "get_weather");
        assert!(r["tools"][0].get("parameters").is_some());
        assert!(r["tools"][0].get("input_schema").is_none());
    }

    #[test]
    fn batch_tool_filtered_out() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [
                {"type": "BatchTool", "name": "batch", "input_schema": {}},
                {"name": "real_tool", "input_schema": {}}
            ]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["tools"].as_array().unwrap().len(), 1);
        assert_eq!(r["tools"][0]["name"], "real_tool");
    }

    #[test]
    fn tool_choice_any_maps_to_required() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": {"type": "any"}
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["tool_choice"], "required");
    }

    #[test]
    fn tool_choice_forced_tool_maps_to_function_object() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tool_choice": {"type": "tool", "name": "get_weather"}
        });
        let r = anthropic_to_responses(body).unwrap();
        assert_eq!(r["tool_choice"]["type"], "function");
        assert_eq!(r["tool_choice"]["name"], "get_weather");
    }

    #[test]
    fn tool_use_lifted_to_top_level_function_call() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me check"},
                    {"type": "tool_use", "id": "call_123", "name": "get_weather", "input": {"location": "Tokyo"}}
                ]
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        let arr = r["input"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "assistant");
        assert_eq!(arr[0]["content"][0]["type"], "output_text");
        assert_eq!(arr[1]["type"], "function_call");
        assert_eq!(arr[1]["call_id"], "call_123");
        assert_eq!(arr[1]["name"], "get_weather");
    }

    #[test]
    fn tool_result_lifted_to_function_call_output() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [{"type": "tool_result", "tool_use_id": "call_123", "content": "Sunny, 25°C"}]
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        let arr = r["input"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["type"], "function_call_output");
        assert_eq!(arr[0]["call_id"], "call_123");
        assert_eq!(arr[0]["output"], "Sunny, 25°C");
    }

    #[test]
    fn thinking_blocks_discarded() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "Internal..."},
                    {"type": "text", "text": "Answer"}
                ]
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        let arr = r["input"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["content"][0]["type"], "output_text");
        assert_eq!(arr[0]["content"][0]["text"], "Answer");
    }

    #[test]
    fn image_block_converted_to_input_image() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is this?"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "abc123"}}
                ]
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        let content = r["input"][0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[1]["type"], "input_image");
        assert_eq!(content[1]["image_url"], "data:image/png;base64,abc123");
    }

    #[test]
    fn cache_control_stripped_from_text_blocks() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": "Hello", "cache_control": {"type": "ephemeral"}}]
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert!(r["input"][0]["content"][0].get("cache_control").is_none());
    }

    #[test]
    fn cache_control_stripped_from_tools() {
        let body = json!({
            "model": "gpt-4o",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "t",
                "input_schema": {},
                "cache_control": {"type": "ephemeral"}
            }]
        });
        let r = anthropic_to_responses(body).unwrap();
        assert!(r["tools"][0].get("cache_control").is_none());
    }

    #[test]
    fn responses_simple_text_to_anthropic() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hello!"}]}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["id"], "resp_1");
        assert_eq!(r["type"], "message");
        assert_eq!(r["content"][0]["type"], "text");
        assert_eq!(r["content"][0]["text"], "Hello!");
        assert_eq!(r["stop_reason"], "end_turn");
        assert_eq!(r["usage"]["input_tokens"], 10);
        assert_eq!(r["usage"]["output_tokens"], 5);
    }

    #[test]
    fn responses_model_can_be_overridden() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hello!"}]}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let r =
            responses_to_anthropic_with_model_override(body, Some("claude-sonnet-4-5")).unwrap();
        assert_eq!(r["model"], "claude-sonnet-4-5");
    }

    #[test]
    fn responses_function_call_becomes_tool_use() {
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
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["content"][0]["type"], "tool_use");
        assert_eq!(r["content"][0]["id"], "call_123");
        assert_eq!(r["content"][0]["name"], "get_weather");
        assert_eq!(r["content"][0]["input"]["location"], "Tokyo");
        assert_eq!(r["stop_reason"], "tool_use");
    }

    #[test]
    fn responses_refusal_block_becomes_text() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "refusal", "refusal": "No."}]}],
            "usage": {"input_tokens": 5, "output_tokens": 2}
        });
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["content"][0]["type"], "text");
        assert_eq!(r["content"][0]["text"], "No.");
    }

    #[test]
    fn responses_reasoning_becomes_thinking() {
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
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["content"][0]["type"], "thinking");
        assert_eq!(r["content"][0]["thinking"], "Thinking...");
        assert_eq!(r["content"][1]["type"], "text");
        assert_eq!(r["content"][1]["text"], "42");
    }

    #[test]
    fn incomplete_status_maps_to_max_tokens() {
        let body = json!({
            "id": "resp_1",
            "status": "incomplete",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Partial"}]}],
            "usage": {"input_tokens": 10, "output_tokens": 4096}
        });
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["stop_reason"], "max_tokens");
    }

    #[test]
    fn incomplete_non_token_reason_maps_to_end_turn() {
        let body = json!({
            "id": "resp_1",
            "status": "incomplete",
            "incomplete_details": {"reason": "content_filter"},
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "..."}]}],
            "usage": {"input_tokens": 10, "output_tokens": 1}
        });
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["stop_reason"], "end_turn");
    }

    #[test]
    fn cache_tokens_from_nested_details() {
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
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["usage"]["cache_read_input_tokens"], 80);
    }

    #[test]
    fn direct_cache_fields_override_nested_details() {
        let body = json!({
            "id": "resp_1",
            "status": "completed",
            "model": "gpt-4o",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": "Hi"}]}],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 60,
                "cache_creation_input_tokens": 20
            }
        });
        let r = responses_to_anthropic(body).unwrap();
        assert_eq!(r["usage"]["cache_read_input_tokens"], 60);
        assert_eq!(r["usage"]["cache_creation_input_tokens"], 20);
    }

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
    fn no_output_field_returns_error() {
        let body = json!({"id": "resp_1", "status": "completed"});
        assert!(responses_to_anthropic(body).is_err());
    }
}
