#[macro_use]
extern crate slog;

mod connection;

pub mod server;
use server::key_value_store::KeyValueStore;
pub mod client;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// This is defined as a convenience.
pub type Result<T> = std::result::Result<T, Error>;
