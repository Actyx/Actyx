use fnv::FnvHashMap;
use futures::{prelude::*, stream::FusedStream};
use parking_lot::{Mutex, MutexGuard};
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
    usize,
};

#[derive(Debug)]
pub struct Observer<T> {
    id: usize,
    inner: Arc<Mutex<VariableInner<T>>>,
}

impl<T> Observer<T> {
    fn new(inner: Arc<Mutex<VariableInner<T>>>) -> Self {
        let id = inner.lock().new_observer_id();
        Self { id, inner }
    }
}

fn poll_next_impl<'a, T, U>(
    mut inner: MutexGuard<'a, VariableInner<T>>,
    id: usize,
    cx: &mut Context<'_>,
    f: &impl Fn(&T) -> U,
) -> std::task::Poll<Option<U>> {
    if inner.writers == 0 {
        // if the sender is gone, make sure that the final value is delivered
        // (the .remove() ensures that next time will return None)
        if let Some(receiver) = inner.observers.remove(&id) {
            if !receiver.received {
                return Poll::Ready(Some(f(&inner.latest)));
            }
        }
        Poll::Ready(None)
    } else if let Some(receiver) = inner.observers.get_mut(&id) {
        if receiver.received {
            receiver.waker = Some(cx.waker().clone());
            // we have already received this value
            Poll::Pending
        } else {
            // got a value, make sure we don't get it again and return it
            receiver.received = true;
            Poll::Ready(Some(f(&inner.latest)))
        }
    } else {
        // this means that the sender was dropped, so end the stream
        Poll::Ready(None)
    }
}

impl<T: Clone> Stream for Observer<T> {
    type Item = T;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        poll_next_impl(self.inner.lock(), self.id, cx, &|x: &T| x.clone())
    }
}

impl<T: Clone> FusedStream for Observer<T> {
    fn is_terminated(&self) -> bool {
        !self.inner.lock().observers.contains_key(&self.id)
    }
}

/// we are just an arc, so we can be moved around
impl<T> Unpin for Observer<T> {}

impl<T> Clone for Observer<T> {
    fn clone(&self) -> Self {
        Observer::new(self.inner.clone())
    }
}

impl<T> Drop for Observer<T> {
    fn drop(&mut self) {
        self.inner.lock().observers.remove(&self.id);
    }
}

#[derive(Debug)]
pub struct Projection<T, F> {
    inner: Arc<Mutex<VariableInner<T>>>,
    f: F,
    id: usize,
}

impl<T, F, R> Projection<T, F>
where
    F: FnOnce(&T) -> R,
{
    fn new(inner: Arc<Mutex<VariableInner<T>>>, f: F) -> Self {
        let id = inner.lock().new_observer_id();
        Self { inner, f, id }
    }
}

impl<T, F, R> Stream for Projection<T, F>
where
    F: Fn(&T) -> R,
{
    type Item = R;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        poll_next_impl(self.inner.lock(), self.id, cx, &self.f)
    }
}

impl<T, F, R> FusedStream for Projection<T, F>
where
    F: Fn(&T) -> R,
{
    fn is_terminated(&self) -> bool {
        !self.inner.lock().observers.contains_key(&self.id)
    }
}

impl<T, F> Clone for Projection<T, F>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            f: self.f.clone(),
            id: self.id,
        }
    }
}

impl<T, F> Drop for Projection<T, F> {
    fn drop(&mut self) {
        self.inner.lock().observers.remove(&self.id);
    }
}

/// A variable that can be observed by an arbitrary number of observer streams
///
/// Observer streams will only get the most recent variable value.
///
/// Having zero observers is often useful, so setting the value will not fail
/// even if there are no observers.
#[derive(Debug)]
pub struct Variable<T> {
    inner: Arc<Mutex<VariableInner<T>>>,
}

impl<T> Variable<T> {
    pub fn new(value: T) -> Self {
        let inner = Arc::new(Mutex::new(VariableInner::new(value)));
        Self { inner }
    }

    /// Number of current observers.
    pub fn observer_count(&self) -> usize {
        self.inner.lock().observers.len()
    }

    /// Send a value and notify all current receivers.
    /// This will not fail even if all receivers are dropped. It will just go into nirvana.
    pub fn set(&self, value: T) {
        self.inner.lock().set(value)
    }

    /// Transform the current value and send an update if the transform is successful and returns a new value
    pub fn transform<E>(&self, f: impl FnOnce(&T) -> std::result::Result<Option<T>, E>) -> std::result::Result<(), E> {
        let mut inner = self.inner.lock();
        if let Some(value) = f(&inner.latest)? {
            inner.set(value);
        }
        Ok(())
    }

    /// Transform the current value in-place and notify observers if the function returns `true`.
    ///
    /// This does not allow a fallible operation because in case of error it is unclear which
    /// state the variable’s value is in.
    ///
    /// Returns what the given transformation function returned.
    pub fn transform_mut(&self, f: impl FnOnce(&mut T) -> bool) -> bool {
        let mut inner = self.inner.lock();
        if f(&mut inner.latest) {
            inner.notify();
            true
        } else {
            false
        }
    }

