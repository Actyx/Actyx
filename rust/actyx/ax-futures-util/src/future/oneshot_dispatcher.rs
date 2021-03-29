use fnv::FnvHashMap;
use futures::channel::oneshot;
use itertools::repeat_n;
use std::{fmt::Debug, hash::Hash};

/// A dispatcher for oneshot sender/receiver pairs
#[derive(Debug)]
pub struct OneShotDispatcher<K: Eq + Hash + Debug, V> {
    items: FnvHashMap<K, Vec<oneshot::Sender<V>>>,
}

impl<K: Eq + Hash + Debug, V: Clone> OneShotDispatcher<K, V> {
    /// Creates a new, empty dispatcher
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    /// registers a sender with the dispatcher for a key `K`
    pub fn register(&mut self, key: K, sender: oneshot::Sender<V>) {
        self.items.entry(key).or_default().push(sender)
    }

    /// notifies all registered senders for the key `K`.
    ///
    /// Since these are oneshot dispatchers, they will all be removed.
    pub fn notify(&mut self, key: K, value: V) {
        if let Some(senders) = self.items.remove(&key) {
            let values = repeat_n(value, senders.len());
            for (sender, value) in senders.into_iter().zip(values) {
                // when this fails, it just means that the receiver end has been dropped -
                // which might be perfectly normal.
                let _ = sender.send(value);
            }
        }
    }

    /// removes all senders for which the receiver side has been dropped
    pub fn gc(&mut self) {
        self.items.retain(|_, v| {
            v.retain(|sender| !sender.is_canceled());
            !v.is_empty()
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.items.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::OneShotDispatcher;
    use futures::{channel, future::join_all, join};

    #[tokio::test]
    async fn smoke() {
        let pairs = (0..10usize).map(|key| {
            let (s, r) = channel::oneshot::channel::<usize>();
            ((key, s), r)
        });
        let (senders, receivers): (Vec<_>, Vec<_>) = pairs.unzip();

        let mut dispatcher = OneShotDispatcher::new();
        let keys = senders.iter().map(|(key, _)| key).cloned().collect::<Vec<_>>();

        // register the senders
        for (key, sender) in senders {
            dispatcher.register(key, sender);
        }

        // register interest in all the receivers
        let consumer = async {
            let results = join_all(receivers).await;
            let expected = keys.iter().cloned().map(Ok).collect::<Vec<_>>();
            assert_eq!(results, expected);
        };

        let producer = async {
            // fire them one by one and watch the dispatcher items map get smaller
            for (i, key) in keys.iter().enumerate() {
                dispatcher.notify(*key, i);
                assert_eq!(dispatcher.items.len(), 10 - i - 1);
            }
        };

        join!(producer, consumer);
    }

    #[tokio::test]
    async fn cleanup() {
        let (sa, ra) = channel::oneshot::channel::<usize>();
        let (sb, rb) = channel::oneshot::channel::<usize>();
        let mut dispatcher = OneShotDispatcher::new();
        dispatcher.register("a", sa);
        dispatcher.register("b", sb);

        // dropping by itself does nothing
        drop(ra);
        assert_eq!(dispatcher.items.len(), 2);

        // gc detects dropped receiver
        dispatcher.gc();
        assert_eq!(dispatcher.items.len(), 1);

        // dropping by itself does nothing
        drop(rb);
        assert_eq!(dispatcher.items.len(), 1);

        // sending detects dropped receiver
        dispatcher.notify("b", 1234);
        assert_eq!(dispatcher.items.len(), 0);
    }
}
