use crossbeam::channel;
use crossbeam::channel::Sender;
use futures::channel::oneshot;
use futures::channel::oneshot::Canceled;
use futures::Future;
use std::sync::Arc;
use std::thread;
use tracing::*;

/// Protects a resource so that it's always tied to a single, dedicated thread. This wrapper allows other
/// places to safely access the underlying resource even if that is not Sync or even Send. This
/// is achieved by having a separate, dedicated thread hosting the resource which then communicates
/// via a message queue with the rest of the system.
///
/// This wrapper is also useful to isolate a blocking resource from a thread-pool that hosts other
/// async tasks.
pub struct PinnedResource<T> {
    stopped: Arc<()>, // Just for keeping track of the count of external handles
    sender: Sender<Box<dyn FnOnce(&mut T) + Send>>,
}

impl<T> Clone for PinnedResource<T> {
    fn clone(&self) -> Self {
        PinnedResource {
            stopped: self.stopped.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<T: 'static> PinnedResource<T> {
    pub fn new<F>(creator: F) -> PinnedResource<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let stopped = Arc::new(());
        let (snd, rcv) = channel::unbounded();
        let pinned_res = PinnedResource {
            stopped: stopped.clone(),
            sender: snd,
        };

        thread::spawn(move || {
            let mut res = creator();

            loop {
                // We should stop when all instances of the Arc except ours have been deallocated.
                // There is no race here, since now we are the sole owner and we don't hand out
                // clones.
                if Arc::strong_count(&stopped) == 1 {
                    return;
                }

                match rcv.recv() {
                    Ok(f) => f(&mut res),
                    Err(_) => return,
                }
            }
        });

        pinned_res
    }

    /// Access the protected value and possibly mutate it. Returns a `Future` that carries
    /// the result of the spawned computation.
    pub fn spawn_mut<U, F>(&self, f: F) -> impl Future<Output = Result<U, Canceled>>
    where
        U: Send + 'static,
        F: FnOnce(&mut T) -> U + Send + 'static,
    {
        // let current = Span::current();
        let (snd, rcv) = oneshot::channel();
        self.sender
            .send(Box::new(move |res| {
                // let span = debug_span!(parent: current, "spawn_mut");
                // let _enter = span.enter();
                let result = f(res);
                if snd.send(result).is_err() {
                    warn!("Could not finish future from PinnedResource");
                }
            }))
            .expect("Could not spawn closure into PinnedResource");

        rcv
    }
}

#[cfg(test)]
mod tests {
    use crate::pinned_resource::PinnedResource;
    use std::cell::UnsafeCell;
    use std::sync::Arc;
    use std::thread;

    // Does nothing, but it can assert that T is Sync, Send and 'static at compile time of the tests
    fn is_sync_send_static<T: Sync + Send + 'static>() {}

    // just for testing
    struct NotSend(pub UnsafeCell<u64>);

    #[test]
    fn pinned_thread_is_sync() {
        is_sync_send_static::<PinnedResource<NotSend>>()
    }

    #[tokio::test]
    async fn works_with_not_send_resource() {
        let pin_res = PinnedResource::new(|| NotSend(UnsafeCell::new(42)));

        let result = pin_res
            .spawn_mut(|res| {
                let res: &mut u64 = unsafe { &mut *res.0.get() };
                *res += 1;
                *res
            })
            .await
            .unwrap();
        assert_eq!(result, 43);

        let result = pin_res
            .spawn_mut(|res| {
                let res: &u64 = unsafe { &*res.0.get() };
                *res
            })
            .await
            .unwrap();

        assert_eq!(result, 43);
    }

    #[test]
    fn properly_serializes_access() {
        let pin_res = PinnedResource::new(|| 0_usize);
        let thread_cnt = 50;
        let start_barrier = Arc::new(std::sync::Barrier::new(thread_cnt));

        let mut results: Vec<usize> = (0..thread_cnt)
            .map(|_| {
                let pin_res_clone = pin_res.clone();
                let barrier = start_barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    futures::executor::block_on(pin_res_clone.spawn_mut(|res| {
                        let x = *res;
                        *res += 1;
                        x
                    }))
                    .unwrap()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        results.sort_unstable();

        assert_eq!(results, (0..thread_cnt).collect::<Vec<usize>>());
    }
}
