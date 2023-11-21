use fnv::FnvHashMap;
use futures::channel::mpsc::UnboundedSender;
use itertools::repeat_n;
use std::{collections::VecDeque, fmt::Debug, hash::Hash};

/// A dispatcher for unbounded stream sender/receiver pairs
#[derive(Debug, Default)]
pub struct StreamDispatcher<K: Eq + Hash + Debug, V> {
    items: FnvHashMap<K, Vec<UnboundedSender<V>>>,
    dropped: VecDeque<K>,
}

impl<K: Clone + Eq + Hash + Debug, V: Clone> StreamDispatcher<K, V> {
    /// Creates a new, empty dispatcher
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            items: Default::default(),
            dropped: Default::default(),
        }
    }

    /// register an unbounded sender for values for key `K`
    pub fn register(&mut self, key: K, sender: UnboundedSender<V>) {
        self.items.entry(key).or_default().push(sender)
    }

    /// notifies all registered senders for the key `K` that a new item has arrived.
    pub fn notify(&mut self, key: K, value: V) {
        if let Some(senders) = self.items.get_mut(&key) {
            let mut values = repeat_n(value, senders.len());
            let dropped = &mut self.dropped;
            senders.retain(|sender| {
                let value = values.next().unwrap();
                if let Err(cause) = sender.unbounded_send(value) {
                    // receiver has been dropped, nothing unusual
                    debug_assert!(cause.is_disconnected());
                    dropped.push_back(key.clone());
                    false
                } else {
                    true
                }
            });
            if senders.is_empty() {
                // no more senders, get rid of the entry
                self.items.remove(&key);
            }
        }
    }

    /// removes all senders for which the receiver side has been dropped
    pub fn gc(&mut self) {
        let dropped = &mut self.dropped;
        self.items.retain(|k, v| {
            v.retain(|sender| !sender.is_closed());
            if v.is_empty() {
                dropped.push_back(k.clone());
            }
            !v.is_empty()
        })
    }
}

impl<K: Eq + Hash + Debug, V> Iterator for StreamDispatcher<K, V> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.dropped.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::StreamDispatcher;
    use futures::{channel, join, stream::StreamExt};

    #[tokio::test]
    async fn smoke() {
        let (s, r) = channel::mpsc::unbounded::<usize>();
        let mut r = r.take(10).enumerate();
        let mut dispatcher = StreamDispatcher::new();
        dispatcher.register("key", s);

        let consumer = async {
            while let Some((i, value)) = r.next().await {
                assert_eq!(i, value);
            }
        };

        let producer = async {
            for i in 0..10usize {
                dispatcher.notify("key", i);
            }
        };

        join!(producer, consumer);
    }

    #[tokio::test]
    async fn cleanup() {
        let (sa, ra) = channel::mpsc::unbounded::<usize>();
        let (sb, rb) = channel::mpsc::unbounded::<usize>();
        let mut dispatcher = StreamDispatcher::new();
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
