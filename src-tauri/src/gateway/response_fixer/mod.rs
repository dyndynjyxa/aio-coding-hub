mod audit;
mod encoding;
mod json;
mod sse;
mod stream;

use axum::body::Bytes;
use futures_core::Stream;
use serde_json::{Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use crate::shared::mutex_ext::MutexExt;

pub(super) const DEFAULT_MAX_JSON_DEPTH: usize = 200;
pub(super) const DEFAULT_MAX_FIX_SIZE: usize = 1024 * 1024;
pub(super) const SPECIAL_SETTINGS_MAX_ENTRIES: usize = 64;
const SPECIAL_SETTINGS_STRING_PREVIEW_BYTES: usize = 512;
const SPECIAL_SETTINGS_ARRAY_MAX_ITEMS: usize = 32;
const SPECIAL_SETTINGS_OBJECT_MAX_FIELDS: usize = 64;
const SPECIAL_SETTINGS_ENTRY_MAX_BYTES: usize = 4 * 1024;
const SPECIAL_SETTINGS_JSON_MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) struct ResponseFixerConfig {
    pub(super) fix_encoding: bool,
    pub(super) fix_sse_format: bool,
    pub(super) fix_truncated_json: bool,
    pub(super) max_json_depth: usize,
    pub(super) max_fix_size: usize,
}

#[derive(Debug)]
pub(super) struct NonStreamFixOutcome {
    pub(super) body: Bytes,
    pub(super) header_value: &'static str,
    pub(super) special_setting: Option<Value>,
}

pub(super) fn special_settings_json(shared: &Arc<Mutex<Vec<Value>>>) -> Option<String> {
    let settings = shared.lock().ok()?.clone();
    special_settings_json_from_values(settings)
}

pub(super) fn special_settings_json_from_values(settings: Vec<Value>) -> Option<String> {
    if settings.is_empty() {
        return None;
    }
    let original_len = settings.len();
    let mut capped: Vec<Value> = settings
        .into_iter()
        .take(SPECIAL_SETTINGS_MAX_ENTRIES)
        .map(bound_special_setting)
        .collect();

    if original_len > SPECIAL_SETTINGS_MAX_ENTRIES {
        mark_special_settings_truncated(&mut capped);
    }

    Some(encode_special_settings_capped(&capped))
}

pub(super) fn push_special_setting(shared: &Arc<Mutex<Vec<Value>>>, setting: Value) {
    let setting = bound_special_setting(setting);
    let mut guard = shared.lock_or_recover();
    push_special_setting_locked(&mut guard, setting);
}

fn push_special_setting_locked(settings: &mut Vec<Value>, setting: Value) {
    if settings.len() < SPECIAL_SETTINGS_MAX_ENTRIES {
        settings.push(setting);
        return;
    }

    if settings.len() > SPECIAL_SETTINGS_MAX_ENTRIES {
        settings.truncate(SPECIAL_SETTINGS_MAX_ENTRIES);
    }
    mark_special_settings_truncated(settings);
}

fn encode_special_settings_capped(settings: &[Value]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(settings.len());

    for (idx, setting) in settings.iter().enumerate() {
        let encoded = serde_json::to_string(setting).unwrap_or_else(|_| "{}".to_string());
        parts.push(encoded);
        if encoded_array_len(&parts) <= SPECIAL_SETTINGS_JSON_MAX_BYTES {
            continue;
        }

        parts.pop();
        push_encoded_truncation_marker(&mut parts, settings.len().saturating_sub(idx));
        break;
    }

    format!("[{}]", parts.join(","))
}

fn encoded_array_len(parts: &[String]) -> usize {
    2 + parts.iter().map(String::len).sum::<usize>() + parts.len().saturating_sub(1)
}

fn push_encoded_truncation_marker(parts: &mut Vec<String>, omitted_entries: usize) {
    let marker = serde_json::json!({
        "type": "special_settings_truncated",
        "scope": "request",
        "reason": "encoded_json_too_large",
        "maxBytes": SPECIAL_SETTINGS_JSON_MAX_BYTES,
        "omittedEntries": omitted_entries,
    });
    let encoded = serde_json::to_string(&marker).unwrap_or_else(|_| "{}".to_string());

    while !parts.is_empty() {
        let next_len = encoded_array_len(parts) + encoded.len() + 1;
        if next_len <= SPECIAL_SETTINGS_JSON_MAX_BYTES {
            break;
        }
        parts.pop();
    }

    if encoded_array_len(parts) + encoded.len() + usize::from(!parts.is_empty())
        <= SPECIAL_SETTINGS_JSON_MAX_BYTES
    {
        parts.push(encoded);
    }
}

fn mark_special_settings_truncated(settings: &mut Vec<Value>) {
    let marker = serde_json::json!({
        "type": "special_settings_truncated",
        "scope": "request",
        "maxEntries": SPECIAL_SETTINGS_MAX_ENTRIES,
    });

    if let Some(last) = settings.last_mut() {
        if last.get("type").and_then(Value::as_str) != Some("special_settings_truncated") {
            *last = marker;
        }
    } else {
        settings.push(marker);
    }
}

