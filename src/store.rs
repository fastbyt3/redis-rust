use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Entry {
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
    state: HashMap<String, Entry>,
}

impl Store {
    pub fn new() -> Self {
        let state = HashMap::new();
        Store { state }
    }

    pub fn get(&mut self, key: &str, now: Instant) -> Option<String> {
        if let Some(entry) = self.state.get(key) {
            if entry.is_expired(now) {
                self.state.remove(key);
                None
            } else {
                Some(entry.get_value())
            }
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, value: String, ttl: Option<u64>) {
        let entry = Entry::new(value, ttl, Instant::now());
        self.state.insert(key, entry);
    }

    pub fn delete(&mut self, key: &str) {
        self.state.remove(key);
    }
}

#[cfg(test)]
mod test {
    use core::time;
    use std::thread;

    use super::*;

    #[test]
    fn test_store() {
        let mut store = Store::new();

        // Insert a KV
        store.insert("key".to_string(), "value".to_string(), Some(200 as u64));

        // Test if its accessible within expiry period
        assert_eq!(store.get("key", Instant::now()), Some("value".to_string()));

        // Test if key is not accessible once the key expires
        thread::sleep(time::Duration::from_millis(300));
        assert_eq!(store.get("key", Instant::now()), None);
    }
}
