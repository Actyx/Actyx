use core::pin::Pin;
use futures::stream::{self, Stream};
use futures::task::Context;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::task::Poll;

/// A struct for putting a stream and its head into the BinaryHeap. Since we need
/// a min-heap, the ordering is **REVERSED**.
#[derive(Debug)]
struct SourceState<Elem, St> {
    current: Option<Elem>,
    stream: Pin<Box<St>>,
}

impl<Elem: Ord, St> Ord for SourceState<Elem, St> {
    fn cmp(&self, other: &Self) -> Ordering {
        debug_assert!(self.current.is_some());
        debug_assert!(other.current.is_some());
        other.current.cmp(&self.current)
    }
}

impl<Elem: Ord, St> PartialOrd for SourceState<Elem, St> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        debug_assert!(self.current.is_some());
        debug_assert!(other.current.is_some());
        Some(other.current.cmp(&self.current))
    }
}

impl<Elem: Ord, St> PartialEq for SourceState<Elem, St> {
    fn eq(&self, other: &Self) -> bool {
        debug_assert!(self.current.is_some());
        debug_assert!(other.current.is_some());
        self.current.eq(&other.current)
    }
}

impl<Elem: Ord, St> Eq for SourceState<Elem, St> {}

#[derive(Debug, PartialEq)]
pub enum NewSourceMode {
    AdmitStragglers,
    DropStragglers,
}

/// ## Merge ordered streams into a (mostly) ordered stream.
///
/// The streams to be merged can be given initially and/or read from a stream
/// of streams. When adding streams after the merge is constructed, the `mode`
/// parameter controls how to inject a new stream into the merge:
///
///  - `NewSourceMode::DropStragglers` will drop that prefix of the new stream
///    which is no longer admissible due to already emitted elements
///  - `NewSourceMode::AdmitStragglers` will insert all elements from the new
///    stream, beginning with all those that compare smaller than the already
///    emitted elements
///
/// ## Implementation notes
///
/// Each active substream is represented with an optional known next value and
/// a reference to the stream, packaged as a `SourceState`. These states have
/// an ordering that is based on the next value alone. While the next value is
/// not known, the SourceState lives in the `to_poll` vector; when the value
/// is known, it is moved into the `to_deliver` binary heap (wrapped in Reverse)
/// because we need a min-heap). Whenever the `to_poll` vector is empty we take
/// one item from the binary heap, emit the value, and enqueue the SourceState
/// for polling again.
///
/// This scheme implies that the `to_poll` vector should almost always have at
/// most one element, making the poll handling naturally efficient.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct MergeOrdered<Elem, St, Si> {
    mode: NewSourceMode,
    to_poll: Vec<SourceState<Elem, St>>,
    to_deliver: BinaryHeap<SourceState<Elem, St>>,
    input: Option<Pin<Box<Si>>>,
    last_emitted: Option<Elem>,
}

impl<Elem, St, Si> Unpin for MergeOrdered<Elem, St, Si> {}

impl<Elem, St, Si> MergeOrdered<Elem, St, Si>
where
    Elem: Ord + Clone,
    St: Stream<Item = Elem> + Send,
    Si: Stream<Item = St> + Send,
{
    /// Create a merge from an initial set of streams, an input stream of streams, and a mode.
    /// The initial streams are guaranteed to be merged in the correct order, while stream
    /// added later are treated according to the NewSourceMode:
    ///
    ///  - `NewSourceMode::DropStragglers` will drop that prefix of the new stream
    ///    which is no longer admissible due to already emitted elements
    ///  - `NewSourceMode::AdmitStragglers` will insert all elements from the new
    ///    stream, beginning with all those that compare smaller than the already
    ///    emitted elements
    ///
    /// This means that AdmitStragglers does NOT guarantee a fully ordered output stream in
    /// the presence of streams being added later.
    pub fn new<I: IntoIterator<Item = St>>(mode: NewSourceMode, streams: I, input: Si) -> MergeOrdered<Elem, St, Si> {
        let iter = streams.into_iter();
        let (lower, _) = iter.size_hint();
        let mut to_poll = Vec::with_capacity(lower);
        for stream in iter {
            to_poll.push(SourceState {
                current: None,
                stream: Box::pin(stream),
            });
        }
        MergeOrdered {
            mode,
            to_poll,
            to_deliver: BinaryHeap::new(),
            input: Some(Box::pin(input)),
            last_emitted: None,
        }
    }
}

