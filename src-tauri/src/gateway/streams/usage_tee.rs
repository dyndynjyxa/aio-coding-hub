//! Usage: Streaming tee wrappers that emit usage/cost and enqueue request logs.

use crate::usage;
use axum::body::{Body, Bytes};
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use super::super::events::{emit_circuit_transition, emit_request_event};
use super::super::proxy::{
    spawn_enqueue_request_log_with_backpressure, ErrorCategory, RequestLogEnqueueArgs,
};
use super::super::response_fixer;
use super::super::util::now_unix_seconds;
use super::{RelayBodyStream, StreamFinalizeCtx};

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

pub(in crate::gateway) struct UsageSseTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    upstream: S,
    tracker: usage::SseUsageTracker,
    ctx: StreamFinalizeCtx,
    first_byte_ms: Option<u128>,
    idle_timeout: Option<Duration>,
    idle_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B> UsageSseTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx,
        idle_timeout: Option<Duration>,
        initial_first_byte_ms: Option<u128>,
    ) -> Self {
        Self {
            upstream,
            tracker: usage::SseUsageTracker::new(&ctx.cli_key),
            ctx,
            first_byte_ms: initial_first_byte_ms,
            idle_timeout,
            idle_sleep: idle_timeout.map(|d| Box::pin(tokio::time::sleep(d))),
            finalized: false,
        }
    }

    fn finalize(&mut self, error_code: Option<&'static str>) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let duration_ms = self.ctx.started.elapsed().as_millis();
        let usage = self.tracker.finalize();
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let requested_model = self
            .ctx
            .requested_model
            .clone()
            .or_else(|| self.tracker.best_effort_model());
        let effective_error_category = if error_code == Some("GW_STREAM_ABORTED") {
            Some(ErrorCategory::ClientAbort.as_str())
        } else {
            self.ctx.error_category
        };

        let now_unix = now_unix_seconds() as i64;
        if error_code.is_some()
            && effective_error_category != Some(ErrorCategory::ClientAbort.as_str())
            && self.ctx.provider_cooldown_secs > 0
        {
            self.ctx.circuit.trigger_cooldown(
                self.ctx.provider_id,
                now_unix,
                self.ctx.provider_cooldown_secs,
            );
        }
        if error_code.is_none() && (200..300).contains(&self.ctx.status) {
            let change = self
                .ctx
                .circuit
                .record_success(self.ctx.provider_id, now_unix);
            if let Some(t) = change.transition {
                emit_circuit_transition(
                    &self.ctx.app,
                    &self.ctx.trace_id,
                    &self.ctx.cli_key,
                    self.ctx.provider_id,
                    &self.ctx.provider_name,
                    &self.ctx.base_url,
                    &t,
                    now_unix,
                );
            }
            if let Some(session_id) = self.ctx.session_id.as_deref() {
                self.ctx.session.bind_success(
                    &self.ctx.cli_key,
                    session_id,
                    self.ctx.provider_id,
                    self.ctx.sort_mode_id,
                    now_unix,
                );
            }
        } else if effective_error_category == Some(ErrorCategory::ProviderError.as_str()) {
            let change = self
                .ctx
                .circuit
                .record_failure(self.ctx.provider_id, now_unix);
            if let Some(t) = change.transition {
                emit_circuit_transition(
                    &self.ctx.app,
                    &self.ctx.trace_id,
                    &self.ctx.cli_key,
                    self.ctx.provider_id,
                    &self.ctx.provider_name,
                    &self.ctx.base_url,
                    &t,
                    now_unix,
                );
            }
        }

        emit_request_event(
            &self.ctx.app,
            self.ctx.trace_id.clone(),
            self.ctx.cli_key.clone(),
            self.ctx.method.clone(),
            self.ctx.path.clone(),
            self.ctx.query.clone(),
            Some(self.ctx.status),
            effective_error_category,
            error_code,
            duration_ms,
            self.first_byte_ms,
            self.ctx.attempts.clone(),
            usage_metrics,
        );

        spawn_enqueue_request_log_with_backpressure(
            self.ctx.app.clone(),
            self.ctx.db.clone(),
            self.ctx.log_tx.clone(),
            RequestLogEnqueueArgs {
                trace_id: self.ctx.trace_id.clone(),
                cli_key: self.ctx.cli_key.clone(),
                session_id: self.ctx.session_id.clone(),
                method: self.ctx.method.clone(),
                path: self.ctx.path.clone(),
                query: self.ctx.query.clone(),
                excluded_from_stats: self.ctx.excluded_from_stats,
                special_settings_json: response_fixer::special_settings_json(
                    &self.ctx.special_settings,
                ),
                status: Some(self.ctx.status),
                error_code,
                duration_ms,
                ttfb_ms: self.first_byte_ms,
                attempts_json: self.ctx.attempts_json.clone(),
                requested_model,
                created_at_ms: self.ctx.created_at_ms,
                created_at: self.ctx.created_at,
                usage,
            },
        );
    }
}

