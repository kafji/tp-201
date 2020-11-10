use crate::{KvsEngine, KvsEngineError};

impl From<sled::Error> for KvsEngineError {
    fn from(value: sled::Error) -> Self {
        KvsEngineError::Other(Box::new(value))
    }
}

impl KvsEngine for sled::Db {
    fn set(&mut self, key: String, value: String) -> Result<(), KvsEngineError> {
        self.insert(key.as_bytes(), value.as_bytes())?;
        (self as &sled::Tree).flush()?;
        Ok(())
    }

    fn get(&mut self, key: &str) -> Result<Option<String>, KvsEngineError> {
        let result = (self as &sled::Tree).get(key.as_bytes())?;
        let v = match result {
            Some(v) => Some(String::from_utf8(v.to_vec()).expect("expected value is in utf-8")),
            None => None,
        };
        Ok(v)
    }

    fn remove(&mut self, key: &str) -> Result<(), KvsEngineError> {
        let result: Option<_> = (self as &sled::Tree).remove(key.as_bytes())?;
        (self as &sled::Tree).flush()?;
        match result {
            Some(_) => Ok(()),
            None => Err(KvsEngineError::EntryNotFound {
                key: key.to_owned(),
            }),
        }
    }
}
