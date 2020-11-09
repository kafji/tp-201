mod engine;
mod protocol;
mod serialization;

pub mod app;
pub mod client;
pub mod server;
pub mod store;

pub use client::KvsClient;
pub use engine::{KvsEngine, KvsEngineError};
pub use server::KvsServer;
pub use store::KvStore;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_ADDR: &str = "127.0.0.1:4000";
pub const DEFAULT_ENGINE: &str = "kvs";

pub struct SledKvsEngine;

/// Declared because tests depends on this type.
/// Do not use this. Declare module scoped result instead.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
