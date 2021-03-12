use crate::immutable_sync::ImmutableOwned;
use crossbeam::epoch::{self, Atomic, Owned, Shared};
use crossbeam::queue::SegQueue;
use crossbeam::utils::CachePadded;
use futures::channel::oneshot::{self, Sender as FSender};
use futures::{future, stream, FutureExt, Stream};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// Provides a mechanism to broadcast without back pressure to a set of receiver streams, and
/// only retain the latest published element.
///
/// The provided [Sender](Sender) allows an unconditional broadcast to all the receiver
/// streams that are ready to consume. [Receiver](Receiver) streams that are currently not available (has
/// not been polled) are simply skipped.
///
/// Once a _new_ or _stale_ receiver is polled, it will check if it has seen the latest element
/// being broadcast, and _receive_ that element immediately if the receiver hasn't seen it,
/// potentially skipping ahead over a received element that hasn't been consumed.
pub fn new<T: Clone + Send + Debug + 'static>() -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(SegQueue::new());
    let latest = Arc::new(CachePadded::new(AtomicLatestDropping(Atomic::null())));
    (
        Sender {
            sequence: Arc::new(AtomicUsize::new(INITIAL_SEQUENCE)),
            tasks: queue.clone(),
            latest: latest.clone(),
        },
        Receiver { tasks: queue, latest },
    )
}

enum Task {
    FinishBatch,
    Wakeup(FSender<()>),
}

#[derive(Clone, Debug)]
struct Latest<T>
where
    T: Clone + Send + Debug,
{
    value: T,
    sequence: usize,
}

#[derive(Debug)]
struct AtomicLatestDropping<T>(Atomic<ImmutableOwned<Latest<T>>>)
where
    T: Clone + Send + Debug;

impl<T> AtomicLatestDropping<T>
where
    T: Clone + Send + Debug,
{
    // Replace current latest value and deallocate the old one.
    fn replace_owned(&self, value: T, sequence: usize) {
        let new_latest = Latest { sequence, value };
        let guard = epoch::pin();

        // This is unsafe because ImmutableOwned is inherently unsafe,
        // and defer_destroy is unsafe if you omit the null-check.
        unsafe {
            let old_latest = self
                .0
                .swap(Owned::new(ImmutableOwned::new(new_latest)), Ordering::SeqCst, &guard);
            if !old_latest.is_null() {
                guard.defer_destroy(old_latest);
            }
        }
    }

    fn load_nullable(&self) -> Option<Latest<T>> {
        let guard = epoch::pin();
        let latest_ptr = self.0.load(Ordering::SeqCst, &guard);

        // No value stored yet.
        if latest_ptr.is_null() {
            return None;
        }

        unsafe {
            // Unsafe because it might be null, but we just checked.
            let l = latest_ptr.deref().deref();
            Some(Latest {
                sequence: l.sequence,
                value: l.value.clone(),
            })
        }
    }
}

impl<T> Drop for AtomicLatestDropping<T>
where
    T: Clone + Send + Debug,
{
    fn drop(&mut self) {
        unsafe {
            // We're the last thread to hold the ptr, so no safety measures needed.
            let guard = epoch::unprotected();
            let old_latest = self.0.swap(Shared::null(), Ordering::SeqCst, &guard);
            if !old_latest.is_null() {
                guard.defer_destroy(old_latest);
            }
        }
    }
}

type AtomicLatest<T> = CachePadded<AtomicLatestDropping<T>>;

#[derive(Clone, Debug)]
pub struct Receiver<T: Clone + Send + Debug + 'static> {
    tasks: Arc<SegQueue<Task>>,
    latest: Arc<AtomicLatest<T>>,
}

#[derive(Clone, Debug)]
pub struct Sender<T: Clone + Send + Debug + 'static> {
    sequence: Arc<AtomicUsize>,
    tasks: Arc<SegQueue<Task>>,
    latest: Arc<AtomicLatest<T>>,
}

const INITIAL_SEQUENCE: usize = 0;

