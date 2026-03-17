//! Intermediate Representation (IR) types for protocol bridge translation.
//!
//! These types decouple N source protocols from M target protocols into N+M
//! adapters.  Each Inbound adapter converts client requests into IR; each
//! Outbound adapter converts IR into provider requests (and vice-versa for
//! responses).

use serde_json::Value;
use std::collections::HashMap;

// ─── Request IR ─────────────────────────────────────────────────────────────

/// Protocol-agnostic LLM request.
#[derive(Debug, Clone)]
pub(crate) struct InternalRequest {
    pub model: String,
    pub messages: Vec<IRMessage>,
    pub system: Option<String>,
    pub tools: Vec<IRToolDefinition>,
    pub tool_choice: Option<IRToolChoice>,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
    /// Protocol-specific fields that cannot be mapped to a common field.
    pub metadata: IRMetadata,
}

/// A single conversation message.
#[derive(Debug, Clone)]
pub(crate) struct IRMessage {
    pub role: IRRole,
    pub content: Vec<IRContentBlock>,
}

/// Conversation role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IRRole {
    User,
    Assistant,
}

/// Content block within a message.
#[derive(Debug, Clone)]
pub(crate) enum IRContentBlock {
    Text {
        text: String,
    },
    Image {
        media_type: String,
        data: String, // base64
    },
    ToolUse {
        id: String,
        name: String,
        input: Value, // Preserve as Value – tool input schemas are highly variable
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Thinking {
        thinking: String,
    },
}

/// Tool (function) definition.
#[derive(Debug, Clone)]
pub(crate) struct IRToolDefinition {
    pub name: String,
    pub description: Option<String>,
    /// JSON Schema for the tool parameters.
    /// Kept as `Value` because schemas are structurally diverse.
    pub parameters: Value,
}

/// Tool-choice mode.
#[derive(Debug, Clone)]
pub(crate) enum IRToolChoice {
    Auto,
    Required,
    None,
    Specific { name: String },
}

/// Escape hatch for protocol-specific fields that do not have a common mapping.
#[derive(Debug, Clone, Default)]
pub(crate) struct IRMetadata {
    pub extra: HashMap<String, Value>,
}

// ─── Response IR ────────────────────────────────────────────────────────────

/// Protocol-agnostic LLM response.
#[derive(Debug, Clone)]
pub(crate) struct InternalResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<IRContentBlock>,
    pub stop_reason: IRStopReason,
    pub usage: IRUsage,
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IRStopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
    Unknown(String),
}

/// Token usage statistics.
#[derive(Debug, Clone, Default)]
pub(crate) struct IRUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

// ─── Streaming Chunk IR ─────────────────────────────────────────────────────

/// A single chunk in a streaming response (protocol-agnostic).
#[derive(Debug, Clone)]
pub(crate) enum IRStreamChunk {
    /// Stream start.
    MessageStart {
        id: String,
        model: String,
        /// Initial usage (input_tokens) included in the opening event.
        /// `None` during real streaming; `Some` when synthesizing SSE from JSON.
        initial_usage: Option<IRUsage>,
    },
    /// A new content block begins.
    ContentBlockStart { index: u32, block_type: IRBlockType },
    /// Incremental data for an open content block.
    ContentBlockDelta { index: u32, delta: IRDelta },
    /// The open content block is complete.
    ContentBlockStop { index: u32 },
    /// Final usage / stop-reason metadata.
    MessageDelta {
        stop_reason: IRStopReason,
        usage: IRUsage,
    },
    /// Stream end marker.
    MessageStop,
    /// Keep-alive ping.
    Ping,
}

/// Block type for a `ContentBlockStart` chunk.
#[derive(Debug, Clone)]
pub(crate) enum IRBlockType {
    Text,
    ToolUse { id: String, name: String },
    Thinking,
}

/// Delta payload for a `ContentBlockDelta` chunk.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)] // "Delta" suffix is intentional — matches Anthropic API naming
pub(crate) enum IRDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_message_debug_format() {
        let msg = IRMessage {
            role: IRRole::User,
            content: vec![IRContentBlock::Text {
                text: "hello".into(),
            }],
        };
        let dbg = format!("{msg:?}");
        assert!(dbg.contains("User"));
        assert!(dbg.contains("hello"));
    }

    #[test]
    fn ir_usage_default_is_zero() {
        let u = IRUsage::default();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 0);
        assert!(u.cache_creation_input_tokens.is_none());
    }

    #[test]
    fn ir_stop_reason_equality() {
        assert_eq!(IRStopReason::EndTurn, IRStopReason::EndTurn);
        assert_ne!(IRStopReason::EndTurn, IRStopReason::ToolUse);
        assert_eq!(
            IRStopReason::Unknown("foo".into()),
            IRStopReason::Unknown("foo".into())
        );
    }
}
