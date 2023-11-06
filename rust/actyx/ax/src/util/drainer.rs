use futures::executor::LocalPool;
use futures::future::ready;
use futures::stream::{self, Stream, StreamExt};
use futures::task::LocalSpawnExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Drainer<T> {
    acc: Arc<Mutex<Vec<T>>>,
    done: Arc<AtomicBool>,
    pool: LocalPool,
}

impl<T: 'static> Drainer<T> {
    pub fn new<St: Stream<Item = T> + 'static>(stream: St) -> Drainer<T> {
        // create communication variables
        let acc2 = Arc::new(Mutex::new(Vec::<T>::new()));
        let done2 = Arc::new(AtomicBool::new(false));

        // cloning for the closures
        let acc = acc2.clone();
        let done = done2.clone();

        // spawn the stream’s task
        let pool = LocalPool::new();
        pool.spawner()
            .spawn_local(
                stream
                    .filter_map(move |x| {
                        let mut acc_guard = acc2.lock().unwrap();
                        acc_guard.push(x);
                        ready(None)
                    })
                    .chain(stream::iter(vec![()]).map(move |_| {
                        done2.store(true, Ordering::Release);
                    }))
                    .for_each(|_| ready(())),
            )
            .expect("cannot spawn stream");

        Drainer { acc, done, pool }
    }
}

impl<T: Clone> Iterator for Drainer<T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // first let the pool run until Pending
        self.pool.run_until_stalled();

        // now check what we find in the communication variables
        let mut acc_guard = self.acc.lock().unwrap();
        if !acc_guard.is_empty() {
            let ret = (*acc_guard).clone();
            acc_guard.clear();
            return Some(ret);
        }
        if self.done.load(Ordering::Acquire) {
            return None;
        }

        // if we’re not done but found no elements, then there were no elements
        Some(Vec::new())
    }
}
