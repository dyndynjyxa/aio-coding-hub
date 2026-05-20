use crate::shared::mutex_ext::MutexExt;
use axum::http::HeaderMap;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

const DEFAULT_SESSION_TTL_SECS: i64 = 300;
const MAX_SESSION_ID_LEN: usize = 256;
const MAX_BINDINGS: usize = 5000;
const SESSION_SUFFIX_LEN: usize = 8;
const SESSION_FINGERPRINT_MAX_SEGMENTS: usize = 3;
const SESSION_FINGERPRINT_TEXT_SAMPLE_BYTES: usize = 32 * 1024;
const SESSION_FINGERPRINT_COMBINED_PREFIX_BYTES: usize = 8 * 1024;
const SESSION_FINGERPRINT_COMBINED_SUFFIX_BYTES: usize = 8 * 1024;
const SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS: usize = 64;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveSessionSnapshot {
    pub cli_key: String,
    pub session_id: String,
    pub session_suffix: String,
    pub provider_id: i64,
    pub expires_at: i64,
}

#[derive(Debug)]
pub struct SessionManager {
    ttl_secs: i64,
    bindings: Mutex<HashMap<SessionKey, SessionBinding>>,
}

#[derive(Debug, Clone)]
struct SessionBinding {
    provider_id: i64,
    sort_mode_id: Option<i64>,
    provider_order: Option<Vec<i64>>,
    expires_at: i64,
    ttl_secs: i64,
}

#[derive(Debug, Clone, Eq)]
struct SessionKey {
    cli_key: String,
    session_id: String,
}

impl PartialEq for SessionKey {
    fn eq(&self, other: &Self) -> bool {
        self.cli_key == other.cli_key && self.session_id == other.session_id
    }
}

