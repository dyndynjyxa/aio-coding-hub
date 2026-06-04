use axum::http::{HeaderMap, HeaderValue};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const CODEX_SESSION_ID_MIN_LENGTH: usize = 21;
const CODEX_SESSION_ID_MAX_LENGTH: usize = 256;
const DEFAULT_TTL_SECS: i64 = 300;
const MAX_CACHE_ENTRIES: usize = 5000;
const FINGERPRINT_MAX_MESSAGE_SEGMENTS: usize = 3;
const FINGERPRINT_INPUT_MAX_ITEMS: usize = 64;
const FINGERPRINT_CONTENT_PARTS_MAX_ITEMS: usize = 64;
const FINGERPRINT_TEXT_SAMPLE_BYTES: usize = 32 * 1024;
const FINGERPRINT_COMBINED_PREFIX_BYTES: usize = 8 * 1024;
const FINGERPRINT_COMBINED_SUFFIX_BYTES: usize = 8 * 1024;

static UUID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct CacheEntry {
    session_id: String,
    expires_at_unix: i64,
}

#[derive(Debug, Default)]
pub(super) struct CodexSessionIdCache {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone)]
pub(super) struct CodexSessionCompletionResult {
    pub applied: bool,
    pub session_id: String,
    pub source: &'static str,
    pub action: &'static str,
    pub changed_headers: bool,
    pub changed_body: bool,
}

fn normalize_codex_session_id(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() < CODEX_SESSION_ID_MIN_LENGTH {
        return None;
    }
    if trimmed.len() > CODEX_SESSION_ID_MAX_LENGTH {
        return None;
    }

    let allowed = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'));
    if !allowed {
        return None;
    }

    Some(trimmed.to_string())
}

fn header_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn credential_fingerprint_prefix(headers: &HeaderMap) -> Option<String> {
    let api_key = header_string(headers, "x-api-key")
        .or_else(|| header_string(headers, "x-goog-api-key"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(key) = api_key {
        let digest = Sha256::digest(key.as_bytes());
        let hex = format!("{digest:x}");
        return hex.get(..10).map(str::to_string);
    }

    let auth = header_string(headers, "authorization")?;
    let token = auth
        .trim()
        .strip_prefix("Bearer ")
        .or_else(|| auth.trim().strip_prefix("bearer "))
        .unwrap_or(auth.trim())
        .trim();
    if token.is_empty() {
        return None;
    }

    let digest = Sha256::digest(token.as_bytes());
    let hex = format!("{digest:x}");
    hex.get(..10).map(str::to_string)
}

fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(forwarded_for) = header_string(headers, "x-forwarded-for") {
        if let Some(first) = forwarded_for
            .split(',')
            .map(str::trim)
            .find(|v| !v.is_empty())
        {
            return first.to_string();
        }
    }

    header_string(headers, "x-real-ip").unwrap_or_else(|| "unknown".to_string())
}

fn extract_user_agent(headers: &HeaderMap) -> String {
    header_string(headers, "user-agent").unwrap_or_else(|| "unknown".to_string())
}

fn extract_initial_message_text_hash(body: Option<&Value>) -> Option<String> {
    let body = body?;
    let input = body.get("input")?.as_array()?;
    if input.is_empty() {
        return None;
    }

    let mut hasher = Sha256::new();
    let mut segments_added = 0usize;

    for item in input.iter().take(FINGERPRINT_INPUT_MAX_ITEMS) {
        if segments_added >= FINGERPRINT_MAX_MESSAGE_SEGMENTS {
            break;
        }
        let Some(obj) = item.as_object() else {
            continue;
        };

        // Only consider "message" items for conversation fingerprinting.
        if let Some(item_type) = obj.get("type").and_then(|v| v.as_str()) {
            if item_type != "message" {
                continue;
            }
        }

        let content = obj.get("content");
        if let Some(Value::String(s)) = content {
            hash_text_segment(&mut hasher, &mut segments_added, s);
        } else if let Some(Value::Array(parts)) = content {
            let mut sample = FingerprintTextSample::default();
            for part in parts.iter().take(FINGERPRINT_CONTENT_PARTS_MAX_ITEMS) {
                let Some(part_obj) = part.as_object() else {
                    continue;
                };
                let Some(text) = part_obj.get("text").and_then(|v| v.as_str()) else {
                    continue;
                };
                if text.is_empty() {
                    continue;
                }
                sample.append_text(text);
            }
            if parts.len() > FINGERPRINT_CONTENT_PARTS_MAX_ITEMS {
                sample.mark_truncated_parts(parts.len());
            }
            if let Some(text) = sample.into_text() {
                hash_text_segment(&mut hasher, &mut segments_added, &text);
            }
        }
    }

    if segments_added == 0 {
        return None;
    }

    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    Some(hex.get(..16).unwrap_or("").to_string()).filter(|v| !v.is_empty())
}

#[derive(Debug, Default)]
struct FingerprintTextSample {
    prefix: String,
    suffix: String,
    total_bytes: usize,
    truncated_parts: Option<usize>,
}

impl FingerprintTextSample {
    fn append_text(&mut self, raw: &str) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }

        self.total_bytes = self.total_bytes.saturating_add(trimmed.len());
        append_utf8_prefix(&mut self.prefix, trimmed, FINGERPRINT_COMBINED_PREFIX_BYTES);
        append_utf8_suffix(&mut self.suffix, trimmed, FINGERPRINT_COMBINED_SUFFIX_BYTES);
    }

    fn mark_truncated_parts(&mut self, total_parts: usize) {
        self.truncated_parts = Some(total_parts);
    }

    fn into_text(self) -> Option<String> {
        if self.total_bytes == 0 {
            return None;
        }

        let exact = self.truncated_parts.is_none() && self.prefix.len() == self.total_bytes;
        if exact {
            return Some(self.prefix);
        }

        Some(format!(
            "{}|aio_codex_fingerprint_sample:v1:bytes={}:parts={}|{}",
            self.prefix,
            self.total_bytes,
            self.truncated_parts.unwrap_or(0),
            self.suffix
        ))
    }
}

