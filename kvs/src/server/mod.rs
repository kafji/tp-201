mod config;

use slog::{info, o, Drain, Logger};
use std::{fmt, net, panic, str};
use thiserror::Error;

pub use config::{Configuration, Engine};

#[derive(Error, Debug)]
pub enum Error {}

pub struct KvsServer {
    log: slog::Logger,
    config: Configuration,
}

impl KvsServer {
    pub fn new(log: Logger, config: Configuration) -> Self {
        Self { log, config }
    }

    pub fn listen(&mut self) -> Result<(), std::io::Error> {
        let addr = self.config.addr;
        let listener = net::TcpListener::bind(addr)?;

        info!(self.log, "waiting for incoming connection");

        loop {
            let (stream, peer) = listener.accept()?;
            let log = self.log.new(o! { "peer" => peer });

            info!(log, "connected");

            // break;
        }

        Ok(())
    }
}