impl<T: Clone + Send + Debug + 'static> Sender<T> {
    pub fn update(&mut self, value: T) {
        let mut sequence = INITIAL_SEQUENCE;
        // Make sure that we never use the INITIAL_SEQUENCE number,
        // even if we wrap the sequence counter by reaching usize::MAX
        while sequence == INITIAL_SEQUENCE {
            sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        }
        trace!("Update {} with {:?}", sequence, value);

        // Finish batch BEFORE updating the value -- this is our marker for all
        // pending wakeups for THIS new value we are going to set.
        self.tasks.push(Task::FinishBatch);

        self.latest.replace_owned(value, sequence);

        self.wake_queued_consumers();
    }

    fn wake_queued_consumers(&mut self) {
        loop {
            match self.tasks.pop() {
                Some(Task::FinishBatch) => return,
                Some(Task::Wakeup(sender)) => {
                    if !sender.is_canceled() && sender.send(()).is_err() {
                        debug!("Wakeup failed in sampled_broadcast!");
                    }
                }
                None => return,
            };
        }
    }
}

impl<T: Clone + Send + Debug + 'static> Receiver<T> {
    pub fn stream(&self) -> impl Stream<Item = T> {
        let tasks = self.tasks.clone();
        let latest_1 = self.latest.clone();

        let unfold_fn = move |latest_emitted_sequence| {
            let latest_2 = latest_1.clone();

            // If a yet-unseen value is immediately available, just immediately emit it.
            if let Some(latest) = check_latest_against_emitted(&latest_1, latest_emitted_sequence) {
                let value = latest.value;
                let sequence = latest.sequence;
                trace!("Emitting latest {} with value {:?}", sequence, value);
                future::ready(Some((value, sequence))).left_future()
            } else {
                // Otherwise, we register ourselves for wakeup when the value is next updated.
                // Note that we may skip an arbitrary amount of values, because the CPU could
                // schedule several update calls before our thread is continued.
                let (snd, rcv) = oneshot::channel();
                tasks.push(Task::Wakeup(snd));
                rcv.map(move |result| match result {
                    Result::Ok(()) => {
                        // Since we were signalled by the Sender, there is always something to read.
                        // Compare against INITIAL_SEQUENCE to ensure that we read the latest.
                        let latest = check_latest_against_emitted(&latest_2, INITIAL_SEQUENCE).unwrap();
                        let sequence = latest.sequence;
                        let value = latest.value;
                        trace!("Emitting latest {} with value {:?}", sequence, value);
                        Some((value, sequence))
                    }
                    _ => {
                        warn!("sampled_broadcast dropped our Sender without sending anything!");
                        None
                    }
                })
                .right_future()
            }
        };

        stream::unfold(INITIAL_SEQUENCE, unfold_fn)
    }
}

fn check_latest_against_emitted<T>(latest: &Arc<AtomicLatest<T>>, sequence: usize) -> Option<Latest<T>>
where
    T: Clone + Send + Debug,
{
    latest
        .load_nullable()
        // Check if we see an older sequence number, or if we likely wrapped the sequence counter
        .filter(|l| sequence < l.sequence || (sequence - l.sequence) > (std::usize::MAX / 2))
}

#[cfg(test)]
mod tests {
    use crate::sampled_broadcast::Receiver;
    use crate::sampled_broadcast::Sender;
    use crossbeam::epoch::Atomic;
    use futures_test::*;
    use std::sync::atomic::Ordering;

    #[derive(Clone, Debug, PartialEq)]
    struct U64(u64);

    // Does nothing, but it can assert that T is Sync, Send and 'static at compile time of the tests
    fn is_sync_send_static<T: Sync + Send + 'static>() {}

    #[test]
    fn pinned_thread_is_sync() {
        is_sync_send_static::<Receiver<U64>>();
        is_sync_send_static::<Sender<U64>>();
    }

    #[test]
    fn broadcast_to_all_streams() {
        let (mut snd, rcv) = super::new::<U64>();

        let mut s1 = rcv.stream();
        let mut s2 = rcv.stream();
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);

