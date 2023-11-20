use crossbeam::channel::{self, RecvError, Sender};
use std::sync::Arc;
use tracing::*;

/// Protects a resource so that is always tied to a single, dedicated thread. This wrapper allows other
/// places to safely access the underlying resource even if that is not Sync or even Send. This
/// is achieved by having a separate, dedicated thread hosting the resource which then communicates
/// via a message queue with the rest of the system.
///
/// This wrapper is also useful to isolate a blocking resource from a thread-pool that hosts other
/// async tasks.
pub struct PinnedResourceSync<T> {
    stopped: Arc<()>, // Just for keeping track of the count of external handles
    sender: Sender<Box<dyn FnOnce(&mut T) + Send>>,
}

impl<T> Clone for PinnedResourceSync<T> {
    fn clone(&self) -> Self {
        PinnedResourceSync {
            stopped: self.stopped.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<T: 'static> PinnedResourceSync<T> {
    pub fn new<F>(creator: F, identifier: &str) -> PinnedResourceSync<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let stopped = Arc::new(());
        let (snd, rcv) = channel::unbounded();
        let pinned_res = PinnedResourceSync {
            stopped: stopped.clone(),
            sender: snd,
        };
        std::thread::Builder::new()
            .name(format!("PinnedResourceSync: {}", identifier))
            .spawn(move || {
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
                        Err(_) => {
                            // "A message could not be received because the channel is empty and disconnected."
                            // This is an expected race if strong_count() was 2, but then the remaining client dropped the handle.
                            info!("PinnedResourceSync: The wrapped resource was dropped while we waited for functions to be sent.");
                            return;
                        }
                    }
                }
            })
            .expect("failed to spawn thread");

        pinned_res
    }

    /// Access the protected value and possibly mutate it. Blocks until the response is available.
    pub fn spawn_mut<U, F>(&self, f: F) -> Result<U, RecvError>
    where
        U: Send + 'static,
        F: FnOnce(&mut T) -> U + Send + 'static,
    {
        let (snd, rcv) = channel::bounded(1);
        self.sender
            .send(Box::new(move |res| {
                let result = f(res);
                if snd.send(result).is_err() {
                    warn!("Could not finish closure from PinnedResourceSync");
                }
            }))
            .expect("Could not spawn closure into PinnedResourceSync");

        rcv.recv()
    }
}