fn bound_special_setting(setting: Value) -> Value {
    let bounded = bound_special_setting_value(setting);
    let encoded = match serde_json::to_vec(&bounded) {
        Ok(encoded) => encoded,
        Err(_) => return oversized_special_setting_marker(None, 0, 0),
    };
    if encoded.len() <= SPECIAL_SETTINGS_ENTRY_MAX_BYTES {
        return bounded;
    }

    oversized_special_setting_marker(Some(&bounded), encoded.len(), hash_bytes(&encoded))
}

fn bound_special_setting_value(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(bound_special_setting_string(&value)),
        Value::Array(values) => {
            let original_len = values.len();
            let mut out: Vec<Value> = values
                .into_iter()
                .take(SPECIAL_SETTINGS_ARRAY_MAX_ITEMS)
                .map(bound_special_setting_value)
                .collect();
            if original_len > SPECIAL_SETTINGS_ARRAY_MAX_ITEMS {
                out.push(serde_json::json!({
                    "type": "special_settings_array_truncated",
                    "maxItems": SPECIAL_SETTINGS_ARRAY_MAX_ITEMS,
                    "omittedItems": original_len - SPECIAL_SETTINGS_ARRAY_MAX_ITEMS,
                }));
            }
            Value::Array(out)
        }
        Value::Object(values) => {
            let original_len = values.len();
            let mut out = Map::new();
            for (idx, (key, value)) in values
                .into_iter()
                .take(SPECIAL_SETTINGS_OBJECT_MAX_FIELDS)
                .enumerate()
            {
                let mut bounded_key = bound_special_setting_string(&key);
                if out.contains_key(&bounded_key) {
                    bounded_key = format!("{bounded_key}_{idx}");
                }
                out.insert(bounded_key, bound_special_setting_value(value));
            }
            if original_len > SPECIAL_SETTINGS_OBJECT_MAX_FIELDS {
                out.insert(
                    "_truncatedFields".to_string(),
                    serde_json::json!({
                        "maxFields": SPECIAL_SETTINGS_OBJECT_MAX_FIELDS,
                        "omittedFields": original_len - SPECIAL_SETTINGS_OBJECT_MAX_FIELDS,
                    }),
                );
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn oversized_special_setting_marker(
    value: Option<&Value>,
    original_bytes: usize,
    hash: u64,
) -> Value {
    let setting_type = value
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .map(bound_special_setting_string)
        .unwrap_or_else(|| "special_setting".to_string());
    let scope = value
        .and_then(|value| value.get("scope"))
        .and_then(Value::as_str)
        .map(bound_special_setting_string);

    let mut marker = Map::new();
    marker.insert("type".to_string(), Value::String(setting_type));
    if let Some(scope) = scope {
        marker.insert("scope".to_string(), Value::String(scope));
    }
    marker.insert("truncated".to_string(), Value::Bool(true));
    marker.insert(
        "reason".to_string(),
        Value::String("special_setting_entry_too_large".to_string()),
    );
    marker.insert(
        "originalBytes".to_string(),
        serde_json::json!(original_bytes as u64),
    );
    marker.insert(
        "maxBytes".to_string(),
        serde_json::json!(SPECIAL_SETTINGS_ENTRY_MAX_BYTES as u64),
    );
    marker.insert("hash".to_string(), Value::String(format!("{hash:016x}")));
    Value::Object(marker)
}

fn bound_special_setting_string(value: &str) -> String {
    if value.len() <= SPECIAL_SETTINGS_STRING_PREVIEW_BYTES {
        return value.to_string();
    }

    let hash = hash_bytes(value.as_bytes());
    format!(
        "{}...[truncated at {}/{} bytes; hash={hash:016x}]",
        utf8_prefix(value, SPECIAL_SETTINGS_STRING_PREVIEW_BYTES),
        SPECIAL_SETTINGS_STRING_PREVIEW_BYTES,
        value.len()
    )
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }

    let mut end = 0usize;
    for (idx, ch) in value.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    &value[..end]
}

fn hash_bytes(input: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn process_non_stream(body: Bytes, config: ResponseFixerConfig) -> NonStreamFixOutcome {
    audit::process_non_stream(body, config)
}

pub(super) struct ResponseFixerStream<S>(stream::ResponseFixerStreamInner<S>)
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin;

impl<S> ResponseFixerStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(super) fn new(
        upstream: S,
        config: ResponseFixerConfig,
        special_settings: Arc<Mutex<Vec<Value>>>,
    ) -> Self {
        Self(stream::ResponseFixerStreamInner::new(
            upstream,
            config,
            special_settings,
        ))
    }
}

impl<S> Stream for ResponseFixerStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        Pin::new(&mut this.0).poll_next(cx)
    }
}

#[cfg(test)]
mod tests;
