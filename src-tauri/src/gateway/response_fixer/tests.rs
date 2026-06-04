use super::encoding::EncodingFixer;
use super::json::JsonFixer;
use super::sse::SseFixer;
use super::{
    process_non_stream, push_special_setting, special_settings_json, ResponseFixerConfig,
    ResponseFixerStream, DEFAULT_MAX_FIX_SIZE, DEFAULT_MAX_JSON_DEPTH,
    SPECIAL_SETTINGS_JSON_MAX_BYTES, SPECIAL_SETTINGS_MAX_ENTRIES,
    SPECIAL_SETTINGS_STRING_PREVIEW_BYTES,
};
use axum::body::Bytes;
use futures_core::Stream;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

#[test]
fn special_settings_push_caps_entries_and_marks_truncation() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));

    for index in 0..(SPECIAL_SETTINGS_MAX_ENTRIES + 3) {
        push_special_setting(
            &special_settings,
            serde_json::json!({
                "type": "entry",
                "index": index,
            }),
        );
    }

    let settings = special_settings.lock().unwrap();
    assert_eq!(settings.len(), SPECIAL_SETTINGS_MAX_ENTRIES);
    assert_eq!(
        settings.first().and_then(|entry| entry.get("index")),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        settings
            .last()
            .and_then(|entry| entry.get("type"))
            .and_then(serde_json::Value::as_str),
        Some("special_settings_truncated")
    );
    drop(settings);

    let encoded = special_settings_json(&special_settings).expect("special settings json");
    let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("valid json");
    assert_eq!(
        decoded.as_array().map(Vec::len),
        Some(SPECIAL_SETTINGS_MAX_ENTRIES)
    );
}

#[test]
fn special_settings_push_bounds_large_string_values() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));
    let large_value = "x".repeat(SPECIAL_SETTINGS_STRING_PREVIEW_BYTES + 2048);

    push_special_setting(
        &special_settings,
        serde_json::json!({
            "type": "large_debug",
            "scope": "request",
            "detail": large_value,
        }),
    );

    let encoded = special_settings_json(&special_settings).expect("special settings json");
    assert!(encoded.contains("truncated at"));
    assert!(encoded.contains("hash="));
    assert!(!encoded.contains(&"x".repeat(SPECIAL_SETTINGS_STRING_PREVIEW_BYTES + 1)));
}

#[test]
fn special_settings_json_caps_total_encoded_bytes() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));

    for index in 0..SPECIAL_SETTINGS_MAX_ENTRIES {
        push_special_setting(
            &special_settings,
            serde_json::json!({
                "type": "large_entry",
                "scope": "request",
                "index": index,
                "parts": vec!["x".repeat(SPECIAL_SETTINGS_STRING_PREVIEW_BYTES + 2048); 5],
            }),
        );
    }

    let encoded = special_settings_json(&special_settings).expect("special settings json");
    assert!(encoded.len() <= SPECIAL_SETTINGS_JSON_MAX_BYTES);
    assert!(encoded.contains("special_settings_truncated"));
    assert!(encoded.contains("encoded_json_too_large"));
}

#[test]
fn encoding_fixer_valid_utf8_passthrough() {
    let input = Bytes::from_static("Hello 世界".as_bytes());
    let res = EncodingFixer::fix_bytes(input.clone());
    assert!(!res.applied);
    assert_eq!(res.data, input);
}

#[test]
fn encoding_fixer_removes_utf8_bom() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
    bytes.extend_from_slice(b"Hello");
    let input = Bytes::from(bytes);
    let res = EncodingFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(std::str::from_utf8(res.data.as_ref()).unwrap(), "Hello");
}

#[test]
fn encoding_fixer_removes_utf16_bom() {
    // UTF-16LE BOM + "A"（0x41 0x00）
    let input = Bytes::from_static(&[0xff, 0xfe, 0x41, 0x00]);
    let res = EncodingFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(std::str::from_utf8(res.data.as_ref()).unwrap(), "A");
}

#[test]
fn encoding_fixer_removes_null_bytes() {
    let input = Bytes::from_static(&[0x48, 0x65, 0x00, 0x6c, 0x6c, 0x6f]);
    let res = EncodingFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(std::str::from_utf8(res.data.as_ref()).unwrap(), "Hello");
}

#[test]
fn encoding_fixer_lossy_fix_invalid_utf8() {
    // 0xC3 0x28 是无效 UTF-8 序列
    let input = Bytes::from_static(&[0xc3, 0x28, 0x61]);
    let res = EncodingFixer::fix_bytes(input);
    assert!(res.applied);
    assert!(std::str::from_utf8(res.data.as_ref()).is_ok());
}

