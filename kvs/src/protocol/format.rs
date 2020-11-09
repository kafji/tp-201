use serde::{de::DeserializeOwned, Serialize};
use std::{convert::From, io};
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(Box<dyn std::error::Error + Send + Sync>);

impl From<bincode::Error> for Error {
    fn from(value: bincode::Error) -> Self {
        Self(Box::new(value))
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self(Box::new(value))
    }
}

pub trait Serialization<'a>: DeserializeOwned + Serialize {
    fn to_writer(&self, writer: &mut impl io::Write) -> Result<(), Error> {
        bincode::serialize_into(writer, &self)?;
        Ok(())
    }

    fn from_reader(reader: &mut impl io::Read) -> Result<Option<Self>, Error> {
        let value = match bincode::deserialize_from::<_, Self>(reader) {
            Ok(v) => Some(v),
            Err(err) => match *err {
                bincode::ErrorKind::Io(err) => match err.kind() {
                    io::ErrorKind::UnexpectedEof => None,
                    _ => return Err(err)?,
                },
                _ => return Err(err)?,
            },
        };
        Ok(value)
    }
}

impl<T> Serialization<'_> for T where T: DeserializeOwned + Serialize {}