impl<S, B> Stream for UsageSseTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        let next = Pin::new(&mut this.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                if let Some(timer) = this.idle_sleep.as_mut() {
                    if timer.as_mut().poll(cx).is_ready() {
                        this.finalize(Some("GW_STREAM_IDLE_TIMEOUT"));
                        return Poll::Ready(None);
                    }
                }
                Poll::Pending
            }
            Poll::Ready(None) => {
                this.finalize(this.ctx.error_code);
                Poll::Ready(None)
            }
            Poll::Ready(Some(Ok(chunk))) => {
                if this.first_byte_ms.is_none() {
                    this.first_byte_ms = Some(this.ctx.started.elapsed().as_millis());
                }
                if let Some(d) = this.idle_timeout {
                    this.idle_sleep = Some(Box::pin(tokio::time::sleep(d)));
                }
                this.tracker.ingest_chunk(chunk.as_ref());
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize(Some("GW_STREAM_ERROR"));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B> Drop for UsageSseTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    fn drop(&mut self) {
        if !self.finalized {
            self.finalize(Some("GW_STREAM_ABORTED"));
        }
    }
}

const SSE_RELAY_BUFFER_CAPACITY: usize = 32;

pub(in crate::gateway) fn spawn_usage_sse_relay_body<S>(
    upstream: S,
    ctx: StreamFinalizeCtx,
    idle_timeout: Option<Duration>,
    initial_first_byte_ms: Option<u128>,
) -> Body
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    let (tx, rx) =
        tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(SSE_RELAY_BUFFER_CAPACITY);

    let mut tee = UsageSseTeeStream::new(upstream, ctx, idle_timeout, initial_first_byte_ms);

    tokio::spawn(async move {
        let mut forwarded_chunks: i64 = 0;
        let mut forwarded_bytes: i64 = 0;
        let mut client_abort_detected_by: Option<&'static str> = None;

        loop {
            tokio::select! {
                // 如果客户端提前断开，但上游短时间没有新 chunk，就会卡在 next_item().await。
                // 这里通过监听 rx 端被 drop 来更早感知断开，避免误记 GW_STREAM_ABORTED。
                _ = tx.closed() => {
                    client_abort_detected_by = Some("rx_closed");
                    break;
                }
                item = next_item(&mut tee) => {
                    let Some(item) = item else {
                        break;
                    };

                    match item {
                        Ok(chunk) => {
                            let chunk_len = chunk.len().min(i64::MAX as usize) as i64;

                            if tx.send(Ok(chunk)).await.is_err() {
                                client_abort_detected_by = Some("send_failed");
                                break;
                            }

                            forwarded_chunks = forwarded_chunks.saturating_add(1);
                            forwarded_bytes = forwarded_bytes.saturating_add(chunk_len);
                        }
                        Err(err) => {
                            // 尽力把流错误透传给客户端
                            let _ = tx.send(Err(err)).await;
                            break;
                        }
                    }
                }
            }
        }

        if let Some(detected_by) = client_abort_detected_by {
            let duration_ms = tee.ctx.started.elapsed().as_millis().min(i64::MAX as u128) as i64;
            let ttfb_ms = tee.first_byte_ms.and_then(|v| {
                if v >= duration_ms as u128 {
                    return None;
                }
                Some(v.min(i64::MAX as u128) as i64)
            });

            if let Ok(mut guard) = tee.ctx.special_settings.lock() {
                guard.push(serde_json::json!({
                    "type": "client_abort",
                    "scope": "stream",
                    "reason": "client_disconnected",
                    "detected_by": detected_by,
                    "duration_ms": duration_ms,
                    "ttfb_ms": ttfb_ms,
                    "forwarded_chunks": forwarded_chunks,
                    "forwarded_bytes": forwarded_bytes,
                    "ts": now_unix_seconds() as i64,
                }));
            }

            // 对齐 claude-code-hub：client abort 不等价于 request failed。
            // 这里按“已开始处理但客户端提前断开”收敛，不写入 GW_STREAM_ABORTED。
            tee.finalize(None);
        }
    });

    Body::from_stream(RelayBodyStream::new(rx))
}