    /// Read and project out a value. This can be cheaper than using get_cloned()
    pub fn project<F, U>(&self, f: F) -> U
    where
        F: Fn(&T) -> U,
    {
        f(&self.inner.lock().latest)
    }

    /// One way of creating a new observer. The other is to clone an existing observer.
    pub fn new_observer(&self) -> Observer<T> {
        Observer::new(self.inner.clone())
    }

    /// a stream of a projection
    pub fn new_projection<F, U>(&self, f: F) -> Projection<T, F>
    where
        F: Fn(&T) -> U,
    {
        Projection::new(self.inner.clone(), f)
    }
}

impl<T: Clone> Variable<T> {
    pub fn get_cloned(&self) -> T {
        self.project(|x| x.clone())
    }
}

impl<T: Copy> Variable<T> {
    pub fn get(&self) -> T {
        self.inner.lock().latest
    }
}

impl<T> Clone for Variable<T> {
    fn clone(&self) -> Self {
        self.inner.lock().writers += 1;
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Variable<T> {
    fn drop(&mut self) {
        self.inner.lock().writers -= 1;
    }
}

impl<T> Unpin for Variable<T> {}

impl<T: Default> Default for Variable<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[derive(Debug)]
struct VariableInner<T> {
    next_id: usize,
    observers: FnvHashMap<usize, ReceiverInner>,
    latest: T,
    writers: usize,
}

impl<T> VariableInner<T> {
    pub fn new(value: T) -> Self {
        Self {
            next_id: 0,
            observers: Default::default(),
            latest: value,
            writers: 1,
        }
    }

    fn set(&mut self, value: T) {
        // we don't check for dupliates. You can send the same value twice.
        self.latest = value;
        self.notify();
    }

    fn notify(&mut self) {
        for observer in self.observers.values_mut() {
            // reset received
            observer.received = false;
            if let Some(waker) = observer.waker.take() {
                waker.wake();
            }
        }
    }

    /// Allocate a new receiver and return its id
    fn new_observer_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        // If the sender is dropped, there is no point in storing a new receiver.
        if self.writers > 0 {
            self.observers.insert(id, ReceiverInner::new());
        }
        id
    }
}

#[derive(Debug, Default)]
struct ReceiverInner {
    received: bool,
    waker: Option<Waker>,
}

impl ReceiverInner {
    fn new() -> Self {
        Self {
            received: false,
            waker: None,
        }
    }
}

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct TeeVariable<S, T> {
        #[pin]
        stream: S,
        variable: Variable<T>,
    }
}

impl<S: Stream> TeeVariable<S, S::Item> {
    pub(crate) fn new(stream: S, variable: Variable<S::Item>) -> Self {
        Self { stream, variable }
    }
}

impl<S: Stream> Stream for TeeVariable<S, S::Item>
where
    S::Item: Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.poll_next(cx) {
            Poll::Ready(Some(v)) => {
                this.variable.set(v.clone());
                Poll::Ready(Some(v))
            }
            x => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::AxStreamExt,
        stream::{interval, Drainer},
    };

    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn smoke() -> anyhow::Result<()> {
        let s = Variable::<usize>::new(1);
        let r1 = s.new_observer();
        // set 1
        s.set(1);
        let (v, r1) = r1.into_future().await;
        assert_eq!(v, Some(1));
        // set 2
        let r2 = r1.clone();
        s.set(2);
        // r2 receives 2
        let (v, r2) = r2.into_future().await;
        assert_eq!(v, Some(2));
        // set 3
        s.set(3);
        // r1 does not receive 2, but just 3
        let (v, r1) = r1.into_future().await;
        assert_eq!(v, Some(3));
        drop(r1);
        drop(r2);
        assert_eq!(s.observer_count(), 0);
        drop(s);
        Ok(())
    }

    #[tokio::test]
    async fn pipe() {
        let s = Variable::new(0u8);
        tokio::spawn(
            interval(Duration::from_millis(10))
                .take(10)
                .enumerate()
                .skip(1) // 0 is initial value, so start at 1 here
                .map(|x| x.0 as u8)
                .tee_variable(&s)
                .drain(),
        );
        let obs = s.new_observer();
        drop(s);
        let r = obs.collect::<Vec<_>>().await;
        assert!(r.len() <= 10, "too many elements in {:?}", r);
        // timing-dependently may lose any element but the last
        assert_eq!(r[r.len() - 1], 9);
    }

    #[tokio::test]
    async fn dropping() {
        let v = Variable::new(0);
        let v2 = v.clone();
        drop(v);
        let mut iter = Drainer::new(v2.new_observer());
        assert_eq!(iter.next(), Some(vec![0]));
        assert_eq!(iter.next(), Some(vec![]));
    }

    #[tokio::test]
    async fn projection() {
        let v = Variable::new((0, 0));
        let mut iter = Drainer::new(v.new_projection(|x| x.0));
        assert_eq!(iter.next(), Some(vec![0]));
        v.transform_mut(|x| {
            x.0 = 1;
            true
        });
        assert_eq!(iter.next(), Some(vec![1]));
        v.transform_mut(|x| {
            x.1 = 1;
            true
        });
        assert_eq!(iter.next(), Some(vec![1]));
        assert_eq!(iter.next(), Some(vec![]));
    }
}
