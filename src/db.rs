use bytes::Bytes;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Db {
    entries: Arc<DashMap<String, Bytes>>,
}

impl Db {
    pub fn new() -> Db {
        Db {
            entries: Arc::new(DashMap::new()),
        }
    }

    pub fn set(&self, key: String, value: Bytes) {
        self.entries.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        self.entries.get(key).map(|entry| entry.value().clone())
    }
}
