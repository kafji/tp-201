use serde::{de::DeserializeOwned, Serialize};
use std::convert::From;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(Box<dyn std::error::Error>);

impl From<bincode::Error> for Error {
    fn from(cause: bincode::Error) -> Self {
        Self(Box::new(cause))
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Wrapper for serialization and deserialization so we can change the format
/// if we ever need to.
///
/// We opt to use Bincode for our serialization format because Bincode manage
/// its serialized size by itself so we doesn't need to have a token as marker
/// to separate log entries.
pub trait Serializable<'a>: DeserializeOwned + Serialize {
    /// Serialize command into a writer.
    fn serialize_into<W>(&self, writer: W) -> Result<()>
    where
        W: std::io::Write,
    {
        bincode::serialize_into(writer, &self)?;
        Ok(())
    }

    /// Deserialize command from a reader.
    fn deserialize_from<R>(reader: R) -> Result<Self>
    where
        R: std::io::Read,
    {
        let command: Self = bincode::deserialize_from(reader)?;
        Ok(command)
    }
}
