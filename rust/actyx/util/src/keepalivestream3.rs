use futures::{
    task::{Context, Poll},
    Stream,
};
use std::pin::Pin;

/// An adapter for keepalive the tcp connection, while pulling from the events stream.
/// We need this because the keepalive stream should stop as soon as the events stream stops.
/// A behaviour which is not available for example with `select`
#[derive(Debug)]
pub struct KeepAliveStream<S1, S2> {
    keepalive: Pin<Box<S1>>,
    driver: Pin<Box<S2>>,
}

impl<S1: Unpin + Stream, S2: Unpin + Stream> Unpin for KeepAliveStream<S1, S2> {}

// Warp (and Hyper) require passed-in streams to be Sync.
// One could wrap both keepalive and driver in Mutex and use the std::sync::Mutex try_lock()
// coupled with Poll::Pending and waker (like async_std or tokio async "mutexes" do).
// But if we are only passing in stream, as in here, we can rely on the fact
// that poll_next method uses &mut reference to self, which per Rust rules
// is exclusively held by one agent only (like mutex).
// Also, the actual reason for Sync is that the underlying future has been made Sync
// by hyper (and thus warp) to work around compiler limitations, which means
// no actually concurrent accesses will be performed. Thus we can safely declare
// the KeepAliveStream Sync. Generally in that situation, a SyncWrapper
// could be used to make sure the API never exposes the immutable reference.
// Example implementation for KeepAliveStream below:

// #[repr(transparent)]
// pub struct SyncWrapper<T: ?Sized>(T);

// impl<T> SyncWrapper<T> {
//     pub fn new(t: T) -> SyncWrapper<T> {
//         SyncWrapper(t)
//     }
//     pub fn into_inner(self) -> T {
//         self.0
//     }
// }

// impl<T: ?Sized> SyncWrapper<T> {
//     pub fn get_mut(&mut self) -> &mut T {
//         &mut self.0
//     }
// }

// impl<T> Stream for SyncWrapper<T>
// where
//     T: Stream + Unpin,
// {
//     type Item = T::Item;
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
//         let inner = self.get_mut();
//         Stream::poll_next(Pin::new(inner.get_mut()), cx)
//     }
// }

// // reason for safety: an immutable reference to SyncWrapper<T> is worthless,
// // hence it’s safe to share such a reference between threads
// unsafe impl<T: ?Sized> Sync for SyncWrapper<T> {}

// impl<S1, S2> KeepAliveStream<S1, S2>
// where
//     S1: Stream + Unpin,
//     S2: Stream<Item = S1::Item> + Unpin,
// {
//     pub fn new(keepalive: S1, driver: S2) -> SyncWrapper<KeepAliveStream<S1, S2>> {
//         let kas = KeepAliveStream {
//             keepalive: Box::pin(keepalive),
//             driver: Box::pin(driver),
//         };
//         SyncWrapper::new(kas)
//     }
// }

// impl<S1, S2> Stream for KeepAliveStream<S1, S2>
// here the implementation would be exactly the same as below

// See the following links for more context:
// https://github.com/hyperium/hyper/issues/2159
// https://github.com/rust-lang/rust/issues/57017
// https://internals.rust-lang.org/t/what-shall-sync-mean-across-an-await/12020

unsafe impl<S1, S2> Sync for KeepAliveStream<S1, S2>
where
    S1: Stream + Unpin,
    S2: Stream<Item = S1::Item> + Unpin,
{
}

impl<S1, S2> KeepAliveStream<S1, S2>
where
    S1: Stream + Unpin,
    S2: Stream<Item = S1::Item> + Unpin,
{
    pub fn new(keepalive: S1, driver: S2) -> KeepAliveStream<S1, S2> {
        KeepAliveStream {
            keepalive: Box::pin(keepalive),
            driver: Box::pin(driver),
        }
    }
}

impl<S1, S2> Stream for KeepAliveStream<S1, S2>
where
    S1: Stream + Unpin,
    S2: Stream<Item = S1::Item> + Unpin,
{
    type Item = S1::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<S1::Item>> {
        let s = self.get_mut();
        if let Poll::Ready(x) = s.driver.as_mut().poll_next(cx) {
            Poll::Ready(x)
        } else {
            s.keepalive.as_mut().poll_next(cx)
        }
    }
}
