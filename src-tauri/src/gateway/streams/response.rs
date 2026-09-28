//! A transport-independent response body shared by HTTP and Responses WebSocket.

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::header::{self, HeaderMap};
use reqwest::StatusCode;
use std::pin::Pin;

pub(crate) type UpstreamStreamError = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type UpstreamByteStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, UpstreamStreamError>> + Send>>;

pub(crate) struct UpstreamResponse {
    status: StatusCode,
    websocket: bool,
    pub(crate) processed: bool,
    headers: HeaderMap,
    content_length: Option<u64>,
    body: UpstreamByteStream,
}

impl UpstreamResponse {
    pub(crate) fn new<S>(status: StatusCode, headers: HeaderMap, body: S) -> Self
    where
        S: Stream<Item = Result<Bytes, UpstreamStreamError>> + Send + 'static,
    {
        let content_length = headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok());
        Self {
            status,
            websocket: false,
            processed: false,
            headers,
            content_length,
            body: Box::pin(body),
        }
    }

    pub(crate) fn new_ws<S>(headers: HeaderMap, body: S) -> Self
    where
        S: Stream<Item = Result<Bytes, UpstreamStreamError>> + Send + 'static,
    {
        let mut response = Self::new(StatusCode::SWITCHING_PROTOCOLS, headers, body);
        response.websocket = true;
        response.content_length = None;
        response
    }

    pub(crate) fn is_websocket(&self) -> bool {
        self.websocket
    }

    pub(crate) fn status(&self) -> StatusCode {
        self.status
    }

    pub(crate) fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub(crate) fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    pub(crate) async fn chunk(&mut self) -> Result<Option<Bytes>, UpstreamStreamError> {
        self.body.next().await.transpose()
    }

    pub(crate) fn bytes_stream(self) -> UpstreamByteStream {
        self.body
    }
}

impl From<reqwest::Response> for UpstreamResponse {
    fn from(response: reqwest::Response) -> Self {
        Self {
            status: response.status(),
            websocket: false,
            processed: false,
            headers: response.headers().clone(),
            content_length: response.content_length(),
            body: Box::pin(
                response
                    .bytes_stream()
                    .map(|result| result.map_err(|error| Box::new(error) as UpstreamStreamError)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;
    use std::io;

    #[tokio::test]
    async fn probing_a_chunk_preserves_the_rest_and_non_http_errors() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("6"));
        let items: Vec<Result<Bytes, UpstreamStreamError>> = vec![
            Ok(Bytes::from_static(b"one")),
            Ok(Bytes::from_static(b"two")),
            Err(io::Error::new(io::ErrorKind::ConnectionReset, "upstream closed").into()),
        ];
        let mut response =
            UpstreamResponse::new(StatusCode::OK, headers, futures_util::stream::iter(items));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.content_length(), Some(6));
        assert_eq!(
            response.chunk().await.unwrap(),
            Some(Bytes::from_static(b"one"))
        );
        let mut stream = response.bytes_stream();
        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            Bytes::from_static(b"two")
        );
        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::ConnectionReset
        );
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn unknown_length_empty_stream_stays_empty() {
        let mut response = UpstreamResponse::new(
            StatusCode::NO_CONTENT,
            HeaderMap::new(),
            futures_util::stream::empty(),
        );
        assert_eq!(response.content_length(), None);
        assert!(response.chunk().await.unwrap().is_none());
    }
}
