mod memory;
mod rocksdb;
mod sleddb;

use crate::{KvError, Kvpair, Value};
pub use memory::MemTable;
pub use rocksdb::*;
pub use sleddb::SledDb;

pub trait Storage {
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    fn set(&self, table: &str, key: &str, value: Value) -> Result<Option<Value>, KvError>;

    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError>;

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError>;

    fn get_iter<'a>(
        &'a self,
        table: &str,
    ) -> Result<Box<dyn Iterator<Item = Kvpair> + 'a>, KvError>;
}

pub struct StorageIter<T> {
    data: T,
}

impl<T> StorageIter<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> Iterator for StorageIter<T>
where
    T: Iterator,
    T::Item: Into<Kvpair>,
{
    type Item = Kvpair;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.next().map(|v| v.into())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn memtable_basic_interface_should_work() {
        let store = MemTable::new();
        test_basic_interface(store);
    }

    #[test]
    fn memtable_get_all_should_work() {
        let store = MemTable::new();
        test_get_all(store);
    }

    #[test]
    fn memtable_iter_should_work() {
        let store = MemTable::new();
        test_get_iter(store);
    }

    fn test_basic_interface(store: impl Storage) {
        let v = store.set("t1", "hello", "world".into());
        assert!(v.unwrap().is_none());
        let v1 = store.set("t1", "hello", "world1".into());
        assert_eq!(v1, Ok(Some("world".into())));
    }

    fn test_get_all(store: impl Storage) {
        store.set("t2", "k1".into(), "v1".into()).unwrap();
        store.set("t2", "k2".into(), "v2".into()).unwrap();
        let mut data = store.get_all("t2").unwrap();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            data,
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into())
            ]
        )
    }

    fn test_get_iter(store: impl Storage) {
        store.set("t1", "k1".into(), "v1".into()).unwrap();
        store.set("t1", "k2".into(), "v2".into()).unwrap();

        let mut data: Vec<_> = store.get_iter("t1").unwrap().collect();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            data,
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into())
            ]
        );
    }

    #[test]
    fn sleddb_basic_interface_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_basic_interface(store);
    }

    #[test]
    fn sleddb_get_all_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_get_all(store);
    }

    #[test]
    fn sleddb_iter_should_work() {
        let dir = tempdir().unwrap();
        let store = SledDb::new(dir);
        test_get_iter(store);
    }

    #[test]
    fn rocksdb_basic_interface_should_work() {
        let dir = tempdir().unwrap();
        let store = RocksDb::new(dir);
        test_basic_interface(store);
    }

    #[test]
    fn rocksdb_get_all_should_work() {
        let dir = tempdir().unwrap();
        let store = RocksDb::new(dir);
        test_get_all(store);
    }

    #[test]
    fn rocksdb_iter_should_work() {
        let dir = tempdir().unwrap();
        let store = RocksDb::new(dir);
        test_get_iter(store);
    }
}
