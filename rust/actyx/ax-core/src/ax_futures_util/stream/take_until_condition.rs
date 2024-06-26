//! copied from futures-rs TakeWhile
use core::{fmt, pin::Pin};
use futures::{
    future::Future,
    ready,
    stream::{FusedStream, Stream},
    task::{Context, Poll},
};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`take_until_condition`](super::AxStreamExt::take_until_condition) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct TakeUntilCondition<St: Stream, Fut, F> {
        #[pin]
        stream: St,
        f: F,
        #[pin]
        pending_fut: Option<Fut>,
        pending_item: Option<St::Item>,
        done_taking: bool,
    }
}

impl<St, Fut, F> fmt::Debug for TakeUntilCondition<St, Fut, F>
where
    St: Stream + fmt::Debug,
    St::Item: fmt::Debug,
    Fut: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TakeUntilCondition")
            .field("stream", &self.stream)
            .field("pending_fut", &self.pending_fut)
            .field("pending_item", &self.pending_item)
            .field("done_taking", &self.done_taking)
            .finish()
    }
}

impl<St, Fut, F> TakeUntilCondition<St, Fut, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> Fut,
    Fut: Future<Output = bool>,
{
    pub fn new(stream: St, f: F) -> TakeUntilCondition<St, Fut, F> {
        TakeUntilCondition {
            stream,
            f,
            pending_fut: None,
            pending_item: None,
            done_taking: false,
        }
    }
}

impl<St, Fut, F> Stream for TakeUntilCondition<St, Fut, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> Fut,
    Fut: Future<Output = bool>,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<St::Item>> {
        if self.done_taking {
            return Poll::Ready(None);
        }

        let mut this = self.project();

        if this.pending_item.is_none() {
            let item = match ready!(this.stream.poll_next(cx)) {
                Some(e) => e,
                None => {
                    *this.done_taking = true;
                    return Poll::Ready(None);
                }
            };
            let fut = (this.f)(&item);
            this.pending_fut.set(Some(fut));
            *this.pending_item = Some(item);
        }

        let done_taking = ready!(this.pending_fut.as_mut().as_pin_mut().unwrap().poll(cx));
        this.pending_fut.set(None);
        let item = this.pending_item.take().unwrap();

        // This is the only change: next poll returns None if condition was false
        *this.done_taking = done_taking;
        Poll::Ready(Some(item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done_taking {
            return (0, Some(0));
        }

        let pending_len = usize::from(self.pending_item.is_some());
        let (_, upper) = self.stream.size_hint();
        let upper = match upper {
            Some(x) => x.checked_add(pending_len),
            None => None,
        };
        (0, upper) // can't know a lower bound, due to the predicate
    }
}

impl<St, Fut, F> FusedStream for TakeUntilCondition<St, Fut, F>
where
    St: Stream,
    F: FnMut(&St::Item) -> Fut,
    Fut: Future<Output = bool>,
{
    fn is_terminated(&self) -> bool {
        self.done_taking
    }
}

#[cfg(test)]
mod tests {
    use crate::ax_futures_util::{future::future_helpers::wait_for, stream::AxStreamExt};
    use futures::{
        future::ready,
        stream::{self, StreamExt},
    };

    #[test]
    fn should_work_with_empty_stream() {
        let res = wait_for(
            stream::empty::<u32>()
                .take_until_condition(|_| ready(true))
                .collect::<Vec<_>>(),
        );
        assert_eq!(res, vec![] as Vec<u32>);
    }

    #[test]
    fn should_work_with_immediately_true_predicate() {
        let res = wait_for(
            stream::iter(vec![1, 2, 3])
                .take_until_condition(|_| ready(true))
                .collect::<Vec<_>>(),
        );
        assert_eq!(res, vec![1]);
    }

    #[test]
    fn should_work_with_later_true_predicate() {
        let res = wait_for(
            stream::iter(vec![1, 2, 3])
                .take_until_condition(|x| ready(*x == 2))
                .collect::<Vec<_>>(),
        );
        assert_eq!(res, vec![1, 2]);
    }

    #[test]
    fn should_work_with_predicate_true_on_last() {
        let res = wait_for(
            stream::iter(vec![1, 2, 3])
                .take_until_condition(|x| ready(*x == 3))
                .collect::<Vec<_>>(),
        );
        assert_eq!(res, vec![1, 2, 3]);
    }

    #[test]
    fn should_work_with_never_true_predicate() {
        let res = wait_for(
            stream::iter(vec![1, 2, 3])
                .take_until_condition(|_| ready(false))
                .collect::<Vec<_>>(),
        );
        assert_eq!(res, vec![1, 2, 3]);
    }
}
