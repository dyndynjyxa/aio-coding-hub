use axum::body::Bytes;
use futures_core::Stream;
use serde_json::Value;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

use super::audit::build_special_setting;
use super::audit::ResponseFixerApplied;
use super::encoding::EncodingFixer;
use super::json::{fix_sse_json_lines, JsonFixer};
use super::sse::SseFixer;
use super::ResponseFixerConfig;

const CHUNK_BUFFER_COMPACT_CHUNK_THRESHOLD: usize = 1024;
const CHUNK_BUFFER_COMPACT_MIN_GROWTH_BYTES: usize = 16 * 1024;

struct ChunkBuffer {
    chunks: Vec<Bytes>,
    head: usize,
    head_offset: usize,
    total: usize,
    processable_end: usize,
    pending_cr: bool,
    next_compact_total: usize,
}

impl ChunkBuffer {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            head: 0,
            head_offset: 0,
            total: 0,
            processable_end: 0,
            pending_cr: false,
            next_compact_total: CHUNK_BUFFER_COMPACT_MIN_GROWTH_BYTES,
        }
    }

    fn len(&self) -> usize {
        self.total
    }

    fn push(&mut self, chunk: Bytes) {
        if chunk.is_empty() {
            return;
        }

        let prev_total = self.total;
        let bytes = chunk.as_ref();
        let chunk_len = bytes.len();

        if self.pending_cr {
            self.processable_end = if bytes.first() == Some(&b'\n') {
                prev_total + 1
            } else {
                prev_total
            };
            self.pending_cr = false;
        }

        for (i, b) in bytes.iter().enumerate() {
            if *b == b'\n' {
                self.processable_end = prev_total + i + 1;
                continue;
            }
            if *b != b'\r' {
                continue;
            }
            if i + 1 < bytes.len() {
                if bytes[i + 1] != b'\n' {
                    self.processable_end = prev_total + i + 1;
                }
                continue;
            }
            self.pending_cr = true;
        }

        self.chunks.push(chunk);
        self.total += chunk_len;
        self.compact_if_fragmented();
    }

    fn find_processable_end(&self) -> usize {
        if self.total == 0 {
            return 0;
        }
        if self.pending_cr {
            return 0;
        }
        self.processable_end
    }

    fn take(&mut self, size: usize) -> Vec<u8> {
        if size == 0 {
            return Vec::new();
        }
        debug_assert!(
            size <= self.total,
            "ChunkBuffer.take size ({size}) exceeds buffered length ({})",
            self.total
        );
        let size = size.min(self.total);
        if size == 0 {
            return Vec::new();
        }

        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut remaining = size;

        while remaining > 0 {
            let chunk = &self.chunks[self.head];
            let available = chunk.len().saturating_sub(self.head_offset);
            let to_copy = available.min(remaining);
            out.extend_from_slice(&chunk.as_ref()[self.head_offset..(self.head_offset + to_copy)]);

            self.head_offset += to_copy;
            self.total -= to_copy;
            remaining -= to_copy;

            if self.head_offset >= chunk.len() {
                self.head += 1;
                self.head_offset = 0;
            }
        }

        if self.total == 0 {
            self.clear();
            return out;
        }

        if self.head > 64 {
            self.chunks.drain(0..self.head);
            self.head = 0;
        }

        self.processable_end = self.processable_end.saturating_sub(size);
        out
    }

    fn drain(&mut self) -> Vec<u8> {
        let size = self.total;
        let out = self.take(size);
        self.clear();
        out
    }

    fn unread_chunk_count(&self) -> usize {
        self.chunks.len().saturating_sub(self.head)
    }

    fn compact_if_fragmented(&mut self) {
        if self.unread_chunk_count() <= CHUNK_BUFFER_COMPACT_CHUNK_THRESHOLD {
            return;
        }
        if self.total < self.next_compact_total {
            return;
        }

        self.compact_unread();
        self.next_compact_total = self
            .total
            .saturating_add(self.total.max(CHUNK_BUFFER_COMPACT_MIN_GROWTH_BYTES));
    }

    fn compact_unread(&mut self) {
        if self.total == 0 {
            self.clear();
            return;
        }

        let mut out = Vec::with_capacity(self.total);
        for i in self.head..self.chunks.len() {
            let chunk = &self.chunks[i];
            if i == self.head && self.head_offset > 0 {
                out.extend_from_slice(&chunk.as_ref()[self.head_offset..]);
                continue;
            }
            out.extend_from_slice(chunk.as_ref());
        }

        self.chunks.clear();
        self.chunks.push(Bytes::from(out));
        self.head = 0;
        self.head_offset = 0;
    }

    fn clear(&mut self) {
        self.chunks.clear();
        self.head = 0;
        self.head_offset = 0;
        self.total = 0;
        self.processable_end = 0;
        self.pending_cr = false;
        self.next_compact_total = CHUNK_BUFFER_COMPACT_MIN_GROWTH_BYTES;
    }
}

pub(super) struct ResponseFixerStreamInner<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    config: ResponseFixerConfig,
    special_settings: Arc<Mutex<Vec<Value>>>,
    started: Instant,
    total_bytes_processed: usize,
    applied: ResponseFixerApplied,
    buffer: ChunkBuffer,
    passthrough: bool,
    queued: VecDeque<Bytes>,
    pending_error: Option<reqwest::Error>,
    upstream_done: bool,
    finalized: bool,
}

