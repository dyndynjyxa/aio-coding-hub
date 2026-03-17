//! Bridge type registry.
//!
//! Maps `bridge_type` strings (e.g. `"cx2cc"`) to factory functions that
//! produce fully assembled [`Bridge`] instances.

use super::bridge::Bridge;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

type BridgeFactory = fn() -> Bridge;

fn registry() -> &'static RwLock<HashMap<&'static str, BridgeFactory>> {
    static REGISTRY: OnceLock<RwLock<HashMap<&'static str, BridgeFactory>>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("cx2cc", cx2cc_factory as BridgeFactory);
        RwLock::new(m)
    })
}

/// Look up a bridge by type identifier and construct it.
pub(crate) fn get_bridge(bridge_type: &str) -> Option<Bridge> {
    registry().read().ok()?.get(bridge_type).map(|f| f())
}

/// Return the list of all registered bridge type identifiers.
#[allow(dead_code)]
pub(crate) fn available_bridge_types() -> Vec<&'static str> {
    registry()
        .read()
        .ok()
        .map(|r| r.keys().copied().collect())
        .unwrap_or_default()
}

// ─── Factory functions ──────────────────────────────────────────────────────

fn cx2cc_factory() -> Bridge {
    Bridge {
        bridge_type: "cx2cc",
        inbound: Box::new(super::inbound::anthropic::AnthropicMessagesInbound),
        outbound: Box::new(super::outbound::openai_responses::OpenAIResponsesOutbound),
        model_mapper: Box::new(super::cx2cc::CX2CCModelMapper),
    }
}
