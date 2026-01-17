use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use tokio::fs;

use crate::db::Db;

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub entries: HashMap<String, Vec<u8>>,
}

pub async fn save(db: &Db, filename: &str) -> io::Result<()> {
    let mut entries = HashMap::new();
    
    for entry in db.entries.iter() {
        let key = entry.key().clone();
        let value = entry.value().to_vec();
        entries.insert(key, value);
    }

    let snapshot = Snapshot { entries };
    let serialized = bincode::serialize(&snapshot)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    fs::write(filename, serialized).await?;

    Ok(())
}

pub async fn load(filename: &str) -> io::Result<HashMap<String, Bytes>> {
    let data = fs::read(filename).await?;

    let snapshot: Snapshot = bincode::deserialize(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut entries = HashMap::new();
    for (key, value) in snapshot.entries {
        entries.insert(key, Bytes::from(value));
    }

    Ok(entries)
}
