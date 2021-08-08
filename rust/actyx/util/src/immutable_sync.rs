use std::ops::Deref;

pub struct ImmutableOwned<T>(T);

unsafe impl<T: Send> Sync for ImmutableOwned<T> {}

/// Wrapper type that takes ownership of a given value, then it never hands out mutable references
/// to it. The benefit of this type that it becomes Sync automatically if the stored type is Send.
/// I.e. if it is safe to access the stored type from a different thread from which it was created
/// on, then it is safe to access concurrently from different threads.
impl<T> ImmutableOwned<T> {
    /// # Safety
    ///
    /// Be aware that this will completely break if you use it on types that have interior mutability,
    /// in other words, those types that can mutate their state even through immutable references.
    /// Wrapping such type and trying to access it from multiple-threads is undefined behavior.
    /// Hence, it is **unsafe** to create an instance of this struct.
    pub unsafe fn new(value: T) -> ImmutableOwned<T> {
        ImmutableOwned(value)
    }
}

impl<T> Deref for ImmutableOwned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<T> for ImmutableOwned<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
