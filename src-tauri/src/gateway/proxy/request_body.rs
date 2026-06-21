//! Usage: request body raw/decoded model for gateway passthrough.

use super::http_util::{gunzip_bytes_with_limit, gzip_bytes_with_limit, has_gzip_content_encoding};
use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestBodyEncoding {
    Identity,
    Gzip,
    Unsupported,
}

#[derive(Debug, Clone)]
pub(super) struct GatewayRequestBody {
    raw: Bytes,
    decoded: Bytes,
    encoding: RequestBodyEncoding,
    original_content_encoding: Option<HeaderValue>,
    decoded_from_raw: bool,
    mutated: bool,
}

impl GatewayRequestBody {
    pub(super) fn from_wire(raw: Bytes, headers: &HeaderMap, max_decoded_bytes: usize) -> Self {
        let encoding = classify_request_encoding(headers);
        let original_content_encoding = headers.get(header::CONTENT_ENCODING).cloned();
        match encoding {
            RequestBodyEncoding::Gzip => {
                match gunzip_bytes_with_limit(raw.as_ref(), max_decoded_bytes) {
                    Ok(decoded) => Self {
                        raw,
                        decoded,
                        encoding,
                        original_content_encoding,
                        decoded_from_raw: true,
                        mutated: false,
                    },
                    Err(err) => {
                        tracing::warn!(error = %err, "failed to decode request gzip body for inspection; preserving raw body");
                        Self {
                            decoded: raw.clone(),
                            raw,
                            encoding,
                            original_content_encoding,
                            decoded_from_raw: false,
                            mutated: false,
                        }
                    }
                }
            }
            RequestBodyEncoding::Identity | RequestBodyEncoding::Unsupported => Self {
                decoded: raw.clone(),
                raw,
                encoding,
                original_content_encoding,
                decoded_from_raw: false,
                mutated: false,
            },
        }
    }

    pub(super) fn decoded(&self) -> &Bytes {
        &self.decoded
    }

    pub(super) fn decoded_clone(&self) -> Bytes {
        self.decoded.clone()
    }

    pub(super) fn semantic_headers(&self, headers: &HeaderMap) -> HeaderMap {
        let mut semantic = headers.clone();
        semantic.remove(header::CONTENT_LENGTH);
        if self.decoded_from_raw {
            semantic.remove(header::CONTENT_ENCODING);
        }
        semantic
    }

    pub(super) fn replace_decoded(&mut self, next: Bytes) {
        if self.decoded != next {
            self.decoded = next;
            self.mutated = true;
        }
    }

    pub(super) fn is_mutated(&self) -> bool {
        self.mutated
    }

    pub(super) fn finalize_for_upstream(
        &self,
        headers: &mut HeaderMap,
        max_encoded_bytes: usize,
    ) -> Bytes {
        headers.remove(header::CONTENT_LENGTH);
        if !self.mutated {
            restore_original_content_encoding(headers, self.original_content_encoding.as_ref());
            return self.raw.clone();
        }

        match self.encoding {
            RequestBodyEncoding::Gzip if self.decoded_from_raw => {
                match gzip_bytes_with_limit(self.decoded.as_ref(), max_encoded_bytes) {
                    Ok(encoded) => {
                        restore_original_content_encoding(
                            headers,
                            self.original_content_encoding.as_ref(),
                        );
                        encoded
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "failed to re-encode request gzip body; sending identity body");
                        headers.remove(header::CONTENT_ENCODING);
                        self.decoded.clone()
                    }
                }
            }
            RequestBodyEncoding::Gzip | RequestBodyEncoding::Unsupported => {
                tracing::warn!(
                    encoding = ?self.encoding,
                    "request body mutated after unsupported content encoding; sending identity body"
                );
                headers.remove(header::CONTENT_ENCODING);
                self.decoded.clone()
            }
            RequestBodyEncoding::Identity => {
                headers.remove(header::CONTENT_ENCODING);
                self.decoded.clone()
            }
        }
    }
}

