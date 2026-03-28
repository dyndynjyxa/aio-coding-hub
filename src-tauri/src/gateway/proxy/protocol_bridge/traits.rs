//! Inbound / Outbound adapter traits and shared context types.
//!
//! - **Inbound**: faces the *client* (e.g. Claude Code CLI).
//!   Converts client requests → IR and IR → client responses.
//!
//! - **Outbound**: faces the *upstream provider* (e.g. OpenAI).
//!   Converts IR → provider requests and provider responses → IR.

use super::ir::*;
use axum::body::Bytes;
use serde_json::Value;
use std::collections::HashMap;

// ─── Inbound Trait ──────────────────────────────────────────────────────────

/// Adapter facing the client that initiated the request.
pub(crate) trait Inbound: Send + Sync {
    /// Protocol identifier, e.g. `"anthropic_messages"`.
    fn protocol(&self) -> &'static str;

    /// Parse a client request JSON body into the IR.
    fn request_to_ir(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<InternalRequest, BridgeError>;

    /// Render an IR response as client-facing JSON (non-stream path).
    fn ir_to_response(
        &self,
        ir: &InternalResponse,
        ctx: &BridgeContext,
    ) -> Result<Value, BridgeError>;

    /// Render an IR stream chunk as one or more SSE frame bytes (stream path).
    fn ir_chunk_to_sse(
        &self,
        chunk: &IRStreamChunk,
        ctx: &BridgeContext,
    ) -> Result<Vec<Bytes>, BridgeError>;
}

// ─── Outbound Trait ─────────────────────────────────────────────────────────

/// Adapter facing the upstream AI provider.
pub(crate) trait Outbound: Send + Sync {
    /// Protocol identifier, e.g. `"openai_responses"`.
    fn protocol(&self) -> &'static str;

    /// Target API path to forward to, e.g. `"/v1/responses"`.
    fn target_path(&self) -> &str;

    /// Render an IR request as provider-facing JSON.
    fn ir_to_request(
        &self,
        ir: &InternalRequest,
        ctx: &BridgeContext,
    ) -> Result<Value, BridgeError>;

    /// Parse a provider non-stream JSON response into the IR.
    fn response_to_ir(
        &self,
        body: Value,
        ctx: &BridgeContext,
    ) -> Result<InternalResponse, BridgeError>;

    /// Parse a single upstream SSE event into zero or more IR stream chunks.
    ///
    /// A single upstream event (e.g. `response.completed`) may produce multiple
    /// IR chunks (e.g. `ContentBlockStop` + `MessageDelta` + `MessageStop`).
    fn sse_event_to_ir(
        &self,
        event_type: &str,
        data: &Value,
        state: &mut StreamState,
    ) -> Result<Vec<IRStreamChunk>, BridgeError>;

    /// Optional provider-specific request compatibility filter.
    ///
    /// Called *after* `ir_to_request` to strip or force fields that a specific
    /// backend variant requires (e.g. ChatGPT backend field whitelist).
    fn compat_filter(&self, _body: &mut Value, _ctx: &BridgeContext) {}
}

// ─── Model Mapper Trait ─────────────────────────────────────────────────────

/// Maps model names between source and target protocols.
pub(crate) trait ModelMapper: Send + Sync {
    /// Map a client-facing model name to a provider-facing model name.
    fn map(&self, source_model: &str, ctx: &BridgeContext) -> String;
}

// ─── Context & State ────────────────────────────────────────────────────────

/// Runtime context for a single bridge translation (immutable within one attempt).
#[derive(Debug, Clone)]
pub(crate) struct BridgeContext {
    /// Provider-level model mapping configuration.
    pub claude_models: crate::domain::providers::ClaudeModels,
    /// CX2CC runtime settings for request/response translation.
    pub cx2cc_settings: crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
    /// Original model name from the client request (before mapping).
    pub requested_model: Option<String>,
    /// Model name after mapping (set after `translate_request`).
    pub mapped_model: Option<String>,
    /// Whether the original client request asked for streaming.
    pub stream_requested: bool,
    /// Whether the source provider is a ChatGPT backend (needs compat filter).
    pub is_chatgpt_backend: bool,
}

/// Mutable state maintained across SSE events during a single streaming response.
#[derive(Debug)]
pub(crate) struct StreamState {
    /// Monotonically increasing content-block index.
    pub block_index: u32,
    /// Whether a content block is currently open.
    pub block_open: bool,
    /// Whether a tool-use block has been seen in this stream.
    pub saw_tool_use: bool,
    /// The currently active tool-use block identity.
    pub active_tool: Option<ActiveToolState>,
    /// Whether the current text block already emitted visible text.
    pub text_emitted: bool,
    /// Whether *any* visible text has been emitted in this stream.
    pub saw_visible_text: bool,
    /// Whether reasoning should be converted into Anthropic thinking blocks.
    pub enable_reasoning_to_thinking: bool,
    /// Provider-specific extension state.
    pub extra: HashMap<String, Value>,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            block_index: 0,
            block_open: false,
            saw_tool_use: false,
            active_tool: None,
            text_emitted: false,
            saw_visible_text: false,
            enable_reasoning_to_thinking: true,
            extra: HashMap::new(),
        }
    }
}

/// Identity of the currently active tool-use block during streaming.
#[derive(Debug)]
pub(crate) struct ActiveToolState {
    pub id: String,
    pub name: String,
}

// ─── Error ──────────────────────────────────────────────────────────────────

/// Errors produced by bridge adapters.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BridgeError {
    #[error("transform failed: {0}")]
    TransformFailed(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
