use axum::body::Bytes;
use serde_json::Value;
use std::time::Instant;

use super::encoding::EncodingFixer;
use super::json::JsonFixer;
use super::ResponseFixerConfig;

#[derive(Debug, Default, Clone)]
pub(super) struct ResponseFixerApplied {
    pub(super) encoding_applied: bool,
    pub(super) encoding_details: Option<&'static str>,
    pub(super) sse_applied: bool,
    pub(super) sse_details: Option<&'static str>,
    pub(super) json_applied: bool,
    pub(super) json_details: Option<&'static str>,
}

fn build_fixers_applied(
    applied: &ResponseFixerApplied,
    include_sse: bool,
) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(if include_sse { 3 } else { 2 });
    out.push(serde_json::json!({
        "fixer": "encoding",
        "applied": applied.encoding_applied,
        "details": applied.encoding_details,
    }));
    if include_sse {
        out.push(serde_json::json!({
            "fixer": "sse",
            "applied": applied.sse_applied,
            "details": applied.sse_details,
        }));
    }
    out.push(serde_json::json!({
        "fixer": "json",
        "applied": applied.json_applied,
        "details": applied.json_details,
    }));
    out
}

pub(super) fn build_special_setting(
    hit: bool,
    applied: &ResponseFixerApplied,
    include_sse: bool,
    total_bytes_processed: usize,
    processing_time_ms: u64,
) -> Value {
    serde_json::json!({
        "type": "response_fixer",
        "scope": "response",
        "hit": hit,
        "fixersApplied": build_fixers_applied(applied, include_sse),
        "totalBytesProcessed": total_bytes_processed as u64,
        "processingTimeMs": processing_time_ms,
    })
}

pub(super) fn process_non_stream(
    body: Bytes,
    config: ResponseFixerConfig,
) -> super::NonStreamFixOutcome {
    let started = Instant::now();
    let mut applied = ResponseFixerApplied::default();

    let mut data = body;
    let total_bytes_processed = data.len();
    if total_bytes_processed > config.max_fix_size {
        return super::NonStreamFixOutcome {
            body: data,
            header_value: "skipped-too-large",
            special_setting: None,
        };
    }

    if config.fix_encoding {
        let res = EncodingFixer::fix_bytes(data);
        if res.applied {
            applied.encoding_applied = true;
            applied.encoding_details = res.details;
        }
        data = res.data;
    }

    if config.fix_truncated_json {
        let fixer = JsonFixer::new(config.max_json_depth, config.max_fix_size);
        let res = fixer.fix_bytes(data);
        if res.applied {
            applied.json_applied = true;
            applied.json_details = res.details;
        }
        data = res.data;
    }

    let audit_hit = applied.encoding_applied || applied.json_applied;
    let processing_time_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

    let special_setting = if audit_hit {
        Some(build_special_setting(
            true,
            &applied,
            false,
            total_bytes_processed,
            processing_time_ms,
        ))
    } else {
        None
    };

    super::NonStreamFixOutcome {
        body: data,
        header_value: if audit_hit { "applied" } else { "not-applied" },
        special_setting,
    }
}