pub(in crate::gateway) struct UsageBodyBufferTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    upstream: S,
    ctx: StreamFinalizeCtx,
    first_byte_ms: Option<u128>,
    buffer: Vec<u8>,
    max_bytes: usize,
    truncated: bool,
    total_timeout: Option<Duration>,
    total_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B> UsageBodyBufferTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx,
        max_bytes: usize,
        total_timeout: Option<Duration>,
    ) -> Self {
        let remaining = total_timeout.and_then(|d| d.checked_sub(ctx.started.elapsed()));
        Self {
            upstream,
            ctx,
            first_byte_ms: None,
            buffer: Vec::new(),
            max_bytes,
            truncated: false,
            total_timeout,
            total_sleep: remaining.map(|d| Box::pin(tokio::time::sleep(d))),
            finalized: false,
        }
    }

    fn finalize(&mut self, error_code: Option<&'static str>) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let duration_ms = self.ctx.started.elapsed().as_millis();
        let usage = if self.truncated || self.buffer.is_empty() {
            None
        } else {
            usage::parse_usage_from_json_bytes(&self.buffer)
        };
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let requested_model = self.ctx.requested_model.clone().or_else(|| {
            if self.truncated || self.buffer.is_empty() {
                None
            } else {
                usage::parse_model_from_json_bytes(&self.buffer)
            }
        });
        let effective_error_category = if error_code == Some("GW_STREAM_ABORTED") {
            Some(ErrorCategory::ClientAbort.as_str())
        } else {
            self.ctx.error_category
        };

        let now_unix = now_unix_seconds() as i64;
        if error_code.is_some()
            && effective_error_category != Some(ErrorCategory::ClientAbort.as_str())
            && self.ctx.provider_cooldown_secs > 0
        {
            self.ctx.circuit.trigger_cooldown(
                self.ctx.provider_id,
                now_unix,
                self.ctx.provider_cooldown_secs,
            );
        }
        if error_code.is_none() && (200..300).contains(&self.ctx.status) {
            let change = self
                .ctx
                .circuit
                .record_success(self.ctx.provider_id, now_unix);
            if let Some(t) = change.transition {
                emit_circuit_transition(
                    &self.ctx.app,
                    &self.ctx.trace_id,
                    &self.ctx.cli_key,
                    self.ctx.provider_id,
                    &self.ctx.provider_name,
                    &self.ctx.base_url,
                    &t,
                    now_unix,
                );
            }
            if let Some(session_id) = self.ctx.session_id.as_deref() {
                self.ctx.session.bind_success(
                    &self.ctx.cli_key,
                    session_id,
                    self.ctx.provider_id,
                    self.ctx.sort_mode_id,
                    now_unix,
                );
            }
        } else if effective_error_category == Some(ErrorCategory::ProviderError.as_str()) {
            let change = self
                .ctx
                .circuit
                .record_failure(self.ctx.provider_id, now_unix);
            if let Some(t) = change.transition {
                emit_circuit_transition(
                    &self.ctx.app,
                    &self.ctx.trace_id,
                    &self.ctx.cli_key,
                    self.ctx.provider_id,
                    &self.ctx.provider_name,
                    &self.ctx.base_url,
                    &t,
                    now_unix,
                );
            }
        }

        emit_request_event(
            &self.ctx.app,
            self.ctx.trace_id.clone(),
            self.ctx.cli_key.clone(),
            self.ctx.method.clone(),
            self.ctx.path.clone(),
            self.ctx.query.clone(),
            Some(self.ctx.status),
            effective_error_category,
            error_code,
            duration_ms,
            self.first_byte_ms,
            self.ctx.attempts.clone(),
            usage_metrics,
        );

        spawn_enqueue_request_log_with_backpressure(
            self.ctx.app.clone(),
            self.ctx.db.clone(),
            self.ctx.log_tx.clone(),
            RequestLogEnqueueArgs {
                trace_id: self.ctx.trace_id.clone(),
                cli_key: self.ctx.cli_key.clone(),
                session_id: self.ctx.session_id.clone(),
                method: self.ctx.method.clone(),
                path: self.ctx.path.clone(),
                query: self.ctx.query.clone(),
                excluded_from_stats: self.ctx.excluded_from_stats,
                special_settings_json: response_fixer::special_settings_json(
                    &self.ctx.special_settings,
                ),
                status: Some(self.ctx.status),
                error_code,
                duration_ms,
                ttfb_ms: self.first_byte_ms,
                attempts_json: self.ctx.attempts_json.clone(),
                requested_model,
                created_at_ms: self.ctx.created_at_ms,
                created_at: self.ctx.created_at,
                usage,
            },
        );
    }
}

impl<S, B> Stream for UsageBodyBufferTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(total) = this.total_timeout {
            if this.ctx.started.elapsed() >= total {
                this.finalize(Some("GW_UPSTREAM_TIMEOUT"));
                return Poll::Ready(None);
            }
        }

        let next = Pin::new(&mut this.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                if let Some(timer) = this.total_sleep.as_mut() {
                    if timer.as_mut().poll(cx).is_ready() {
                        this.finalize(Some("GW_UPSTREAM_TIMEOUT"));
                        return Poll::Ready(None);
                    }
                }
                Poll::Pending
            }
            Poll::Ready(None) => {
                this.finalize(this.ctx.error_code);
                Poll::Ready(None)
            }
            Poll::Ready(Some(Ok(chunk))) => {
                if this.first_byte_ms.is_none() {
                    this.first_byte_ms = Some(this.ctx.started.elapsed().as_millis());
                }
                if !this.truncated {
                    let bytes = chunk.as_ref();
                    if this.buffer.len().saturating_add(bytes.len()) <= this.max_bytes {
                        this.buffer.extend_from_slice(bytes);
                    } else {
                        this.truncated = true;
                        this.buffer.clear();
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize(Some("GW_STREAM_ERROR"));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B> Drop for UsageBodyBufferTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    fn drop(&mut self) {
        if !self.finalized {
            self.finalize(Some("GW_STREAM_ABORTED"));
        }
    }
}
