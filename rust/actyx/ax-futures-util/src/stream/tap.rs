use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    prelude::*,
    Stream,
};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    pub struct Tap<St, U, F> {
        #[pin]
        inner: St,
        sender: UnboundedSender<U>,
        f: F,
    }
}

impl<St, U, F> Tap<St, U, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> Option<U>,
{
    pub fn new(inner: St, f: F) -> (Self, UnboundedReceiver<U>) {
        let (sender, receiver) = mpsc::unbounded();
        (Self { inner, f, sender }, receiver)
    }
}

impl<St, U, F> Stream for Tap<St, U, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> Option<U>,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        let value = this.inner.poll_next_unpin(cx);
        if let Poll::Ready(Some(item)) = &value {
            if let Some(u) = (this.f)(item) {
                // the only error that can happen here is that the receiver is dropped.
                // we can just ignore it.
                let _ = this.sender.unbounded_send(u);
            }
        };
        value
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::executor::block_on_stream;
    use futures::stream;

    #[test]
    fn tap_should_work() {
        let elems: Vec<usize> = vec![1, 2, 3, 4];
        let initial = stream::iter(elems.clone());
        let just_odd = |x: &usize| {
            if x % 2 == 1 {
                Some(*x)
            } else {
                None
            }
        };
        let (orig, tap) = initial.tap(just_odd);
        let orig = block_on_stream(orig).collect::<Vec<_>>();
        let tap = block_on_stream(tap).collect::<Vec<_>>();
        assert_eq!(orig, elems);
        assert_eq!(tap, vec![1, 3]);
    }
}
