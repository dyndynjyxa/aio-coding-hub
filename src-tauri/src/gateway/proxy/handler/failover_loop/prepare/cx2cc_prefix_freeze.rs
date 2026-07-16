//! Usage: Session-scoped freeze of CX2CC→Grok Responses body prefix fields.
//!
//! xAI prompt cache requires a stable byte-level prefix across turns. Claude
//! Code + CX2CC re-translates Anthropic payloads every request, so tools /
//! instructions / early input items can drift. For Grok-mapped models we pin
//! the first-seen prefix per (session, model) and reuse it when the new
//! request still starts with that prefix (append-only growth).
//!
//! Phase 1: instructions / tools / append-only input.
//! Phase 1.5: also pin top-level control fields that can churn every Claude
//! turn (`tool_choice`, `parallel_tool_calls`, `reasoning`, `text`, `include`,
//! `max_output_tokens`) plus short digests for log/DB diagnosis.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const MAX_ENTRIES: usize = 2000;
const DEFAULT_TTL_SECS: i64 = 6 * 3600; // 6h idle sessions

/// Top-level Responses fields pinned after first seed (session, model).
/// Order is stable for report emission only; freeze is per-key.
const EXTRA_PIN_KEYS: &[&str] = &[
    "tool_choice",
    "parallel_tool_calls",
    "reasoning",
    "text",
    "include",
    "max_output_tokens",
];

#[derive(Debug, Clone)]
struct PrefixEntry {
    /// Pinned top-level JSON fields (`instructions`, `tools`, EXTRA_PIN_KEYS).
    fields: HashMap<String, Value>,
    /// Full input array from the last accepted turn (grows append-only when prefix matches).
    input_prefix: Option<Vec<Value>>,
    expires_at_unix: i64,
}

#[derive(Debug, Default)]
struct PrefixFreezeCache {
    entries: HashMap<String, PrefixEntry>,
}

fn cache() -> &'static Mutex<PrefixFreezeCache> {
    static CACHE: OnceLock<Mutex<PrefixFreezeCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(PrefixFreezeCache::default()))
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cache_key(session_id: &str, model: &str) -> String {
    format!(
        "{}|{}",
        session_id.trim(),
        model.trim().to_ascii_lowercase()
    )
}

fn short_sha16(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn value_sha16(value: Option<&Value>) -> String {
    match value {
        None => "absent".to_string(),
        Some(v) => match serde_json::to_vec(v) {
            Ok(bytes) => short_sha16(&bytes),
            Err(_) => "err".to_string(),
        },
    }
}

fn str_sha16(s: Option<&str>) -> String {
    match s {
        None => "absent".to_string(),
        Some(v) => short_sha16(v.as_bytes()),
    }
}

/// Result of applying prefix freeze to a Responses body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PrefixFreezeReport {
    pub applied: bool,
    pub session_id: String,
    pub model: String,
    pub instructions: &'static str,
    pub tools: &'static str,
    pub input: &'static str,
    /// Compact "key=action,..." for EXTRA_PIN_KEYS.
    pub extras: String,
    pub instructions_sha16: String,
    pub tools_sha16: String,
    pub input_len: usize,
    pub input_prefix_len: usize,
    pub max_output_tokens: Option<String>,
    pub tool_choice_sha16: String,
    pub reasoning_sha16: String,
}

impl PrefixFreezeReport {
    pub fn skipped(reason: &'static str) -> Self {
        Self {
            applied: false,
            session_id: String::new(),
            model: String::new(),
            instructions: reason,
            tools: reason,
            input: reason,
            extras: reason.to_string(),
            instructions_sha16: String::new(),
            tools_sha16: String::new(),
            input_len: 0,
            input_prefix_len: 0,
            max_output_tokens: None,
            tool_choice_sha16: String::new(),
            reasoning_sha16: String::new(),
        }
    }

    /// Compact one-line summary for gateway logs.
    pub fn log_line(&self) -> String {
        format!(
            "session={} model={} instructions={} tools={} input={} extras=[{}] instr_sha={} tools_sha={} input_len={} prefix_len={} max_out={} tool_choice_sha={} reasoning_sha={}",
            self.session_id,
            self.model,
            self.instructions,
            self.tools,
            self.input,
            self.extras,
            self.instructions_sha16,
            self.tools_sha16,
            self.input_len,
            self.input_prefix_len,
            self.max_output_tokens.as_deref().unwrap_or("-"),
            self.tool_choice_sha16,
            self.reasoning_sha16,
        )
    }
}