impl<S> ResponseFixerStreamInner<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(super) fn new(
        upstream: S,
        config: ResponseFixerConfig,
        special_settings: Arc<Mutex<Vec<Value>>>,
    ) -> Self {
        Self {
            upstream,
            config,
            special_settings,
            started: Instant::now(),
            total_bytes_processed: 0,
            applied: ResponseFixerApplied::default(),
            buffer: ChunkBuffer::new(),
            passthrough: false,
            queued: VecDeque::new(),
            pending_error: None,
            upstream_done: false,
            finalized: false,
        }
    }

    fn finalize_if_needed(&mut self) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let hit =
            self.applied.encoding_applied || self.applied.sse_applied || self.applied.json_applied;
        if !hit {
            return;
        }

        let processing_time_ms = self.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let special = build_special_setting(
            true,
            &self.applied,
            true,
            self.total_bytes_processed,
            processing_time_ms,
        );

        super::push_special_setting(&self.special_settings, special);
    }

    fn process_bytes(&mut self, input: Bytes) -> Bytes {
        let mut data = input;

        if self.config.fix_encoding {
            let res = EncodingFixer::fix_bytes(data);
            if res.applied {
                self.applied.encoding_applied = true;
                if self.applied.encoding_details.is_none() {
                    self.applied.encoding_details = res.details;
                }
            }
            data = res.data;
        }

        if self.config.fix_sse_format {
            let res = SseFixer::fix_bytes(data);
            if res.applied {
                self.applied.sse_applied = true;
                self.applied.sse_details = self.applied.sse_details.or(res.details);
            }
            data = res.data;
        }

        if self.config.fix_truncated_json {
            let json_fixer = JsonFixer::new(self.config.max_json_depth, self.config.max_fix_size);
            let res = fix_sse_json_lines(data, &json_fixer);
            if res.applied {
                self.applied.json_applied = true;
                self.applied.json_details = self.applied.json_details.or(res.details);
            }
            data = res.data;
        }

        data
    }
}

impl<S> Stream for ResponseFixerStreamInner<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        loop {
            if let Some(next) = this.queued.pop_front() {
                return Poll::Ready(Some(Ok(next)));
            }

            if let Some(err) = this.pending_error.take() {
                return Poll::Ready(Some(Err(err)));
            }

            if this.upstream_done {
                this.finalize_if_needed();
                return Poll::Ready(None);
            }

            match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    if this.buffer.len() > 0 && !this.passthrough {
                        let drained = Bytes::from(this.buffer.drain());
                        let fixed = this.process_bytes(drained);
                        if !fixed.is_empty() {
                            this.queued.push_back(fixed);
                        }
                    } else {
                        this.buffer.clear();
                    }

                    this.upstream_done = true;
                    this.finalize_if_needed();
                    continue;
                }
                Poll::Ready(Some(Err(err))) => {
                    if this.buffer.len() > 0 && !this.passthrough {
                        let drained = Bytes::from(this.buffer.drain());
                        let fixed = this.process_bytes(drained);
                        if !fixed.is_empty() {
                            this.queued.push_back(fixed);
                        }
                    } else {
                        this.buffer.clear();
                    }

                    this.pending_error = Some(err);
                    this.upstream_done = true;
                    this.finalize_if_needed();
                    continue;
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    this.total_bytes_processed =
                        this.total_bytes_processed.saturating_add(chunk.len());

                    if this.passthrough {
                        return Poll::Ready(Some(Ok(chunk)));
                    }

                    // 安全保护：如果长时间无换行，buffer 会持续增长。达到上限后降级为透传，避免内存无界增长。
                    if this.buffer.len().saturating_add(chunk.len()) > this.config.max_fix_size {
                        this.passthrough = true;
                        let buffered = Bytes::from(this.buffer.drain());
                        if !buffered.is_empty() {
                            this.queued.push_back(buffered);
                        }
                        this.queued.push_back(chunk);
                        continue;
                    }

                    this.buffer.push(chunk);

                    let end = this.buffer.find_processable_end();
                    if end == 0 {
                        continue;
                    }

                    let to_process = Bytes::from(this.buffer.take(end));
                    let fixed = this.process_bytes(to_process);
                    if !fixed.is_empty() {
                        this.queued.push_back(fixed);
                    }
                    continue;
                }
            }
        }
    }
}

impl<S> Drop for ResponseFixerStreamInner<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    fn drop(&mut self) {
        self.finalize_if_needed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_buffer_compacts_fragmented_tiny_chunks() {
        let mut buffer = ChunkBuffer::new();
        let total = CHUNK_BUFFER_COMPACT_MIN_GROWTH_BYTES + 1;

        for _ in 0..total {
            buffer.push(Bytes::from_static(b"a"));
        }

        assert_eq!(buffer.len(), total);
        assert_eq!(buffer.find_processable_end(), 0);
        assert!(buffer.unread_chunk_count() < CHUNK_BUFFER_COMPACT_CHUNK_THRESHOLD);

        let drained = buffer.drain();
        assert_eq!(drained.len(), total);
        assert!(drained.iter().all(|b| *b == b'a'));
    }
}