        snd.update(U64(1));
        assert_stream_next!(s1, U64(1));
        assert_stream_next!(s2, U64(1));
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);

        snd.update(U64(2));
        assert_stream_next!(s1, U64(2));
        assert_stream_next!(s2, U64(2));
    }

    #[test]
    fn get_the_latest_item_when_starting_late() {
        let (mut snd, rcv) = super::new::<U64>();

        let mut s1 = rcv.stream();
        assert_stream_pending!(s1);

        snd.update(U64(1));
        assert_stream_next!(s1, U64(1));
        assert_stream_pending!(s1);

        snd.update(U64(2));
        assert_stream_next!(s1, U64(2));
        assert_stream_pending!(s1);

        let mut s2 = rcv.stream();
        assert_stream_next!(s2, U64(2));
        assert_stream_pending!(s2);
    }

    #[test]
    fn get_the_latest_item_when_wrapping_the_counter() {
        let (mut snd, rcv) = super::new::<U64>();

        let mut s1 = rcv.stream();
        assert_stream_pending!(s1);

        snd.update(U64(1));
        assert_stream_next!(s1, U64(1));
        assert_stream_pending!(s1);

        // set up the test to store at std::usize::MAX before we wrap
        snd.sequence.store(std::usize::MAX, Ordering::SeqCst);

        snd.update(U64(2));
        assert_stream_next!(s1, U64(2));
        assert_stream_pending!(s1);

        snd.update(U64(3));
        assert_stream_next!(s1, U64(3));
        assert_stream_pending!(s1);
        assert_eq!(2, snd.sequence.load(Ordering::SeqCst))
    }

    #[test]
    fn catch_up_after_not_being_polled() {
        // This test also checks that the broadcast isn't stalled by unpolled streams
        let (mut snd, rcv) = super::new::<U64>();

        let mut s1 = rcv.stream();
        let mut s2 = rcv.stream();
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);

        snd.update(U64(1));
        assert_stream_next!(s1, U64(1));
        assert_stream_next!(s2, U64(1));
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);

        snd.update(U64(2));
        assert_stream_next!(s1, U64(2));
        assert_stream_pending!(s1);

        snd.update(U64(3));
        assert_stream_next!(s1, U64(3));
        assert_stream_next!(s2, U64(3));
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);
    }

    #[test]
    fn skip_directly_to_the_latest_element_when_trying_to_catch_up() {
        // This test also checks that the broadcast isn't stalled by unpolled streams
        let (mut snd, rcv) = super::new::<U64>();

        let mut s1 = rcv.stream();
        let mut s2 = rcv.stream();
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);

        snd.update(U64(1));
        assert_stream_next!(s1, U64(1));
        assert_stream_pending!(s1);

        snd.update(U64(2));
        assert_stream_next!(s1, U64(2));
        assert_stream_pending!(s1);

        snd.update(U64(3));
        assert_stream_next!(s1, U64(3));
        assert_stream_next!(s2, U64(3));
        assert_stream_pending!(s1);
        assert_stream_pending!(s2);
    }

    #[test]
    fn update_from_null() {
        let k = super::AtomicLatestDropping::<U64>(Atomic::null());
        k.replace_owned(U64(2), 1);

        let latest = k.load_nullable().unwrap();

        assert_eq!(U64(2), latest.value);
        assert_eq!(1, latest.sequence);
    }

    #[test]
    fn update_repeatedly() {
        let k = super::AtomicLatestDropping::<U64>(Atomic::null());
        k.replace_owned(U64(2), 1);

        let latest = k.load_nullable().unwrap();
        assert_eq!(U64(2), latest.value);
        assert_eq!(1, latest.sequence);

        k.replace_owned(U64(50), 2);

        let latest = k.load_nullable().unwrap();
        assert_eq!(U64(50), latest.value);
        assert_eq!(2, latest.sequence);
    }

    #[test]
    fn load_null() {
        let k = super::AtomicLatestDropping::<U64>(Atomic::null());
        let latest = k.load_nullable();

        assert!(latest.is_none());
    }
}
