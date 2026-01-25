//! Usage: Stream adapters for relaying upstream response bodies.

use axum::body::Bytes;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(in crate::gateway) struct RelayBodyStream {
    rx: tokio::sync::mpsc::Receiver<Result<Bytes, reqwest::Error>>,
}

impl RelayBodyStream {
    pub(in crate::gateway) fn new(
        rx: tokio::sync::mpsc::Receiver<Result<Bytes, reqwest::Error>>,
    ) -> Self {
        Self { rx }
    }
}

impl Stream for RelayBodyStream {
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        Pin::new(&mut this.rx).poll_recv(cx)
    }
}

pub(in crate::gateway) struct FirstChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    first: Option<Bytes>,
    rest: S,
}

impl<S> FirstChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(first: Option<Bytes>, rest: S) -> Self {
        Self { first, rest }
    }
}

impl<S> Stream for FirstChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(first) = this.first.take() {
            return Poll::Ready(Some(Ok(first)));
        }
        Pin::new(&mut this.rest).poll_next(cx)
    }
}
