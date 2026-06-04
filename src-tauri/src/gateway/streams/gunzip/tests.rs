use super::*;
use flate2::{write::GzEncoder, Compression};
use std::collections::VecDeque;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll};

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

fn gzip_bytes(input: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input).expect("gzip write");
    encoder.finish().expect("gzip finish")
}

#[tokio::test]
async fn gunzip_stream_decompresses_gzip_body() {
    let original = b"hello\nworld\n";
    let gz = gzip_bytes(original);

    let mid = gz.len() / 2;
    let upstream = VecBytesStream::new(vec![
        Ok(Bytes::copy_from_slice(&gz[..mid])),
        Ok(Bytes::copy_from_slice(&gz[mid..])),
    ]);

    let out = collect_ok_bytes(GunzipStream::new(upstream)).await;
    assert_eq!(out, original);
}

#[tokio::test]
async fn gunzip_stream_ignores_truncated_gzip_and_returns_partial_output() {
    let original = b"{\"ok\":true}\n";
    let mut gz = gzip_bytes(original);
    // gzip footer is 8 bytes (CRC32 + ISIZE). Truncating it should trigger an error, but the
    // decompressor should still output the full payload in most cases.
    if gz.len() > 8 {
        gz.truncate(gz.len() - 8);
    }

    let upstream = VecBytesStream::new(vec![Ok(Bytes::from(gz))]);
    let out = collect_ok_bytes(GunzipStream::new(upstream)).await;
    assert_eq!(out, original);
}

#[tokio::test]
async fn gunzip_stream_splits_large_output_from_single_compressed_chunk() {
    let original = vec![b'x'; (GUNZIP_OUTPUT_CHUNK_BYTES * 2) + 17];
    let gz = gzip_bytes(&original);
    let upstream = VecBytesStream::new(vec![Ok(Bytes::from(gz))]);
    let mut stream = GunzipStream::new(upstream);

    let first = next_item(&mut stream)
        .await
        .expect("first chunk")
        .expect("first chunk ok");
    let second = next_item(&mut stream)
        .await
        .expect("second chunk")
        .expect("second chunk ok");
    let third = next_item(&mut stream)
        .await
        .expect("third chunk")
        .expect("third chunk ok");

    assert_eq!(first.len(), GUNZIP_OUTPUT_CHUNK_BYTES);
    assert_eq!(second.len(), GUNZIP_OUTPUT_CHUNK_BYTES);
    assert_eq!(third.len(), 17);
    assert!(next_item(&mut stream).await.is_none());

    let mut out = Vec::new();
    out.extend_from_slice(&first);
    out.extend_from_slice(&second);
    out.extend_from_slice(&third);
    assert_eq!(out, original);
}
