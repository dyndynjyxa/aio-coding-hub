//! Usage: Timing-only tee wrapper used for non-stream responses.

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
use super::StreamFinalizeCtx;

pub(in crate::gateway) struct TimingOnlyTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    upstream: S,
    ctx: StreamFinalizeCtx,
    first_byte_ms: Option<u128>,
    total_timeout: Option<Duration>,
    total_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B> TimingOnlyTeeStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx,
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

        let duration_ms = self.ctx.started.elapsed().as_millis();
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
            None,
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
                requested_model: self.ctx.requested_model.clone(),
                created_at_ms: self.ctx.created_at_ms,
                created_at: self.ctx.created_at,
                usage: None,
            },
        );
    }
}

impl<S, B> Stream for TimingOnlyTeeStream<S, B>
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
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize(Some("GW_STREAM_ERROR"));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B> Drop for TimingOnlyTeeStream<S, B>
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
