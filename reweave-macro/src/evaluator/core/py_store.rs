use super::*;

impl Evaluator {
    pub fn pystore_set(&mut self, key: String, value: String) {
        self.py_store.insert(key, value);
    }

    pub fn pystore_get(&self, key: &str) -> String {
        self.py_store.get(key).cloned().unwrap_or_default()
    }
}
