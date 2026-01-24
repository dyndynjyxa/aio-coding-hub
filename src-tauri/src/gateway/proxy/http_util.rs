//! Usage: Low-level HTTP helpers for proxying (headers, encoding, response building).

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::io::Read;

pub(super) fn is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

pub(super) fn has_gzip_content_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|enc| !enc.is_empty())
                .any(|enc| enc.eq_ignore_ascii_case("gzip"))
        })
        .unwrap_or(false)
}

pub(super) fn has_non_identity_content_encoding(headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .any(|enc| !enc.eq_ignore_ascii_case("identity"))
}

pub(super) fn maybe_gunzip_response_body_bytes_with_limit(
    body: Bytes,
    headers: &mut HeaderMap,
    max_output_bytes: usize,
) -> Bytes {
    if !has_gzip_content_encoding(headers) {
        return body;
    }

    if body.is_empty() {
        headers.remove(header::CONTENT_ENCODING);
        headers.remove(header::CONTENT_LENGTH);
        return body;
    }

    let mut decoder = flate2::read::GzDecoder::new(body.as_ref());
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    let mut had_any_output = false;
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                had_any_output = true;
                if out.len().saturating_add(n) > max_output_bytes {
                    // 保护性降级：输出过大时，不解压，避免把巨大响应读入内存。
                    return body;
                }
                out.extend_from_slice(&buf[..n]);
            }
            Err(_) => {
                // 容错：忽略解压错误（例如 gzip 流被提前截断），尽可能返回已产出的部分数据。
                if !had_any_output {
                    return body;
                }
                break;
            }
        }
    }

    headers.remove(header::CONTENT_ENCODING);
    headers.remove(header::CONTENT_LENGTH);
    Bytes::from(out)
}

pub(super) fn build_response(
    status: StatusCode,
    headers: &HeaderMap,
    trace_id: &str,
    body: Body,
) -> Response {
    let mut builder = Response::builder().status(status);
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    builder = builder.header("x-trace-id", trace_id);

    match builder.body(body) {
        Ok(r) => r,
        Err(_) => {
            let mut fallback =
                (StatusCode::INTERNAL_SERVER_ERROR, "GW_RESPONSE_BUILD_ERROR").into_response();
            fallback.headers_mut().insert(
                "x-trace-id",
                HeaderValue::from_str(trace_id).unwrap_or(HeaderValue::from_static("unknown")),
            );
            fallback
        }
    }
}
