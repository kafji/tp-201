#![feature(seek_convenience)]
#![feature(with_options)]

mod command;
mod serialization;

use command::*;
use serialization::Serializable;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, SeekFrom},
    io::{BufRead, BufWriter, Seek},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Serialization(#[from] serialization::Error),

    #[error("Key does not exists")]
    KeyNotFound,

    #[error("Index is desynced/corrupted")]
    IndexDesynced,
}

pub type Result<T> = std::result::Result<T, Error>;

/*
# Terminology

  - command - A request or the representation of a request made to the database. These are issued on the command line or over the network. They have an in-memory representation, a textual representation, and a machine-readable serialized representation.

  - log - An on-disk sequence of commands, in the order originally received and executed. Our database's on-disk format is almost entirely made up of logs. It will be simple, but also surprisingly efficient.

  - log pointer - A file offset into the log. Sometimes we'll just call this a "file offset".

  - log compaction - As writes are issued to the database they sometimes invalidate old log entries. For example, writing key/value a = 0 then writing a = 1, makes the first log entry for "a" useless. Compaction — in our database at least — is the process of reducing the size of the database by remove stale commands from the log.

  - in-memory index (or index) - A map of keys to log pointers. When a read request is issued, the in-memory index is searched for the appropriate log pointer, and when it is found the value is retrieved from the on-disk log. In our key/value store, like in bitcask, the index for the entire database is stored in memory.

  - index file - The on-disk representation of the in-memory index. Without this the log would need to be completely replayed to restore the state of the in-memory index each time the database is started.
*/

pub struct KvStore {
    log: File,
    index: HashMap<String, u64>,
}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = {
            let mut p = path.into();
            p.push("log");
            p
        };
        let log = File::with_options()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        let mut reader = BufReader::new(&log);
        let index = Self::build_index(&mut reader)?;
        let store = KvStore { log, index };
        Ok(store)
    }

    /// Set value for a key.
    ///
    /// If the key already exists, it will replace the value.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::Set(Set {
            key: key.clone(),
            value,
        });
        let mut writer = BufWriter::new(&self.log);

        // Move pointer/offset to the end of file
        let offset = writer.seek(io::SeekFrom::End(0))?;

        // Append log
        command.serialize_into(&mut writer)?;

        // Update index
        self.index.insert(key, offset);

        Ok(())
    }

    /// Get value of a key.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        // Lookup index
        let offset = match self.index.get(&key) {
            Some(x) => *x,
            None => return Ok(None),
        };

        let mut reader = BufReader::new(&self.log);

        // Move pointer/cursor to offset
        reader.seek(SeekFrom::Start(offset))?;

        let command = Command::deserialize_from(&mut reader)?;
        match command {
            Command::Set(set) => Ok(Some(set.value)),
            _ => Err(Error::IndexDesynced),
        }
    }

    /// Remove entry.
    pub fn remove(&mut self, key: String) -> Result<()> {
        // Early exit when key does not exists
        if !self.index.contains_key(&key) {
            return Err(Error::KeyNotFound);
        }

        let command = Command::Rm(Rm { key: key.clone() });
        let mut writer = BufWriter::new(&self.log);

        // Move pointer/offset to the end of file
        writer.seek(io::SeekFrom::End(0))?;

        // Append log
        command.serialize_into(&mut writer)?;

        // Update index
        self.index.remove(&key);

        Ok(())
    }

    /// List all entries.
    ///
    /// Only used for testing & debugging.
    pub fn list(&self) -> Result<Vec<(String, String)>> {
        let mut entries = Vec::with_capacity(self.index.len());
        for (key, _) in self.index.iter() {
            let result = self.get(key.to_owned())?;
            if let Some(value) = result {
                entries.push((key.to_owned(), value));
            }
        }
        Ok(entries)
    }

    fn build_index<T>(reader: &mut T) -> Result<HashMap<String, u64>>
    where
        T: BufRead + Seek,
    {
        let mut index = HashMap::new();
        loop {
            let offset = reader.stream_position()?;

            // Check EOF
            let buf = reader.fill_buf()?;
            if buf.is_empty() {
                break;
            }

            let command: Command = Command::deserialize_from(&mut *reader)?;
            match command {
                Command::Set(set) => {
                    index.insert(set.key, offset);
                }
                Command::Rm(rm) => {
                    index.remove(&rm.key);
                }
            };
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_build_index() {
        let commands: Vec<Command> = vec![
            Command::Set(Set {
                key: "key0".to_owned(),
                value: "value0".to_owned(),
            }),
            Command::Set(Set {
                key: "key1".to_owned(),
                value: "value1".to_owned(),
            }),
            Command::Set(Set {
                key: "key2".to_owned(),
                value: "value2".to_owned(),
            }),
            Command::Rm(Rm {
                key: "key2".to_owned(),
            }),
            Command::Set(Set {
                key: "key3".to_owned(),
                value: "value3".to_owned(),
            }),
            Command::Set(Set {
                key: "key3".to_owned(),
                value: "value33".to_owned(),
            }),
        ];
        let mut serialized = Vec::new();
        for cmd in commands.iter() {
            cmd.serialize_into(&mut serialized).unwrap();
        }

        let mut reader = Cursor::new(&serialized);
        let index = KvStore::build_index(&mut reader).unwrap();

        assert_eq!(index.get("key0"), Some(&0));

        let size = {
            let mut buf = Vec::new();
            commands.get(0).unwrap().serialize_into(&mut buf).unwrap();
            buf.len() as u64
        };
        assert_eq!(index.get("key1"), Some(&size));

        assert_eq!(index.get("key2"), None);

        let size = {
            let mut buf = Vec::new();
            for cmd in &commands[0..5] {
                cmd.serialize_into(&mut buf).unwrap();
            }
            buf.len() as u64
        };
        assert_eq!(index.get("key3"), Some(&size));
    }
}
