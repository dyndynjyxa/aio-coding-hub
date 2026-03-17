//! Protocol Bridge — extensible CLI protocol translation framework.
//!
//! Translates between different AI CLI protocols (Anthropic Messages API,
//! OpenAI Responses API, Gemini, etc.) using an Intermediate Representation
//! (IR) with Inbound/Outbound adapter pairs.
//!
//! # Architecture
//!
//! ```text
//! Client → Inbound.request_to_ir → IR → Outbound.ir_to_request → Provider
//! Client ← Inbound.ir_to_response ← IR ← Outbound.response_to_ir ← Provider
//! ```
//!
//! # Adding a new protocol pair
//!
//! 1. Implement `Inbound` for the client protocol (or reuse existing).
//! 2. Implement `Outbound` for the provider protocol (or reuse existing).
//! 3. Implement `ModelMapper` for the mapping.
//! 4. Register a factory in `registry.rs`.

// Many types/methods in these modules are reserved for future protocol extensions.
#[allow(dead_code)]
pub(crate) mod bridge;
#[allow(dead_code)]
pub(crate) mod ir;
pub(crate) mod registry;
#[allow(dead_code)]
pub(crate) mod stream;
#[allow(dead_code)]
pub(crate) mod traits;

pub(crate) mod cx2cc;
#[allow(dead_code)]
pub(crate) mod inbound;
#[allow(dead_code)]
pub(crate) mod outbound;

#[cfg(test)]
mod e2e_tests;

// Re-export the most commonly used types.
pub(crate) use registry::get_bridge;
pub(crate) use traits::BridgeContext;