#[test]
fn sse_fixer_fixes_data_space() {
    let input = Bytes::from_static(b"data:{\"test\":true}\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: {\"test\":true}\n");
}

#[test]
fn sse_fixer_valid_sse_passthrough_ptr_eq() {
    let input = Bytes::from_static(b"data: {\"test\": true}\n");
    let res = SseFixer::fix_bytes(input.clone());
    assert!(!res.applied);
    assert_eq!(res.data, input);
}

#[test]
fn sse_fixer_wraps_json_line_and_done() {
    let json_line = Bytes::from_static(b"{\"content\":\"hello\"}\n");
    let res = SseFixer::fix_bytes(json_line);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: {\"content\":\"hello\"}\n");

    let done = Bytes::from_static(b"[DONE]\n");
    let res = SseFixer::fix_bytes(done);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: [DONE]\n");
}

#[test]
fn sse_fixer_keeps_comment_and_fixes_fields() {
    let input =
        Bytes::from_static(b": this is a comment\nevent:message\nid:123\nretry:1000\ndata: test\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(
        res.data.as_ref(),
        b": this is a comment\nevent: message\nid: 123\nretry: 1000\ndata: test\n"
    );
}

#[test]
fn sse_fixer_normalizes_crlf_and_cr() {
    let input = Bytes::from_static(b"data: test\r\ndata: test2\r\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: test\ndata: test2\n");

    let input = Bytes::from_static(b"data: test\rdata: test2\r");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: test\ndata: test2\n");
}

#[test]
fn sse_fixer_fixes_data_case_and_data_space_variants() {
    let input = Bytes::from_static(b"Data:{\"test\": true}\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: {\"test\": true}\n");

    let input = Bytes::from_static(b"data :{\"test\": true}\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: {\"test\": true}\n");
}

#[test]
fn sse_fixer_merges_consecutive_blank_lines() {
    let input = Bytes::from_static(b"data: test\n\n\n\ndata: test2\n");
    let res = SseFixer::fix_bytes(input);
    assert!(res.applied);
    assert_eq!(res.data.as_ref(), b"data: test\n\ndata: test2\n");
}

#[test]
fn json_fixer_repairs_truncated_object() {
    let fixer = JsonFixer::new(200, 1024 * 1024);
    let input = Bytes::from_static(br#"{"key":"value""#);
    let res = fixer.fix_bytes(input);
    assert!(res.applied);
    assert!(serde_json::from_slice::<serde_json::Value>(res.data.as_ref()).is_ok());
}

#[test]
fn json_fixer_repairs_common_truncations() {
    let fixer = JsonFixer::new(200, 1024 * 1024);

    for input in [
        Bytes::from_static(br#"{"key":"value""#),
        Bytes::from_static(br#"[1, 2, 3"#),
        Bytes::from_static(br#"{"key":"val"#),
        Bytes::from_static(br#"{"a": 1,}"#),
        Bytes::from_static(br#"[1, 2,]"#),
        Bytes::from_static(b"{\"key\":"),
        Bytes::from_static(br#"{"key":"value", "outer": {"inner": [1, 2"#),
    ] {
        let res = fixer.fix_bytes(input);
        assert!(serde_json::from_slice::<serde_json::Value>(res.data.as_ref()).is_ok());
    }
}

#[test]
fn json_fixer_appends_null_when_missing_value() {
    let fixer = JsonFixer::new(200, 1024 * 1024);
    let input = Bytes::from_static(b"{\"key\":");
    let res = fixer.fix_bytes(input);
    let v: serde_json::Value = serde_json::from_slice(res.data.as_ref()).unwrap();
    assert_eq!(v, serde_json::json!({"key": null}));
}

#[test]
fn json_fixer_depth_and_size_protection() {
    let input = Bytes::from_static(br#"{"a":{"b":{"c":{"d":"#);
    let fixer = JsonFixer::new(3, 1024 * 1024);
    let res = fixer.fix_bytes(input.clone());
    assert!(!res.applied);
    assert_eq!(res.data.as_ref(), input.as_ref());

    let input = Bytes::from_static(br#"{"key":"very long value"}"#);
    let fixer = JsonFixer::new(200, 10);
    let res = fixer.fix_bytes(input.clone());
    assert!(!res.applied);
    assert_eq!(res.data.as_ref(), input.as_ref());
}

#[test]
fn response_fixer_non_stream_writes_special_setting_when_hit() {
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: DEFAULT_MAX_FIX_SIZE,
    };

    let mut bom_json = Vec::new();
    bom_json.extend_from_slice(&[0xef, 0xbb, 0xbf]);
    bom_json.extend_from_slice(br#"{"a":1}"#);

    let out = process_non_stream(Bytes::from(bom_json), config);
    assert_eq!(out.body.as_ref(), br#"{"a":1}"#);
    assert_eq!(out.header_value, "applied");
    assert!(out.special_setting.is_some());
}

#[test]
fn response_fixer_non_stream_skips_oversized_body_before_encoding_fix() {
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: 4,
    };
    let input = Bytes::from_static(&[0x48, 0x00, 0x65, 0x00, 0x6c]);

    let out = process_non_stream(input.clone(), config);

    assert_eq!(out.body, input);
    assert_eq!(out.header_value, "skipped-too-large");
    assert!(out.special_setting.is_none());
}

struct VecBytesStream {
    items: VecDeque<Result<Bytes, reqwest::Error>>,
}

impl VecBytesStream {
    fn new(items: Vec<Result<Bytes, reqwest::Error>>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }
}

impl Stream for VecBytesStream {
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.items.pop_front())
    }
}

struct NextFuture<'a, S: Stream + Unpin>(&'a mut S);

impl<'a, S: Stream + Unpin> Future for NextFuture<'a, S> {
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.0).poll_next(cx)
    }
}

async fn next_item<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    NextFuture(stream).await
}

async fn collect_ok_bytes<S>(mut stream: S) -> Vec<u8>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    let mut out: Vec<u8> = Vec::new();
    while let Some(item) = next_item(&mut stream).await {
        let bytes = item.expect("stream should not error in test");
        out.extend_from_slice(bytes.as_ref());
    }
    out
}

#[tokio::test]
async fn response_fixer_stream_fixes_truncated_json_in_data_line_across_chunks() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: DEFAULT_MAX_FIX_SIZE,
    };

    let upstream = VecBytesStream::new(vec![
        Ok(Bytes::from_static(b"data: {\"key\":")),
        Ok(Bytes::from_static(b"\n\n")),
    ]);

    let stream = ResponseFixerStream::new(upstream, config, special_settings.clone());
    let out = collect_ok_bytes(stream).await;
    assert_eq!(out, b"data: {\"key\":null}\n\n");

    let settings = special_settings.lock().unwrap();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0]["type"], "response_fixer");
    assert_eq!(settings[0]["hit"], true);
}