impl Hash for SessionKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cli_key.hash(state);
        self.session_id.hash(state);
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            ttl_secs: DEFAULT_SESSION_TTL_SECS,
            bindings: Mutex::new(HashMap::new()),
        }
    }

    pub fn clear_cli_bindings(&self, cli_key: &str) -> usize {
        let cli_key = cli_key.trim();
        if cli_key.is_empty() {
            return 0;
        }

        let mut guard = self.bindings.lock_or_recover();
        let before = guard.len();
        guard.retain(|k, _| k.cli_key != cli_key);
        before.saturating_sub(guard.len())
    }

    pub fn extract_session_id_from_json(
        headers: &HeaderMap,
        root: Option<&Value>,
    ) -> Option<String> {
        // 1) client headers
        if let Some(v) = header_string(headers, "session_id") {
            if let Some(id) = sanitize_session_id(&v) {
                return Some(id);
            }
        }
        if let Some(v) = header_string(headers, "x-session-id") {
            if let Some(id) = sanitize_session_id(&v) {
                return Some(id);
            }
        }

        // 2) best-effort JSON extraction
        if let Some(root) = root {
            // Common: { "session_id": "..." }
            if let Some(id) = root.get("session_id").and_then(|v| v.as_str()) {
                if let Some(id) = sanitize_session_id(id) {
                    return Some(id);
                }
            }

            // Common: { "conversation_id": "..." } or { "thread_id": "..." }
            for key in ["conversation_id", "thread_id", "chat_id"] {
                if let Some(id) = root.get(key).and_then(|v| v.as_str()) {
                    if let Some(id) = sanitize_session_id(id) {
                        return Some(id);
                    }
                }
            }

            // Codex-style: prompt_cache_key (UUID-like, prefer when present)
            if let Some(id) = root.get("prompt_cache_key").and_then(|v| v.as_str()) {
                let trimmed = id.trim();
                if trimmed.len() > 20 {
                    if let Some(id) = sanitize_session_id(trimmed) {
                        return Some(id);
                    }
                }
            }

            if let Some(meta) = root.get("metadata").and_then(|v| v.as_object()) {
                if let Some(id) = meta.get("session_id").and_then(|v| v.as_str()) {
                    if let Some(id) = sanitize_session_id(id) {
                        return Some(id);
                    }
                }

                if let Some(user_id) = meta.get("user_id").and_then(|v| v.as_str()) {
                    let marker = "_session_";
                    if let Some(idx) = user_id.find(marker) {
                        let extracted = &user_id[idx + marker.len()..];
                        if let Some(id) = sanitize_session_id(extracted) {
                            return Some(id);
                        }
                    }
                }
            }

            // Codex-style fallback: previous_response_id
            if let Some(prev) = root.get("previous_response_id").and_then(|v| v.as_str()) {
                if let Some(id) = sanitize_session_id(prev) {
                    if let Some(out) = sanitize_session_id(&format!("codex_prev_{id}")) {
                        return Some(out);
                    }
                }
            }
        }

        deterministic_session_id(headers, root).and_then(|id| sanitize_session_id(&id))
    }

    pub fn get_bound_provider(
        &self,
        cli_key: &str,
        session_id: &str,
        now_unix: i64,
    ) -> Option<i64> {
        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        match guard.get_mut(&key) {
            Some(binding) if binding.expires_at > now_unix => {
                binding.expires_at = now_unix.saturating_add(binding.ttl_secs.max(1));
                (binding.provider_id > 0).then_some(binding.provider_id)
            }
            Some(_) => {
                guard.remove(&key);
                None
            }
            None => None,
        }
    }

    // Returns `Some(sort_mode_id)` when a session binding exists (even if the bound mode is `None`).
    pub fn get_bound_sort_mode_id(
        &self,
        cli_key: &str,
        session_id: &str,
        now_unix: i64,
    ) -> Option<Option<i64>> {
        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        match guard.get_mut(&key) {
            Some(binding) if binding.expires_at > now_unix => {
                binding.expires_at = now_unix.saturating_add(binding.ttl_secs.max(1));
                Some(binding.sort_mode_id)
            }
            Some(_) => {
                guard.remove(&key);
                None
            }
            None => None,
        }
    }

    // Bind (or refresh) the session's sort_mode for stickiness across retries.
    // If a binding already exists, its sort_mode_id is preserved and only TTL is refreshed.
    pub fn bind_sort_mode(
        &self,
        cli_key: &str,
        session_id: &str,
        sort_mode_id: Option<i64>,
        provider_order: Option<Vec<i64>>,
        now_unix: i64,
    ) {
        if cli_key.trim().is_empty() || session_id.trim().is_empty() {
            return;
        }

        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        if guard.len() >= MAX_BINDINGS {
            drop_expired(&mut guard, now_unix);
            if guard.len() >= MAX_BINDINGS {
                evict_oldest_quarter(&mut guard);
            }
        }

        if let Some(existing) = guard.get_mut(&key) {
            if existing.expires_at > now_unix {
                existing.expires_at = now_unix.saturating_add(self.ttl_secs.max(1));
                if existing.provider_order.is_none() {
                    existing.provider_order = provider_order;
                }
                return;
            }
            guard.remove(&key);
        }

        guard.insert(
            key,
            SessionBinding {
                provider_id: 0,
                sort_mode_id,
                provider_order,
                expires_at: now_unix.saturating_add(self.ttl_secs.max(1)),
                ttl_secs: self.ttl_secs,
            },
        );
    }

    pub fn get_bound_provider_order(
        &self,
        cli_key: &str,
        session_id: &str,
        now_unix: i64,
    ) -> Option<Vec<i64>> {
        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        match guard.get_mut(&key) {
            Some(binding) if binding.expires_at > now_unix => {
                binding.expires_at = now_unix.saturating_add(binding.ttl_secs.max(1));
                binding.provider_order.clone()
            }
            Some(_) => {
                guard.remove(&key);
                None
            }
            None => None,
        }
    }

    pub fn bind_success(
        &self,
        cli_key: &str,
        session_id: &str,
        provider_id: i64,
        sort_mode_id: Option<i64>,
        now_unix: i64,
    ) {
        if cli_key.trim().is_empty() || session_id.trim().is_empty() || provider_id <= 0 {
            return;
        }

        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        if guard.len() >= MAX_BINDINGS {
            drop_expired(&mut guard, now_unix);
            if guard.len() >= MAX_BINDINGS {
                evict_oldest_quarter(&mut guard);
            }
        }

        let expires_at = now_unix.saturating_add(self.ttl_secs.max(1));
        if let Some(existing) = guard.get_mut(&key) {
            if existing.expires_at > now_unix {
                existing.provider_id = provider_id;
                existing.expires_at = expires_at;
                if existing.sort_mode_id.is_none() {
                    existing.sort_mode_id = sort_mode_id;
                }
                return;
            }
            guard.remove(&key);
        }

        guard.insert(
            key,
            SessionBinding {
                provider_id,
                sort_mode_id,
                provider_order: None,
                expires_at,
                ttl_secs: self.ttl_secs,
            },
        );
    }

    pub fn clear_bound_provider(&self, cli_key: &str, session_id: &str, now_unix: i64) -> bool {
        if cli_key.trim().is_empty() || session_id.trim().is_empty() {
            return false;
        }

        let key = SessionKey {
            cli_key: cli_key.to_string(),
            session_id: session_id.to_string(),
        };

        let mut guard = self.bindings.lock_or_recover();
        match guard.get_mut(&key) {
            Some(binding) if binding.expires_at > now_unix => {
                binding.provider_id = 0;
                true
            }
            Some(_) => {
                guard.remove(&key);
                true
            }
            None => false,
        }
    }

    pub fn list_active(&self, now_unix: i64, limit: usize) -> Vec<ActiveSessionSnapshot> {
        if limit == 0 {
            return Vec::new();
        }

        let mut guard = self.bindings.lock_or_recover();
        drop_expired(&mut guard, now_unix);

        let mut rows: Vec<ActiveSessionSnapshot> = guard
            .iter()
            .map(|(k, v)| ActiveSessionSnapshot {
                cli_key: k.cli_key.clone(),
                session_id: k.session_id.clone(),
                session_suffix: session_suffix(&k.session_id),
                provider_id: v.provider_id,
                expires_at: v.expires_at,
            })
            .collect();

        rows.sort_by(|a, b| b.expires_at.cmp(&a.expires_at));
        rows.truncate(limit);
        rows
    }
}

