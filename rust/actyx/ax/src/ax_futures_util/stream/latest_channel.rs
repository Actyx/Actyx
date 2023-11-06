use std::{
    fmt, result,
    sync::Arc,
    task::{Poll, Waker},
};

use futures::prelude::*;
use parking_lot::Mutex;
use stream::FusedStream;

/// A channel that is bounded to a size of 1, so always just remembers the latest element
/// Sending is always possible, and replaces the most recent value regardless of whether
/// the receiver has already processed it or not.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Mutex::new(LatestChannelInner::new()));
    (Sender(inner.clone()), Receiver(inner))
}

struct LatestChannelInner<T> {
    value: Option<T>,
    waker: Option<Waker>,
    sender_dropped: bool,
    receiver_dropped: bool,
}

impl<T> LatestChannelInner<T> {
    fn new() -> Self {
        Self {
            value: None,
            waker: None,
            sender_dropped: false,
            receiver_dropped: false,
        }
    }
}
pub struct Sender<T>(Arc<Mutex<LatestChannelInner<T>>>);

#[derive(Debug)]
pub struct ReceiverDropped;

impl fmt::Display for ReceiverDropped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiverDropped").finish()
    }
}

impl std::error::Error for ReceiverDropped {}

impl<T> Sender<T> {
    /// Sends a value.
    ///
    /// The only reason this can fail is if the receiver has been dropped.
    pub fn send(&self, value: T) -> result::Result<(), ReceiverDropped> {
        let mut inner = self.0.lock();
        if inner.receiver_dropped {
            return Err(ReceiverDropped);
        }
        inner.value = Some(value);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        Ok(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.0.lock();
        inner.sender_dropped = true;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

pub struct Receiver<T>(Arc<Mutex<LatestChannelInner<T>>>);

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut inner = self.0.lock();
        inner.receiver_dropped = true;
        inner.value = None;
        inner.waker = None;
    }
}

impl<T> FusedStream for Receiver<T> {
    fn is_terminated(&self) -> bool {
        let inner = self.0.lock();
        inner.sender_dropped && inner.value.is_none()
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut inner = self.0.lock();
        if inner.sender_dropped {
            // no point in storing a waker, nobody will wake it up
            Poll::Ready(inner.value.take())
        } else if let Some(value) = inner.value.take() {
            Poll::Ready(Some(value))
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