fn classify_request_encoding(headers: &HeaderMap) -> RequestBodyEncoding {
    let Some(value) = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
    else {
        return RequestBodyEncoding::Identity;
    };
    let encodings = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if encodings.is_empty()
        || encodings
            .iter()
            .all(|item| item.eq_ignore_ascii_case("identity"))
    {
        return RequestBodyEncoding::Identity;
    }
    if encodings.len() == 1 && has_gzip_content_encoding(headers) {
        return RequestBodyEncoding::Gzip;
    }
    RequestBodyEncoding::Unsupported
}

fn restore_original_content_encoding(headers: &mut HeaderMap, original: Option<&HeaderValue>) {
    match original {
        Some(value) => {
            headers.insert(header::CONTENT_ENCODING, value.clone());
        }
        None => {
            headers.remove(header::CONTENT_ENCODING);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).expect("gzip write");
        encoder.finish().expect("gzip finish")
    }

    fn gunzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut decoder = flate2::read::GzDecoder::new(input);
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut out).expect("gzip read");
        out
    }

    fn gzip_headers(content_len: usize) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_len.to_string()).expect("len header"),
        );
        headers
    }

    #[test]
    fn unchanged_gzip_body_uses_semantic_headers_for_hooks_and_raw_bytes_for_upstream() {
        let plain = Bytes::from_static(br#"{"input":"hello 13344441520"}"#);
        let raw = Bytes::from(gzip_bytes(plain.as_ref()));
        let wire_headers = gzip_headers(raw.len());

        let body = GatewayRequestBody::from_wire(raw.clone(), &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(body.decoded(), &plain);
        assert_eq!(body.decoded_clone(), plain);
        assert!(!body.is_mutated());
        assert_eq!(upstream, raw);
        assert!(body
            .semantic_headers(&wire_headers)
            .get(header::CONTENT_ENCODING)
            .is_none());
        assert!(body
            .semantic_headers(&wire_headers)
            .get(header::CONTENT_LENGTH)
            .is_none());
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_gzip_body_is_reencoded_and_length_is_removed() {
        let plain = Bytes::from_static(br#"{"input":"hello 13344441520"}"#);
        let raw = Bytes::from(gzip_bytes(plain.as_ref()));
        let wire_headers = gzip_headers(raw.len());
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from(r#"{"input":"hello [电话]"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert!(body.is_mutated());
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
        assert_eq!(
            gunzip_bytes(upstream.as_ref()),
            r#"{"input":"hello [电话]"}"#.as_bytes()
        );
    }

    #[test]
    fn invalid_gzip_body_stays_raw_when_unchanged() {
        let raw = Bytes::from_static(b"not-gzip");
        let wire_headers = gzip_headers(raw.len());

        let body = GatewayRequestBody::from_wire(raw.clone(), &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(body.decoded(), &raw);
        assert_eq!(upstream, raw);
        assert_eq!(hook_headers.get(header::CONTENT_ENCODING).unwrap(), "gzip");
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_invalid_gzip_body_falls_back_to_identity() {
        let raw = Bytes::from_static(b"not-gzip");
        let wire_headers = gzip_headers(raw.len());
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from_static(br#"{"input":"changed"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(upstream, Bytes::from_static(br#"{"input":"changed"}"#));
        assert!(hook_headers.get(header::CONTENT_ENCODING).is_none());
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }

    #[test]
    fn mutated_unsupported_encoding_drops_encoding_header() {
        let raw = Bytes::from_static(br#"{"input":"hello"}"#);
        let mut wire_headers = HeaderMap::new();
        wire_headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("br"));
        wire_headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("17"));
        let mut body = GatewayRequestBody::from_wire(raw, &wire_headers, 1024 * 1024);
        let mut hook_headers = body.semantic_headers(&wire_headers);

        body.replace_decoded(Bytes::from_static(br#"{"input":"changed"}"#));
        let upstream = body.finalize_for_upstream(&mut hook_headers, 1024 * 1024);

        assert_eq!(upstream, Bytes::from_static(br#"{"input":"changed"}"#));
        assert!(hook_headers.get(header::CONTENT_ENCODING).is_none());
        assert!(hook_headers.get(header::CONTENT_LENGTH).is_none());
    }
}