fn header_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn credential_fingerprint_prefix(headers: &HeaderMap) -> Option<String> {
    let api_key_prefix = header_string(headers, "x-api-key")
        .or_else(|| header_string(headers, "x-goog-api-key"))
        .and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            let prefix: String = trimmed.chars().take(10).collect();
            sanitize_deterministic_part(&prefix)
        });
    if api_key_prefix.is_some() {
        return api_key_prefix;
    }

    let auth = header_string(headers, "authorization")?;
    let trimmed = auth.trim();
    if trimmed.is_empty() {
        return None;
    }
    let token = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed)
        .trim();
    if token.is_empty() {
        return None;
    }

    let digest = Sha256::digest(token.as_bytes());
    let hex = format!("{digest:x}");
    let short = hex.get(..10)?;
    sanitize_deterministic_part(short)
}

fn deterministic_session_id(headers: &HeaderMap, root: Option<&Value>) -> Option<String> {
    let credential_prefix = credential_fingerprint_prefix(headers);

    if let Some(message_fingerprint_hex) = root.and_then(extract_initial_message_fingerprint_hex32)
    {
        // Prefer message fingerprint for session stickiness across a conversation.
        // Avoid mixing in user-agent/ip so CLI upgrades don't break reuse.
        let mut raw = format!("v2|m:{message_fingerprint_hex}");
        if let Some(prefix) = credential_prefix.as_deref() {
            raw.push_str("|k:");
            raw.push_str(prefix);
        }

        let hash = Sha256::digest(raw.as_bytes());
        let hex = format!("{hash:x}");
        let short = hex.get(..32)?;
        return Some(format!("sess_{short}"));
    }

    let user_agent =
        header_string(headers, "user-agent").and_then(|v| sanitize_deterministic_part(&v));

    let forwarded_for = header_string(headers, "x-forwarded-for").and_then(|raw| {
        raw.split(',')
            .map(str::trim)
            .find(|v| !v.is_empty())
            .and_then(sanitize_deterministic_part)
    });
    let real_ip = header_string(headers, "x-real-ip").and_then(|v| sanitize_deterministic_part(&v));
    let ip = forwarded_for.or(real_ip);

    let parts: Vec<String> = [user_agent, ip, credential_prefix]
        .into_iter()
        .flatten()
        .collect();
    if parts.is_empty() {
        return None;
    }

    let joined = parts.join(":");
    let hash = Sha256::digest(joined.as_bytes());
    let hex = format!("{hash:x}");
    let short = hex.get(..32)?;
    Some(format!("sess_{short}"))
}

