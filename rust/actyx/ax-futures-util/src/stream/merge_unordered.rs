use futures::stream::{SelectAll, Stream};
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// ## Merge a stream of streams into a single stream
///
/// This is the equivalent of RxJS `mergeMap`. The resulting stream only completes
/// once the input stream of streams has completed and all nested streams have
/// completed as well. Elements from each substream appear in their original order
/// within the merged stream, but there is no ordering between the elements of
/// different substreams.
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct MergeUnordered<St, Si> {
    input: Option<Pin<Box<Si>>>,
    streams: Pin<Box<SelectAll<St>>>,
}

impl<St: Stream + Unpin + Send, Si: Stream<Item = St> + Send> MergeUnordered<St, Si> {
    pub fn new(input: Si) -> MergeUnordered<St, Si> {
        MergeUnordered {
            input: Some(Box::pin(input)),
            streams: Box::pin(SelectAll::new()),
        }
    }

    /// Create a MergeUnordered without a stream of streams to poll.
    ///
    /// Streams to be merged need to be injected using [`push()`](#method.push).
    pub fn without_input() -> Self {
        Self {
            input: None,
            streams: Box::pin(SelectAll::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }

    /// Add the given stream to the merge pool.
    pub fn push(&mut self, input: St) {
        self.streams.push(input);
    }
}

impl<St: Stream + Unpin, Si: Stream<Item = St>> Stream for MergeUnordered<St, Si> {
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();

        // first look for new incoming streams
        if let Some(input) = &mut s.input {
            while let Poll::Ready(x) = input.as_mut().poll_next(cx) {
                match x {
                    Some(item) => s.streams.push(item),
                    None => {
                        s.input = None;
                        break;
                    }
                }
            }
        }

        // now look for new data from the merged streams
        match s.streams.as_mut().poll_next(cx) {
            Poll::Ready(None) => {
                if s.input.is_none() {
                    // no more substreams and all streams finished
                    Poll::Ready(None)
                } else {
                    // the SelectAll is now terminated, we need a new one to service
                    // potential future input streams
                    s.streams = Box::pin(SelectAll::new());
                    Poll::Pending
                }
            }
            // otherwise return item or pending
            x => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::future::future_helpers::{delay_ms, wait_for};
    use crate::prelude::*;
    use futures::future::{ready, FutureExt};
    use futures::stream::{self, StreamExt};

    fn mk_streams() -> Vec<Pin<Box<dyn Stream<Item = i32> + Send>>> {
        (0..10)
            .map(|i| stream::iter((i..100).step_by(10)).boxed())
            .collect::<Vec<_>>()
    }

    #[test]
    fn must_keep_substreams_ordered() {
        let streams = mk_streams();

        // this should work because mk_streams will be polled to completion before merging
        let merged = stream::iter(streams).merge_unordered();
        let res = wait_for(merged.collect::<Vec<_>>());

        // check the whole thing for completeness
        let mut sorted = res.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>());

        // check each substream for orderedness
        for i in 0..10 {
            let sub = res
                .iter()
                .filter_map(|x| if *x % 10 == i { Some(*x) } else { None })
                .collect::<Vec<i32>>();
            assert_eq!(sub, (i..100).step_by(10).collect::<Vec<i32>>());
        }
    }

    #[test]
    fn must_keep_running_as_long_as_input_runs() {
        let streams = mk_streams();

        let mut idx = -1;
        let inputs = stream::iter(streams).then(move |s| {
            idx += 1;
            if idx == 5 {
                delay_ms(100, s).left_future()
            } else {
                ready(s).right_future()
            }
        });

        let mut res = wait_for(inputs.merge_unordered().collect::<Vec<_>>());

        res.sort_unstable();
        assert_eq!(res, (0..100).collect::<Vec<_>>());
    }
}
