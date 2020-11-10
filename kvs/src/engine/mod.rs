use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvsEngineError {
    #[error("entry with key `{key}` not found")]
    EntryNotFound { key: String },

    #[error("{0}")]
    Other(Box<dyn std::error::Error>),
}

pub trait KvsEngine {
    /// Set the value of a string key to a string. Return an error if the value is not written
    /// successfully.
    fn set(&mut self, key: String, value: String) -> Result<(), KvsEngineError>;

    /// Get the string value of a string key. If the key does not exists, return `None`. Return an
    /// error if the value is not read successfully.
    fn get(&mut self, key: &str) -> Result<Option<String>, KvsEngineError>;

    /// Remove a given string key. Return an error if the key does not exit or value is not read
    /// successfully.
    fn remove(&mut self, key: &str) -> Result<(), KvsEngineError>;
}

impl<T> KvsEngine for Box<T>
where
    T: KvsEngine + ?Sized,
{
    fn set(&mut self, key: String, value: String) -> Result<(), KvsEngineError> {
        (self as &mut T).set(key, value)
    }

    fn get(&mut self, key: &str) -> Result<Option<String>, KvsEngineError> {
        (self as &mut T).get(key)
    }

    fn remove(&mut self, key: &str) -> Result<(), KvsEngineError> {
        (self as &mut T).remove(key)
    }
}