impl<Elem, St> MergeOrdered<Elem, St, stream::Empty<St>>
where
    Elem: Ord + Clone,
    St: Stream<Item = Elem> + Send,
{
    /// Create a new merge from a fixed set of input streams.
    pub fn new_fixed<I: IntoIterator<Item = St>>(streams: I) -> MergeOrdered<Elem, St, stream::Empty<St>> {
        Self::new(NewSourceMode::DropStragglers, streams, stream::empty())
    }
}

fn fill_to_poll<Elem, St: Stream<Item = Elem>, Si: Stream<Item = St>>(
    s: &mut MergeOrdered<Elem, St, Si>,
    cx: &mut Context<'_>,
) {
    if let Some(input) = &mut s.input {
        while let Poll::Ready(x) = input.as_mut().poll_next(cx) {
            match x {
                Some(item) => s.to_poll.push(SourceState {
                    current: None,
                    stream: Box::pin(item),
                }),
                None => {
                    s.input = None;
                    break;
                }
            }
        }
    }
}

fn poll_to_deliver<Elem: Ord, St: Stream<Item = Elem>, Si: Stream<Item = St>>(
    s: &mut MergeOrdered<Elem, St, Si>,
    cx: &mut Context<'_>,
) {
    for i in (0..s.to_poll.len()).rev() {
        let st = &mut s.to_poll[i];
        debug_assert!(st.current.is_none());
        match st.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                st.current = Some(item);
                s.to_deliver.push(s.to_poll.remove(i));
            }
            Poll::Ready(None) => {
                s.to_poll.remove(i);
            }
            Poll::Pending => (),
        }
    }
}

