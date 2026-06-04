//! Usage: Streaming tee wrappers that emit usage/cost and enqueue request logs.

use crate::gateway::response_fixer;
use crate::usage;
use axum::body::{Body, Bytes};
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use super::super::events::{emit_gateway_debug_log, emit_gateway_debug_log_lazy};
use super::super::proxy::{
    is_fake_200_non_stream_body, upstream_client_error_rules, GatewayErrorCode,
};
use super::super::util::{lossy_utf8_preview, now_unix_seconds, MAX_DEBUG_BODY_PREVIEW_BYTES};
use super::request_end::{emit_request_event_and_spawn_request_log, StreamRequestCompletion};
use super::{RelayBodyStream, StreamFinalizeCtx};

fn is_codex_responses_path(cli_key: &str, path: &str) -> bool {
    if cli_key != "codex" {
        return false;
    }
    matches!(path.trim_end_matches('/'), "/v1/responses" | "/responses")
}

#[allow(clippy::too_many_arguments)]
fn is_codex_client_abort_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
    terminal_error_seen: bool,
    upstream_ended_normally: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        // For codex, downstream disconnect can race with trailing markers.
        // If completion/usage is already observed, do not downgrade to 499.
        && (usage_seen
            || completion_seen
            // If downstream disconnected and upstream never naturally ended, trailing terminal
            // markers are often disconnect side-effects and should not force a 499.
            || !terminal_error_seen
            || !upstream_ended_normally)
}

fn is_codex_drop_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
    terminal_error_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        // If completion/usage is observed, tolerate terminal markers during teardown.
        && (usage_seen || completion_seen || !terminal_error_seen)
}

fn is_codex_stream_terminal_error_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
) -> bool {
    is_codex_stream_tail_error_successish(
        cli_key,
        path,
        status,
        saw_stream_output,
        completion_seen,
        usage_seen,
    )
}

fn is_codex_stream_tail_error_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        && (usage_seen || completion_seen)
}

fn is_codex_body_buffer_drop_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    usage_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        && usage_seen
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

pub(in crate::gateway) struct UsageSseTeeStream<S, B, R = tauri::Wry>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    upstream: S,
    tracker: usage::SseUsageTracker,
    ctx: StreamFinalizeCtx<R>,
    first_byte_ms: Option<u128>,
    idle_timeout: Option<Duration>,
    idle_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
    defer_terminal_error: bool,
}

