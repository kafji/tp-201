use slog::{info, o, Drain, Logger};
use std::panic;

pub struct KvsServer {
    log: slog::Logger,
}

impl KvsServer {
    pub fn new(log: Logger) -> Self {
        Self { log }
    }
}