fn extract_initial_message_fingerprint_hex32(root: &Value) -> Option<String> {
    if let Some(input) = root.get("input").and_then(|v| v.as_str()) {
        let mut hasher = Sha256::new();
        let mut segments_added = 0usize;
        update_hasher_with_text(&mut hasher, &mut segments_added, input);
        if segments_added > 0 {
            let digest = hasher.finalize();
            let hex = format!("{digest:x}");
            return hex.get(..32).map(str::to_string);
        }
    }

    let items: Option<&Vec<Value>> = root
        .get("messages")
        .and_then(|v| v.as_array())
        .or_else(|| root.get("input").and_then(|v| v.as_array()))
        .or_else(|| root.get("contents").and_then(|v| v.as_array()))
        .or_else(|| {
            root.get("request")
                .and_then(|v| v.get("contents"))
                .and_then(|v| v.as_array())
        });

    let items = items?;
    if items.is_empty() {
        return None;
    }

    let mut hasher = Sha256::new();
    let mut segments_added = 0usize;

    for item in items.iter() {
        if segments_added >= SESSION_FINGERPRINT_MAX_SEGMENTS {
            break;
        }
        update_hasher_from_message_item(&mut hasher, &mut segments_added, item);
    }

    if segments_added == 0 {
        return None;
    }

    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    hex.get(..32).map(str::to_string)
}

fn update_hasher_from_message_item(hasher: &mut Sha256, segments_added: &mut usize, item: &Value) {
    if *segments_added >= SESSION_FINGERPRINT_MAX_SEGMENTS {
        return;
    }

    match item {
        Value::String(s) => update_hasher_with_text(hasher, segments_added, s),
        Value::Object(obj) => {
            if let Some(item_type) = obj.get("type").and_then(|v| v.as_str()) {
                if item_type != "message" {
                    return;
                }
            }

            let mut combined = FingerprintTextSample::default();

            if let Some(content) = obj.get("content") {
                append_fingerprint_text_from_content(&mut combined, content);
            }

            if let Some(parts) = obj.get("parts").and_then(|v| v.as_array()) {
                for part in parts
                    .iter()
                    .take(SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS)
                {
                    let Some(part_obj) = part.as_object() else {
                        continue;
                    };
                    if let Some(text) = part_obj.get("text").and_then(|v| v.as_str()) {
                        combined.append_text(text);
                        continue;
                    }
                    if let Some(text) = part_obj.get("content").and_then(|v| v.as_str()) {
                        combined.append_text(text);
                    }
                }
                if parts.len() > SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS {
                    combined.mark_truncated_parts(parts.len());
                }
            }

            if let Some(text) = combined.into_text() {
                update_hasher_with_text(hasher, segments_added, &text);
            }
        }
        _ => {}
    }
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
        append_utf8_prefix(
            &mut self.prefix,
            trimmed,
            SESSION_FINGERPRINT_COMBINED_PREFIX_BYTES,
        );
        append_utf8_suffix(
            &mut self.suffix,
            trimmed,
            SESSION_FINGERPRINT_COMBINED_SUFFIX_BYTES,
        );
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
            "{}|aio_fingerprint_sample:v1:bytes={}:parts={}|{}",
            self.prefix,
            self.total_bytes,
            self.truncated_parts.unwrap_or(0),
            self.suffix
        ))
    }
}

