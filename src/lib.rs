use std::{collections::HashMap, path::PathBuf};

pub type Result<T> = std::result::Result<T, ()>;

/// In memory key-value store.
pub struct KvStore {
    storage: HashMap<String, String>,
}

impl KvStore {
    /// Create a new KvStore.
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let store = KvStore::new();
    /// ```
    pub fn new() -> Self {
        return KvStore {
            storage: HashMap::new(),
        };
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        todo!()
    }

    /// Set value for a key.
    ///
    /// If the key already exists, it will replace the value.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.storage.insert(key, value);
        Ok(())
    }

    /// Get value of a key.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        let result = match self.storage.get(&key) {
            Some(value) => Some(value.clone()),
            None => None,
        };
        Ok(result)
    }

    /// Remove entry.
    ///
    /// Will success even when the key doesn't exists.
    pub fn remove(&mut self, key: String) -> Result<()> {
        self.storage.remove(&key);
        Ok(())
    }
}
