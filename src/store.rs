use std::collections::HashMap;

#[derive(Debug)]
pub struct Store {
    state: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        let state = HashMap::new();
        Store { state }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.state.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.state.get(key).map(|s| s.to_string())
    }
}
