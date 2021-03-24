use crate::storage::StorageWrapper;
use std::ops::RangeInclusive;
use util::formats::logs::LogEvent;

pub(crate) struct StorageQuery<'a> {
    range: RangeInclusive<i64>,
    curr: i64,
    chunk_size: i64,
    storage: &'a StorageWrapper,
}
impl<'a> StorageQuery<'a> {
    pub fn new(range: std::ops::RangeInclusive<i64>, storage: &'a StorageWrapper, chunk_size: i64) -> StorageQuery<'a> {
        let curr = *range.start();
        Self {
            range,
            curr,
            chunk_size,
            storage,
        }
    }
}
impl Iterator for StorageQuery<'_> {
    type Item = Vec<LogEvent>;
    fn next(&mut self) -> Option<Self::Item> {
        // get new chunk
        let from = self.curr;
        let to = (self.curr + self.chunk_size - 1).min(*self.range.end());
        if self.curr > to {
            None
        } else {
            let new_chunk = self
                .storage
                .spawn_mut(move |s| s.get_logs_by_seq(from, Some(to)))
                .unwrap()
                .unwrap();
            self.curr = to + 1;
            new_chunk.into()
        }
    }
}

#[cfg(test)]
mod test {
    use super::StorageQuery;
    use crate::storage::{test::insert_dummy_logs, Storage, StorageWrapper};

    fn mk_storage() -> StorageWrapper {
        let mut storage = Storage::in_memory();
        insert_dummy_logs(&mut storage, 100);
        StorageWrapper::new(move || storage, "")
    }

    #[test]
    fn smoke() {
        let storage = mk_storage();
        let chunk_size = 3;

        let query = StorageQuery::new(6..=42, &storage, chunk_size);

        let mut i = 0;
        for (iteration, iter) in query.enumerate() {
            // 12 full batches, last batch with 1 element
            assert_eq!(iter.len(), if iteration == 12 { 1 } else { chunk_size as usize });
            i += 1;
        }
        assert_eq!(i, 13);
    }
    #[test]
    fn inclusive_range() {
        let storage = mk_storage();
        let chunk_size = 3;

        let query = StorageQuery::new(41..=42, &storage, chunk_size);

        let x = query.flatten().collect::<Vec<_>>();
        assert_eq!(x.len(), 2);

        let query = StorageQuery::new(42..=42, &storage, chunk_size);

        let x = query.flatten().collect::<Vec<_>>();
        assert_eq!(x.len(), 1);
    }
}
