use crate::protocol::{Request, Response, Serialization};
use slog::{info, Logger};
use std::io::Write;
use std::{io, net::SocketAddr, net::TcpStream, time::Duration};

pub struct KvsClient {
    log: Logger,
    stream: TcpStream,
}

impl KvsClient {
    pub fn new(
        log: impl Into<Option<Logger>>,
        address: impl Into<SocketAddr>,
    ) -> Result<Self, io::Error> {
        let log = log
            .into()
            .unwrap_or_else(|| slog::Logger::root(slog::Discard, slog::o!()));
        info!(log, "connecting");
        let address = address.into();
        let stream = TcpStream::connect_timeout(&address, Duration::from_millis(100))?;
        info!(log, "connected");
        let client = Self { log, stream };
        Ok(client)
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let request = Request::Get { key };
        request.to_writer(&mut self.stream)?;

        let response = Response::from_reader(&mut self.stream)?;
        match response {
            Some(Response::Success(v)) => Ok(v),
            Some(Response::Failure(m)) => Err(m)?,
            None => Err("no response")?,
        }
    }

    pub fn set(&mut self, key: String, value: String) -> Result<(), Box<dyn std::error::Error>> {
        let request = Request::Set { key, value };
        request.to_writer(&mut self.stream)?;

        let response = Response::from_reader(&mut self.stream)?;
        match response {
            Some(Response::Success(None)) => Ok(()),
            Some(Response::Failure(m)) => Err(m)?,
            None => Err("no response")?,
            _ => Err("unexpected response")?,
        }
    }

    pub fn rm(&mut self, key: String) -> Result<(), Box<dyn std::error::Error>> {
        let request = Request::Rm { key };
        request.to_writer(&mut self.stream)?;

        let response = Response::from_reader(&mut self.stream)?;
        match response {
            Some(Response::Success(None)) => Ok(()),
            Some(Response::Failure(m)) => Err(m)?,
            None => Err("no response")?,
            _ => Err("unexpected response")?,
        }
    }
}
