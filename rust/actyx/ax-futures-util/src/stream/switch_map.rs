use futures::{prelude::*, Stream};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    pub struct SwitchMap<St, U, F> {
        #[pin]
        inner: St,
        #[pin]
        current: Option<U>,
        f: F,
    }
}

impl<St, U, F> SwitchMap<St, U, F>
where
    St: Stream,
    F: FnMut(St::Item) -> U,
    U: Stream,
{
    pub fn new(inner: St, f: F) -> Self {
        Self {
            inner,
            f,
            current: None,
        }
    }
}

impl<St, U, F> Stream for SwitchMap<St, U, F>
where
    St: Stream,
    St::Item: std::fmt::Debug,
    F: FnMut(St::Item) -> U,
    U: Stream,
{
    type Item = U::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        tracing::trace!("poll_next");
        while let Poll::Ready(next) = this.inner.poll_next_unpin(cx) {
            if let Some(value) = next {
                let _s = tracing::trace_span!("transform incoming", value = debug(&value));
                let _s = _s.enter();
                this.current.set(Some((this.f)(value)));
            } else {
                return Poll::Ready(None);
            }
        }
        if let Some(mut current) = this.current.as_mut().as_pin_mut() {
            let _s = tracing::trace_span!("poll inner");
            let _s = _s.enter();
            match current.poll_next_unpin(cx) {
                Poll::Ready(Some(value)) => Poll::Ready(Some(value)),
                Poll::Ready(None) => {
                    this.current.set(None);
                    Poll::Pending
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}