impl<S, B, R> UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx<R>,
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
            defer_terminal_error: false,
        }
    }

    pub(in crate::gateway) fn with_defer_terminal_error(mut self) -> Self {
        self.defer_terminal_error = true;
        self
    }

    fn poll_next_inner(
        &mut self,
        cx: &mut Context<'_>,
        enforce_idle_timeout: bool,
    ) -> Poll<Option<Result<B, reqwest::Error>>> {
        let next = Pin::new(&mut self.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                if enforce_idle_timeout {
                    if let Some(timer) = self.idle_sleep.as_mut() {
                        if timer.as_mut().poll(cx).is_ready() {
                            self.finalize(Some(GatewayErrorCode::StreamIdleTimeout.as_str()));
                            return Poll::Ready(None);
                        }
                    }
                }
                Poll::Pending
            }
            Poll::Ready(None) => {
                // When defer_terminal_error is set and the tracker saw a terminal
                // error, skip finalization here — the relay task will decide the
                // final error_code with Codex-specific tolerance logic.
                if !(self.defer_terminal_error && self.tracker.terminal_error_seen()) {
                    self.finalize(self.ctx.error_code);
                }
                Poll::Ready(None)
            }
            Poll::Ready(Some(Ok(chunk))) => {
                if self.first_byte_ms.is_none() {
                    self.first_byte_ms = Some(self.ctx.started.elapsed().as_millis());
                }
                // Reuse existing Box allocation via Sleep::reset() to avoid heap churn per chunk
                if let Some(d) = self.idle_timeout {
                    if let Some(ref mut sleep) = self.idle_sleep {
                        sleep.as_mut().reset(tokio::time::Instant::now() + d);
                    } else {
                        self.idle_sleep = Some(Box::pin(tokio::time::sleep(d)));
                    }
                }
                emit_gateway_debug_log_lazy(&self.ctx.app, || {
                    format!(
                        "[SSE_CHUNK] trace_id={} len={}\n  {}",
                        self.ctx.trace_id,
                        chunk.as_ref().len(),
                        lossy_utf8_preview(chunk.as_ref(), MAX_DEBUG_BODY_PREVIEW_BYTES),
                    )
                });
                let was_terminal_error = self.tracker.terminal_error_seen();
                self.tracker.ingest_chunk(chunk.as_ref());
                if self.tracker.terminal_error_seen() {
                    if !was_terminal_error {
                        emit_gateway_debug_log(
                            &self.ctx.app,
                            format!(
                                "[SSE] terminal_error_seen triggered — trace_id={} cli_key={} path={} fake_200={}",
                                self.ctx.trace_id,
                                self.ctx.cli_key,
                                self.ctx.path,
                                self.tracker.fake_200_detected(),
                            ),
                        );
                    }
                    if !self.defer_terminal_error {
                        let code = if self.tracker.fake_200_detected() {
                            GatewayErrorCode::Fake200.as_str()
                        } else {
                            GatewayErrorCode::StreamError.as_str()
                        };
                        self.finalize(Some(code));
                        return Poll::Ready(None);
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                let completion_seen = self.tracker.completion_seen();
                let codex_successish = is_codex_stream_tail_error_successish(
                    &self.ctx.cli_key,
                    &self.ctx.path,
                    self.ctx.status,
                    self.first_byte_ms.is_some(),
                    completion_seen,
                    completion_seen,
                );
                if codex_successish {
                    emit_gateway_debug_log(
                        &self.ctx.app,
                        format!(
                            "[SSE] tolerating late stream read error after completion trace_id={} cli_key={} path={} err={}",
                            self.ctx.trace_id, self.ctx.cli_key, self.ctx.path, err
                        ),
                    );
                    self.finalize(None);
                    Poll::Ready(None)
                } else {
                    self.finalize(Some(GatewayErrorCode::StreamError.as_str()));
                    Poll::Ready(Some(Err(err)))
                }
            }
        }
    }

    fn finalize(&mut self, error_code: Option<&'static str>) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let usage = self.tracker.finalize();

        // Propagate fake 200 detection from tracker to finalize context.
        if self.tracker.fake_200_detected() {
            self.ctx.fake_200_detected = true;
        }
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let requested_model = self
            .ctx
            .requested_model
            .clone()
            .or_else(|| self.tracker.best_effort_model());

        emit_request_event_and_spawn_request_log(
            &self.ctx,
            StreamRequestCompletion::from_error_code(
                error_code,
                self.first_byte_ms,
                requested_model,
                usage_metrics,
                usage,
            ),
        );
    }
}

impl<S, B, R> Stream for UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        this.poll_next_inner(cx, true)
    }
}

struct DrainNextFuture<'a, S, B, R>(&'a mut UsageSseTeeStream<S, B, R>)
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin;

impl<'a, S, B, R> Future for DrainNextFuture<'a, S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Output = Option<Result<B, reqwest::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_next_inner(cx, false)
    }
}

async fn next_drain_item<S, B, R>(
    stream: &mut UsageSseTeeStream<S, B, R>,
) -> Option<Result<B, reqwest::Error>>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    DrainNextFuture(stream).await
}

impl<S, B, R> Drop for UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    fn drop(&mut self) {
        if !self.finalized {
            // Best-effort flush for trailing partial SSE data before deciding abort/success.
            let usage = self.tracker.finalize();
            let usage_seen = usage.is_some();
            let completion_seen = self.tracker.completion_seen();
            let terminal_error_seen = self.tracker.terminal_error_seen();

            let codex_successish = is_codex_drop_successish(
                &self.ctx.cli_key,
                &self.ctx.path,
                self.ctx.status,
                self.first_byte_ms.is_some(),
                completion_seen,
                usage_seen,
                terminal_error_seen,
            );

            if codex_successish {
                self.finalize(None);
            } else {
                self.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    }
}

const SSE_RELAY_BUFFER_CAPACITY: usize = 32;

pub(in crate::gateway) fn spawn_usage_sse_relay_body<S, R>(
    upstream: S,
    ctx: StreamFinalizeCtx<R>,
    idle_timeout: Option<Duration>,
    initial_first_byte_ms: Option<u128>,
) -> Body
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let (tx, rx) =
        tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(SSE_RELAY_BUFFER_CAPACITY);

    let mut tee = UsageSseTeeStream::new(upstream, ctx, idle_timeout, initial_first_byte_ms)
        .with_defer_terminal_error();

