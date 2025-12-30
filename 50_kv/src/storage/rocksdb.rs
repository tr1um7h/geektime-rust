use rust_rocksdb::{DB, Options};
use std::{convert::TryInto, path::Path, str};

use crate::{KvError, Kvpair, Storage, StorageIter, Value};

#[derive(Debug)]
pub struct RocksDb(DB);

impl RocksDb {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(DB::open_default(path).unwrap())
    }

    // 在 rocksdb 里，因为它可以 scan_prefix，我们用 prefix
    // 来模拟一个 table。当然，还可以用其它方案。
    fn get_full_key(table: &str, key: &str) -> String {
        format!("{}:{}", table, key)
    }

    // 遍历 table 的 key 时，我们直接把 prefix: 当成 table
    fn get_table_prefix(table: &str) -> String {
        format!("{}:", table)
    }
}

impl Storage for RocksDb {
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let name = RocksDb::get_full_key(table, key);
        let result = self.0.get(name.as_bytes())?.map(|v| v.try_into());
        result.transpose()
    }

    fn set(&self, table: &str, key: &str, value: Value) -> Result<Option<Value>, KvError> {
        let name = RocksDb::get_full_key(table, &key);
        let data: Vec<u8> = value.try_into()?;

        let result: Option<Result<Value, KvError>> =
            self.0.get(name.clone())?.map(|v| v.try_into());

        self.0.put(name.clone(), data)?;

        result.transpose()
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError> {
        let name = RocksDb::get_full_key(table, &key);

        Ok(self.0.key_may_exist(name))
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let name = RocksDb::get_full_key(table, &key);

        self.0.delete(name)?;
        Ok(Some(Value::default()))
    }

    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError> {
        let prefix = RocksDb::get_table_prefix(table);
        let result = self.0.prefix_iterator(prefix).map(|v| v.into()).collect();

        Ok(result)
    }

    fn get_iter<'a>(
        &'a self,
        table: &str,
    ) -> Result<Box<dyn Iterator<Item = Kvpair> + 'a>, KvError> {
        let prefix = RocksDb::get_table_prefix(table);
        let iter = StorageIter::new(self.0.prefix_iterator(prefix));
        Ok(Box::new(iter))
    }
}

impl From<Result<(Box<[u8]>, Box<[u8]>), rust_rocksdb::Error>> for Kvpair {
    fn from(data: Result<(Box<[u8]>, Box<[u8]>), rust_rocksdb::Error>) -> Self {
        println!("data: {:?}", data);
        match data {
            Ok((k, v)) => {
                let res = Kvpair::new(ivec_to_key(&k), ivec_to_value(&v).into());
                println!("res: {:?}", res);
                res
            }
            _ => Kvpair::default(),
        }
    }
}

fn ivec_to_key(ivec: &Box<[u8]>) -> &str {
    let s = str::from_utf8(&ivec).unwrap();
    let mut iter = s.split(":");
    iter.next();
    iter.next().unwrap()
}

fn ivec_to_value(ivec: &Box<[u8]>) -> &str {
    println!("v: {:?}", &ivec[2..]);
    let s = str::from_utf8(&ivec[2..]).unwrap();
    s
}