fn append_fingerprint_text_from_content(sample: &mut FingerprintTextSample, content: &Value) {
    match content {
        Value::String(s) => sample.append_text(s),
        Value::Array(parts) => {
            for part in parts
                .iter()
                .take(SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS)
            {
                let Some(part_obj) = part.as_object() else {
                    continue;
                };

                if let Some(text) = part_obj.get("text").and_then(|v| v.as_str()) {
                    sample.append_text(text);
                    continue;
                }

                if let Some(text) = part_obj.get("content").and_then(|v| v.as_str()) {
                    sample.append_text(text);
                    continue;
                }
            }
            if parts.len() > SESSION_FINGERPRINT_CONTENT_PARTS_MAX_ITEMS {
                sample.mark_truncated_parts(parts.len());
            }
        }
        Value::Object(obj) => {
            if let Some(text) = obj
                .get("text")
                .and_then(|v| v.as_str())
                .or_else(|| obj.get("content").and_then(|v| v.as_str()))
            {
                sample.append_text(text);
            }
        }
        _ => {}
    }
}

fn update_hasher_with_text(hasher: &mut Sha256, segments_added: &mut usize, raw: &str) {
    if *segments_added >= SESSION_FINGERPRINT_MAX_SEGMENTS {
        return;
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() <= SESSION_FINGERPRINT_TEXT_SAMPLE_BYTES {
        hasher.update(bytes);
    } else {
        let edge_bytes = SESSION_FINGERPRINT_TEXT_SAMPLE_BYTES / 2;
        hasher.update(b"sample:v1:bytes=");
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update(b":head:");
        hasher.update(&bytes[..edge_bytes]);
        hasher.update(b":tail:");
        hasher.update(&bytes[bytes.len() - edge_bytes..]);
    }
    hasher.update(b"|");
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

fn sanitize_deterministic_part(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(trimmed.len().min(MAX_SESSION_ID_LEN));
    for c in trimmed.chars() {
        if matches!(c, '\n' | '\r' | '\t') {
            continue;
        }
        if out.len().saturating_add(c.len_utf8()) > MAX_SESSION_ID_LEN {
            break;
        }
        out.push(c);
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn sanitize_session_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(trimmed.len().min(MAX_SESSION_ID_LEN));
    for c in trimmed.chars() {
        // Avoid newlines/whitespace causing log injection if someone mistakenly logs it.
        if matches!(c, '\n' | '\r' | '\t') {
            continue;
        }
        if out.len().saturating_add(c.len_utf8()) > MAX_SESSION_ID_LEN {
            break;
        }
        out.push(c);
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn session_suffix(session_id: &str) -> String {
    let suffix: Vec<char> = session_id.chars().rev().take(SESSION_SUFFIX_LEN).collect();
    suffix.into_iter().rev().collect()
}

fn drop_expired(map: &mut HashMap<SessionKey, SessionBinding>, now_unix: i64) {
    map.retain(|_, v| v.expires_at > now_unix);
}

/// Evict the oldest 25% of bindings by `expires_at` to make room for new entries.
/// This preserves the most recently active sessions instead of clearing all bindings.
fn evict_oldest_quarter(map: &mut HashMap<SessionKey, SessionBinding>) {
    let count = map.len();
    if count == 0 {
        return;
    }

    let evict_count = (count / 4).max(1);

    // Collect (key, expires_at) and sort by expires_at ascending (oldest first)
    let mut entries: Vec<(SessionKey, i64)> =
        map.iter().map(|(k, v)| (k.clone(), v.expires_at)).collect();
    entries.sort_by_key(|(_, expires_at)| *expires_at);

    // Remove the oldest entries
    for (key, _) in entries.into_iter().take(evict_count) {
        map.remove(&key);
    }
}

#[cfg(test)]
mod tests;
