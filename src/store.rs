use std::ops::{Deref, DerefMut};
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Entry {
    value: String,
    expires_at: Option<std::time::Instant>,
}

impl Entry {
    pub fn new(value: String, ttl: Option<u64>, now: Instant) -> Self {
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
    pub fn new() -> Self {
        let state = RwLock::new(HashMap::new());
        Store { state }
    }

    pub fn get(&self, key: &str, now: Instant) -> Option<String> {
        let guard = self.state.read().unwrap();
        if let Some(entry) = guard.get(key) {
            if entry.is_expired(now) {
                drop(guard);
                self.state.write().unwrap().remove(key);
                None
            } else {
                Some(entry.get_value())
            }
        } else {
            None
        }
    }

    pub fn insert(&self, key: String, value: String, ttl: Option<u64>) {
        let entry = Entry::new(value, ttl, Instant::now());
        self.state.write().unwrap().insert(key, entry);
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

#[cfg(test)]
mod test {
    use core::time;
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn test_store() {
        let store = Store::new();

        // Insert a KV
        store.insert("key".to_string(), "value".to_string(), Some(200 as u64));

        // Test if its accessible within expiry period
        assert_eq!(store.get("key", Instant::now()), Some("value".to_string()));

        // Test if key is not accessible once the key expires
        thread::sleep(time::Duration::from_millis(300));
        assert_eq!(store.get("key", Instant::now()), None);
    }

    #[test]
    fn concurrent_access_test() {
        let store = Arc::new(Store::new());

        store.insert("key1".to_string(), "value1".to_string(), None);

        let reader_store = store.clone();
        let writer_store = store.clone();

        let reader_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            let now = std::time::Instant::now();
            let result = reader_store.get("key1", now);
            assert_eq!(result, Some("value1".to_string()));
        });

        let modifier_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            writer_store.insert("key1".to_string(), "value2".to_string(), None);

            let now = std::time::Instant::now();
            let result = writer_store.get("key1", now);
            assert_eq!(result, Some("value2".to_string()));
        });

        reader_thread.join().unwrap();
        modifier_thread.join().unwrap();
    }
}
