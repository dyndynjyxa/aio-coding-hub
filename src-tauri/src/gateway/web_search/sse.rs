//! Anthropic Messages SSE response synthesizer.
//!
//! Builds the exact event sequence that Claude Code's `WebSearchTool`
//! (`src/tools/WebSearchTool/WebSearchTool.ts`) expects to see in a stream
//! after a `web_search_20250305` server-tool call:
//!
//!   message_start (with usage.server_tool_use.web_search_requests = 1)
//!   content_block_start  (type: server_tool_use, id: srvtoolu_*)
//!   content_block_delta  (input_json_delta, partial_json: {"query": "..."})
//!   content_block_stop
//!   content_block_start  (type: web_search_tool_result, tool_use_id, content[])
//!   content_block_stop
//!   message_delta        (usage.server_tool_use.web_search_requests = 1)
//!   message_stop
//!
//! Event payloads are encoded exactly the way the official SDK produces them
//! (one JSON object per `data:` line, terminated by `\n\n`).

use crate::gateway::web_search::backend::SearchHit;
use serde_json::{json, Value};
use std::fmt::Write as _;

/// Synthesize a complete Anthropic Messages SSE response that represents
/// a successful `web_search_20250305` call.
///
/// Returns a `String` containing the entire event stream (UTF-8), suitable
/// for use as the body of an HTTP response with `Content-Type:
/// text/event-stream; charset=utf-8`.
pub fn build_search_success_response(
    request_model: &str,
    trace_id: &str,
    query: &str,
    hits: &[SearchHit],
) -> String {
    let msg_id = format!("msg_aio_ws_{trace_id}");
    let tool_use_id = format!("srvtoolu_{}", short_id(trace_id));
    let mut out = String::with_capacity(1024 + hits.len() * 256);

    // ── message_start ────────────────────────────────────────────────────
    let input_tokens = estimate_input_tokens(query);
    write_event(
        &mut out,
        "message_start",
        &json!({
            "type": "message_start",
            "message": {
                "id": msg_id,
                "type": "message",
                "role": "assistant",
                "model": request_model,
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": input_tokens,
                    "output_tokens": 1,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "server_tool_use": {
                        "web_search_requests": 1
                    }
                }
            }
        }),
    );

    // ── content_block_start: server_tool_use ─────────────────────────────
    write_event(
        &mut out,
        "content_block_start",
        &json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "server_tool_use",
                "id": tool_use_id,
                "name": "web_search",
                "input": {}
            }
        }),
    );

    // ── content_block_delta: input_json_delta ────────────────────────────
    // The model receives this as a synthetic "tool call" with the user query.
    let partial =
        serde_json::to_string(&json!({ "query": query })).expect("static query object serializes");
    write_event(
        &mut out,
        "content_block_delta",
        &json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": partial
            }
        }),
    );

    // ── content_block_stop (server_tool_use) ──────────────────────────────
    write_event(
        &mut out,
        "content_block_stop",
        &json!({"type": "content_block_stop", "index": 0}),
    );

    // ── content_block_start: web_search_tool_result ──────────────────────
    let result_content: Vec<Value> = hits
        .iter()
        .map(|h| {
            json!({
                "type": "web_search_result",
                "title": h.title,
                "url": h.url,
            })
        })
        .collect();

    write_event(
        &mut out,
        "content_block_start",
        &json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": {
                "type": "web_search_tool_result",
                "tool_use_id": tool_use_id,
                "content": result_content
            }
        }),
    );

    write_event(
        &mut out,
        "content_block_stop",
        &json!({"type": "content_block_stop", "index": 1}),
    );

    // ── message_delta ────────────────────────────────────────────────────
    write_event(
        &mut out,
        "message_delta",
        &json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn",
                "stop_sequence": null
            },
            "usage": {
                "output_tokens": 1,
                "server_tool_use": {
                    "web_search_requests": 1
                }
            }
        }),
    );

    // ── message_stop ─────────────────────────────────────────────────────
    write_event(&mut out, "message_stop", &json!({"type": "message_stop"}));

    out
}

