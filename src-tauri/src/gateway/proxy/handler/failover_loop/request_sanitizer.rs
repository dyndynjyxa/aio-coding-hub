//! Request body sanitization before sending upstream.
//!
//! This module owns request-body cleaning logic that is independent of
//! authentication concerns.  For example, Claude OAuth upstreams reject
//! messages that contain empty text blocks, so this module strips them
//! before the request is forwarded.

use super::provider_iterator::PreparedProvider;
use crate::gateway::proxy::request_context::RequestContext;
use axum::body::Bytes;

/// Clean request body (e.g. remove empty text blocks for Claude OAuth).
pub(super) fn clean_body(input: &RequestContext, prepared: &PreparedProvider) -> Bytes {
    if input.cli_key == "claude" && prepared.oauth_adapter.is_some() {
        if let Ok(mut json) =
            serde_json::from_slice::<serde_json::Value>(&prepared.upstream_body_bytes)
        {
            if let Some(messages) = json.get_mut("messages").and_then(|v| v.as_array_mut()) {
                for msg in messages {
                    if let Some(content) = msg.get_mut("content").and_then(|v| v.as_array_mut()) {
                        content.retain(|block| {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                !text.trim().is_empty()
                            } else {
                                true
                            }
                        });
                    }
                }
            }
            return serde_json::to_vec(&json)
                .unwrap_or_else(|_| prepared.upstream_body_bytes.to_vec())
                .into();
        }
    }
    prepared.upstream_body_bytes.clone()
}