#[tokio::test]
async fn response_fixer_stream_valid_sse_should_not_write_special_settings() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: DEFAULT_MAX_FIX_SIZE,
    };

    let upstream = VecBytesStream::new(vec![Ok(Bytes::from_static(b"data: {\"a\":1}\n\n"))]);
    let stream = ResponseFixerStream::new(upstream, config, special_settings.clone());
    let out = collect_ok_bytes(stream).await;
    assert_eq!(out, b"data: {\"a\":1}\n\n");

    let settings = special_settings.lock().unwrap();
    assert!(settings.is_empty());
}

#[tokio::test]
async fn response_fixer_stream_degrades_when_exceeding_max_fix_size_without_newlines() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: 12,
    };

    let upstream = VecBytesStream::new(vec![
        Ok(Bytes::from_static(b"data: {\"k\":")),
        Ok(Bytes::from_static(b"\"v\"")),
    ]);

    let mut stream = ResponseFixerStream::new(upstream, config, special_settings.clone());
    let first = next_item(&mut stream)
        .await
        .expect("should produce some output")
        .expect("should be ok");
    assert!(!first.is_empty());

    // 清理：拉取到结束，确保 finalize 运行
    while let Some(_item) = next_item(&mut stream).await {}

    let settings = special_settings.lock().unwrap();
    assert!(settings.is_empty());
}

#[tokio::test]
async fn response_fixer_stream_collapses_buffered_chunks_when_degrading_to_passthrough() {
    let special_settings = Arc::new(Mutex::new(Vec::new()));
    let config = ResponseFixerConfig {
        fix_encoding: true,
        fix_sse_format: true,
        fix_truncated_json: true,
        max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        max_fix_size: 4,
    };

    let upstream = VecBytesStream::new(vec![
        Ok(Bytes::from_static(b"a")),
        Ok(Bytes::from_static(b"b")),
        Ok(Bytes::from_static(b"c")),
        Ok(Bytes::from_static(b"d")),
        Ok(Bytes::from_static(b"e")),
    ]);

    let mut stream = ResponseFixerStream::new(upstream, config, special_settings.clone());
    let first = next_item(&mut stream)
        .await
        .expect("should produce buffered output")
        .expect("should be ok");
    let second = next_item(&mut stream)
        .await
        .expect("should produce passthrough output")
        .expect("should be ok");

    assert_eq!(first, Bytes::from_static(b"abcd"));
    assert_eq!(second, Bytes::from_static(b"e"));
    assert!(next_item(&mut stream).await.is_none());

    let settings = special_settings.lock().unwrap();
    assert!(settings.is_empty());
}
