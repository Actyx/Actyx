//! heavily inspired by futures-rs TakeWhile
use core::fmt;
use core::pin::Pin;
use futures::ready;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`dedup`](super::AxStreamExt::dedup) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Dedup<St: Stream> {
        #[pin]
        stream: St,
        last_item: Option<St::Item>,
    }
}

impl<St> fmt::Debug for Dedup<St>
where
    St: Stream + fmt::Debug,
    St::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dedup")
            .field("stream", &self.stream)
            .field("last_item", &self.last_item)
            .finish()
    }
}

impl<St> Dedup<St>
where
    St: Stream,
{
    pub fn new(stream: St) -> Dedup<St> {
        Dedup {
            stream,
            last_item: None,
        }
    }
}

impl<St> Stream for Dedup<St>
where
    St: Stream,
    St::Item: Eq + Clone,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<St::Item>> {
        let mut this = self.project();
        loop {
            let item = match ready!(this.stream.as_mut().poll_next(cx)) {
                None => return Poll::Ready(None),
                item => item,
            };
            if *this.last_item != item {
                *this.last_item = item.clone();
                return Poll::Ready(item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, stream::drainer::Drainer};
    use futures::channel::mpsc;
    use futures::executor::block_on;
    use futures::stream::{empty, pending, StreamExt};

    #[test]
    fn should_work_with_empty_stream() {
        let res = block_on(empty::<u32>().dedup().collect::<Vec<_>>());
        assert_eq!(res, vec![] as Vec<u32>);
    }

    #[test]
    fn should_work_with_pending_stream() {
        let mut d = Drainer::new(pending::<u32>().dedup());
        assert_eq!(d.next(), Some(vec![]));
    }

    #[test]
    fn should_deduplicate() {
        let (mut send, recv) = mpsc::unbounded::<u32>();
        let mut d = Drainer::new(recv.dedup());

        for i in &[1, 2, 3] {
            send.start_send(*i).unwrap();
        }
        assert_eq!(d.next(), Some(vec![1, 2, 3]));

        for i in &[3, 4, 4, 4, 3, 5] {
            send.start_send(*i).unwrap();
        }
        assert_eq!(d.next(), Some(vec![4, 3, 5]));
    }
}
