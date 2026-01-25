//! Usage: `Stream` adaptor that gunzips an upstream `bytes_stream()`.

use axum::body::Bytes;
use flate2::write::GzDecoder;
use futures_core::Stream;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Default)]
struct VecWriteBuffer {
    buf: Vec<u8>,
}

impl Write for VecWriteBuffer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl VecWriteBuffer {
    fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }
}

pub(in crate::gateway) struct GunzipStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    decoder: GzDecoder<VecWriteBuffer>,
    queued: Option<Bytes>,
    pending_error: Option<reqwest::Error>,
    upstream_done: bool,
}

impl<S> GunzipStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(upstream: S) -> Self {
        Self {
            upstream,
            decoder: GzDecoder::new(VecWriteBuffer::default()),
            queued: None,
            pending_error: None,
            upstream_done: false,
        }
    }

    fn drain_output_if_any(&mut self) {
        if self.queued.is_some() {
            return;
        }
        let out = self.decoder.get_mut().take();
        if out.is_empty() {
            return;
        }
        self.queued = Some(Bytes::from(out));
    }

    fn flush_and_drain(&mut self) {
        let _ = self.decoder.flush();
        self.drain_output_if_any();
    }
}

impl<S> Stream for GunzipStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        loop {
            if let Some(bytes) = this.queued.take() {
                return Poll::Ready(Some(Ok(bytes)));
            }

            if this.upstream_done {
                if let Some(err) = this.pending_error.take() {
                    return Poll::Ready(Some(Err(err)));
                }
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    this.upstream_done = true;
                    this.flush_and_drain();
                    continue;
                }
                Poll::Ready(Some(Err(err))) => {
                    this.upstream_done = true;
                    this.pending_error = Some(err);
                    this.flush_and_drain();
                    continue;
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    let mut had_error = false;
                    if this.decoder.write_all(chunk.as_ref()).is_err() {
                        had_error = true;
                    }
                    if this.decoder.flush().is_err() {
                        had_error = true;
                    }
                    this.drain_output_if_any();

                    if had_error {
                        // 容错：解压失败（常见于 gzip 流被提前截断）。尽可能输出已解压内容，然后直接结束流。
                        this.upstream_done = true;
                    }
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::collections::VecDeque;
    use std::future::Future;
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
}
