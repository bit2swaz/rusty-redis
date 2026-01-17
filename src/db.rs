use bytes::Bytes;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::debug;

#[derive(Clone)]
pub struct Db {
    pub entries: Arc<DashMap<String, Bytes>>,
    expirations: Arc<DashMap<String, Instant>>,
    pub_sub: Arc<DashMap<String, broadcast::Sender<Bytes>>>,
}

impl Db {
    pub fn new() -> Db {
        let db = Db {
            entries: Arc::new(DashMap::new()),
            expirations: Arc::new(DashMap::new()),
            pub_sub: Arc::new(DashMap::new()),
        };
        db.start_eviction_task();
        db
    }

    pub fn set(&self, key: String, value: Bytes, duration: Option<Duration>) {
        self.entries.insert(key.clone(), value);
        
        if let Some(dur) = duration {
            let expiry = Instant::now() + dur;
            self.expirations.insert(key, expiry);
        } else {
            self.expirations.remove(&key);
        }
    }

    pub fn bulk_insert(&self, entries: std::collections::HashMap<String, Bytes>) {
        for (key, value) in entries {
            self.entries.insert(key, value);
        }
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        if let Some(expiry_entry) = self.expirations.get(key) {
            if Instant::now() > *expiry_entry.value() {
                drop(expiry_entry);
                self.entries.remove(key);
                self.expirations.remove(key);
                return None;
            }
        }
        
        self.entries.get(key).map(|entry| entry.value().clone())
    }

    fn start_eviction_task(&self) {
        let entries = Arc::clone(&self.entries);
        let expirations = Arc::clone(&self.expirations);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                let mut evicted = 0;

                let keys_to_check: Vec<String> = expirations
                    .iter()
                    .take(20)
                    .map(|entry| entry.key().clone())
                    .collect();

                for key in keys_to_check {
                    if let Some(expiry_entry) = expirations.get(&key) {
                        if now > *expiry_entry.value() {
                            drop(expiry_entry);
                            entries.remove(&key);
                            expirations.remove(&key);
                            evicted += 1;
                        }
                    }
                }

                if evicted > 0 {
                    debug!("evicted {} expired keys", evicted);
                }
            }
        });
    }

    pub fn subscribe(&self, channel: String) -> broadcast::Receiver<Bytes> {
        self.pub_sub
            .entry(channel)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(32);
                tx
            })
            .value()
            .subscribe()
    }

    pub fn publish(&self, channel: String, msg: Bytes) -> usize {
        if let Some(tx) = self.pub_sub.get(&channel) {
            tx.send(msg).unwrap_or(0)
        } else {
            0
        }
    }
}