    tokio::spawn(async move {
        let mut forwarded_chunks: i64 = 0;
        let mut forwarded_bytes: i64 = 0;
        let mut drained_chunks: i64 = 0;
        let mut drained_bytes: i64 = 0;
        let mut client_abort_detected_by: Option<&'static str> = None;
        let mut downstream_closed = false;
        let mut upstream_ended_normally = false;

        let is_codex_responses = is_codex_responses_path(&tee.ctx.cli_key, &tee.ctx.path);
        let mut drain_deadline: Option<tokio::time::Instant> = None;
        let drain_grace = {
            // Codex ChatGPT backend: response.completed (carrying usage) may arrive
            // several seconds after the last output chunk; use a longer drain window.
            let cap = if is_codex_responses {
                Duration::from_secs(15)
            } else {
                Duration::from_secs(5)
            };
            let floor = Duration::from_millis(500);
            match idle_timeout {
                Some(d) if d < floor => floor,
                Some(d) if d > cap => cap,
                Some(d) => d,
                None => {
                    if is_codex_responses {
                        Duration::from_secs(10)
                    } else {
                        Duration::from_secs(2)
                    }
                }
            }
        };

        loop {
            if downstream_closed {
                if !is_codex_responses {
                    break;
                }
                // Keep draining until completion/deadline/end-of-stream.
                // Some Codex backend flows can emit transient error-like markers before
                // `response.completed` (with usage) arrives.
                if tee.tracker.completion_seen() {
                    break;
                }
                let Some(deadline) = drain_deadline else {
                    break;
                };
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    break;
                }

                let remaining = deadline.saturating_duration_since(now);
                match tokio::time::timeout(remaining, next_drain_item(&mut tee)).await {
                    Ok(Some(Ok(chunk))) => {
                        let chunk_len = chunk.len().min(i64::MAX as usize) as i64;
                        drained_chunks = drained_chunks.saturating_add(1);
                        drained_bytes = drained_bytes.saturating_add(chunk_len);
                    }
                    Ok(Some(Err(_))) => {
                        break;
                    }
                    Ok(None) => {
                        upstream_ended_normally = true;
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
                continue;
            }

            tokio::select! {
                biased;
                // 如果客户端提前断开，但上游短时间没有新 chunk，就会卡在 next_item().await。
                // 这里通过监听 rx 端被 drop 来更早感知断开，避免误记 GW_STREAM_ABORTED。
                // 如果断开和 idle timeout 同时 ready，断开应优先进入 Codex drain。
                _ = tx.closed() => {
                    client_abort_detected_by = Some("rx_closed");
                    downstream_closed = true;
                    if is_codex_responses {
                        drain_deadline = Some(tokio::time::Instant::now() + drain_grace);
                        continue;
                    }
                    break;
                }
                item = next_item(&mut tee) => {
                    let Some(item) = item else {
                        upstream_ended_normally = true;
                        break;
                    };

                    match item {
                        Ok(chunk) => {
                            let chunk_len = chunk.len().min(i64::MAX as usize) as i64;

                            if tx.send(Ok(chunk)).await.is_err() {
                                client_abort_detected_by = Some("send_failed");
                                downstream_closed = true;
                                if is_codex_responses {
                                    drain_deadline = Some(tokio::time::Instant::now() + drain_grace);
                                    continue;
                                }
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

        // When the stream ended normally (no client abort) but the tracker
        // detected a terminal-error-like SSE event, apply Codex-specific
        // tolerance: if we already saw output AND completion/usage, treat the
        // request as successful instead of marking it GW_STREAM_ERROR.
        if client_abort_detected_by.is_none() && tee.tracker.terminal_error_seen() {
            let completion_seen = tee.tracker.completion_seen();
            let saw_stream_output = tee.first_byte_ms.is_some()
                || forwarded_chunks > 0
                || forwarded_bytes > 0
                || drained_chunks > 0
                || drained_bytes > 0;

            // For Codex /v1/responses, completion_seen implies response.completed
            // was received (which carries usage). We treat this as a proxy for
            // usage_seen to avoid consuming tracker state before tee.finalize().
            let codex_successish = is_codex_stream_terminal_error_successish(
                &tee.ctx.cli_key,
                &tee.ctx.path,
                tee.ctx.status,
                saw_stream_output,
                completion_seen,
                completion_seen,
            );

            if codex_successish {
                tee.finalize(None);
            } else {
                let code = if tee.tracker.fake_200_detected() {
                    GatewayErrorCode::Fake200.as_str()
                } else {
                    GatewayErrorCode::StreamError.as_str()
                };
                tee.finalize(Some(code));
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
            // Flush pending partial SSE data before deciding abort/success.
            let usage = tee.tracker.finalize();
            let usage_seen = usage.is_some();
            let completion_seen = tee.tracker.completion_seen();
            let terminal_error_seen = tee.tracker.terminal_error_seen();
            let saw_stream_output = tee.first_byte_ms.is_some()
                || forwarded_chunks > 0
                || forwarded_bytes > 0
                || drained_chunks > 0
                || drained_bytes > 0;

            response_fixer::push_special_setting(
                &tee.ctx.special_settings,
                serde_json::json!({
                    "type": "client_abort",
                    "scope": "stream",
                    "reason": "client_disconnected",
                    "detected_by": detected_by,
                    "duration_ms": duration_ms,
                    "ttfb_ms": ttfb_ms,
                    "forwarded_chunks": forwarded_chunks,
                    "forwarded_bytes": forwarded_bytes,
                    "drained_chunks": drained_chunks,
                    "drained_bytes": drained_bytes,
                    "upstream_ended_normally": upstream_ended_normally,
                    "completion_seen": completion_seen,
                    "terminal_error_seen": terminal_error_seen,
                    "saw_stream_output": saw_stream_output,
                    "ts": now_unix_seconds() as i64,
                }),
            );

            // Codex SSE: 2xx + saw output + no terminal error => treat client disconnect as success.
            // Do NOT require completion_seen: ChatGPT backend's response.completed may arrive
            // after the client disconnects and the drain window may not capture it.
            let codex_successish = is_codex_client_abort_successish(
                &tee.ctx.cli_key,
                &tee.ctx.path,
                tee.ctx.status,
                saw_stream_output,
                completion_seen,
                usage_seen,
                terminal_error_seen,
                upstream_ended_normally,
            );
            if codex_successish {
                tee.finalize(None);
            } else {
                tee.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    });

    Body::from_stream(RelayBodyStream::new(rx))
}

pub(in crate::gateway) struct UsageBodyBufferTeeStream<S, B, R = tauri::Wry>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    upstream: S,
    ctx: StreamFinalizeCtx<R>,
    first_byte_ms: Option<u128>,
    buffer: Vec<u8>,
    max_bytes: usize,
    truncated: bool,
    total_timeout: Option<Duration>,
    total_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B, R> UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx<R>,
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

        let effective_error_code = if error_code.is_none()
            && !self.truncated
            && !self.buffer.is_empty()
            && is_fake_200_non_stream_body(&self.buffer)
        {
            Some(GatewayErrorCode::Fake200.as_str())
        } else {
            error_code
        };
        if effective_error_code == Some(GatewayErrorCode::Fake200.as_str()) {
            self.ctx.fake_200_detected = true;
            self.ctx.fake_200_quota_exhausted =
                upstream_client_error_rules::match_quota_exhausted(&self.buffer);
        }

        let usage = if self.truncated || self.buffer.is_empty() {
            None
        } else {
            usage::parse_usage_from_json_or_sse_bytes(&self.ctx.cli_key, &self.buffer)
        };
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let requested_model = self.ctx.requested_model.clone().or_else(|| {
            if self.truncated || self.buffer.is_empty() {
                None
            } else {
                usage::parse_model_from_json_or_sse_bytes(&self.ctx.cli_key, &self.buffer)
            }
        });

        emit_request_event_and_spawn_request_log(
            &self.ctx,
            StreamRequestCompletion::from_error_code(
                effective_error_code,
                self.first_byte_ms,
                requested_model,
                usage_metrics,
                usage,
            ),
        );
    }
}

impl<S, B, R> Stream for UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(total) = this.total_timeout {
            if this.ctx.started.elapsed() >= total {
                this.finalize(Some(GatewayErrorCode::UpstreamTimeout.as_str()));
                return Poll::Ready(None);
            }
        }

        let next = Pin::new(&mut this.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                if let Some(timer) = this.total_sleep.as_mut() {
                    if timer.as_mut().poll(cx).is_ready() {
                        this.finalize(Some(GatewayErrorCode::UpstreamTimeout.as_str()));
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
                this.finalize(Some(GatewayErrorCode::StreamError.as_str()));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B, R> Drop for UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    fn drop(&mut self) {
        if !self.finalized {
            let usage_seen = !self.truncated
                && !self.buffer.is_empty()
                && usage::parse_usage_from_json_or_sse_bytes(&self.ctx.cli_key, &self.buffer)
                    .is_some();

            let codex_successish = is_codex_body_buffer_drop_successish(
                &self.ctx.cli_key,
                &self.ctx.path,
                self.ctx.status,
                self.first_byte_ms.is_some(),
                usage_seen,
            );

            if codex_successish {
                self.finalize(None);
            } else {
                self.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_codex_body_buffer_drop_successish, is_codex_client_abort_successish,
        is_codex_drop_successish, is_codex_responses_path, is_codex_stream_tail_error_successish,
        is_codex_stream_terminal_error_successish, next_item, spawn_usage_sse_relay_body,
        RelayBodyStream, StreamFinalizeCtx,
    };
    use crate::{circuit_breaker, db, request_logs, session_manager};
    use axum::body::Bytes;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn test_stream_finalize_ctx(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    ) -> StreamFinalizeCtx<tauri::test::MockRuntime> {
        StreamFinalizeCtx {
            app,
            db,
            log_tx,
            circuit: Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            session: Arc::new(session_manager::SessionManager::new()),
            session_id: Some("sess-usage-tee-drain".to_string()),
            sort_mode_id: None,
            trace_id: "trace-usage-tee-drain".to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            observe: true,
            query: None,
            excluded_from_stats: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            status: 200,
            error_category: None,
            error_code: None,
            started: Instant::now(),
            attempts: Vec::new(),
            attempts_json: "[]".to_string(),
            requested_model: None,
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            provider_cooldown_secs: 0,
            provider_id: 1,
            provider_name: "test-provider".to_string(),
            base_url: "https://upstream.example".to_string(),
            auth_mode: "api_key".to_string(),
            fake_200_detected: false,
            fake_200_quota_exhausted: false,
        }
    }

    #[test]
    fn codex_responses_path_accepts_v1_and_backend_style_paths() {
        assert!(is_codex_responses_path("codex", "/v1/responses"));
        assert!(is_codex_responses_path("codex", "/responses"));
        assert!(is_codex_responses_path("codex", "/v1/responses/"));
        assert!(!is_codex_responses_path("claude", "/v1/responses"));
        assert!(!is_codex_responses_path("codex", "/v1/chat/completions"));
    }

    #[test]
    fn codex_client_abort_successish_allows_terminal_marker_when_upstream_not_ended() {
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            true,
            false
        ));
    }

    #[test]
    fn codex_client_abort_successish_allows_completion_or_usage_when_upstream_ended() {
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            true,
            true
        ));
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true,
            true,
            true
        ));
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            false,
            true
        ));
        assert!(!is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            true,
            true
        ));
    }

    #[test]
    fn codex_drop_successish_allows_completion_or_usage_with_terminal_marker() {
        assert!(is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            true
        ));
        assert!(is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true,
            true
        ));
        assert!(!is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            true
        ));
    }

