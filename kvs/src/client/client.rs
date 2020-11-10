use crate::protocol::{Request, Response, Serialization};
use slog::{debug, info, o, Discard, Logger};
use std::{io, net::SocketAddr, net::TcpStream, time::Duration};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("{0}")]
    ErrorResponse(String),
    #[error("failed to write request, caused by {0}")]
    RequestError(Box<dyn std::error::Error>),
    #[error("failed to read response, caused by {0}")]
    ResponseError(Box<dyn std::error::Error>),
    #[error("no response")]
    NoResponse,
    #[error("unexpected response, response was {0}")]
    UnexpectedResponse(Response),
}

pub struct KvsClient {
    log: Logger,
    stream: TcpStream,
}

impl KvsClient {
    pub fn new(
        log: impl Into<Option<Logger>>,
        address: impl Into<SocketAddr>,
    ) -> Result<Self, io::Error> {
        let log = log.into().unwrap_or_else(|| Logger::root(Discard, o!()));
        debug!(log, "connecting");
        let address = address.into();
        let stream = TcpStream::connect_timeout(&address, Duration::from_millis(100))?;
        info!(log, "connected");
        let client = Self { log, stream };
        Ok(client)
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>, ClientError> {
        let log = &self.log;

        let request = Request::Get { key };
        debug!(log, "sending request"; "request" => ?request);
        request
            .to_writer(&mut self.stream)
            .map_err(|x| ClientError::RequestError(Box::new(x)))?;

        let response = Response::from_reader(&mut self.stream)
            .map_err(|x| ClientError::ResponseError(Box::new(x)))?;
        match response {
            Some(Response::Success(v)) => Ok(v),
            Some(Response::Failure(m)) => Err(ClientError::ErrorResponse(m)),
            None => Err(ClientError::NoResponse),
        }
    }

    pub fn set(&mut self, key: String, value: String) -> Result<(), ClientError> {
        let log = &self.log;

        let request = Request::Set { key, value };
        debug!(log, "sending request"; "request" => ?request);
        request
            .to_writer(&mut self.stream)
            .map_err(|x| ClientError::RequestError(Box::new(x)))?;

        let response = Response::from_reader(&mut self.stream)
            .map_err(|x| ClientError::ResponseError(Box::new(x)))?;
        match response {
            Some(Response::Success(None)) => Ok(()),
            Some(Response::Failure(m)) => Err(ClientError::ErrorResponse(m)),
            Some(response) => Err(ClientError::UnexpectedResponse(response)),
            None => Err(ClientError::NoResponse),
        }
    }

    pub fn rm(&mut self, key: String) -> Result<(), ClientError> {
        let log = &self.log;

        let request = Request::Rm { key };
        debug!(log, "sending request"; "request" => ?request);
        request
            .to_writer(&mut self.stream)
            .map_err(|x| ClientError::RequestError(Box::new(x)))?;

        let response = Response::from_reader(&mut self.stream)
            .map_err(|x| ClientError::ResponseError(Box::new(x)))?;
        match response {
            Some(Response::Success(None)) => Ok(()),
            Some(Response::Failure(m)) => Err(ClientError::ErrorResponse(m)),
            Some(response) => Err(ClientError::UnexpectedResponse(response)),
            None => Err(ClientError::NoResponse),
        }
    }
}
