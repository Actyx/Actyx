//! A mutex that panics instead of deadlocking when used in a reentrant way
use parking_lot::{Condvar, Mutex};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    thread::ThreadId,
};

/// A mutex that safely panics on reentrant use instead of deadlocking
///
/// Other than that, it behaves exactly like a parking_lot mutex.
pub struct ReentrantSafeMutex<T: ?Sized> {
    thread: Mutex<Option<ThreadId>>,
    condvar: Condvar,
    value: UnsafeCell<T>,
}

pub struct ReentrantSafeMutexGuard<'a, T> {
    mutex: &'a ReentrantSafeMutex<T>,
}

impl<T> ReentrantSafeMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            thread: Mutex::new(None),
            condvar: Condvar::new(),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> ReentrantSafeMutexGuard<'_, T> {
        let current_thread_id = std::thread::current().id();
        let mut thread = self.thread.lock();
        // parking_lot supposedly has no spurious wakeups.
        // https://docs.rs/parking_lot/0.11.1/parking_lot/struct.Condvar.html#differences-from-the-standard-library-condvar
        // But in tests I have seen the wait exiting with the thread not being None.
        // Hence the while loop to be safe against spurious wakeups.
        // since when coming out of the wait we already have the lock, just checking one more time
        // will be extremely cheap.
        while let Some(id) = *thread {
            assert!(id != current_thread_id, "Reentrant locking attempt");
            self.condvar.wait(&mut thread);
        }
        debug_assert!(*thread == None);
        *thread = Some(current_thread_id);
        ReentrantSafeMutexGuard { mutex: self }
    }
}

impl<'a, T> Drop for ReentrantSafeMutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut thread = self.mutex.thread.lock();
        *thread = None;
        self.mutex.condvar.notify_one();
    }
}

impl<'a, T> Deref for ReentrantSafeMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for ReentrantSafeMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

unsafe impl<T: ?Sized + Send> Send for ReentrantSafeMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for ReentrantSafeMutex<T> {}

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

    /// Hammer the mutex with small ops from several threads to make sure it does the job.
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
