//! Bridge compositor: combines an Inbound + Outbound + ModelMapper into a
//! single translation unit that handles the full request → IR → provider
//! round-trip.

use super::traits::*;
use serde_json::Value;

/// A fully assembled protocol bridge (Inbound + Outbound + ModelMapper).
#[allow(dead_code)]
pub(crate) struct Bridge {
    pub bridge_type: &'static str,
    pub inbound: Box<dyn Inbound>,
    pub outbound: Box<dyn Outbound>,
    pub model_mapper: Box<dyn ModelMapper>,
}

/// Result of translating a client request into a provider request.
pub(crate) struct TranslatedRequest {
    /// Provider-facing request JSON body.
    pub body: Value,
    /// Target path to forward to (e.g. `"/v1/responses"`).
    pub target_path: String,
    /// Model name after mapping (for embedding back in the response).
    pub original_model: String,
}

impl Bridge {
    // ── Request path ────────────────────────────────────────────────────

    /// Full request translation: Client JSON → IR → Provider JSON.
    pub fn translate_request(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<TranslatedRequest, BridgeError> {
        // 1. Client body → IR
        let mut ir = self.inbound.request_to_ir(body, ctx)?;

        // 2. Model mapping
        let original_model = ir.model.clone();
        ir.model = self.model_mapper.map(&ir.model, ctx);

        // 3. IR → Provider body
        let provider_body = self.outbound.ir_to_request(&ir, ctx)?;

        Ok(TranslatedRequest {
            body: provider_body,
            target_path: self.outbound.target_path().to_string(),
            original_model,
        })
    }

    // ── Non-stream response path ────────────────────────────────────────

    /// Full non-stream response translation: Provider JSON → IR → Client JSON.
    pub fn translate_response(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<Value, BridgeError> {
        let ir = self.outbound.response_to_ir(body, ctx)?;
        self.inbound.ir_to_response(&ir, ctx)
    }

    /// Translate a non-stream provider response into synthesized client SSE
    /// bytes.  Used when the upstream returned JSON but the client requested
    /// streaming.
    pub fn translate_response_to_sse(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<axum::body::Bytes, BridgeError> {
        use super::ir::*;

        let ir = self.outbound.response_to_ir(body, ctx)?;
        let mut frames: Vec<u8> = Vec::new();

        // message_start (with initial usage so input_tokens is preserved)
        let start = IRStreamChunk::MessageStart {
            id: ir.id.clone(),
            model: ir.model.clone(),
            initial_usage: Some(ir.usage.clone()),
        };
        for b in self.inbound.ir_chunk_to_sse(&start, ctx)? {
            frames.extend_from_slice(&b);
        }

        // content blocks
        for (idx, block) in ir.content.iter().enumerate() {
            let index = idx as u32;
            let block_type = match block {
                IRContentBlock::Text { .. } => IRBlockType::Text,
                IRContentBlock::ToolUse { id, name, .. } => IRBlockType::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                },
                IRContentBlock::Thinking { .. } => IRBlockType::Thinking,
                _ => continue,
            };

            // block start
            for b in self
                .inbound
                .ir_chunk_to_sse(&IRStreamChunk::ContentBlockStart { index, block_type }, ctx)?
            {
                frames.extend_from_slice(&b);
            }

            // block delta
            let delta = match block {
                IRContentBlock::Text { text } => Some(IRDelta::TextDelta { text: text.clone() }),
                IRContentBlock::ToolUse { input, .. } => {
                    let json_str =
                        serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string());
                    Some(IRDelta::InputJsonDelta {
                        partial_json: json_str,
                    })
                }
                IRContentBlock::Thinking { thinking } => Some(IRDelta::ThinkingDelta {
                    thinking: thinking.clone(),
                }),
                _ => None,
            };
            if let Some(delta) = delta {
                for b in self
                    .inbound
                    .ir_chunk_to_sse(&IRStreamChunk::ContentBlockDelta { index, delta }, ctx)?
                {
                    frames.extend_from_slice(&b);
                }
            }

            // block stop
            for b in self
                .inbound
                .ir_chunk_to_sse(&IRStreamChunk::ContentBlockStop { index }, ctx)?
            {
                frames.extend_from_slice(&b);
            }
        }

        // message_delta + message_stop
        for b in self.inbound.ir_chunk_to_sse(
            &IRStreamChunk::MessageDelta {
                stop_reason: ir.stop_reason,
                usage: ir.usage,
            },
            ctx,
        )? {
            frames.extend_from_slice(&b);
        }
        for b in self
            .inbound
            .ir_chunk_to_sse(&IRStreamChunk::MessageStop, ctx)?
        {
            frames.extend_from_slice(&b);
        }

        Ok(axum::body::Bytes::from(frames))
    }

    // ── Stream response path ────────────────────────────────────────────

    /// Create a streaming translator that processes upstream SSE events
    /// one-by-one through the Outbound → IR → Inbound pipeline.
    pub fn create_stream_translator(&self) -> StreamTranslator<'_> {
        StreamTranslator {
            inbound: &*self.inbound,
            outbound: &*self.outbound,
            state: StreamState::default(),
        }
    }

    // ── Delegate helpers ────────────────────────────────────────────────

    /// Delegate to `Outbound::compat_filter`.
    pub fn compat_filter(&self, body: &mut Value, ctx: &BridgeContext) {
        self.outbound.compat_filter(body, ctx);
    }
}

/// Per-stream translator that holds mutable `StreamState`.
pub(crate) struct StreamTranslator<'b> {
    inbound: &'b dyn Inbound,
    outbound: &'b dyn Outbound,
    state: StreamState,
}

