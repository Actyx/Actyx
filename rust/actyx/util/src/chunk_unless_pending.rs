use core::mem;
use core::pin::Pin;
use futures::stream::StreamExt;
use futures::stream::{Fuse, FusedStream, Stream};
use futures::task::{Context, Poll};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use std::vec::Vec;

/// Stream for the chunk_unless_pending method.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ChunkUnlessPending<St: Stream> {
    stream: Fuse<St>,
    items: Vec<St::Item>,
    cap: usize, // https://github.com/rust-lang-nursery/futures-rs/issues/1475
}

impl<St: Unpin + Stream> Unpin for ChunkUnlessPending<St> {}

impl<St: Stream> ChunkUnlessPending<St>
where
    St: Stream,
{
    unsafe_unpinned!(items: Vec<St::Item>);
    unsafe_pinned!(stream: Fuse<St>);

    pub fn new(stream: St, capacity: usize) -> Self {
        assert!(capacity > 0);

        ChunkUnlessPending {
            stream: stream.fuse(),
            items: Vec::new(),
            cap: capacity,
        }
    }

    fn take(mut self: Pin<&mut Self>) -> Vec<St::Item> {
        mem::take(self.as_mut().items())
    }
}

impl<St: Stream> Stream for ChunkUnlessPending<St> {
    type Item = Vec<St::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.as_mut().stream().poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    self.as_mut().items().push(item);
                    if self.items.len() >= self.cap {
                        return Poll::Ready(Some(self.as_mut().take()));
                    }
                }
                Poll::Ready(None) => {
                    if !self.items.is_empty() {
                        return Poll::Ready(Some(self.as_mut().take()));
                    } else {
                        return Poll::Ready(None);
                    };
                }
                Poll::Pending => {
                    if !self.items.is_empty() {
                        return Poll::Ready(Some(self.as_mut().take()));
                    } else {
                        return Poll::Pending;
                    };
                }
            }
        }
    }
}

impl<St: FusedStream> FusedStream for ChunkUnlessPending<St> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated() && self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drainer::Drainer;

    /// A stream that just surfaces an iterator of poll items.
    struct TestStream<I>(I);

    impl<T, I: Iterator<Item = Poll<Option<T>>>> TestStream<I> {
        fn new(iter: I) -> Self {
            Self(iter)
        }
    }

    impl<T, I: Iterator<Item = Poll<Option<T>>> + Unpin> Stream for TestStream<I> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if let Some(value) = self.get_mut().0.next() {
                if value.is_pending() {
                    // seems strange that calling the waker synchronously is sufficient, but it seems to work
                    // if this test ever suddenly just blocks, this is probably the reason...
                    cx.waker().wake_by_ref();
                }
                value
            } else {
                Poll::Ready(None)
            }
        }
    }

    #[test]
    fn test_chunk_unless_pending() {
        let iter = vec![
            Poll::Ready(Some(1u8)),
            Poll::Pending,
            Poll::Ready(Some(2)),
            Poll::Ready(Some(3)),
            Poll::Pending,
            Poll::Pending,
            Poll::Pending,
            Poll::Pending,
            Poll::Ready(Some(4)),
            Poll::Ready(Some(5)),
            Poll::Ready(Some(6)),
            Poll::Ready(Some(7)),
            Poll::Pending,
            Poll::Ready(Some(8)),
            Poll::Ready(Some(9)),
            Poll::Ready(Some(10)),
            Poll::Ready(Some(11)),
            Poll::Ready(Some(12)),
            Poll::Ready(None),
            Poll::Ready(Some(13)),
            Poll::Ready(Some(14)),
            Poll::Pending,
            Poll::Ready(Some(15)),
            Poll::Ready(Some(16)),
        ]
        .into_iter();
        let inner = TestStream::new(iter);
        let outer = ChunkUnlessPending::new(inner, 3);

        let result = Drainer::new(outer).flatten().collect::<Vec<_>>();
        assert_eq!(
            result,
            vec![
                vec![1],
                vec![2, 3],
                vec![4, 5, 6],
                vec![7],
                vec![8, 9, 10],
                vec![11, 12]
            ]
        );
    }
}