/// Apply freeze when `model` is a Grok id and `session_id` is non-empty.
pub(super) fn apply_grok_prefix_freeze(
    body: &mut Value,
    session_id: Option<&str>,
    model: Option<&str>,
) -> PrefixFreezeReport {
    let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) else {
        return PrefixFreezeReport::skipped("missing_model");
    };
    if !super::codex_session_id_completion::is_grok_upstream_model(Some(model)) {
        return PrefixFreezeReport::skipped("not_grok_model");
    }
    let Some(session_id) = session_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return PrefixFreezeReport::skipped("missing_session");
    };
    let Some(obj) = body.as_object_mut() else {
        return PrefixFreezeReport::skipped("body_not_object");
    };

    let key = cache_key(session_id, model);
    let now = now_unix();
    let (instructions_action, tools_action, input_action, extras_actions, input_prefix_len) = {
        let mut guard = match cache().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        purge_expired(&mut guard, now);

        let actions = {
            let entry = guard.entries.entry(key).or_insert_with(|| PrefixEntry {
                fields: HashMap::new(),
                input_prefix: None,
                expires_at_unix: now + DEFAULT_TTL_SECS,
            });
            entry.expires_at_unix = now + DEFAULT_TTL_SECS;

            let instructions_action = freeze_json_field(&mut entry.fields, obj, "instructions");
            let tools_action = freeze_json_field(&mut entry.fields, obj, "tools");
            let input_action = freeze_input(entry, obj);
            let extras_actions = freeze_extra_keys(entry, obj);
            let input_prefix_len = entry.input_prefix.as_ref().map(|a| a.len()).unwrap_or(0);
            (
                instructions_action,
                tools_action,
                input_action,
                extras_actions,
                input_prefix_len,
            )
        };

        if guard.entries.len() > MAX_ENTRIES {
            evict_oldest(&mut guard, now);
        }

        actions
    };

    let input_len = obj
        .get("input")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let max_output_tokens = obj.get("max_output_tokens").map(|v| match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    });

    PrefixFreezeReport {
        applied: true,
        session_id: session_id.to_string(),
        model: model.to_string(),
        instructions: instructions_action,
        tools: tools_action,
        input: input_action,
        extras: extras_actions,
        instructions_sha16: str_sha16(obj.get("instructions").and_then(|v| v.as_str())),
        tools_sha16: value_sha16(obj.get("tools")),
        input_len,
        input_prefix_len,
        max_output_tokens,
        tool_choice_sha16: value_sha16(obj.get("tool_choice")),
        reasoning_sha16: value_sha16(obj.get("reasoning")),
    }
}

/// Pin first-seen JSON value for a top-level body key (seed / reuse_exact / reuse_forced).
fn freeze_json_field(
    stored: &mut HashMap<String, Value>,
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
) -> &'static str {
    let current = obj.get(key).cloned();
    match (stored.get(key), current) {
        (Some(frozen), Some(cur)) if frozen == &cur => "reuse_exact",
        (Some(frozen), Some(_)) | (Some(frozen), None) => {
            obj.insert(key.to_string(), frozen.clone());
            "reuse_forced"
        }
        (None, Some(cur)) => {
            stored.insert(key.to_string(), cur);
            "seed"
        }
        (None, None) => "absent",
    }
}

fn freeze_input(entry: &mut PrefixEntry, obj: &mut serde_json::Map<String, Value>) -> &'static str {
    let Some(current_val) = obj.get_mut("input") else {
        return "absent";
    };
    let Some(current) = current_val.as_array_mut() else {
        return "not_array";
    };

    let Some(frozen) = entry.input_prefix.clone() else {
        entry.input_prefix = Some(current.clone());
        return "seed";
    };

    let frozen_len = frozen.len();
    if current.len() >= frozen_len && current.as_slice()[..frozen_len] == frozen[..] {
        // Stable prefix: rewrite head from freeze, keep new tail, then grow freeze.
        let tail: Vec<Value> = current[frozen_len..].to_vec();
        let mut merged = frozen;
        let prefix_len = merged.len();
        merged.extend(tail);
        *current = merged.clone();
        entry.input_prefix = Some(merged);
        if current.len() == prefix_len {
            "reuse_exact"
        } else {
            "reuse_extend"
        }
    } else if current.as_slice() == frozen.as_slice() {
        "reuse_exact"
    } else {
        // History diverged (edit / compact / retranslate drift) — reseed.
        entry.input_prefix = Some(current.clone());
        "bust_reseed"
    }
}

/// Pin first-seen control fields so Claude thinking budget / tool_choice churn
/// does not invalidate the xAI prefix.
fn freeze_extra_keys(entry: &mut PrefixEntry, obj: &mut serde_json::Map<String, Value>) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(EXTRA_PIN_KEYS.len());
    for key in EXTRA_PIN_KEYS {
        let action = freeze_json_field(&mut entry.fields, obj, key);
        parts.push(format!("{key}={action}"));
    }
    parts.join(",")
}

fn purge_expired(cache: &mut PrefixFreezeCache, now: i64) {
    cache.entries.retain(|_, e| e.expires_at_unix > now);
}

fn evict_oldest(cache: &mut PrefixFreezeCache, now: i64) {
    // Drop expired first, then drop arbitrary until under cap.
    purge_expired(cache, now);
    while cache.entries.len() > MAX_ENTRIES {
        if let Some(k) = cache.entries.keys().next().cloned() {
            cache.entries.remove(&k);
        } else {
            break;
        }
    }
}