impl<Elem, St, Si> Stream for MergeOrdered<Elem, St, Si>
where
    Elem: Ord + Clone,
    St: Stream<Item = Elem>,
    Si: Stream<Item = St>,
{
    type Item = Elem;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let s = self.get_mut();

        loop {
            // first look for new incoming streams
            fill_to_poll(s, cx);

            // poll all streams where we don’t currently have a value
            poll_to_deliver(s, cx);

            // bail out if at least one stream returned Pending
            if !s.to_poll.is_empty() {
                return Poll::Pending;
            }

            // using Reverse makes this a min-heap: we need the smallest element
            if let Some(mut st) = s.to_deliver.pop() {
                let item = st.current.take();
                s.to_poll.push(st);
                if s.mode == NewSourceMode::AdmitStragglers || item >= s.last_emitted {
                    s.last_emitted = item.clone();
                    return Poll::Ready(item);
                }
                // here we drop the straggler element and go back to polling
                continue;
            }

            // the loop is only for the continue above
            break;
        }

        // The above can only have been a None if there are no streams left.
        // Terminate, when inputs are exhausted.
        if s.input.is_none() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MergeOrdered, NewSourceMode};
    use crate::future::future_helpers::{delay_ms, wait_for};
    use crate::prelude::*;
    use futures::channel::oneshot;
    use futures::future::{ready, FutureExt};
    use futures::stream::{self, Stream, StreamExt};
    use rand::random;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn mk_streams() -> Vec<Pin<Box<dyn Stream<Item = i32> + Send>>> {
        (0..10)
            .map(|i| {
                stream::iter((i..100).step_by(10))
                    .then(|x| delay_ms((random::<u8>() / 4).into(), x))
                    .boxed()
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn should_assemble_stream() {
        let colls = mk_streams();
        let merge = MergeOrdered::new(NewSourceMode::AdmitStragglers, colls, stream::empty());
        let res: Vec<i32> = wait_for(merge.collect());
        assert_eq!(res, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn should_assemble_stream_from_stream() {
        // this should work because mk_streams will be polled to completion before merging
        let merge = stream::iter(mk_streams()).merge_ordered();
        let res: Vec<i32> = wait_for(merge.collect());
        assert_eq!(res, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn should_assemble_stream_from_stream_with_stragglers() {
        // this should work because mk_streams will be polled to completion before merging
        let merge = stream::iter(mk_streams()).merge_ordered_with_stragglers();
        let res: Vec<i32> = wait_for(merge.collect());
        assert_eq!(res, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn should_assemble_stream_from_stream_with_initials() {
        let initials = vec![stream::iter(vec![100, 101, 102, 103]).boxed()];
        let merge = stream::iter(mk_streams()).merge_ordered_with_initials(initials);
        let res: Vec<i32> = wait_for(merge.collect());
        assert_eq!(res, (0..104).collect::<Vec<_>>());
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn should_add_source_allowing_stragglers() {
        let mut colls = mk_streams();
        let straggler = colls.pop().expect("colls didn’t have an element");

        // construct a channel through which the straggler is passed into the merge later
        let (snd, rcv) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some((snd, straggler))));
        let input = rcv.into_stream().filter_map(|v| ready(v.ok())).boxed();

        // construct merge with initial streams and the straggler source
        let merge = MergeOrdered::new(NewSourceMode::AdmitStragglers, colls, input);
        let merge = merge
            .inspect(move |x| {
                // inject the straggler when about half done
                if *x == 51 {
                    let mut opt = sender.lock().unwrap();
                    if let Some((sender, straggler)) = opt.take() {
                        sender.send(straggler).map_err(|_| "cannot send").unwrap();
                    }
                }
            })
            .collect::<Vec<_>>();

        let res = wait_for(merge);

        let mut expected = (0..100).filter(|x| *x > 50 || *x % 10 != 9).collect::<Vec<_>>();
        expected.splice(47..47, vec![9, 19, 29, 39, 49]);
        assert_eq!(res, expected);
    }

    #[test]
    fn should_add_source_dropping_stragglers() {
        let mut colls = mk_streams();
        let straggler = colls.pop().expect("colls didn’t have an element");

        // construct a channel through which the straggler is passed into the merge later
        let (snd, rcv) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some((snd, straggler))));
        let input = rcv.into_stream().filter_map(|v| ready(v.ok())).boxed();

        // construct merge with initial streams and the straggler source
        let merge = MergeOrdered::new(NewSourceMode::DropStragglers, colls, input);
        let merge = merge
            .inspect(move |x| {
                // inject the straggler when about half done
                if *x == 51 {
                    let mut opt = sender.lock().unwrap();
                    if let Some((sender, straggler)) = opt.take() {
                        sender.send(straggler).map_err(|_| "cannot send").unwrap();
                    }
                }
            })
            .collect::<Vec<_>>();

        let res = wait_for(merge);

        let expected = (0..100).filter(|x| *x > 50 || *x % 10 != 9).collect::<Vec<_>>();
        assert_eq!(res, expected);
    }

    #[test]
    fn should_keep_running_until_input_exhausted_allowing_stragglers() {
        let mut colls = mk_streams();
        let straggler = colls.pop().expect("colls didn’t have an element");

        // construct a channel through which the straggler is passed into the merge later
        let (snd, rcv) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some((snd, straggler))));
        let input = rcv.into_stream().filter_map(|v| ready(v.ok())).boxed();

        // construct merge with initial streams and the straggler source
        let merge = MergeOrdered::new(NewSourceMode::AdmitStragglers, colls, input);
        let merge = merge
            .inspect(move |x| {
                // inject the straggler only after all other sources finished
                if *x == 98 {
                    let sender = sender.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(50));
                        let mut opt = sender.lock().unwrap();
                        if let Some((sender, straggler)) = opt.take() {
                            sender.send(straggler).map_err(|_| "cannot send").unwrap();
                        }
                    });
                }
            })
            .collect::<Vec<_>>();

        let res = wait_for(merge);

        let expected = (0..100)
            .filter(|x| *x % 10 != 9)
            .chain((9..100).step_by(10))
            .collect::<Vec<_>>();
        assert_eq!(res, expected);
    }

    #[test]
    fn should_keep_running_until_input_exhausted_dropping_stragglers() {
        let mut colls = mk_streams();
        let straggler = colls.pop().expect("colls didn’t have an element");

        // construct a channel through which the straggler is passed into the merge later
        let (snd, rcv) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some((snd, straggler))));
        let input = rcv.into_stream().filter_map(|v| ready(v.ok())).boxed();

        // construct merge with initial streams and the straggler source
        let merge = MergeOrdered::new(NewSourceMode::DropStragglers, colls, input);
        let merge = merge
            .inspect(move |x| {
                // inject the straggler only after all other sources finished
                if *x == 98 {
                    let sender = sender.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(50));
                        let mut opt = sender.lock().unwrap();
                        if let Some((sender, straggler)) = opt.take() {
                            sender.send(straggler).map_err(|_| "cannot send").unwrap();
                        }
                    });
                }
            })
            .collect::<Vec<_>>();

        let res = wait_for(merge);

        let expected = (0..100).filter(|x| *x % 10 != 9).chain(99..100).collect::<Vec<_>>();
        assert_eq!(res, expected);
    }
}
