mod config;

use slog::{info, Logger};
use std::{io, net::TcpStream, time::Duration};

pub use config::Configuration;

pub struct KvsClient {
    log: Logger,
    config: Configuration,
}

impl KvsClient {
    pub fn new(log: Logger, config: Configuration) -> Self {
        Self { log, config }
    }

    pub fn connect(&self) -> Result<(), io::Error> {
        info!(self.log, "connecting");
        let stream = TcpStream::connect_timeout(&self.config.addr, Duration::from_millis(100))?;
        info!(self.log, "connected");
        Ok(())
    }
}
