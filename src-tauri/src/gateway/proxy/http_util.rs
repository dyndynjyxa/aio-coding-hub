//! Usage: Low-level HTTP helpers for proxying (headers, encoding, response building).

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::io::{Read, Write};

use super::GatewayErrorCode;

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

pub(super) fn gunzip_bytes_with_limit(
    input: &[u8],
    max_output_bytes: usize,
) -> Result<Bytes, String> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = decoder
            .read(&mut buf)
            .map_err(|err| format!("failed to decode gzip body: {err}"))?;
        if n == 0 {
            break;
        }
        if out.len().saturating_add(n) > max_output_bytes {
            return Err(format!(
                "gzip decoded body exceeded limit: limit={max_output_bytes} bytes"
            ));
        }
        out.extend_from_slice(&buf[..n]);
    }
    Ok(Bytes::from(out))
}

pub(super) fn gzip_bytes_with_limit(
    input: &[u8],
    max_output_bytes: usize,
) -> Result<Bytes, String> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(input)
        .map_err(|err| format!("failed to encode gzip body: {err}"))?;
    let out = encoder
        .finish()
        .map_err(|err| format!("failed to finish gzip body: {err}"))?;
    if out.len() > max_output_bytes {
        return Err(format!(
            "gzip encoded body exceeded limit: limit={max_output_bytes} bytes"
        ));
    }
    Ok(Bytes::from(out))
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
            let mut fallback = (
                StatusCode::INTERNAL_SERVER_ERROR,
                GatewayErrorCode::ResponseBuildError.as_str(),
            )
                .into_response();
            fallback.headers_mut().insert(
                "x-trace-id",
                HeaderValue::from_str(trace_id).unwrap_or(HeaderValue::from_static("unknown")),
            );
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::maybe_gunzip_response_body_bytes_with_limit;
    use axum::body::Bytes;
    use axum::http::{header, HeaderMap, HeaderValue};
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gzip_bytes(input: &[u8]) -> Bytes {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).expect("gzip write");
        Bytes::from(encoder.finish().expect("gzip finish"))
    }

    fn gzip_headers(content_length: usize) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_length.to_string()).expect("content length"),
        );
        headers
    }

    #[test]
    fn maybe_gunzip_decodes_within_limit_and_removes_encoding_headers() {
        let plain = Bytes::from_static(b"{\"ok\":true}");
        let compressed = gzip_bytes(plain.as_ref());
        let mut headers = gzip_headers(compressed.len());

        let decoded =
            maybe_gunzip_response_body_bytes_with_limit(compressed, &mut headers, plain.len());

        assert_eq!(decoded, plain);
        assert!(headers.get(header::CONTENT_ENCODING).is_none());
        assert!(headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn maybe_gunzip_preserves_compressed_body_when_output_limit_exceeded() {
        let plain = Bytes::from(vec![b'a'; 128 * 1024]);
        let compressed = gzip_bytes(plain.as_ref());
        let mut headers = gzip_headers(compressed.len());

        let output =
            maybe_gunzip_response_body_bytes_with_limit(compressed.clone(), &mut headers, 1024);

        assert_eq!(output, compressed);
        assert_eq!(headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(headers.get(header::CONTENT_LENGTH).is_some());
    }

    #[test]
    fn gzip_round_trip_helpers_preserve_body() {
        let plain = Bytes::from_static(br#"{"input":"hello"}"#);

        let encoded = super::gzip_bytes_with_limit(plain.as_ref(), 1024).expect("encode");
        let decoded = super::gunzip_bytes_with_limit(encoded.as_ref(), 1024).expect("decode");

        assert_eq!(decoded, plain);
    }

    #[test]
    fn gzip_decode_helper_rejects_oversized_output() {
        let plain = Bytes::from(vec![b'a'; 128 * 1024]);
        let encoded = gzip_bytes(plain.as_ref());

        let err = super::gunzip_bytes_with_limit(encoded.as_ref(), 1024)
            .expect_err("should exceed output limit");

        assert!(err.contains("gzip decoded body exceeded limit"));
    }

    #[test]
    fn gzip_encode_helper_rejects_oversized_output() {
        let plain = vec![b'a'; 128 * 1024];

        let err = super::gzip_bytes_with_limit(&plain, 4).expect_err("should exceed tiny limit");

        assert!(err.contains("gzip encoded body exceeded limit"));
    }
}
