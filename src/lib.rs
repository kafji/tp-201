use std::collections::HashMap;

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

    /// Set value for a key.
    ///
    /// If the key already exists, it will replace the value.
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    ///
    /// store.set("hello".to_owned(), "world".to_owned());
    /// assert_eq!(store.get("hello".to_owned()).unwrap(), "world");
    ///
    /// store.set("hello".to_owned(), "darkness my old friend".to_owned());
    /// assert_eq!(store.get("hello".to_owned()).unwrap(), "darkness my old friend");
    /// ```
    pub fn set(&mut self, key: String, value: String) {
        self.storage.insert(key, value);
    }

    /// Get value of a key.
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.set("hello".to_owned(), "world".to_owned());
    ///
    /// let value = store.get("hello".to_owned()).unwrap();
    /// assert_eq!(value, "world");
    /// ```
    pub fn get(&self, key: String) -> Option<String> {
        match self.storage.get(&key) {
            Some(value) => Some(value.clone()),
            None => None,
        }
    }

    /// Remove entry.
    ///
    /// Will success even when the key doesn't exists.
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.set("hello".to_owned(), "world".to_owned());
    ///
    /// store.remove("hello".to_owned());
    ///
    /// assert_eq!(store.get("hello".to_owned()), None);
    ///
    /// store.remove("hello".to_owned()); // a-ok
    /// ```
    pub fn remove(&mut self, key: String) {
        self.storage.remove(&key);
    }
}