/// Test helper: clear process cache.
#[cfg(test)]
pub(super) fn clear_for_tests() {
    if let Ok(mut g) = cache().lock() {
        g.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn freezes_instructions_and_tools_on_second_turn() {
        clear_for_tests();
        let session = "sess-prefix-001-aaaaaaaa";
        let model = "grok-4.5";

        let mut turn1 = json!({
            "model": model,
            "instructions": "SYS_V1",
            "tools": [{"type":"function","name":"Read","parameters":{"type":"object"}}],
            "input": [{"role":"user","content":[{"type":"input_text","text":"hi"}]}],
            "stream": true,
            "max_output_tokens": 1024,
            "tool_choice": "auto",
            "reasoning": {"effort": "low"}
        });
        let r1 = apply_grok_prefix_freeze(&mut turn1, Some(session), Some(model));
        assert!(r1.applied);
        assert_eq!(r1.instructions, "seed");
        assert_eq!(r1.tools, "seed");
        assert_eq!(r1.input, "seed");
        assert!(r1.extras.contains("max_output_tokens=seed"));
        assert!(r1.extras.contains("tool_choice=seed"));
        assert!(r1.extras.contains("reasoning=seed"));
        assert_ne!(r1.instructions_sha16, "absent");
        assert_eq!(r1.input_len, 1);
        assert_eq!(r1.input_prefix_len, 1);

        let mut turn2 = json!({
            "model": model,
            "instructions": "SYS_V2_DRIFT",
            "tools": [{"type":"function","name":"Read","parameters":{"type":"object","extra":true}}],
            "input": [
                {"role":"user","content":[{"type":"input_text","text":"hi"}]},
                {"role":"assistant","content":[{"type":"output_text","text":"yo"}]},
                {"role":"user","content":[{"type":"input_text","text":"next"}]}
            ],
            "stream": true,
            "max_output_tokens": 8192,
            "tool_choice": {"type":"function","name":"Read"},
            "reasoning": {"effort": "high"}
        });
        let r2 = apply_grok_prefix_freeze(&mut turn2, Some(session), Some(model));
        assert_eq!(r2.instructions, "reuse_forced");
        assert_eq!(r2.tools, "reuse_forced");
        assert_eq!(r2.input, "reuse_extend");
        assert!(r2.extras.contains("max_output_tokens=reuse_forced"));
        assert!(r2.extras.contains("tool_choice=reuse_forced"));
        assert!(r2.extras.contains("reasoning=reuse_forced"));
        assert_eq!(turn2["instructions"], "SYS_V1");
        assert_eq!(
            turn2["tools"],
            json!([{"type":"function","name":"Read","parameters":{"type":"object"}}])
        );
        assert_eq!(turn2["max_output_tokens"], 1024);
        assert_eq!(turn2["tool_choice"], "auto");
        assert_eq!(turn2["reasoning"], json!({"effort": "low"}));
        // Prefix item must match frozen turn1 first message
        assert_eq!(
            turn2["input"][0],
            json!({"role":"user","content":[{"type":"input_text","text":"hi"}]})
        );
        assert_eq!(turn2["input"].as_array().unwrap().len(), 3);
        assert_eq!(r2.input_len, 3);
        assert_eq!(r2.input_prefix_len, 3);
        // Digests should match post-freeze stable content across turns for instructions/tools
        assert_eq!(r1.instructions_sha16, r2.instructions_sha16);
        assert_eq!(r1.tools_sha16, r2.tools_sha16);
    }

    #[test]
    fn skips_non_grok_models() {
        clear_for_tests();
        let mut body = json!({"model":"gpt-4.1","instructions":"x","input":[]});
        let r = apply_grok_prefix_freeze(&mut body, Some("sess"), Some("gpt-4.1"));
        assert!(!r.applied);
        assert_eq!(r.instructions, "not_grok_model");
    }

    #[test]
    fn input_bust_reseeds_when_history_diverges() {
        clear_for_tests();
        let session = "sess-bust-001-bbbbbbbb";
        let model = "grok-4.5";
        let mut t1 = json!({
            "model": model,
            "input": [{"role":"user","content":[{"type":"input_text","text":"a"}]}]
        });
        apply_grok_prefix_freeze(&mut t1, Some(session), Some(model));

        let mut t2 = json!({
            "model": model,
            "input": [{"role":"user","content":[{"type":"input_text","text":"DIFFERENT"}]}]
        });
        let r = apply_grok_prefix_freeze(&mut t2, Some(session), Some(model));
        assert_eq!(r.input, "bust_reseed");
        assert_eq!(t2["input"][0]["content"][0]["text"], "DIFFERENT");
    }

    #[test]
    fn extras_absent_when_fields_missing() {
        clear_for_tests();
        let session = "sess-extras-absent-cccc";
        let model = "grok-4.5";
        let mut body = json!({
            "model": model,
            "instructions": "sys",
            "input": []
        });
        let r = apply_grok_prefix_freeze(&mut body, Some(session), Some(model));
        assert!(r.applied);
        assert!(r.extras.contains("max_output_tokens=absent"));
        assert!(r.extras.contains("tool_choice=absent"));
        assert_eq!(r.max_output_tokens, None);
        assert_eq!(r.tool_choice_sha16, "absent");
    }
}