impl<'b> StreamTranslator<'b> {
    /// Translate a single upstream SSE event into client-facing SSE bytes.
    pub fn translate_event(
        &mut self,
        event_type: &str,
        data: &Value,
        ctx: &BridgeContext,
    ) -> Result<Vec<axum::body::Bytes>, BridgeError> {
        self.state.enable_reasoning_to_thinking = ctx.cx2cc_settings.enable_reasoning_to_thinking;
        // Upstream SSE → IR chunks
        let ir_chunks = self
            .outbound
            .sse_event_to_ir(event_type, data, &mut self.state)?;

        // IR chunks → client SSE bytes
        let mut output = Vec::new();
        for chunk in &ir_chunks {
            let mut frames = self.inbound.ir_chunk_to_sse(chunk, ctx)?;
            output.append(&mut frames);
        }
        Ok(output)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::proxy::protocol_bridge::ir::*;
    use axum::body::Bytes;
    use serde_json::json;

    // Minimal stub implementations for testing the Bridge compositor logic.

    struct StubInbound;
    impl Inbound for StubInbound {
        fn protocol(&self) -> &'static str {
            "stub_in"
        }
        fn request_to_ir(
            &self,
            body: Value,
            _ctx: &BridgeContext,
        ) -> Result<InternalRequest, BridgeError> {
            Ok(InternalRequest {
                model: body
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                messages: vec![],
                system: body
                    .get("system")
                    .and_then(|s| s.as_str())
                    .map(String::from),
                tools: vec![],
                tool_choice: None,
                max_tokens: None,
                temperature: None,
                top_p: None,
                stop_sequences: vec![],
                stream: false,
                metadata: IRMetadata::default(),
            })
        }
        fn ir_to_response(
            &self,
            ir: &InternalResponse,
            _ctx: &BridgeContext,
        ) -> Result<Value, BridgeError> {
            Ok(json!({ "id": ir.id, "model": ir.model, "type": "stub_response" }))
        }
        fn ir_chunk_to_sse(
            &self,
            _chunk: &IRStreamChunk,
            _ctx: &BridgeContext,
        ) -> Result<Vec<Bytes>, BridgeError> {
            Ok(vec![Bytes::from("event: stub\ndata: {}\n\n")])
        }
    }

    struct StubOutbound;
    impl Outbound for StubOutbound {
        fn protocol(&self) -> &'static str {
            "stub_out"
        }
        fn target_path(&self) -> &str {
            "/v1/stub"
        }
        fn ir_to_request(
            &self,
            ir: &InternalRequest,
            _ctx: &BridgeContext,
        ) -> Result<Value, BridgeError> {
            Ok(json!({ "model": ir.model, "type": "stub_request" }))
        }
        fn response_to_ir(
            &self,
            body: Value,
            _ctx: &BridgeContext,
        ) -> Result<InternalResponse, BridgeError> {
            Ok(InternalResponse {
                id: body
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("resp-1")
                    .to_string(),
                model: body
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stub")
                    .to_string(),
                content: vec![IRContentBlock::Text {
                    text: "hello".into(),
                }],
                stop_reason: IRStopReason::EndTurn,
                usage: IRUsage::default(),
            })
        }
        fn sse_event_to_ir(
            &self,
            _event_type: &str,
            _data: &Value,
            _state: &mut StreamState,
        ) -> Result<Vec<IRStreamChunk>, BridgeError> {
            Ok(vec![IRStreamChunk::Ping])
        }
    }

    struct IdentityMapper;
    impl ModelMapper for IdentityMapper {
        fn map(&self, source_model: &str, _ctx: &BridgeContext) -> String {
            format!("mapped-{source_model}")
        }
    }

    fn stub_bridge() -> Bridge {
        Bridge {
            bridge_type: "test",
            inbound: Box::new(StubInbound),
            outbound: Box::new(StubOutbound),
            model_mapper: Box::new(IdentityMapper),
        }
    }

    fn stub_ctx() -> BridgeContext {
        BridgeContext {
            claude_models: crate::domain::providers::ClaudeModels::default(),
            cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings::default(),
            requested_model: Some("claude-sonnet".into()),
            mapped_model: None,
            stream_requested: false,
            is_chatgpt_backend: false,
        }
    }

    #[test]
    fn translate_request_maps_model_and_delegates() {
        let bridge = stub_bridge();
        let ctx = stub_ctx();
        let body = json!({ "model": "claude-sonnet-4", "system": "Be helpful" });

        let result = bridge.translate_request(body, &ctx).unwrap();
        assert_eq!(result.target_path, "/v1/stub");
        assert_eq!(result.original_model, "claude-sonnet-4");
        assert_eq!(
            result.body.get("model").unwrap().as_str().unwrap(),
            "mapped-claude-sonnet-4"
        );
    }

    #[test]
    fn translate_response_round_trips() {
        let bridge = stub_bridge();
        let ctx = stub_ctx();
        let provider_resp = json!({ "id": "resp-42", "model": "gpt-4.1" });

        let client_resp = bridge.translate_response(provider_resp, &ctx).unwrap();
        assert_eq!(client_resp.get("id").unwrap().as_str().unwrap(), "resp-42");
    }

    #[test]
    fn stream_translator_produces_bytes() {
        let bridge = stub_bridge();
        let ctx = stub_ctx();
        let mut translator = bridge.create_stream_translator();

        let frames = translator
            .translate_event("response.created", &json!({}), &ctx)
            .unwrap();
        assert!(!frames.is_empty());
    }
}
