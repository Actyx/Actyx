//! A mutex that panics instead of deadlocking when used in a reentrant way
use parking_lot::{Mutex, MutexGuard};
use std::{
    ops::{Deref, DerefMut},
    thread::ThreadId,
};

/// A mutex that safely panics on reentrant use instead of deadlocking
///
/// Other than that, it behaves exactly like a parking_lot mutex.
pub struct ReentrantSafeMutex<T> {
    /// The thread that currently holds the main mutex
    ///
    /// This can be replaced by an atomic once https://github.com/rust-lang/rust/issues/67939 is solved
    thread: Mutex<Option<ThreadId>>,

    /// the inner mutex
    inner: Mutex<T>,
}

pub struct ReentrantSafeMutexGuard<'a, T> {
    /// reference to the thread, so we can clear it.
    thread: &'a Mutex<Option<ThreadId>>,
    /// the guard itself, wrapped in an option so we can control drop order. Yuck!
    guard: Option<MutexGuard<'a, T>>,
}

impl<T> ReentrantSafeMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            thread: Mutex::new(None),
            inner: Mutex::new(value),
        }
    }

    pub fn lock(&self) -> ReentrantSafeMutexGuard<'_, T> {
        let current_thread_id = std::thread::current().id();
        let mut thread = self.thread.lock();
        if *thread == Some(current_thread_id) {
            panic!("Reentrant locking attempt!")
        }
        let guard = Some(self.inner.lock());
        *thread = Some(current_thread_id);
        ReentrantSafeMutexGuard {
            thread: &self.thread,
            guard,
        }
    }
}

impl<'a, T> Drop for ReentrantSafeMutexGuard<'a, T> {
    fn drop(&mut self) {
        // need to first let go of the inner guard, otherwise deadlock!
        self.guard = None;
        *self.thread.lock() = None
    }
}

impl<'a, T> Deref for ReentrantSafeMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap().deref()
    }
}

impl<'a, T> DerefMut for ReentrantSafeMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    #[should_panic(expected = "Reentrant locking attempt")]
    fn panics_on_reentrant_use() {
        let m = ReentrantSafeMutex::new(1);
        let g1 = m.lock();
        // this will panic. For a normal Mutex it would deadlock instead.
        let g2 = m.lock();
        drop(g1);
        drop(g2);
    }

    #[test]
    fn does_not_panic_on_sequential_use() {
        let m = ReentrantSafeMutex::new(1);
        let g1 = m.lock();
        drop(g1);
        let g2 = m.lock();
        drop(g2);
    }

    #[test]
    fn works_as_a_mutex() {
        let m = Arc::new(ReentrantSafeMutex::new(0));
        let handles = (0..10)
            .map(|i| {
                let m = m.clone();
                let inc = (i % 2) * 2 - 1;
                std::thread::spawn(move || {
                    for _j in 0..1000 {
                        *m.lock() += inc;
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(*m.lock() == 0);
    }
}
