# Protocol Bridge — Extensible CLI Protocol Translation Framework

## Overview

The Protocol Bridge framework translates between different AI CLI protocols
(Anthropic Messages API, OpenAI Responses API, Gemini, etc.) using a typed
Intermediate Representation (IR) with Inbound/Outbound adapter pairs.

```
Client JSON → Inbound.request_to_ir() → IR → Outbound.ir_to_request() → Provider JSON
Client JSON ← Inbound.ir_to_response() ← IR ← Outbound.response_to_ir() ← Provider JSON
Client SSE  ← Inbound.ir_chunk_to_sse() ← IR ← Outbound.sse_event_to_ir() ← Provider SSE
```

## Module Structure

```
protocol_bridge/
├── mod.rs           # Public API, re-exports
├── ir.rs            # Typed Intermediate Representation
├── traits.rs        # Inbound / Outbound / ModelMapper traits
├── bridge.rs        # Bridge compositor (Inbound + Outbound + ModelMapper)
├── stream.rs        # BridgeStream — unified SSE stream translator
├── registry.rs      # Bridge type registry + factory functions
├── e2e_tests.rs     # End-to-end integration tests
│
├── inbound/         # Client-facing adapters
│   └── anthropic.rs # Anthropic Messages API ↔ IR
│
├── outbound/        # Provider-facing adapters
│   └── openai_responses.rs  # OpenAI Responses API ↔ IR
│
└── cx2cc/           # CX2CC-specific configuration
    └── mod.rs       # Model mapper + ChatGPT compat helpers
```

## Adding a New Protocol Pair

### Step 1: Implement an Adapter

If the **client protocol** is new, implement `Inbound`:

```rust
// inbound/new_protocol.rs
pub(crate) struct NewProtocolInbound;

impl Inbound for NewProtocolInbound {
    fn protocol(&self) -> &'static str { "new_protocol" }
    fn request_to_ir(&self, body: Value) -> Result<InternalRequest, BridgeError> { ... }
    fn ir_to_response(&self, ir: &InternalResponse, ctx: &BridgeContext) -> Result<Value, BridgeError> { ... }
    fn ir_chunk_to_sse(&self, chunk: &IRStreamChunk, ctx: &BridgeContext) -> Result<Vec<Bytes>, BridgeError> { ... }
}
```

If the **provider protocol** is new, implement `Outbound`:

```rust
// outbound/new_provider.rs
pub(crate) struct NewProviderOutbound;

impl Outbound for NewProviderOutbound {
    fn protocol(&self) -> &'static str { "new_provider" }
    fn target_path(&self) -> &str { "/v1/new_endpoint" }
    fn ir_to_request(&self, ir: &InternalRequest, ctx: &BridgeContext) -> Result<Value, BridgeError> { ... }
    fn response_to_ir(&self, body: Value) -> Result<InternalResponse, BridgeError> { ... }
    fn sse_event_to_ir(&self, event_type: &str, data: &Value, state: &mut StreamState) -> Result<Vec<IRStreamChunk>, BridgeError> { ... }
}
```

### Step 2: Implement a ModelMapper

```rust
pub(crate) struct NewBridgeModelMapper;
impl ModelMapper for NewBridgeModelMapper {
    fn map(&self, source_model: &str, ctx: &BridgeContext) -> String { ... }
}
```

### Step 3: Register in `registry.rs`

```rust
fn new_bridge_factory() -> Bridge {
    Bridge {
        bridge_type: "new_bridge",
        inbound: Box::new(AnthropicMessagesInbound),  // reuse existing!
        outbound: Box::new(NewProviderOutbound),       // new
        model_mapper: Box::new(NewBridgeModelMapper),  // new
    }
}

// In registry initialization:
m.insert("new_bridge", new_bridge_factory as BridgeFactory);
```

### Step 4: Add `bridge_type` to DB

The `bridge_type` column in the `providers` table stores the identifier.
Add a data migration if needed.

## N+M Scaling

With 3 client protocols and 3 provider protocols:
- **Without IR**: 3 × 3 = 9 translators needed
- **With IR**: 3 Inbound + 3 Outbound = 6 adapters (each independently testable)

## Key Types

| Type | Purpose |
|------|---------|
| `InternalRequest` | Protocol-agnostic LLM request |
| `InternalResponse` | Protocol-agnostic LLM response |
| `IRStreamChunk` | Single streaming event in IR form |
| `IRContentBlock` | Text / Image / ToolUse / ToolResult / Thinking |
| `BridgeContext` | Runtime config (model mapping, stream mode) |
| `StreamState` | Mutable state across SSE events |

## Testing

```bash
# All protocol_bridge tests (unit + e2e)
cargo test --lib -- protocol_bridge

# E2E integration tests only
cargo test --lib -- protocol_bridge::e2e_tests
```
