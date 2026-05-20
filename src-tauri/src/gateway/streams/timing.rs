//! Usage: Timing-only tee wrapper used for non-stream responses.

use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use super::super::proxy::GatewayErrorCode;
use super::request_end::{emit_request_event_and_spawn_request_log, StreamRequestCompletion};
use super::StreamFinalizeCtx;

pub(in crate::gateway) struct TimingOnlyTeeStream<S, B, R = tauri::Wry>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    upstream: S,
    ctx: StreamFinalizeCtx<R>,
    first_byte_ms: Option<u128>,
    total_timeout: Option<Duration>,
    total_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B, R> TimingOnlyTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx<R>,
        total_timeout: Option<Duration>,
    ) -> Self {
        let remaining = total_timeout.and_then(|d| d.checked_sub(ctx.started.elapsed()));
        Self {
            upstream,
            ctx,
            first_byte_ms: None,
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

        emit_request_event_and_spawn_request_log(
            &self.ctx,
            StreamRequestCompletion::from_error_code(
                error_code,
                self.first_byte_ms,
                self.ctx.requested_model.clone(),
                None,
                None,
            ),
        );
    }
}

impl<S, B, R> Stream for TimingOnlyTeeStream<S, B, R>
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
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize(Some(GatewayErrorCode::StreamError.as_str()));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B, R> Drop for TimingOnlyTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    fn drop(&mut self) {
        if !self.finalized {
            self.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
        }
    }
}
