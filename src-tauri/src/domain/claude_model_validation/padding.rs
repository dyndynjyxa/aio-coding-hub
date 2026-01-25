//! Usage: Helpers for enforcing stream mode and prompt-cache padding.

pub(super) fn force_stream_true(body: &mut serde_json::Value) -> bool {
    let Some(obj) = body.as_object_mut() else {
        return false;
    };
    match obj.get("stream").and_then(|v| v.as_bool()) {
        Some(true) => false,
        _ => {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
            true
        }
    }
}

pub(super) fn infer_cache_min_tokens_for_model(model: Option<&str>) -> usize {
    let m = model.unwrap_or("").trim().to_lowercase();
    if m.contains("haiku-4-5") {
        return 4096;
    }
    if m.contains("haiku-3-5") || m.contains("haiku-3") {
        return 2048;
    }
    1024
}

pub(super) fn apply_prompt_cache_padding(
    body: &mut serde_json::Value,
    min_tokens: usize,
) -> (bool, usize) {
    // Best-effort padding: repeat a simple token-like word to exceed min_tokens.
    // Keep deterministic to maximize cache hit probability across Step1/Step2.
    let word_count = min_tokens.saturating_add(256);
    let marker_begin = "[AIO_CACHE_PAD_BEGIN]";
    let marker_end = "[AIO_CACHE_PAD_END]";

    let Some(obj) = body.as_object_mut() else {
        return (false, word_count);
    };

    let system = obj
        .entry("system")
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    if !system.is_array() {
        *system = serde_json::Value::Array(vec![]);
    }
    let system_arr = system.as_array_mut().unwrap();

    let mut applied = false;
    for block in system_arr.iter_mut() {
        let Some(block_obj) = block.as_object_mut() else {
            continue;
        };
        let is_ephemeral = block_obj
            .get("cache_control")
            .and_then(|v| v.as_object())
            .and_then(|cc| cc.get("type"))
            .and_then(|v| v.as_str())
            == Some("ephemeral");
        if !is_ephemeral {
            continue;
        }
        let Some(text) = block_obj.get_mut("text").and_then(|v| v.as_str()) else {
            continue;
        };
        if text.contains(marker_begin) {
            return (false, word_count);
        }

        let mut padded = String::with_capacity(text.len() + word_count * 6);
        padded.push_str(text);
        padded.push('\n');
        padded.push_str(marker_begin);
        padded.push('\n');
        for i in 0..word_count {
            if i > 0 {
                padded.push(' ');
            }
            padded.push_str("cachepad");
        }
        padded.push('\n');
        padded.push_str(marker_end);

        block_obj.insert("text".to_string(), serde_json::Value::String(padded));
        applied = true;
        break;
    }

    if !applied {
        let mut padded = String::with_capacity(word_count * 6 + 64);
        padded.push_str("AIO prompt caching validation (auto padding)\n");
        padded.push_str(marker_begin);
        padded.push('\n');
        for i in 0..word_count {
            if i > 0 {
                padded.push(' ');
            }
            padded.push_str("cachepad");
        }
        padded.push('\n');
        padded.push_str(marker_end);
        system_arr.insert(
            0,
            serde_json::json!({
                "type": "text",
                "text": padded,
                "cache_control": { "type": "ephemeral", "ttl": "5m" }
            }),
        );
        applied = true;
    }

    (applied, word_count)
}
