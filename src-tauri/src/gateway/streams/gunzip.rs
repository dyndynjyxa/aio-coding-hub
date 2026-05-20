//! Usage: `Stream` adaptor that gunzips an upstream `bytes_stream()`.

use axum::body::Bytes;
use flate2::{Decompress, FlushDecompress, Status};
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

const GUNZIP_OUTPUT_CHUNK_BYTES: usize = 64 * 1024;

enum PumpOutcome {
    Output(Bytes),
    NeedInput,
    Done,
}

pub(in crate::gateway) struct GunzipStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    decoder: Decompress,
    current_input: Option<Bytes>,
    current_offset: usize,
    pending_error: Option<reqwest::Error>,
    upstream_done: bool,
    decoder_done: bool,
}

impl<S> GunzipStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(upstream: S) -> Self {
        Self {
            upstream,
            decoder: Decompress::new_gzip(15),
            current_input: None,
            current_offset: 0,
            pending_error: None,
            upstream_done: false,
            decoder_done: false,
        }
    }

    fn pump_decoder(&mut self) -> PumpOutcome {
        if self.decoder_done {
            return PumpOutcome::Done;
        }

        loop {
            let mut output = vec![0u8; GUNZIP_OUTPUT_CHUNK_BYTES];
            let (status, consumed, written, input_was_empty) = {
                let input = self
                    .current_input
                    .as_ref()
                    .map(|chunk| &chunk.as_ref()[self.current_offset..])
                    .unwrap_or(&[]);

                if input.is_empty() && !self.upstream_done {
                    return PumpOutcome::NeedInput;
                }

                let flush = if self.upstream_done {
                    FlushDecompress::Finish
                } else {
                    FlushDecompress::None
                };
                let before_in = self.decoder.total_in();
                let before_out = self.decoder.total_out();
                let status = match self.decoder.decompress(input, &mut output, flush) {
                    Ok(status) => status,
                    Err(err) => {
                        tracing::warn!(error = %err, "ending gzip stream after decompression error");
                        self.decoder_done = true;
                        return PumpOutcome::Done;
                    }
                };
                (
                    status,
                    self.decoder.total_in().saturating_sub(before_in) as usize,
                    self.decoder.total_out().saturating_sub(before_out) as usize,
                    input.is_empty(),
                )
            };

            if consumed > 0 {
                self.current_offset = self.current_offset.saturating_add(consumed);
                if self
                    .current_input
                    .as_ref()
                    .is_some_and(|chunk| self.current_offset >= chunk.len())
                {
                    self.current_input = None;
                    self.current_offset = 0;
                }
            }

            if written > 0 {
                output.truncate(written);
                if status == Status::StreamEnd {
                    self.decoder_done = true;
                }
                return PumpOutcome::Output(Bytes::from(output));
            }

            match status {
                Status::StreamEnd => {
                    self.decoder_done = true;
                    return PumpOutcome::Done;
                }
                Status::Ok if consumed > 0 => continue,
                Status::Ok | Status::BufError if input_was_empty && !self.upstream_done => {
                    return PumpOutcome::NeedInput;
                }
                Status::Ok | Status::BufError if self.upstream_done => {
                    self.decoder_done = true;
                    return PumpOutcome::Done;
                }
                Status::Ok | Status::BufError => {
                    tracing::warn!("ending gzip stream after decompressor stopped making progress");
                    self.decoder_done = true;
                    return PumpOutcome::Done;
                }
            }
        }
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
            if let PumpOutcome::Output(bytes) = this.pump_decoder() {
                return Poll::Ready(Some(Ok(bytes)));
            }

            if this.decoder_done {
                if let Some(err) = this.pending_error.take() {
                    return Poll::Ready(Some(Err(err)));
                }
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    this.upstream_done = true;
                    continue;
                }
                Poll::Ready(Some(Err(err))) => {
                    this.upstream_done = true;
                    this.pending_error = Some(err);
                    continue;
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    this.current_input = Some(chunk);
                    this.current_offset = 0;
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