/// Synthesize an error response: a `web_search_tool_result` block whose
/// `content` is a `WebSearchToolResultError` object. Claude Code's
/// `WebSearchTool.ts:115-122` already handles this shape, surfacing the
/// error code to the user.
pub fn build_search_error_response(
    request_model: &str,
    trace_id: &str,
    query: &str,
    error_code: &str,
) -> String {
    let msg_id = format!("msg_aio_ws_{trace_id}");
    let tool_use_id = format!("srvtoolu_{}", short_id(trace_id));
    let mut out = String::with_capacity(512);

    let input_tokens = estimate_input_tokens(query);

    write_event(
        &mut out,
        "message_start",
        &json!({
            "type": "message_start",
            "message": {
                "id": msg_id,
                "type": "message",
                "role": "assistant",
                "model": request_model,
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": input_tokens,
                    "output_tokens": 0,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "server_tool_use": {"web_search_requests": 1}
                }
            }
        }),
    );

    write_event(
        &mut out,
        "content_block_start",
        &json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "server_tool_use",
                "id": tool_use_id,
                "name": "web_search",
                "input": {}
            }
        }),
    );

    let partial =
        serde_json::to_string(&json!({ "query": query })).expect("static query object serializes");
    write_event(
        &mut out,
        "content_block_delta",
        &json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": partial
            }
        }),
    );

    write_event(
        &mut out,
        "content_block_stop",
        &json!({"type": "content_block_stop", "index": 0}),
    );

    write_event(
        &mut out,
        "content_block_start",
        &json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": {
                "type": "web_search_tool_result",
                "tool_use_id": tool_use_id,
                "content": {
                    "type": "web_search_tool_result_error",
                    "error_code": error_code
                }
            }
        }),
    );

    write_event(
        &mut out,
        "content_block_stop",
        &json!({"type": "content_block_stop", "index": 1}),
    );

    write_event(
        &mut out,
        "message_delta",
        &json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": {
                "output_tokens": 0,
                "server_tool_use": {"web_search_requests": 1}
            }
        }),
    );

    write_event(&mut out, "message_stop", &json!({"type": "message_stop"}));

    out
}

fn write_event(out: &mut String, event: &str, data: &Value) {
    // SSE wire format: "event: <name>\ndata: <json>\n\n".
    let _ = writeln!(out, "event: {event}");
    let _ = writeln!(out, "data: {data}");
    out.push('\n');
}

fn short_id(trace_id: &str) -> String {
    trace_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(24)
        .collect()
}

/// Best-effort input token estimate. Used purely for `usage.input_tokens` in
/// the synthesized message_start; not used for billing.
fn estimate_input_tokens(query: &str) -> u32 {
    // 1 token ≈ 4 chars for English / CJK; very rough but matches what
    // Claude Code's `WebSearchTool` does internally for its own message.
    let chars = query.chars().count() as u32 + "Perform a web search for the query: ".len() as u32;
    (chars / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_response_contains_required_blocks() {
        let body = build_search_success_response(
            "claude-sonnet-4-5",
            "abc123",
            "rust async",
            &[
                SearchHit {
                    title: "Rust docs".into(),
                    url: "https://doc.rust-lang.org".into(),
                    snippet: "The Rust language".into(),
                    published_at: None,
                },
                SearchHit {
                    title: "Async book".into(),
                    url: "https://rust-lang.github.io/async-book/".into(),
                    snippet: String::new(),
                    published_at: None,
                },
            ],
        );

        assert!(body.contains("event: message_start"));
        assert!(body.contains("event: content_block_start"));
        assert!(body.contains("event: content_block_delta"));
        assert!(body.contains("event: content_block_stop"));
        assert!(body.contains("event: message_delta"));
        assert!(body.contains("event: message_stop"));
        assert!(body.contains("\"type\":\"server_tool_use\""));
        assert!(body.contains("\"type\":\"web_search_tool_result\""));
        assert!(body.contains("\"type\":\"web_search_result\""));
        assert!(body.contains("\"web_search_requests\":1"));
        // The query is emitted inside a JSON-escaped string within the
        // `input_json_delta` payload; verify the raw escaped form so this
        // test does not depend on serde_json's spacing policy.
        assert!(
            body.contains("rust async"),
            "expected the query text to appear in the stream; got:\n{body}"
        );
    }

    #[test]
    fn error_response_uses_error_block() {
        let body =
            build_search_error_response("claude-sonnet-4-5", "abc123", "rust", "too_many_requests");
        assert!(body.contains("web_search_tool_result_error"));
        assert!(body.contains("too_many_requests"));
    }

    #[test]
    fn empty_hits_still_emits_a_result_block() {
        let body =
            build_search_success_response("claude-sonnet-4-5", "tid", "no results query", &[]);
        // An empty result block is still emitted so the parser sees the
        // server_tool_use → web_search_tool_result pairing.
        assert!(body.contains("\"type\":\"web_search_tool_result\""));
        assert!(body.contains("\"content\":[]"));
    }
}
