use std::task::Poll;

use futures::Stream;
use tokio::time::{self, Instant};

pub struct Interval(time::Interval);

impl Interval {
    pub(crate) fn new(period: time::Duration) -> Self {
        Self(time::interval(period))
    }
}

impl Stream for Interval {
    type Item = Instant;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.get_mut().0.poll_tick(cx) {
            Poll::Ready(x) => Poll::Ready(Some(x)),
            Poll::Pending => Poll::Pending,
        }
    }
}
