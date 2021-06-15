mod dedup;
mod drain;
mod drainer;
mod inspect_poll;
mod interval;
mod merge_ordered;
mod merge_unordered;
mod stream_dispatcher;
mod switch_map;
mod take_until_condition;
mod take_until_signaled;
mod tap;
mod yield_after;

pub mod latest_channel;
pub mod variable;

use std::task::Poll;

pub use dedup::Dedup;
pub use drain::Drain;
pub use drainer::Drainer;
pub use inspect_poll::InspectPoll;
pub use interval::Interval;
pub use merge_ordered::{MergeOrdered, NewSourceMode};
pub use merge_unordered::MergeUnordered;
pub use stream_dispatcher::StreamDispatcher;
pub use switch_map::SwitchMap;
pub use take_until_condition::TakeUntilCondition;
pub use take_until_signaled::TakeUntilSignaled;
pub use tap::Tap;
pub use yield_after::YieldAfter;

use futures::{channel::mpsc::UnboundedReceiver, prelude::*};
use tokio::time::Duration;
use variable::TeeVariable;

use variable::Variable;

/// Create a stream of ticks starting immediately and with the given cadence.
pub fn interval(period: Duration) -> Interval {
    Interval::new(period)
}

pub trait AxStreamExt: Stream + Sized {
    /// Poll the inner stream until a new inner stream becomes available.
    fn switch_map<U, F>(self, f: F) -> SwitchMap<Self, U, F>
    where
        F: FnMut(Self::Item) -> U,
        U: Stream,
        Self: Sized,
    {
        SwitchMap::new(self, f)
    }

    /// Create a merge from an input stream of ordered streams, merging concurrent streams
    /// such that the resulting stream is ordered, and filtering new streams such that
    /// their stragglers (elements older than the last emitted one) are dropped.
    fn merge_ordered<Elem, St>(self) -> MergeOrdered<Elem, St, Self>
    where
        Elem: Ord + Clone,
        St: Stream<Item = Elem> + Send,
        Self: Stream<Item = St> + Send,
    {
        MergeOrdered::new(NewSourceMode::DropStragglers, std::iter::empty(), self)
    }

    /// Create a merge from an input stream of ordered streams, merging concurrent streams
    /// such that the resulting stream is ordered, and adding new streams by first emitting
    /// their stragglers (elements older than the latest previously emitted element) and
    /// then ordering their events with the rest of the currently open streams.
    fn merge_ordered_with_stragglers<Elem, St>(self) -> MergeOrdered<Elem, St, Self>
    where
        Elem: Ord + Clone,
        St: Stream<Item = Elem> + Send,
        Self: Stream<Item = St> + Send,
    {
        MergeOrdered::new(NewSourceMode::AdmitStragglers, std::iter::empty(), self)
    }

    /// Create a merge from an initial set of ordered streams and an input stream of ordered streams.
    /// The initial streams are guaranteed to be merged in the correct order, streams
    /// being added later from the outer stream will be filtered such that stragglers
    /// (older than the latest emitted element) are dropped.
    fn merge_ordered_with_initials<Elem, St, I: IntoIterator<Item = St>>(
        self,
        streams: I,
    ) -> MergeOrdered<Elem, St, Self>
    where
        Elem: Ord + Clone,
        St: Stream<Item = Elem> + Send,
        Self: Stream<Item = St> + Send,
    {
        MergeOrdered::new(NewSourceMode::DropStragglers, streams, self)
    }

    /// ## Merge a stream of streams into a single stream
    ///
    /// This is the equivalent of RxJS `mergeMap`. The resulting stream only completes
    /// once the input stream of streams has completed and all nested streams have
    /// completed as well. Elements from each substream appear in their original order
    /// within the merged stream, but there is no ordering between the elements of
    /// different substreams.
    fn merge_unordered<St>(self) -> MergeUnordered<St, Self>
    where
        St: Stream + Unpin + Send,
        Self: Stream<Item = St> + Send,
    {
        MergeUnordered::new(self)
    }

    /// Take from this stream up to and including the element on which the predicate turns true.
    fn take_until_condition<Fut, F>(self, f: F) -> TakeUntilCondition<Self, Fut, F>
    where
        F: FnMut(&Self::Item) -> Fut,
        Fut: Future<Output = bool>,
    {
        TakeUntilCondition::new(self, f)
    }

    /// Take from this stream until the given future completes.
    fn take_until_signaled<F>(self, f: F) -> TakeUntilSignaled<Self, F>
    where
        F: Future,
    {
        TakeUntilSignaled::new(self, f)
    }

    /// Deduplicate consecutive runs of the same value.
    fn dedup(self) -> Dedup<Self>
    where
        Self::Item: Eq + Clone,
    {
        Dedup::new(self)
    }

    /// Creates a new stream that will resubmit its `Task` after a certain number
    /// of successfully polled elements. The purpose of this combinator is to
    /// ensure fairness between competing `Stream` instances on the same
    /// executor, especially on event loops. Without yielding a long-running
    /// stream (one that can be polled successfully for a large number of elements),
    /// it can cause other, unrelated streams to starve for execution resources.
    ///
    /// Using this combinator the stream will produce only up to `yield_after`
    /// elements before it returns `Async::NotReady`, then it immediately
    /// unparks its `Task` so the executor can continue the stream later.
    ///
    /// If the original stream suspends itself then the yield counter is
    /// reset, i.e. this limit only takes effect if the original stream
    /// does not suspend itself after the specified elements have been polled.
    /// For example if `yield_after` is set to 100, but the original stream
    /// always returns `Async::NotReady` after 10 elements then this
    /// combinator will not intervene as its counter is reset to 100 every
    /// time the original stream signals it is not ready.
    ///
    /// Please note that this combinator can only ensure fairness if the
    /// underlying executor is fair.
    fn yield_after(self, items: u64) -> YieldAfter<Self> {
        YieldAfter::new(self, items)
    }

    /// Feed values into the referenced [`Variable`](variable/struct.Variable.html) in addition to yielding them downstream.
    fn tee_variable(self, var: &Variable<Self::Item>) -> TeeVariable<Self, Self::Item>
    where
        Self::Item: Clone,
    {
        TeeVariable::new(self, var.clone())
    }

    /// Feed all values into a supermassive black hole and return a Future that completes when done.
    fn drain(self) -> Drain<Self> {
        Drain::new(self)
    }

    fn tap<F: FnMut(&Self::Item) -> Option<U>, U>(self, f: F) -> (Tap<Self, U, F>, UnboundedReceiver<U>) {
        Tap::new(self, f)
    }

    fn inspect_poll<F: FnMut(&Poll<Option<Self::Item>>)>(self, f: F) -> InspectPoll<Self, F> {
        InspectPoll::new(self, f)
    }
}

impl<T: Sized + Stream> AxStreamExt for T {}