    #[test]
    fn codex_body_buffer_drop_successish_when_usage_seen() {
        assert!(is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true
        ));
        assert!(is_codex_body_buffer_drop_successish(
            "codex",
            "/responses",
            204,
            true,
            true
        ));
    }

    #[test]
    fn codex_body_buffer_drop_successish_requires_usage_and_stream_output() {
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            false,
            true
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "claude",
            "/v1/responses",
            200,
            true,
            true
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/chat/completions",
            200,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_successish_with_completion_and_usage() {
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            true
        ));
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/responses",
            200,
            true,
            true,
            false
        ));
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_without_completion_or_usage() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_for_non_codex() {
        assert!(!is_codex_stream_terminal_error_successish(
            "claude",
            "/v1/responses",
            200,
            true,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_without_stream_output() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            false,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_on_error_status() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            500,
            true,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_tail_error_successish_after_completion() {
        assert!(is_codex_stream_tail_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false
        ));
        assert!(!is_codex_stream_tail_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_disconnect_drain_ignores_stream_idle_timeout_until_completion() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-drain.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let ctx = test_stream_finalize_ctx(app_handle, db, log_tx);
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let body = spawn_usage_sse_relay_body(
            RelayBodyStream::new(upstream_rx),
            ctx,
            Some(Duration::from_millis(10)),
            None,
        );

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
            )))
            .await
            .expect("send first output chunk");

        let mut body_stream = body.into_data_stream();
        let first = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
            .await
            .expect("first output chunk should arrive")
            .expect("body should yield first output")
            .expect("first output should be ok");
        assert!(first.as_ref().starts_with(b"data:"));

        drop(body_stream);

        let completion_tx = upstream_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = completion_tx
                .send(Ok(Bytes::from_static(
                    b"event: response.completed\n\
                      data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
                )))
                .await;
        });

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");

        assert_eq!(log.error_code, None);
        assert_eq!(log.status, Some(200));
        assert_eq!(log.input_tokens, Some(1));
        assert_eq!(log.output_tokens, Some(2));
        assert_eq!(log.total_tokens, Some(3));
        assert!(log
            .special_settings_json
            .as_deref()
            .is_some_and(|value| value.contains("\"client_abort\"")));
    }
}
