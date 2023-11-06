use core::pin::Pin;
use futures::stream::{FusedStream, Stream};
use futures::task::{Context, Poll};
use futures::Future;
use pin_utils::{unsafe_pinned, unsafe_unpinned};

/// Stream for the chunk_unless_pending method.
#[derive(Debug)]
#[must_use = "futures/streams do nothing unless polled"]
pub struct TracePoll<T> {
    t: T,
    name: &'static str,
}

impl<T: Unpin> Unpin for TracePoll<T> {}

impl<T> TracePoll<T> {
    unsafe_pinned!(t: T);
    unsafe_unpinned!(name: &'static str);

    pub fn new(t: T, name: &'static str) -> Self {
        TracePoll { t, name }
    }
}

impl<St: Stream> Stream for TracePoll<St> {
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let span = tracing::trace_span!("poll_next", "{}", &self.name);
        let _enter = span.enter();
        self.t().poll_next(cx)
    }
}

impl<St: FusedStream> FusedStream for TracePoll<St> {
    fn is_terminated(&self) -> bool {
        self.t.is_terminated()
    }
}

impl<F: Future> Future for TracePoll<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let span = tracing::trace_span!("poll_next", "{}", &self.name);
        let _enter = span.enter();
        self.t().poll(cx)
    }
}
