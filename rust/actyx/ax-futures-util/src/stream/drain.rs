use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use pin_project_lite::pin_project;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct Drain<S> {
        #[pin]
        stream: S
    }
}

impl<S: Stream> Drain<S> {
    pub(crate) fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S: Stream> Future for Drain<S> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project().stream.poll_next(cx) {
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
