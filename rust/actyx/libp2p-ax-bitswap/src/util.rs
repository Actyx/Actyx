/// Repeat a value n times by cloning it n-1 times
///
/// Avoids 1 clone, which can matter if your average size is ~1 and the clone is expensive
struct RepeatN<T> {
    count: usize,
    value: Option<T>,
}

impl<T> RepeatN<T> {
    pub fn new(value: T, count: usize) -> Self {
        Self {
            value: Some(value),
            count,
        }
    }
}

impl<T: Clone> Iterator for RepeatN<T> {
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        // we know exactly how many items are remaining...
        (self.count, Some(self.count))
    }

    fn next(&mut self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            self.count -= 1;
            if self.count > 0 {
                // clone for all but the last item
                self.value.as_ref().cloned()
            } else {
                // just give it away for the last item - we are not going to need it anymore
                self.value.take()
            }
        }
    }
}

/// Zip the given vector elements with the value and n-1 clones of it
pub fn zip_with_clones<A, B: Clone>(a: Vec<A>, b: B) -> impl Iterator<Item = (A, B)> {
    let bs = RepeatN::new(b, a.len());
    a.into_iter().zip(bs)
}

#[cfg(test)]
#[allow(clippy::mutex_atomic)]
mod tests {
    use super::{zip_with_clones, RepeatN};
    use std::sync::{Arc, Mutex};

    struct CloneCounter(Arc<Mutex<usize>>);

    impl Clone for CloneCounter {
        fn clone(&self) -> Self {
            let mut value = self.0.lock().unwrap();
            *value += 1;
            CloneCounter(self.0.clone())
        }
    }

    #[test]
    fn clone_count_1() {
        let counter = Arc::new(Mutex::new(0));
        let value = CloneCounter(counter.clone());
        let _result = RepeatN::new(value, 10).collect::<Vec<_>>();
        assert_eq!(*counter.lock().unwrap(), 9);
    }

    #[test]
    fn clone_count_2() {
        let counter = Arc::new(Mutex::new(0));
        let value = CloneCounter(counter.clone());
        let elems = (0..10).collect::<Vec<_>>();
        let _result = zip_with_clones(elems, value).collect::<Vec<_>>();
        assert_eq!(*counter.lock().unwrap(), 9);
    }
}