fn hash_text_segment(hasher: &mut Sha256, segments_added: &mut usize, raw: &str) {
    if *segments_added >= FINGERPRINT_MAX_MESSAGE_SEGMENTS {
        return;
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }

    if *segments_added > 0 {
        hasher.update(b"|");
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() <= FINGERPRINT_TEXT_SAMPLE_BYTES {
        hasher.update(bytes);
    } else {
        let edge_bytes = FINGERPRINT_TEXT_SAMPLE_BYTES / 2;
        hasher.update(b"sample:v1:bytes=");
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update(b":head:");
        hasher.update(&bytes[..edge_bytes]);
        hasher.update(b":tail:");
        hasher.update(&bytes[bytes.len() - edge_bytes..]);
    }

    *segments_added += 1;
}

fn append_utf8_prefix(out: &mut String, raw: &str, max_bytes: usize) {
    if out.len() >= max_bytes {
        return;
    }

    for ch in raw.chars() {
        let len = ch.len_utf8();
        if out.len().saturating_add(len) > max_bytes {
            break;
        }
        out.push(ch);
    }
}

fn append_utf8_suffix(out: &mut String, raw: &str, max_bytes: usize) {
    if raw.len() >= max_bytes {
        out.clear();
        out.push_str(utf8_suffix_slice(raw, max_bytes));
        return;
    }

    out.push_str(raw);
    while out.len() > max_bytes {
        let Some(ch) = out.chars().next() else {
            break;
        };
        out.drain(..ch.len_utf8());
    }
}

fn utf8_suffix_slice(raw: &str, max_bytes: usize) -> &str {
    if raw.len() <= max_bytes {
        return raw;
    }

    let mut start = raw.len();
    for (idx, _) in raw.char_indices().rev() {
        if raw.len().saturating_sub(idx) > max_bytes {
            break;
        }
        start = idx;
    }
    &raw[start..]
}

fn calculate_fingerprint_hash(headers: &HeaderMap, body: Option<&Value>) -> String {
    let ip = extract_client_ip(headers);
    let ua = extract_user_agent(headers);
    let message_hash = extract_initial_message_text_hash(body).unwrap_or_else(|| "unknown".into());
    let mut raw = format!("v2|ip:{ip}|ua:{ua}|m:{message_hash}");
    if let Some(key_hash) = credential_fingerprint_prefix(headers) {
        raw.push_str("|k:");
        raw.push_str(&key_hash);
    }
    let digest = Sha256::digest(raw.as_bytes());
    format!("{digest:x}")
}

fn generate_uuid_v7_like(now_unix_ms: i64) -> String {
    let ts = now_unix_ms.max(0) as u64;
    let seq = UUID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() as u64;

    let mut seed: Vec<u8> = Vec::with_capacity(24);
    seed.extend_from_slice(&ts.to_be_bytes());
    seed.extend_from_slice(&seq.to_be_bytes());
    seed.extend_from_slice(&pid.to_be_bytes());
    let digest = Sha256::digest(seed);

    let mut bytes = [0u8; 16];
    let mut t = ts;
    // 48-bit big-endian Unix timestamp in milliseconds
    bytes[5] = (t & 0xff) as u8;
    t >>= 8;
    bytes[4] = (t & 0xff) as u8;
    t >>= 8;
    bytes[3] = (t & 0xff) as u8;
    t >>= 8;
    bytes[2] = (t & 0xff) as u8;
    t >>= 8;
    bytes[1] = (t & 0xff) as u8;
    t >>= 8;
    bytes[0] = (t & 0xff) as u8;

    // Fill remaining bytes with deterministic entropy.
    bytes[6..].copy_from_slice(&digest[..10]);

    // Version (7): high nibble of byte 6
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    // Variant (RFC 4122): 10xx xxxx in byte 8
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn prune_cache(cache: &mut CodexSessionIdCache, now_unix: i64) {
    cache
        .entries
        .retain(|_, entry| entry.expires_at_unix > now_unix);
    trim_cache_to_capacity(cache);
}

fn trim_cache_to_capacity(cache: &mut CodexSessionIdCache) {
    while cache.entries.len() > MAX_CACHE_ENTRIES {
        evict_oldest_cache_entry(cache);
    }
}

fn evict_oldest_cache_entry_if_full(cache: &mut CodexSessionIdCache, incoming_key: &str) {
    if cache.entries.len() < MAX_CACHE_ENTRIES || cache.entries.contains_key(incoming_key) {
        return;
    }

    evict_oldest_cache_entry(cache);
}

fn evict_oldest_cache_entry(cache: &mut CodexSessionIdCache) {
    if let Some(oldest_key) = cache
        .entries
        .iter()
        .min_by_key(|(_, entry)| entry.expires_at_unix)
        .map(|(key, _)| key.clone())
    {
        cache.entries.remove(&oldest_key);
    }
}

fn get_or_create_from_fingerprint(
    cache: &mut CodexSessionIdCache,
    now_unix: i64,
    now_unix_ms: i64,
    headers: &HeaderMap,
    body: Option<&Value>,
) -> (String, &'static str, &'static str) {
    prune_cache(cache, now_unix);

    let fingerprint_hash = calculate_fingerprint_hash(headers, body);
    if let Some(entry) = cache.entries.get(&fingerprint_hash) {
        if entry.expires_at_unix > now_unix
            && normalize_codex_session_id(Some(&entry.session_id)).is_some()
        {
            return (
                entry.session_id.clone(),
                "fingerprint_cache",
                "reused_fingerprint_cache",
            );
        }
    }

    let candidate = generate_uuid_v7_like(now_unix_ms);
    evict_oldest_cache_entry_if_full(cache, &fingerprint_hash);
    cache.entries.insert(
        fingerprint_hash,
        CacheEntry {
            session_id: candidate.clone(),
            expires_at_unix: now_unix.saturating_add(DEFAULT_TTL_SECS.max(1)),
        },
    );

    (candidate, "generated_uuid_v7", "generated_uuid_v7")
}

pub(super) fn complete_codex_session_identifiers(
    cache: &mut CodexSessionIdCache,
    now_unix: i64,
    now_unix_ms: i64,
    headers: &mut HeaderMap,
    request_body: Option<&mut Value>,
) -> CodexSessionCompletionResult {
    let header_session_id =
        normalize_codex_session_id(header_string(headers, "session_id").as_deref());
    let header_x_session_id =
        normalize_codex_session_id(header_string(headers, "x-session-id").as_deref());
    let body_prompt_cache_key = request_body
        .as_deref()
        .and_then(|v| v.get("prompt_cache_key"))
        .and_then(|v| v.as_str())
        .and_then(|v| normalize_codex_session_id(Some(v)));
    let body_metadata_session_id = request_body
        .as_deref()
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.as_object())
        .and_then(|meta| meta.get("session_id"))
        .and_then(|v| v.as_str())
        .and_then(|v| normalize_codex_session_id(Some(v)));
    let body_previous_response_id = request_body
        .as_deref()
        .and_then(|v| v.get("previous_response_id"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| {
            let candidate = format!("codex_prev_{v}");
            normalize_codex_session_id(Some(&candidate))
        });

    let missing_header = header_session_id.is_none() && header_x_session_id.is_none();
    let missing_body = body_prompt_cache_key.is_none();

    let existing: Option<(String, &'static str)> = header_session_id
        .clone()
        .map(|v| (v, "header_session_id"))
        .or_else(|| {
            header_x_session_id
                .clone()
                .map(|v| (v, "header_x_session_id"))
        })
        .or_else(|| {
            body_prompt_cache_key
                .clone()
                .map(|v| (v, "body_prompt_cache_key"))
        })
        .or_else(|| {
            body_metadata_session_id
                .clone()
                .map(|v| (v, "body_metadata_session_id"))
        })
        .or_else(|| {
            body_previous_response_id
                .clone()
                .map(|v| (v, "body_previous_response_id"))
        });

    let (mut session_id, mut source, mut action) = if let Some((value, src)) = existing.clone() {
        (value, src, "none")
    } else {
        let (value, src, act) = get_or_create_from_fingerprint(
            cache,
            now_unix,
            now_unix_ms,
            headers,
            request_body.as_deref(),
        );
        (value, src, act)
    };

    // If both required fields present (session_id + prompt_cache_key), keep as-is (idempotent).
    if header_session_id.is_some() && body_prompt_cache_key.is_some() && existing.is_some() {
        return CodexSessionCompletionResult {
            applied: false,
            session_id,
            source,
            action,
            changed_headers: false,
            changed_body: false,
        };
    }

    let mut applied = false;
    let mut changed_headers = false;
    let mut changed_body = false;

    // Header completion
    if missing_header {
        if let Ok(v) = HeaderValue::from_str(&session_id) {
            headers.insert("session_id", v.clone());
            headers.insert("x-session-id", v);
            applied = true;
            changed_headers = true;
        }
    } else if header_session_id.is_none() && header_x_session_id.is_some() {
        // Keep both header keys present for downstream compatibility.
        if let Some(xid) = header_x_session_id.clone() {
            session_id = xid.clone();
            source = "header_x_session_id";
            if let Ok(v) = HeaderValue::from_str(&xid) {
                headers.insert("session_id", v);
                applied = true;
                changed_headers = true;
            }
        }
    } else if header_session_id.is_some() && header_x_session_id.is_none() {
        if let Some(sid) = header_session_id.clone() {
            if let Ok(v) = HeaderValue::from_str(&sid) {
                headers.insert("x-session-id", v);
                applied = true;
                changed_headers = true;
            }
        }
    }

    // Body completion
    if missing_body {
        if let Some(body) = request_body {
            if let Some(obj) = body.as_object_mut() {
                obj.insert(
                    "prompt_cache_key".to_string(),
                    Value::String(session_id.clone()),
                );
                applied = true;
                changed_body = true;
            }
        }
    }

    if existing.is_some() && (changed_headers || changed_body) {
        action = "completed_missing_fields";
    }

    CodexSessionCompletionResult {
        applied,
        session_id,
        source,
        action,
        changed_headers,
        changed_body,
    }
}

#[cfg(test)]
mod tests;
