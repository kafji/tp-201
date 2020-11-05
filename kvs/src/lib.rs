mod command;
mod compaction;
mod index;
mod serialization;
mod server;

use command::*;
use index::build_index;
use serialization::Serializable;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufReader, SeekFrom},
    io::{BufWriter, Seek},
    path::PathBuf,
};
use thiserror::Error;

pub use server::KvsServer;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Serialization(#[from] serialization::Error),

    #[error("Path is not a directory")]
    InvalidPath,

    #[error("Key does not exists")]
    KeyNotFound,

    #[error("Index is desynced/corrupted")]
    IndexDesynced,

    #[error("TODO")]
    TODO,
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

#[derive(Debug)]
pub struct KvStore {
    directory: PathBuf,
    log_file: File,
    index: HashMap<String, u64>,
}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let directory: PathBuf = path.into();

        if !directory.is_dir() {
            return Err(Error::InvalidPath);
        }

        let log_file_path: PathBuf = {
            let mut p = directory.clone();
            p.push("log.kvs");
            p
        };
        let log_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&log_file_path)?;

        let mut reader = BufReader::new(&log_file);
        let index = build_index(&mut reader)?;
        let store = KvStore {
            directory,
            log_file,
            index,
        };
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

        // TODO(KFJ):
        //   Q: Should I use buffer here?
        //   Q: What happen if I write it directly? Is there any performance impact?
        //   Q: Or would it be better if I keep and use a single buffer through-out the session?
        //   Next topic is about benchmarking. ~I should~Hopefully can answer it by then.
        let mut writer = BufWriter::new(&self.log_file);
        // Move pointer/offset to the end of file
        let offset = writer.seek(SeekFrom::End(0))?;
        // Append log
        command.serialize_into(writer)?;
        self.log_file.sync_data()?;

        // Update index
        self.index.insert(key, offset);

        let size = self.log_file.seek(SeekFrom::End(0))?;
        let one_mb = 1024 * 1024;
        if size > one_mb {
            compaction::compact(&mut self.log_file, &self.index)?;
            let mut reader = BufReader::new(&self.log_file);
            self.index = build_index(&mut reader)?;
        }

        Ok(())
    }

    /// Get value of a key.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        // Lookup index
        let offset = match self.index.get(&key) {
            Some(x) => *x,
            None => return Ok(None),
        };

        let mut reader = BufReader::new(&self.log_file);

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

        let mut writer = BufWriter::new(&self.log_file);
        // Move pointer/offset to the end of file
        writer.seek(SeekFrom::End(0))?;
        // Append log
        command.serialize_into(&mut writer)?;
        self.log_file.sync_data()?;

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
}

pub struct KvsClient;

pub trait KvsEngine {
    /// Set the value of a string key to a string. Return an error if the value is not written
    /// successfully.
    fn set(&mut self, key: String, value: String) -> Result<()>;

    /// Get the string value of a string key. If the key does not exists, return `None`. Return an
    /// error if the value is not read successfully.
    fn get(&mut self, key: String) -> Result<Option<String>>;

    /// Remove a given string key. Return an error if the key does not exit or value is not read
    /// successfully.
    fn remove(&mut self, key: String) -> Result<()>;
}

pub struct SledKvsEngine;
