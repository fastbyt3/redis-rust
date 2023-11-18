use std::ops::{Deref, DerefMut};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Entry {
    value: String,
    expires_at: Option<std::time::Instant>,
}

impl Entry {
    pub fn new(
        value: String,
        ttl: Option<u64>,
        expires_at_ts: Option<Instant>,
        now: Instant,
    ) -> Self {
        let expires_at = ttl.map(|expires_in| {
            now.checked_add(Duration::from_millis(expires_in))
                .expect("Error during adding ttl to now instant")
        });
        Entry { value, expires_at }
    }

    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires_at
            .map(|expires_at| expires_at <= now)
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct Store {
    state: RwLock<HashMap<String, Entry>>,
}

impl Store {
    pub fn new(rdb_kv_data: Option<HashMap<String, Entry>>) -> Self {
        let hm = rdb_kv_data.map_or_else(|| HashMap::new(), |rdb_hm| rdb_hm.clone());
        Self {
            state: RwLock::new(hm),
        }
    }

    pub async fn get(&self, key: &str, now: Instant) -> Option<String> {
        let guard = self.state.read().await;
        if let Some(entry) = guard.get(key) {
            if entry.is_expired(now) {
                drop(guard);
                self.state.write().await.remove(key);
                None
            } else {
                Some(entry.get_value())
            }
        } else {
            None
        }
    }

    pub async fn insert(
        &self,
        key: String,
        value: String,
        ttl: Option<u64>,
        expires_at_ts: Option<Instant>,
    ) {
        let entry = Entry::new(value, ttl, expires_at_ts, Instant::now());
        self.state.write().await.insert(key, entry);
    }
}

impl Deref for Store {
    type Target = RwLock<HashMap<String, Entry>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for Store {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

// #[cfg(test)]
// mod test {
//     use core::time;
//     use std::{future::Future, thread};

//     use tokio::runtime::Runtime;

//     use super::*;

//     fn run_async_tests<F: Future>(f: F) {
//         let runtime = Runtime::new().unwrap();
//         runtime.block_on(f);
//     }

//     #[test]
//     fn test_store() {
//         run_async_tests(async {
//             let store = Store::new();

//             // Insert a KV
//             store
//                 .insert("key".to_string(), "value".to_string(), Some(200 as u64))
//                 .await;

//             // Test if its accessible within expiry period
//             assert_eq!(
//                 store.get("key", Instant::now()).await,
//                 Some("value".to_string())
//             );

//             // Test if key is not accessible once the key expires
//             thread::sleep(time::Duration::from_millis(300));
//             assert_eq!(store.get("key", Instant::now()).await, None);
//         })
//     }
// }
